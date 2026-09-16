use chrono::Local;
use log::{debug, info, warn};
use rusqlite::{params, Connection, Transaction};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use super::models::SqlDateTime;
use crate::error::DatabaseError;
use crate::project::Project;
use crate::models::{Plugin, PluginKey, Sample};

/// Ableton's name for a reference, recovering it from the identifier when the project
/// file's `<Name>` element was blank.
fn reference_name(plugin: &Plugin) -> String {
    if !plugin.name.trim().is_empty() {
        return plugin.name.clone();
    }
    crate::models::dev_identifier_display_name(&plugin.dev_identifier).unwrap_or_default()
}

/// One project reference, and the plugin we resolved it to.
struct PluginRef {
    dev_identifier: String,
    plugin_key: PluginKey,
    ableton_name: String,
    ableton_format: String,
    resolved_via: &'static str,
}

struct BatchTransaction<'a> {
    tx: Transaction<'a>,
    /// Keyed on the plugin's own identity (ADR-0005), not on Ableton's reference
    /// string: one plugin can be referenced by several `dev_identifier`s.
    unique_plugins: HashMap<PluginKey, Plugin>,
    /// Every class a known VST3 bundle exports, mapped to the bundle that owns it.
    /// A project references whichever processor class the user instantiated, which
    /// need not be the bundle's top-level uid.
    class_index: HashMap<PluginKey, PluginKey>,
    /// Reference rows to write once the plugins they point at exist.
    plugin_refs: Vec<PluginRef>,
    unique_samples: HashMap<String, Sample>, // path -> Sample
    plugin_id_map: HashMap<String, String>,  // old_uuid -> canonical_uuid
    sample_id_map: HashMap<String, String>,  // old_uuid -> canonical_uuid
    stats: BatchStats,
}

impl<'a> BatchTransaction<'a> {
    fn new(conn: &'a mut Connection) -> Result<Self, DatabaseError> {
        Ok(Self {
            tx: conn.transaction()?,
            unique_plugins: HashMap::new(),
            class_index: HashMap::new(),
            plugin_refs: Vec::new(),
            unique_samples: HashMap::new(),
            plugin_id_map: HashMap::new(),
            sample_id_map: HashMap::new(),
            stats: BatchStats::default(),
        })
    }

    fn load_existing_plugins(&mut self) -> Result<(), DatabaseError> {
        debug!("Loading existing plugins from database");
        let mut stmt = self.tx.prepare(
            "SELECT uid, id, dev_identifier, name, format, installed, vendor, version
             FROM plugins",
        )?;

        let existing_plugins = stmt.query_map([], |row| {
            let uid: String = row.get(0)?;
            Ok((
                uid,
                Plugin {
                    id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    dev_identifier: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    name: row.get(3)?,
                    plugin_format: row.get::<_, String>(4)?.parse().unwrap(),
                    installed: row.get(5)?,
                    vendor: row.get(6)?,
                    version: row.get(7)?,
                },
            ))
        })?;

        for row in existing_plugins {
            let (uid, plugin) = row?;
            match PluginKey::from_uid_hex(&uid) {
                Some(key) => {
                    self.unique_plugins.insert(key, plugin);
                }
                // Only reachable if something wrote a uid by a route other than
                // PluginKey::uid_hex(); skipping is safer than guessing.
                None => warn!("Skipping plugin row with unparseable uid: {}", uid),
            }
        }
        debug!("Loaded {} existing plugins", self.unique_plugins.len());
        Ok(())
    }

    /// Build the class-ID lookup used when a reference names a bundle's non-primary
    /// processor class rather than the bundle itself.
    fn load_class_index(&mut self) -> Result<(), DatabaseError> {
        let mut stmt = self.tx.prepare(
            "SELECT c.class_id, p.uid
             FROM plugin_classes c
             JOIN plugins p ON p.id = c.plugin_id",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (class_id, owner_uid) = row?;
            if let (Some(class_key), Some(owner_key)) = (
                PluginKey::from_uid_hex(&class_id),
                PluginKey::from_uid_hex(&owner_uid),
            ) {
                self.class_index.insert(class_key, owner_key);
            }
        }
        debug!("Loaded {} plugin classes", self.class_index.len());
        Ok(())
    }

