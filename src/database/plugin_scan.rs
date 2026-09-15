//! Persisting the results of a system plugin scan.
//!
//! Mirrors [`batch`](super::batch)'s role: one module owning one write path, in one
//! transaction. The scan is the only thing that writes `plugins.installed` — parsing a
//! project never does, because a project file cannot know what is installed.

use chrono::Local;
use log::{debug, info, warn};
use rusqlite::{params, OptionalExtension, Transaction};
use uuid::Uuid;

use vst_meta::meta::{FormatExtra, PluginMeta};
use vst_meta::protocol::Outcome;

use super::models::SqlDateTime;
use crate::database::LiveSetDatabase;
use crate::error::DatabaseError;
use crate::models::PluginKey;
use crate::scan::plugins::ScanReport;

/// Records when a full plugin scan last completed.
///
/// Deliberately not "is the plugins table empty". A machine whose plugins all fail to
/// load would leave that table empty forever, so an empty-table trigger would rescan on
/// every project scan and never stop. What matters is whether we have *looked*.
pub const PLUGIN_SCAN_COMPLETED_KEY: &str = "plugins_last_scanned_at";

/// What a persist run changed.
#[derive(Debug, Default, Clone)]
pub struct PluginScanPersist {
    pub inserted: usize,
    pub updated: usize,
    /// Rows a full scan looked for and did not find.
    pub marked_missing: usize,
    /// Phantom rows merged into the bundle that actually owns them.
    pub reconciled: usize,
    /// Scan results whose uid could not be parsed into an identity.
    pub skipped: usize,
}

impl LiveSetDatabase {
    /// Write a scan report into the database.
    ///
    /// `full_scan` must be false when the scan was narrowed (`--paths`) or cut short
    /// (`budget_exhausted`). A partial scan may only report what it saw; it must not
    /// declare everything it did not reach to be missing.
    pub fn persist_plugin_scan(
        &mut self,
        report: &ScanReport,
        full_scan: bool,
    ) -> Result<PluginScanPersist, DatabaseError> {
        let tx = self.conn.transaction()?;
        let mut result = PluginScanPersist::default();
        let now = SqlDateTime::from(Local::now());

        let mut seen: Vec<String> = Vec::new();

        for scanned in report.results.iter() {
            let Outcome::Success { plugins } = &scanned.outcome else {
                continue;
            };

            for meta in plugins {
                let Some(key) = PluginKey::from_uid_hex(&meta.uid) else {
                    warn!(
                        "Skipping scanned plugin with unparseable uid {:?} at {}",
                        meta.uid, meta.path
                    );
                    result.skipped += 1;
                    continue;
                };

                let plugin_id = upsert_scanned_plugin(&tx, &key, meta, &now, &mut result)?;
                replace_child_rows(&tx, &plugin_id, meta)?;
                seen.push(key.uid_hex());
            }
        }

        if full_scan {
            result.marked_missing = sweep_unseen(&tx, &seen, &now)?;
        } else {
            debug!("Partial scan: leaving plugins it did not reach untouched");
        }

        result.reconciled = reconcile_phantoms(&tx)?;

        tx.commit()?;

        // Only a full scan counts as having looked. A narrowed one leaves the question
        // open, so it must not suppress the first-run scan.
        if full_scan {
            self.set_app_state(PLUGIN_SCAN_COMPLETED_KEY, &Local::now().to_rfc3339())?;
        }

        info!(
            "Persisted plugin scan: {} inserted, {} updated, {} marked missing, {} reconciled, {} skipped",
            result.inserted, result.updated, result.marked_missing, result.reconciled, result.skipped
        );

        Ok(result)
    }

    /// Whether a full plugin scan has ever completed against this database.
    ///
    /// Drives the first-run scan. False means the `installed` column is entirely
    /// unknown rather than merely negative.
    pub fn has_scanned_plugins(&self) -> Result<bool, DatabaseError> {
        Ok(self.get_app_state(PLUGIN_SCAN_COMPLETED_KEY)?.is_some())
    }
}

