//! Supervising the plugin scanner subprocess.
//!
//! The worker is expected to die. Plugins segfault during init, hang on license
//! checks, and occasionally call `exit()`; none of that is catchable in-process, which
//! is why the scanning happens somewhere we can afford to lose. This module's job is
//! to notice, attribute the failure to the right plugin, and carry on.
//!
//! One worker handles a whole batch rather than one plugin each, because process
//! startup plus VST3 runtime init costs 20-40ms -- half a minute of pure overhead
//! across a thousand-plugin library. Crash isolation is recovered by the `begin`
//! line the worker emits before touching each binary: a `begin` with no matching
//! `result` names the plugin that killed it, so the supervisor can record the failure
//! against that path and restart from the one after it.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use thiserror::Error;
use vst_meta::protocol::{ErrorType, Event, Outcome};

/// Name of the sidecar worker binary.
const WORKER_BIN: &str = if cfg!(windows) {
    "vst-meta.exe"
} else {
    "vst-meta"
};

/// Overrides the sidecar lookup. Needed in development, where the worker sits in
/// `target/debug/` rather than next to an installed executable.
const WORKER_PATH_ENV: &str = "SEULA_VST_META_PATH";

/// Restart budget, as a multiple of the number of candidates. A library where every
/// plugin crashes still terminates; a supervisor that respawns forever does not.
const RESTART_BUDGET_FACTOR: usize = 3;

#[derive(Debug, Error)]
pub enum PluginScanError {
    #[error(
        "Could not find the plugin scanner binary '{0}'. Expected it next to the current \
         executable, or named by the {1} environment variable."
    )]
    WorkerNotFound(String, &'static str),

    #[error("Failed to start the plugin scanner: {0}")]
    Spawn(#[from] std::io::Error),
}

/// What became of one candidate path.
#[derive(Debug, Clone)]
pub struct PluginScanResult {
    pub path: PathBuf,
    pub outcome: Outcome,
}

impl PluginScanResult {
    /// The failure classification, or `None` if the scan succeeded.
    pub fn error_type(&self) -> Option<ErrorType> {
        match &self.outcome {
            Outcome::Success { .. } => None,
            Outcome::Error { error_type, .. } => Some(*error_type),
        }
    }
}

/// The outcome of a full scan.
#[derive(Debug, Clone)]
pub struct ScanReport {
    pub results: Vec<PluginScanResult>,
    /// How many times the worker had to be restarted. Non-zero means plugins on this
    /// system crashed or hung the scanner -- exactly what the subprocess buys us.
    pub restarts: usize,
    /// True if the restart budget ran out and some candidates were never attempted.
    pub budget_exhausted: bool,
}

impl ScanReport {
    pub fn succeeded(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, Outcome::Success { .. }))
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results.len() - self.succeeded()
    }

    /// Total plugin records extracted. Exceeds `succeeded()` when VST2 shells expand
    /// into their contained plugins.
    pub fn plugin_count(&self) -> usize {
        self.results
            .iter()
            .map(|r| match &r.outcome {
                Outcome::Success { plugins } => plugins.len(),
                Outcome::Error { .. } => 0,
            })
            .sum()
    }
}

pub struct Spawner {
    worker: PathBuf,
    timeout: Duration,
}

impl Spawner {
    /// Locate the worker binary and prepare a supervisor.
    pub fn new(timeout: Duration) -> Result<Self, PluginScanError> {
        Ok(Self {
            worker: locate_worker()?,
            timeout,
        })
    }

    /// Supervise a specific worker binary.
    ///
    /// The recovery paths only fire when a plugin crashes or hangs the scanner, which
    /// no healthy machine reproduces on demand; this lets a test stand in a stub
    /// worker that misbehaves deliberately.
    pub fn with_worker(worker: PathBuf, timeout: Duration) -> Self {
        Self { worker, timeout }
    }

    /// Scan every path, restarting the worker past any plugin that kills it.
    ///
    /// Returns one result per attempted path, in the order given. This never fails
    /// part-way: a plugin that crashes the scanner is an ordinary result carrying
    /// [`ErrorType::Crashed`], not an error in the scan itself.
    pub fn scan(&self, paths: &[PathBuf]) -> ScanReport {
        self.scan_with_progress(paths, &mut |_, _, _| {})
    }

