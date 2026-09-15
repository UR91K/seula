use std::sync::Arc;

use crate::models::PluginFormat;

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

/// The format *as Ableton classifies it*, including its instrument/effect call.
///
/// This is display and filtering information, not identity — Ableton's `instr` vs
/// `audiofx` is its own opinion and disagrees with the plugin's in practice. For
/// matching, use [`PluginKey`](crate::models::PluginKey), which structurally cannot
/// see the category. See ADR-0005.
pub(crate) fn parse_plugin_format(dev_identifier: &str) -> Option<PluginFormat> {
    let parts = crate::models::split_dev_identifier(dev_identifier)?;

    match (parts.kind, parts.category) {
        ("vst3", "instr") => Some(PluginFormat::VST3Instrument),
        ("vst3", "audiofx") => Some(PluginFormat::VST3AudioFx),
        ("vst", "instr") => Some(PluginFormat::VST2Instrument),
        ("vst", "audiofx") => Some(PluginFormat::VST2AudioFx),
        _ => None,
    }
}