/// Insert or update the row for one scanned plugin, and return its id.
///
/// Scanner data wins over anything a project reference wrote: it came from the binary.
fn upsert_scanned_plugin(
    tx: &Transaction,
    key: &PluginKey,
    meta: &PluginMeta,
    now: &SqlDateTime,
    result: &mut PluginScanPersist,
) -> Result<String, DatabaseError> {
    // `.optional()?` rather than `.ok()`: the latter would fold a real database error
    // into "no such row", which is exactly the confusion that made
    // `refresh_plugin_installation_status` mark every plugin installed (ADR-0006).
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM plugins WHERE plugin_kind = ? AND uid = ?",
            params![key.kind(), key.uid_hex()],
            |row| row.get(0),
        )
        .optional()?;

    let plugin_id = existing
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if existing.is_some() {
        result.updated += 1;
    } else {
        result.inserted += 1;
    }

    // The four-variant format is display information. Derived from the plugin's own
    // category rather than Ableton's opinion of it -- ADR-0005 keeps that opinion out
    // of identity, and here we simply have a better source.
    let format = match (key, meta.is_instrument) {
        (PluginKey::Vst2(_), true) => "VST2 Instrument",
        (PluginKey::Vst2(_), false) => "VST2 Effect",
        (PluginKey::Vst3(_), true) => "VST3 Instrument",
        (PluginKey::Vst3(_), false) => "VST3 Effect",
    };

    let (fourcc, preset_chunks, f64_precision, silent_when_stopped, midi_in, midi_out, factory_flags) =
        match &meta.extra {
            FormatExtra::Vst2 {
                fourcc,
                preset_chunks,
                f64_precision,
                silent_when_stopped,
                midi_in_channels,
                midi_out_channels,
            } => (
                Some(fourcc.clone()),
                Some(*preset_chunks),
                Some(*f64_precision),
                Some(*silent_when_stopped),
                Some(*midi_in_channels),
                Some(*midi_out_channels),
                None,
            ),
            FormatExtra::Vst3 { factory_flags, .. } => {
                (None, None, None, None, None, None, Some(*factory_flags))
            }
        };

    tx.execute(
        "INSERT INTO plugins (
            id, plugin_kind, uid, name, format, vendor, version,
            installed, last_scanned_at,
            path, category, is_instrument,
            audio_in_channels, audio_out_channels, audio_in_buses, audio_out_buses,
            has_midi_input, has_midi_output, presets, parameters, latency_samples,
            has_gui, vendor_url, vendor_email, is_shell, shell_parent_uid,
            fourcc, preset_chunks, f64_precision, silent_when_stopped,
            midi_in_channels, midi_out_channels, factory_flags
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?,
            1, ?,
            ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?
        )
        ON CONFLICT(plugin_kind, uid) DO UPDATE SET
            name = EXCLUDED.name,
            format = EXCLUDED.format,
            vendor = EXCLUDED.vendor,
            version = EXCLUDED.version,
            installed = 1,
            last_scanned_at = EXCLUDED.last_scanned_at,
            path = EXCLUDED.path,
            category = EXCLUDED.category,
            is_instrument = EXCLUDED.is_instrument,
            audio_in_channels = EXCLUDED.audio_in_channels,
            audio_out_channels = EXCLUDED.audio_out_channels,
            audio_in_buses = EXCLUDED.audio_in_buses,
            audio_out_buses = EXCLUDED.audio_out_buses,
            has_midi_input = EXCLUDED.has_midi_input,
            has_midi_output = EXCLUDED.has_midi_output,
            presets = EXCLUDED.presets,
            parameters = EXCLUDED.parameters,
            latency_samples = EXCLUDED.latency_samples,
            has_gui = EXCLUDED.has_gui,
            vendor_url = EXCLUDED.vendor_url,
            vendor_email = EXCLUDED.vendor_email,
            is_shell = EXCLUDED.is_shell,
            shell_parent_uid = EXCLUDED.shell_parent_uid,
            fourcc = EXCLUDED.fourcc,
            preset_chunks = EXCLUDED.preset_chunks,
            f64_precision = EXCLUDED.f64_precision,
            silent_when_stopped = EXCLUDED.silent_when_stopped,
            midi_in_channels = EXCLUDED.midi_in_channels,
            midi_out_channels = EXCLUDED.midi_out_channels,
            factory_flags = EXCLUDED.factory_flags
        ",
        params![
            plugin_id,
            key.kind(),
            key.uid_hex(),
            meta.name,
            format,
            non_empty(&meta.vendor),
            non_empty(&meta.version),
            now,
            meta.path,
            non_empty(&meta.category),
            meta.is_instrument,
            meta.audio_in_channels,
            meta.audio_out_channels,
            meta.audio_in_buses,
            meta.audio_out_buses,
            meta.has_midi_input,
            meta.has_midi_output,
            meta.presets,
            meta.parameters,
            meta.latency_samples,
            meta.has_gui,
            meta.vendor_url,
            meta.vendor_email,
            meta.is_shell,
            // Normalise so it joins against uid the same way everything else does.
            meta.shell_parent_uid
                .as_deref()
                .and_then(PluginKey::from_uid_hex)
                .map(|k| k.uid_hex()),
            fourcc,
            preset_chunks,
            f64_precision,
            silent_when_stopped,
            midi_in,
            midi_out,
            factory_flags,
        ],
    )?;

    Ok(plugin_id)
}