    fn load_existing_samples(&mut self) -> Result<(), DatabaseError> {
        debug!("Loading existing samples from database");
        let mut stmt = self
            .tx
            .prepare("SELECT id, name, path, is_present FROM samples")?;

        let existing_samples = stmt.query_map([], |row| {
            Ok(Sample {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                is_present: row.get(3)?,
            })
        })?;

        for sample in existing_samples {
            let sample = sample?;
            self.unique_samples
                .insert(sample.path.to_string_lossy().to_string(), sample);
        }
        debug!("Loaded {} existing samples", self.unique_samples.len());
        Ok(())
    }

    fn merge_plugin_metadata(existing: &mut Plugin, new: &Plugin) {
        // A scanned plugin's name, vendor and version came from the binary itself.
        // A project reference must never overwrite them with Ableton's version of
        // the same facts.
        if existing.installed == Some(true) {
            return;
        }

        // Keep non-null values from new plugin if they exist
        // Merge name: use new name if existing name is empty or if new name is non-empty
        if !new.name.trim().is_empty() && (existing.name.trim().is_empty() || existing.name != new.name) {
            existing.name = new.name.clone();
        }
        if new.vendor.is_some() {
            existing.vendor = new.vendor.clone();
        }
        if new.version.is_some() {
            existing.version = new.version.clone();
        }
        // `installed` is owned by the plugin scan, never by parsing a project, so
        // merging two references must not touch it. Keep whatever the scan last wrote
        // (or None, if no scan has looked).
        if existing.installed.is_none() {
            existing.installed = new.installed;
        }
    }

    fn collect_items(&mut self, live_sets: &[Project]) -> Result<(), DatabaseError> {
        // First load existing items
        self.load_existing_plugins()?;
        self.load_class_index()?;
        self.load_existing_samples()?;

        for live_set in live_sets {
            // Collect and merge plugins
            for plugin in &live_set.plugins {
                let old_id = plugin.id.to_string();

                let Some(ref_key) = PluginKey::from_dev_identifier(&plugin.dev_identifier)
                else {
                    // The parser only emits references whose identifier parsed, so
                    // this means the two disagree about the format's shape.
                    warn!(
                        "Skipping plugin reference with unparseable dev_identifier: {}",
                        plugin.dev_identifier
                    );
                    continue;
                };

                let (target_key, resolved_via) = if self.unique_plugins.contains_key(&ref_key) {
                    (ref_key, "uid")
                } else if let Some(owner) = self.class_index.get(&ref_key).copied() {
                    // A class of a bundle we already have. Resolve to the bundle so
                    // this does not become a phantom duplicate of a plugin we own.
                    (owner, "class_id")
                } else {
                    // Nothing knows this plugin yet. The reference still carries a
                    // real identity, so it gets an ordinary row with `installed`
                    // left unknown until a scan looks.
                    (ref_key, "created")
                };

                let entry = self
                    .unique_plugins
                    .entry(target_key)
                    .and_modify(|existing| Self::merge_plugin_metadata(existing, plugin))
                    .or_insert_with(|| plugin.clone());

                // Map the old UUID to the canonical UUID
                self.plugin_id_map.insert(old_id, entry.id.to_string());

                self.plugin_refs.push(PluginRef {
                    dev_identifier: plugin.dev_identifier.clone(),
                    plugin_key: target_key,
                    ableton_name: reference_name(plugin),
                    ableton_format: plugin.plugin_format.to_string(),
                    resolved_via,
                });
            }

            // Collect and merge samples
            for sample in &live_set.samples {
                let old_id = sample.id.to_string();
                let path_str = sample.path.to_string_lossy().to_string();

                // Only update is_present status for existing samples
                let entry = self
                    .unique_samples
                    .entry(path_str)
                    .and_modify(|existing| {
                        if sample.is_present {
                            existing.is_present = true;
                        }
                    })
                    .or_insert_with(|| sample.clone());

                // Map the old UUID to the canonical UUID
                self.sample_id_map.insert(old_id, entry.id.to_string());
            }
        }

        debug!(
            "Found {} unique plugins and {} unique samples",
            self.unique_plugins.len(),
            self.unique_samples.len()
        );
        Ok(())
    }