    /// As [`scan`](Self::scan), reporting each plugin as it is attempted.
    ///
    /// The callback fires on the worker's `begin` line — *before* the plugin is
    /// loaded, not after. That is deliberate: a scan of a real library takes minutes,
    /// and the plugin a user most wants named is the one that is currently hanging,
    /// which by definition never reports a result.
    ///
    /// Arguments are `(index, total, path)`, with `index` counting from zero across
    /// the whole scan rather than the current batch.
    pub fn scan_with_progress(
        &self,
        paths: &[PathBuf],
        on_progress: &mut dyn FnMut(usize, usize, &Path),
    ) -> ScanReport {
        let mut results: Vec<PluginScanResult> = Vec::with_capacity(paths.len());
        let mut cursor = 0usize;
        let mut restarts = 0usize;
        let budget = paths.len().saturating_mul(RESTART_BUDGET_FACTOR);
        let mut budget_exhausted = false;

        while cursor < paths.len() {
            let batch = &paths[cursor..];
            let run = match self.run_batch(batch, cursor, paths.len(), on_progress) {
                Ok(run) => run,
                Err(e) => {
                    // The worker would not start at all. That is not a plugin's fault
                    // and retrying will not fix it, so stop rather than burn the
                    // budget spawning a binary that isn't there.
                    tracing::error!("Plugin scanner could not be started: {}", e);
                    break;
                }
            };

            for (offset, outcome) in run.completed {
                results.push(PluginScanResult {
                    path: batch[offset].clone(),
                    outcome,
                });
            }

            match run.interrupted {
                None => {
                    // Clean exit: the whole remaining batch is accounted for.
                    cursor = paths.len();
                }
                Some((offset, error_type)) => {
                    let culprit = &batch[offset];
                    tracing::warn!(
                        "Plugin scanner {} on {} -- recording it and resuming",
                        error_type,
                        culprit.display()
                    );
                    results.push(PluginScanResult {
                        path: culprit.clone(),
                        outcome: Outcome::Error {
                            error_type,
                            error: format!(
                                "The scanner subprocess {} while loading this plugin",
                                error_type
                            ),
                        },
                    });

                    // Resume after the offender.
                    cursor += offset + 1;
                    restarts += 1;

                    if restarts > budget {
                        tracing::error!(
                            "Plugin scanner restarted {} times; abandoning the remaining {} \
                             candidates",
                            restarts,
                            paths.len().saturating_sub(cursor)
                        );
                        budget_exhausted = true;
                        break;
                    }
                }
            }
        }

        ScanReport {
            results,
            restarts,
            budget_exhausted,
        }
    }

