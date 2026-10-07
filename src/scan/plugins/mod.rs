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
    tracing::info!("Found {} plugin candidate(s) to scan", candidates.len());

    scan_paths_with_progress(&candidates, timeout, on_progress)
}

/// Which files a scan loads (ADR-0067).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScanMode {
    /// Only files that are new, or whose size or mtime changed since they were last
    /// loaded. Unchanged files keep what they gave last time, a failure included.
    #[default]
    Changes,
    /// Every file, retrying the ones that failed.
    All,
}

impl ScanMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "changes" => Some(Self::Changes),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

/// A plugin file as discovery found it, with what decides whether it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginFile {
    pub path: PathBuf,
    pub size: u64,
    /// Epoch seconds; `None` when the platform could not say.
    pub modified: Option<i64>,
}

/// One scan: every file discovery found, and the report for the ones it loaded.
#[derive(Debug)]
pub struct PluginScan {
    pub files: Vec<PluginFile>,
    pub report: ScanReport,
    /// Files skipped as unchanged since they were last loaded.
    pub unchanged: usize,
}

/// Discover the plugin files under `roots`, and load the ones `mode` asks for:
/// `unchanged` says which files a scan for changes may skip (ADR-0067). Blocking, and
/// minutes long when there is much to load.
pub fn scan_files(
    roots: &[PathBuf],
    timeout: Duration,
    mode: ScanMode,
    unchanged: impl FnOnce(&[PluginFile]) -> std::collections::HashSet<PathBuf>,
    on_progress: &mut dyn FnMut(usize, usize, &std::path::Path),
) -> Result<PluginScan, PluginScanError> {
    let files = discover_files(roots);
    let skip = match mode {
        ScanMode::All => Default::default(),
        ScanMode::Changes => unchanged(&files),
    };
    let load: Vec<PathBuf> = files
        .iter()
        .filter(|f| !skip.contains(&f.path))
        .map(|f| f.path.clone())
        .collect();
    tracing::info!(
        "Found {} plugin file(s); loading {} ({:?})",
        files.len(),
        load.len(),
        mode
    );
    let report = scan_paths_with_progress(&load, timeout, on_progress)?;
    Ok(PluginScan {
        unchanged: files.len() - load.len(),
        files,
        report,
    })
}

/// Every plugin file under `roots` (the configured search paths, or the platform
/// defaults when empty), with its size and mtime. Reading metadata loads nothing.
pub fn discover_files(roots: &[PathBuf]) -> Vec<PluginFile> {
    discovery::discover(roots)
        .into_iter()
        .map(|path| discovery::stat(&path))
        .collect()
}

/// Load each of `paths` in the worker, reporting each as it is attempted.
pub fn scan_paths_with_progress(
    paths: &[PathBuf],
    timeout: Duration,
    on_progress: &mut dyn FnMut(usize, usize, &std::path::Path),
) -> Result<ScanReport, PluginScanError> {
    if paths.is_empty() {
        // Nothing changed: no worker to start.
        return Ok(ScanReport {
            results: Vec::new(),
            restarts: 0,
            budget_exhausted: false,
        });
    }
    let spawner = Spawner::new(timeout)?;
    Ok(spawner.scan_with_progress(paths, on_progress))
}
