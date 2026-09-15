use super::models::SqlDateTime;
use crate::error::DatabaseError;
use crate::live_set::LiveSet;
use crate::models::{AbletonVersion, KeySignature, Plugin, PluginKey, Sample, TimeSignature};
use chrono::{Local, TimeZone};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

/// Insert or update a single plugin reference, returning the id of the row it maps to.
///
/// Returns `None` when the `dev_identifier` is not a plugin reference at all, in which
/// case the caller must not link a project to it.
///
/// This upserts rather than using `INSERT OR REPLACE`. The difference matters: replace
/// deletes the conflicting row first, which under `PRAGMA foreign_keys = ON` cascades
/// into `project_plugins` and silently unlinks the plugin from every *other* project
/// that uses it.
pub fn insert_plugin(tx: &Transaction, plugin: &Plugin) -> Result<Option<String>, DatabaseError> {
    let Some(ref_key) = PluginKey::from_dev_identifier(&plugin.dev_identifier) else {
        log::warn!(
            "Skipping plugin reference with unparseable dev_identifier: {}",
            plugin.dev_identifier
        );
        return Ok(None);
    };

    // A reference may name one of a bundle's exported classes rather than the bundle
    // itself, so fall back to the class index before concluding this is a new plugin.
    let resolved: Option<(String, &'static str)> = tx
        .query_row(
            "SELECT id FROM plugins WHERE plugin_kind = ? AND uid = ?",
            params![ref_key.kind(), ref_key.uid_hex()],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|id| (id, "uid"))
        .or_else(|| {
            tx.query_row(
                "SELECT plugin_id FROM plugin_classes WHERE class_id = ?",
                params![ref_key.uid_hex()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|id| (id, "class_id"))
        });

    let (plugin_id, resolved_via) =
        resolved.unwrap_or_else(|| (plugin.id.to_string(), "created"));

    tx.execute(
        "INSERT INTO plugins (
            id, plugin_kind, uid, dev_identifier, name, format, vendor, version
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(plugin_kind, uid) DO UPDATE SET
            name = CASE WHEN plugins.installed IS 1 THEN plugins.name ELSE EXCLUDED.name END,
            format = CASE WHEN plugins.installed IS 1 THEN plugins.format ELSE EXCLUDED.format END,
            vendor = COALESCE(plugins.vendor, EXCLUDED.vendor),
            version = COALESCE(plugins.version, EXCLUDED.version),
            dev_identifier = COALESCE(plugins.dev_identifier, EXCLUDED.dev_identifier)
            -- `installed` is untouched: only a plugin scan may write it.
        ",
        params![
            plugin_id,
            ref_key.kind(),
            ref_key.uid_hex(),
            plugin.dev_identifier,
            plugin.name,
            plugin.plugin_format.to_string(),
            plugin.vendor,
            plugin.version,
        ],
    )?;

    tx.execute(
        "INSERT INTO plugin_refs (
            dev_identifier, plugin_id, ableton_name, ableton_format, resolved_via, first_seen_at
        ) VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(dev_identifier) DO UPDATE SET
            plugin_id = EXCLUDED.plugin_id,
            ableton_name = EXCLUDED.ableton_name,
            ableton_format = EXCLUDED.ableton_format,
            resolved_via = EXCLUDED.resolved_via",
        params![
            plugin.dev_identifier,
            plugin_id,
            // Fall back to the identifier's own display name when the project file's
            // <Name> was blank.
            if plugin.name.trim().is_empty() {
                crate::models::dev_identifier_display_name(&plugin.dev_identifier)
                    .unwrap_or_default()
            } else {
                plugin.name.clone()
            },
            plugin.plugin_format.to_string(),
            resolved_via,
            SqlDateTime::from(Local::now()),
        ],
    )?;

    Ok(Some(plugin_id))
}

/// Build a [`Plugin`] from a `plugins` row.
///
/// Reads by column name rather than position: the table's column order is not a
/// contract, and every schema change used to silently reassign the indices that ten
/// separate copies of this code depended on.
pub fn row_to_plugin(row: &Row) -> rusqlite::Result<Plugin> {
    Ok(Plugin {
        id: Uuid::parse_str(&row.get::<_, String>("id")?).unwrap_or_else(|_| Uuid::new_v4()),
        dev_identifier: row
            .get::<_, Option<String>>("dev_identifier")?
            .unwrap_or_default(),
        name: row.get("name")?,
        plugin_format: row
            .get::<_, String>("format")?
            .parse()
            .map_err(rusqlite::Error::InvalidParameterName)?,
        installed: row.get("installed")?,
        vendor: row.get("vendor")?,
        version: row.get("version")?,
    })
}

/// Insert a sample into the database
pub fn insert_sample(tx: &Transaction, sample: &Sample) -> Result<(), DatabaseError> {
    tx.execute(
        "INSERT OR REPLACE INTO samples (id, name, path, is_present) VALUES (?, ?, ?, ?)",
        params![
            sample.id.to_string(),
            sample.name,
            sample.path.to_string_lossy().to_string(),
            sample.is_present,
        ],
    )?;
    Ok(())
}

/// Link a project to a plugin
pub fn link_project_plugin(
    tx: &Transaction,
    project_id: &str,
    plugin_id: &str,
) -> Result<(), DatabaseError> {
    tx.execute(
        "INSERT OR REPLACE INTO project_plugins (project_id, plugin_id) VALUES (?, ?)",
        params![project_id, plugin_id],
    )?;
    Ok(())
}

/// Link a project to a sample
pub fn link_project_sample(
    tx: &Transaction,
    project_id: &str,
    sample_id: &str,
) -> Result<(), DatabaseError> {
    tx.execute(
        "INSERT OR REPLACE INTO project_samples (project_id, sample_id) VALUES (?, ?)",
        params![project_id, sample_id],
    )?;
    Ok(())
}

/// Convert a database row to a LiveSet object
pub fn row_to_live_set(row: &Row) -> rusqlite::Result<LiveSet> {
    let id: String = row.get("id")?;
    let created_timestamp: i64 = row.get("created_at")?;
    let modified_timestamp: i64 = row.get("modified_at")?;
    let parsed_timestamp: i64 = row.get("last_parsed_at")?;
    let duration_secs: Option<i64> = row.get("duration_seconds")?;

    Ok(LiveSet {
        is_active: row.get("is_active")?,
        id: Uuid::parse_str(&id).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        file_path: PathBuf::from(row.get::<_, String>("path")?),
        name: row.get("name")?,
        file_hash: row.get("hash")?,
        created_time: Local
            .timestamp_opt(created_timestamp, 0)
            .single()
            .ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid timestamp",
                    )),
                )
            })?,
        modified_time: Local
            .timestamp_opt(modified_timestamp, 0)
            .single()
            .ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid timestamp",
                    )),
                )
            })?,
        last_parsed_timestamp: Local
            .timestamp_opt(parsed_timestamp, 0)
            .single()
            .ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid timestamp",
                    )),
                )
            })?,

        tempo: row.get("tempo")?,
        time_signature: TimeSignature {
            numerator: row.get("time_signature_numerator")?,
            denominator: row.get("time_signature_denominator")?,
        },
        key_signature: match (
            row.get::<_, Option<String>>("key_signature_tonic")?,
            row.get::<_, Option<String>>("key_signature_scale")?,
        ) {
            (Some(tonic), Some(scale)) => Some(KeySignature {
                tonic: tonic.parse().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                    )
                })?,
                scale: scale.parse().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                    )
                })?,
            }),
            _ => None,
        },
        furthest_bar: row.get("furthest_bar")?,

        ableton_version: AbletonVersion {
            major: row.get("ableton_version_major")?,
            minor: row.get("ableton_version_minor")?,
            patch: row.get("ableton_version_patch")?,
            beta: row.get("ableton_version_beta")?,
        },

        estimated_duration: duration_secs.map(chrono::Duration::seconds),
        plugins: HashSet::new(), // These will be loaded separately when needed
        samples: HashSet::new(), // These will be loaded separately when needed
        tags: HashSet::new(),    // These will be loaded separately when needed
    })
}