    /// Run one worker over `batch`, returning what it managed before stopping.
    fn run_batch(
        &self,
        batch: &[PathBuf],
        base: usize,
        total: usize,
        on_progress: &mut dyn FnMut(usize, usize, &Path),
    ) -> Result<BatchRun, PluginScanError> {
        let mut child = self.spawn(batch)?;

        // Feed the path list on stdin rather than argv: a large library would blow
        // past the command-line length limit. A dedicated thread keeps the write from
        // ever interleaving badly with our reads.
        let stdin = child.stdin.take().expect("stdin was piped");
        let list: Vec<String> = batch.iter().map(|p| p.display().to_string()).collect();
        thread::spawn(move || {
            let mut stdin = stdin;
            for path in &list {
                if writeln!(stdin, "{}", path).is_err() {
                    return; // Worker died early; nothing to do about it here.
                }
            }
            // Dropping stdin closes the pipe, which is how the worker knows the list
            // has ended.
        });

        // Surface the worker's diagnostics without letting them pollute stdout.
        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    tracing::debug!("vst-meta: {}", line);
                }
            });
        }

        let events = self.read_events(&mut child);
        let run = self.collect(events, batch, &mut child, base, total, on_progress);

        // Reap the child regardless of how we got here.
        let _ = child.wait();

        Ok(run)
    }

    fn spawn(&self, batch: &[PathBuf]) -> Result<Child, PluginScanError> {
        tracing::debug!(
            "Spawning {} for {} candidate(s)",
            self.worker.display(),
            batch.len()
        );

        let mut cmd = Command::new(&self.worker);
        cmd.arg("scan")
            .arg("--stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Keep a plugin that tries to open a window from flashing something at the
        // user during a background scan.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        Ok(cmd.spawn()?)
    }

    /// Move stdout onto a channel so the supervisor can apply a timeout to it.
    ///
    /// There is no portable way to read a pipe with a deadline, but a channel has
    /// `recv_timeout`. When we kill a hung worker the pipe closes, the reader thread
    /// unblocks, and the channel hangs up on its own.
    fn read_events(&self, child: &mut Child) -> Receiver<Event> {
        let stdout = child.stdout.take().expect("stdout was piped");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Event>(&line) {
                    Ok(event) => {
                        if tx.send(event).is_err() {
                            return; // Supervisor stopped listening.
                        }
                    }
                    Err(e) => tracing::warn!("Unparseable line from plugin scanner: {} ({})", line, e),
                }
            }
        });

        rx
    }

    /// Consume the event stream, tracking which path is in flight.
    fn collect(
        &self,
        events: Receiver<Event>,
        batch: &[PathBuf],
        child: &mut Child,
        base: usize,
        total: usize,
        on_progress: &mut dyn FnMut(usize, usize, &Path),
    ) -> BatchRun {
        let mut completed = Vec::new();
        // The worker processes paths in the order we gave them, so position in the
        // stream identifies the path. The echoed path is used only to sanity-check.
        let mut next = 0usize;
        let mut in_flight: Option<usize> = None;

        loop {
            match events.recv_timeout(self.timeout) {
                Ok(Event::Begin { path }) => {
                    if next < batch.len() && batch[next].display().to_string() != path {
                        tracing::warn!(
                            "Plugin scanner reported an unexpected path: got {}, expected {}",
                            path,
                            batch[next].display()
                        );
                    }
                    in_flight = Some(next);

                    if let Some(path) = batch.get(next) {
                        on_progress(base + next, total, path);
                    }
                }

                Ok(Event::Result { outcome, .. }) => {
                    if let Some(offset) = in_flight.take() {
                        completed.push((offset, outcome));
                        next = offset + 1;
                    } else {
                        tracing::warn!("Plugin scanner sent a result with nothing in flight");
                    }
                }

                Ok(Event::Done { .. }) => {
                    return BatchRun {
                        completed,
                        interrupted: None,
                    };
                }

                Err(RecvTimeoutError::Timeout) => {
                    // Silent for the whole timeout: whatever is loading has hung.
                    // Killing the process is the only way out -- a blocked thread
                    // cannot be safely terminated from inside.
                    let _ = child.kill();
                    return BatchRun {
                        interrupted: blame(in_flight, next, batch.len())
                            .map(|offset| (offset, ErrorType::Timeout)),
                        completed,
                    };
                }

                Err(RecvTimeoutError::Disconnected) => {
                    // stdout closed without a `Done`, so the worker died rather than
                    // finishing its list.
                    return BatchRun {
                        interrupted: blame(in_flight, next, batch.len())
                            .map(|offset| (offset, ErrorType::Crashed)),
                        completed,
                    };
                }
            }
        }
    }
}

/// Which path to blame when the worker stops without reporting a result.
///
/// Normally it is the one in flight -- that is what the `begin` line is for. When the
/// worker went quiet before announcing anything, blame the path it was about to
/// attempt instead: something is wrong either way, and the supervisor has to keep
/// moving or every remaining candidate is dropped without explanation.
fn blame(in_flight: Option<usize>, next: usize, batch_len: usize) -> Option<usize> {
    match in_flight {
        Some(offset) => Some(offset),
        None if next < batch_len => {
            tracing::warn!(
                "Plugin scanner stopped after {} of {} candidates without announcing the next",
                next,
                batch_len
            );
            Some(next)
        }
        // Everything was accounted for; the worker simply exited.
        None => None,
    }
}

/// What one worker run produced.
struct BatchRun {
    /// Results by offset into the batch.
    completed: Vec<(usize, Outcome)>,
    /// The path that was in flight when the worker stopped, and why. `None` means it
    /// finished the batch cleanly.
    interrupted: Option<(usize, ErrorType)>,
}

/// Find the sidecar worker binary.
fn locate_worker() -> Result<PathBuf, PluginScanError> {
    if let Some(path) = std::env::var_os(WORKER_PATH_ENV) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        tracing::warn!(
            "{} points at {}, which is not a file; falling back to the sidecar lookup",
            WORKER_PATH_ENV,
            path.display()
        );
    }

    let exe = std::env::current_exe()?;
    if let Some(dir) = exe.parent() {
        let candidate: &Path = &dir.join(WORKER_BIN);
        if candidate.is_file() {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(PluginScanError::WorkerNotFound(
        WORKER_BIN.to_string(),
        WORKER_PATH_ENV,
    ))
}