    fn insert_plugins(&mut self) -> Result<(), DatabaseError> {
        debug!("Upserting {} plugins", self.unique_plugins.len());

        for (key, plugin) in &self.unique_plugins {
            self.tx.execute(
                "INSERT INTO plugins (
                    id, plugin_kind, uid, dev_identifier, name, format, vendor, version
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(plugin_kind, uid) DO UPDATE SET
                    -- A scanned plugin's identity fields came from the binary, so a
                    -- project reference must not overwrite them.
                    name = CASE WHEN plugins.installed IS 1 THEN plugins.name ELSE EXCLUDED.name END,
                    format = CASE WHEN plugins.installed IS 1 THEN plugins.format ELSE EXCLUDED.format END,
                    vendor = COALESCE(plugins.vendor, EXCLUDED.vendor),
                    version = COALESCE(plugins.version, EXCLUDED.version),
                    dev_identifier = COALESCE(plugins.dev_identifier, EXCLUDED.dev_identifier)
                    -- `installed` is deliberately absent: it belongs to the plugin
                    -- scan, and a project scan knows nothing about it.
                ",
                params![
                    plugin.id.to_string(),
                    key.kind(),
                    key.uid_hex(),
                    plugin.dev_identifier,
                    plugin.name,
                    plugin.plugin_format.to_string(),
                    plugin.vendor,
                    plugin.version,
                ],
            )?;
            self.stats.plugins_inserted += 1;
        }
        Ok(())
    }

    /// Record which reference string resolved to which plugin.
    ///
    /// This is what lets a later `plugin refresh` re-resolve references without
    /// reparsing any project file (ADR-0009).
    fn insert_plugin_refs(&mut self) -> Result<(), DatabaseError> {
        let now = SqlDateTime::from(Local::now());

        for reference in &self.plugin_refs {
            let Some(plugin) = self.unique_plugins.get(&reference.plugin_key) else {
                continue;
            };

            self.tx.execute(
                "INSERT INTO plugin_refs (
                    dev_identifier, plugin_id, ableton_name, ableton_format,
                    resolved_via, first_seen_at
                ) VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(dev_identifier) DO UPDATE SET
                    plugin_id = EXCLUDED.plugin_id,
                    ableton_name = EXCLUDED.ableton_name,
                    ableton_format = EXCLUDED.ableton_format,
                    resolved_via = EXCLUDED.resolved_via
                    -- first_seen_at keeps its original value
                ",
                params![
                    reference.dev_identifier,
                    plugin.id.to_string(),
                    reference.ableton_name,
                    reference.ableton_format,
                    reference.resolved_via,
                    now,
                ],
            )?;
        }
        Ok(())
    }

    fn insert_samples(&mut self) -> Result<(), DatabaseError> {
        debug!("Upserting {} samples", self.unique_samples.len());

        for sample in self.unique_samples.values() {
            let sample_id = sample.id.to_string();
            self.tx.execute(
                "INSERT INTO samples (
                    id, name, path, is_present
                ) VALUES (?, ?, ?, ?)
                ON CONFLICT(path) DO UPDATE SET
                    name = EXCLUDED.name,
                    is_present = EXCLUDED.is_present OR samples.is_present
                ",
                params![
                    sample_id,
                    sample.name,
                    sample.path.to_string_lossy().to_string(),
                    sample.is_present,
                ],
            )?;
            self.stats.samples_inserted += 1;
        }
        Ok(())
    }

    fn insert_projects(&mut self, live_sets: &[Project]) -> Result<(), DatabaseError> {
        for live_set in live_sets {
            let project_id = live_set.id.to_string();

            // Insert project
            self.tx.execute(
                "INSERT OR REPLACE INTO projects (
                    id, name, path, hash, created_at, modified_at,
                    last_parsed_at, tempo, time_signature_numerator,
                    time_signature_denominator, key_signature_tonic,
                    key_signature_scale, furthest_bar, duration_seconds,
                    daw_type, daw_version_display,
                    notes
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    project_id,
                    live_set.name,
                    live_set.file_path.to_string_lossy().to_string(),
                    live_set.file_hash,
                    SqlDateTime::from(live_set.created_time),
                    SqlDateTime::from(live_set.modified_time),
                    SqlDateTime::from(live_set.last_parsed_timestamp),
                    live_set.tempo,
                    live_set.time_signature.numerator,
                    live_set.time_signature.denominator,
                    live_set.key_signature.as_ref().map(|k| k.tonic.to_string()),
                    live_set.key_signature.as_ref().map(|k| k.scale.to_string()),
                    live_set.furthest_bar,
                    live_set.estimated_duration.map(|d| d.num_seconds()),
                    live_set.daw_type,
                    live_set.daw_version_display,
                    None::<String>,
                ],
            )?;

            // Ableton's structured version data lives in its own side table
            // (ADR-0015). `INSERT OR REPLACE` on `projects` above cascade-deletes any
            // existing row here (ON DELETE CASCADE), so this must run after it.
            self.tx.execute(
                "INSERT OR REPLACE INTO project_ableton_metadata (
                    project_id, version_major, version_minor, version_patch, version_beta
                ) VALUES (?, ?, ?, ?, ?)",
                params![
                    project_id,
                    live_set.ableton_metadata.major,
                    live_set.ableton_metadata.minor,
                    live_set.ableton_metadata.patch,
                    live_set.ableton_metadata.beta,
                ],
            )?;

            // Link plugins using the mapped IDs
            for plugin in &live_set.plugins {
                let old_id = plugin.id.to_string();
                let canonical_id = self.plugin_id_map.get(&old_id).unwrap();
                self.tx.execute(
                    "INSERT OR IGNORE INTO project_plugins (project_id, plugin_id)
                     VALUES (?, ?)",
                    params![project_id, canonical_id],
                )?;
            }

            // Link samples using the mapped IDs
            for sample in &live_set.samples {
                let old_id = sample.id.to_string();
                let canonical_id = self.sample_id_map.get(&old_id).unwrap();
                self.tx.execute(
                    "INSERT OR IGNORE INTO project_samples (project_id, sample_id)
                     VALUES (?, ?)",
                    params![project_id, canonical_id],
                )?;
            }

            self.stats.projects_inserted += 1;
        }
        Ok(())
    }

    fn update_search_indexes(&self, live_sets: &[Project]) -> Result<(), DatabaseError> {
        debug!("Updating search indexes for {} projects", live_sets.len());

        for live_set in live_sets {
            let project_id = live_set.id.to_string();

            self.tx.execute(
                "UPDATE project_search SET
                    plugins = (
                        SELECT GROUP_CONCAT(pl.name || ' ' || COALESCE(pl.vendor, ''), ' ')
                        FROM plugins pl
                        JOIN project_plugins pp ON pp.plugin_id = pl.id
                        WHERE pp.project_id = ?
                    ),
                    samples = (
                        SELECT GROUP_CONCAT(s.name, ' ')
                        FROM samples s
                        JOIN project_samples ps ON ps.sample_id = s.id
                        WHERE ps.project_id = ?
                    ),
                    tags = (
                        SELECT GROUP_CONCAT(t.name, ' ')
                        FROM tags t
                        JOIN project_tags pt ON pt.tag_id = t.id
                        WHERE pt.project_id = ?
                    )
                WHERE project_id = ?",
                params![project_id, project_id, project_id, project_id],
            )?;
        }
        Ok(())
    }

    fn commit(self) -> Result<BatchStats, DatabaseError> {
        self.tx.commit()?;
        Ok(self.stats)
    }
}

