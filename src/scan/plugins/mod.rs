//! Scanning the plugins installed on this system.
//!
//! Plugin metadata can only be read by loading the plugin's own binary, which means
//! running arbitrary third-party native code. That code crashes, hangs, and cannot
//! reliably be unloaded, so it runs in the `vst-meta` worker subprocess and this
//! module supervises it. See [`spawner`] for the recovery protocol.

pub mod discovery;
pub mod spawner;

use std::path::PathBuf;
use std::time::Duration;

pub use spawner::{PluginScanError, PluginScanResult, ScanReport, Spawner};

/// Discover and scan every plugin under `roots`.
///
/// `roots` empty means "use the configured search paths, or the platform defaults".
/// Failures of individual plugins are recorded in the report rather than returned as
/// errors; the `Err` case here means the scan could not be started at all.
pub fn scan_system(roots: &[PathBuf], timeout: Duration) -> Result<ScanReport, PluginScanError> {
    scan_system_with_progress(roots, timeout, &mut |_, _, _| {})
}

/// As [`scan_system`], reporting each plugin as it is attempted.
///
/// A scan of a real library takes minutes, so anything driving one in front of a user
/// needs to say what it is doing. See [`Spawner::scan_with_progress`].
pub fn scan_system_with_progress(
    roots: &[PathBuf],
    timeout: Duration,
    on_progress: &mut dyn FnMut(usize, usize, &std::path::Path),
) -> Result<ScanReport, PluginScanError> {
    let candidates = discovery::discover(roots);
    log::info!("Found {} plugin candidate(s) to scan", candidates.len());

    let spawner = Spawner::new(timeout)?;
    Ok(spawner.scan_with_progress(&candidates, on_progress))
}
