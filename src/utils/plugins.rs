use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    error::{DatabaseError, FileError},
    models::PluginFormat,
};

// LINE TRACKER FOR DEBUGGING

#[derive(Clone)]
pub(crate) struct LineTrackingBuffer {
    data: Arc<Vec<u8>>,
    current_line: usize,
    current_position: usize,
}

impl LineTrackingBuffer {
    pub(crate) fn new(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(data),
            current_line: 1,
            current_position: 0,
        }
    }

    pub(crate) fn get_line_number(&mut self, byte_position: u64) -> usize {
        let byte_position_usize =
            usize::try_from(byte_position).unwrap_or_else(|_| self.data.len()); // Clamp to max usize or data length

        while self.current_position < byte_position_usize && self.current_position < self.data.len()
        {
            if self.data[self.current_position] == b'\n' {
                self.current_line += 1;
            }
            self.current_position += 1;
        }
        self.current_line
    }

    #[allow(dead_code)]
    pub(crate) fn update_position(&mut self, byte_position: u64) {
        self.get_line_number(byte_position);
    }
}

pub(crate) fn get_most_recent_db_file(directory: &PathBuf) -> Result<PathBuf, DatabaseError> {
    fs::read_dir(directory)
        .map_err(|_| FileError::NotFound(directory.clone()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("db") {
                entry
                    .metadata()
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .map(|modified| (path, modified))
            } else {
                None
            }
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path)
        .ok_or_else(|| FileError::NotFound(directory.clone()))
        .and_then(|path| {
            if path.is_file() {
                Ok(path)
            } else {
                Err(FileError::NotAFile(path))
            }
        })
        .map_err(DatabaseError::FileError)
}

/// Returns the most recent Ableton "plugins" database file in the given directory.
///
/// Ableton maintains multiple SQLite files in the Live Database directory (e.g., files, plugins).
/// We specifically need the plugins DB (e.g., `Live-plugins-*.db`) because it contains
/// the `plugins` table used to enrich parsed plugin metadata. Selecting the most recent generic
/// `.db` can accidentally pick the files DB (e.g., `Live-files-*.db`) which does not define
/// the required `plugins` table and causes runtime errors like "no such table: plugins".
pub(crate) fn get_most_recent_plugins_db_file(directory: &PathBuf) -> Result<PathBuf, DatabaseError> {
    fs::read_dir(directory)
        .map_err(|_| FileError::NotFound(directory.clone()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_string_lossy().to_lowercase();
            if path.extension().and_then(|ext| ext.to_str()) == Some("db") && file_name.contains("plugins") {
                entry
                    .metadata()
                    .ok()
                    .and_then(|meta| meta.modified().ok())
                    .map(|modified| (path, modified))
            } else {
                None
            }
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path)
        .ok_or_else(|| FileError::NotFound(directory.clone()))
        .and_then(|path| {
            if path.is_file() {
                Ok(path)
            } else {
                Err(FileError::NotAFile(path))
            }
        })
        .map_err(DatabaseError::FileError)
}

pub(crate) fn parse_plugin_format(dev_identifier: &str) -> Option<PluginFormat> {
    if dev_identifier.starts_with("device:vst3:instr:") {
        Some(PluginFormat::VST3Instrument)
    } else if dev_identifier.starts_with("device:vst3:audiofx:") {
        Some(PluginFormat::VST3AudioFx)
    } else if dev_identifier.starts_with("device:vst:instr:") {
        Some(PluginFormat::VST2Instrument)
    } else if dev_identifier.starts_with("device:vst:audiofx:") {
        Some(PluginFormat::VST2AudioFx)
    } else {
        None
    }
}