/// Manages batch insertion of LiveSets into the database
pub struct BatchInsertManager<'a> {
    conn: &'a mut Connection,
    live_sets: Arc<Vec<Project>>,
}

impl<'a> BatchInsertManager<'a> {
    pub fn new(conn: &'a mut Connection, live_sets: Arc<Vec<Project>>) -> Self {
        Self { conn, live_sets }
    }

    /// Execute the batch insert operation
    pub fn execute(&mut self) -> Result<BatchStats, DatabaseError> {
        debug!("Starting batch insert of {} projects", self.live_sets.len());

        // Create transaction and execute all operations
        let mut batch = BatchTransaction::new(self.conn)?;

        // Collect all unique items
        batch.collect_items(&self.live_sets)?;

        // First insert all plugins and samples. plugin_refs must follow the plugins
        // it points at.
        batch.insert_plugins()?;
        batch.insert_plugin_refs()?;
        batch.insert_samples()?;

        // Then insert projects and their relationships
        batch.insert_projects(&self.live_sets)?;

        // Finally update search indexes
        batch.update_search_indexes(&self.live_sets)?;

        // Commit and get stats
        let stats = batch.commit()?;

        info!(
            "Batch insert complete: {} projects, {} plugins, {} samples",
            stats.projects_inserted, stats.plugins_inserted, stats.samples_inserted
        );

        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct BatchStats {
    pub projects_inserted: usize,
    pub plugins_inserted: usize,
    pub samples_inserted: usize,
}
