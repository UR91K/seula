//! Everything the plugin scan recorded about one plugin, for the plugins view's
//! inspector.
//!
//! The list routes carry only the identifying fields. The scanner columns, the VST3
//! classes and buses, and the identifiers projects know the plugin by are read here, one
//! plugin at a time. A plugin no scan has found (missing, or never scanned) has no
//! scanner data, so every scanner field is `None` and the lists of classes and buses
//! are empty. Its references are still there: they come from project files.

use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;

use crate::error::DatabaseError;

use super::ProjectDatabase;

/// The scanner's record of one plugin. Fields are `None` when the plugin has not been
/// found by a scan, and for the format-specific extras, when it is the other format.
#[derive(Debug, Serialize)]
pub struct PluginDetails {
    /// `VST2` or `VST3`: half of the plugin's identity (ADR-0005).
    pub plugin_kind: String,
    /// The other half, lowercase hex (`PluginKey::uid_hex`).
    pub uid: String,
    /// When a scan last ruled on this plugin, found or not. Unix seconds.
    pub last_scanned_at: Option<i64>,
    /// One location of possibly several (ADR-0010).
    pub path: Option<String>,
    pub category: Option<String>,
    pub is_instrument: Option<bool>,
    pub audio_in_channels: Option<i32>,
    pub audio_out_channels: Option<i32>,
    pub audio_in_buses: Option<i32>,
    pub audio_out_buses: Option<i32>,
    pub has_midi_input: Option<bool>,
    pub has_midi_output: Option<bool>,
    pub presets: Option<i32>,
    pub parameters: Option<i32>,
    pub latency_samples: Option<i32>,
    pub has_gui: Option<bool>,
    pub vendor_url: Option<String>,
    pub vendor_email: Option<String>,
    pub is_shell: bool,
    pub shell_parent_uid: Option<String>,
    /// VST2 only.
    pub fourcc: Option<String>,
    pub preset_chunks: Option<bool>,
    pub f64_precision: Option<bool>,
    pub silent_when_stopped: Option<bool>,
    pub midi_in_channels: Option<i32>,
    pub midi_out_channels: Option<i32>,
    /// VST3 only.
    pub factory_flags: Option<i32>,
    /// Every class a VST3 bundle's factory exports.
    pub classes: Vec<PluginClass>,
    pub buses: Vec<PluginBus>,
    /// The identifiers projects refer to this plugin by, and what they call it
    /// (ADR-0009).
    pub references: Vec<PluginReference>,
}

#[derive(Debug, Serialize)]
pub struct PluginClass {
    pub name: String,
    pub category: String,
    pub class_id: String,
    pub cardinality: i32,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct PluginBus {
    pub direction: String,
    pub media: String,
    pub name: String,
    pub channel_count: i32,
    pub bus_type: i32,
    pub flags: i32,
}

#[derive(Debug, Serialize)]
pub struct PluginReference {
    pub dev_identifier: String,
    /// The name the project file gives the plugin, which can differ from the scanned
    /// name.
    pub ableton_name: Option<String>,
    /// Ableton's instrument/effect call, which is never identity (ADR-0005).
    pub ableton_format: Option<String>,
    /// How the reference was matched: `uid`, `class_id` or `created`.
    pub resolved_via: String,
    pub first_seen_at: i64,
}

impl ProjectDatabase {
    /// The scan record for one plugin, or `None` when there is no such plugin.
    pub fn get_plugin_details(&self, plugin_id: &str) -> Result<Option<PluginDetails>, DatabaseError> {
        let Some(mut details) = self
            .conn
            .query_row("SELECT * FROM plugins WHERE id = ?", params![plugin_id], row_to_details)
            .optional()?
        else {
            return Ok(None);
        };

        let mut stmt = self.conn.prepare(
            "SELECT name, category, class_id, cardinality, version FROM plugin_classes
             WHERE plugin_id = ? ORDER BY name",
        )?;
        details.classes = stmt
            .query_map(params![plugin_id], |row| {
                Ok(PluginClass {
                    name: row.get(0)?,
                    category: row.get(1)?,
                    class_id: row.get(2)?,
                    cardinality: row.get(3)?,
                    version: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        // Inputs before outputs, then in the order the scanner reported them.
        let mut stmt = self.conn.prepare(
            "SELECT direction, media, name, channel_count, bus_type, flags FROM plugin_buses
             WHERE plugin_id = ? ORDER BY direction, rowid",
        )?;
        details.buses = stmt
            .query_map(params![plugin_id], |row| {
                Ok(PluginBus {
                    direction: row.get(0)?,
                    media: row.get(1)?,
                    name: row.get(2)?,
                    channel_count: row.get(3)?,
                    bus_type: row.get(4)?,
                    flags: row.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT dev_identifier, ableton_name, ableton_format, resolved_via, first_seen_at
             FROM plugin_refs WHERE plugin_id = ? ORDER BY first_seen_at",
        )?;
        details.references = stmt
            .query_map(params![plugin_id], |row| {
                Ok(PluginReference {
                    dev_identifier: row.get(0)?,
                    ableton_name: row.get(1)?,
                    ableton_format: row.get(2)?,
                    resolved_via: row.get(3)?,
                    first_seen_at: row.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        Ok(Some(details))
    }
}

fn row_to_details(row: &Row) -> rusqlite::Result<PluginDetails> {
    Ok(PluginDetails {
        plugin_kind: row.get("plugin_kind")?,
        uid: row.get("uid")?,
        last_scanned_at: row.get("last_scanned_at")?,
        path: row.get("path")?,
        category: row.get("category")?,
        is_instrument: row.get("is_instrument")?,
        audio_in_channels: row.get("audio_in_channels")?,
        audio_out_channels: row.get("audio_out_channels")?,
        audio_in_buses: row.get("audio_in_buses")?,
        audio_out_buses: row.get("audio_out_buses")?,
        has_midi_input: row.get("has_midi_input")?,
        has_midi_output: row.get("has_midi_output")?,
        presets: row.get("presets")?,
        parameters: row.get("parameters")?,
        latency_samples: row.get("latency_samples")?,
        has_gui: row.get("has_gui")?,
        vendor_url: row.get("vendor_url")?,
        vendor_email: row.get("vendor_email")?,
        is_shell: row.get("is_shell")?,
        shell_parent_uid: row.get("shell_parent_uid")?,
        fourcc: row.get("fourcc")?,
        preset_chunks: row.get("preset_chunks")?,
        f64_precision: row.get("f64_precision")?,
        silent_when_stopped: row.get("silent_when_stopped")?,
        midi_in_channels: row.get("midi_in_channels")?,
        midi_out_channels: row.get("midi_out_channels")?,
        factory_flags: row.get("factory_flags")?,
        classes: Vec::new(),
        buses: Vec::new(),
        references: Vec::new(),
    })
}