/// Replace a plugin's class and bus rows wholesale.
///
/// Diffing them would buy nothing: they are small, and a rescan is the only thing that
/// writes them.
fn replace_child_rows(
    tx: &Transaction,
    plugin_id: &str,
    meta: &PluginMeta,
) -> Result<(), DatabaseError> {
    tx.execute(
        "DELETE FROM plugin_classes WHERE plugin_id = ?",
        params![plugin_id],
    )?;
    tx.execute(
        "DELETE FROM plugin_buses WHERE plugin_id = ?",
        params![plugin_id],
    )?;

    let FormatExtra::Vst3 { classes, buses, .. } = &meta.extra else {
        return Ok(());
    };

    for class in classes {
        // Store the class ID in the same normalised form as plugins.uid, or the
        // matching fallback silently fails to join.
        let Some(class_key) = PluginKey::from_uid_hex(&class.class_id) else {
            warn!(
                "Skipping VST3 class with unparseable class_id {:?} on {}",
                class.class_id, meta.name
            );
            continue;
        };

        tx.execute(
            "INSERT INTO plugin_classes (
                plugin_id, name, category, class_id, cardinality, version
            ) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                plugin_id,
                class.name,
                class.category,
                class_key.uid_hex(),
                class.cardinality,
                class.version,
            ],
        )?;
    }

    for bus in buses {
        tx.execute(
            "INSERT INTO plugin_buses (
                plugin_id, direction, media, name, channel_count, bus_type, flags
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                plugin_id,
                bus.direction,
                bus.media,
                bus.name,
                bus.channel_count,
                bus.bus_type,
                bus.flags,
            ],
        )?;
    }

    Ok(())
}

/// Mark everything a full scan did not find as not installed.
///
/// Only valid after a scan that covered every configured search path: a narrowed or
/// truncated scan has no grounds to call anything missing.
fn sweep_unseen(
    tx: &Transaction,
    seen: &[String],
    now: &SqlDateTime,
) -> Result<usize, DatabaseError> {
    // Build the exclusion list explicitly rather than with a temp table: a plugin
    // library is hundreds of rows, not millions.
    let placeholders = if seen.is_empty() {
        "''".to_string()
    } else {
        std::iter::repeat("?")
            .take(seen.len())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let sql = format!(
        "UPDATE plugins
         SET installed = 0, last_scanned_at = ?
         WHERE uid NOT IN ({}) AND (installed IS NULL OR installed = 1)",
        placeholders
    );

    let mut values: Vec<&dyn rusqlite::ToSql> = vec![now];
    for uid in seen {
        values.push(uid);
    }

    Ok(tx.execute(&sql, values.as_slice())?)
}

/// Merge rows that a project reference created from a class ID into the bundle that
/// actually exports that class.
///
/// A project can reference a bundle's non-primary processor class before that bundle
/// has ever been scanned. The reference is a real identity, so it gets its own row —
/// but once the bundle is scanned, that row is a duplicate of a plugin we own. This is
/// what makes "install the missing plugin, run refresh" work without reparsing
/// anything.
fn reconcile_phantoms(tx: &Transaction) -> Result<usize, DatabaseError> {
    let pairs: Vec<(String, String)> = {
        let mut stmt = tx.prepare(
            "SELECT phantom.id, cls.plugin_id
             FROM plugins phantom
             JOIN plugin_classes cls ON cls.class_id = phantom.uid
             WHERE phantom.id != cls.plugin_id
               AND phantom.installed IS NOT 1",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        rows.collect::<Result<Vec<_>, _>>()?
    };

    for (phantom_id, real_id) in &pairs {
        debug!("Reconciling phantom plugin {} into {}", phantom_id, real_id);

        tx.execute(
            "UPDATE plugin_refs SET plugin_id = ?, resolved_via = 'class_id' WHERE plugin_id = ?",
            params![real_id, phantom_id],
        )?;

        // The junction is a primary key pair, so a project already linked to the real
        // plugin must not gain a duplicate row.
        tx.execute(
            "INSERT OR IGNORE INTO project_plugins (project_id, plugin_id)
             SELECT project_id, ? FROM project_plugins WHERE plugin_id = ?",
            params![real_id, phantom_id],
        )?;
        tx.execute(
            "DELETE FROM project_plugins WHERE plugin_id = ?",
            params![phantom_id],
        )?;

        tx.execute("DELETE FROM plugins WHERE id = ?", params![phantom_id])?;
    }

    Ok(pairs.len())
}

/// The scanner reports absent strings as empty; the database prefers NULL.
fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
