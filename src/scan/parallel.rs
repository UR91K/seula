use tracing::{debug, trace};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::error::LiveSetError;
use crate::project::Project;
use crate::scan::daw::{AnyDawParser, ParseError};

/// Result type for parsing operations
type ParseResult = Result<(PathBuf, Project), (PathBuf, ParseError)>;

/// Worker for parsing individual project files
pub struct ParserWorker {
    sender: Sender<ParseResult>,
}

impl ParserWorker {
    fn new(sender: Sender<ParseResult>) -> Self {
        Self { sender }
    }

    fn process_file(&self, path: PathBuf) {
        let result = match AnyDawParser::for_path(&path) {
            Some(parser) => parser
                .parse(&path)
                .map(|project| (path.clone(), project))
                .map_err(|err| (path, err)),
            None => Err((path.clone(), ParseError::UnsupportedFileType(path))),
        };

        // Send result back to coordinator
        let _ = self.sender.send(result);
    }
}

/// Pull items off a shared receiver, handing each to `f`.
///
/// The lock **must** be released before `f` runs. Writing this as
/// `while let Ok(item) = rx.lock().unwrap().recv() { f(item) }` looks equivalent and
/// is not: temporaries created in the scrutinee of a `while let` (or `match`) live
/// until the end of the block, so the `MutexGuard` would stay alive for the whole
/// body. Every worker would then hold the queue lock for the entire duration of its
/// parse, and the pool would run strictly one file at a time while still reporting
/// N threads. Rust 2024's `if let` rescoping does not extend to `while let`.
///
/// This existed as that exact bug until 2026-09-15; `worker_loop_runs_bodies_concurrently`
/// is the regression test. Binding the item first is the whole fix.
fn drain<T>(rx: &Arc<Mutex<Receiver<T>>>, mut f: impl FnMut(T)) {
    loop {
        // Bind first: this statement ends here, so the guard is dropped before `f`.
        let item = match rx.lock().unwrap().recv() {
            Ok(item) => item,
            Err(_) => break, // Sender dropped; no more work is coming.
        };
        f(item);
    }
}

/// Manages parallel parsing of Live Set files
pub struct ParallelParser {
    #[allow(unused)]
    thread_count: usize,
    workers: Vec<JoinHandle<()>>,
    results_rx: Receiver<ParseResult>,
    work_tx: Arc<Mutex<Option<Sender<PathBuf>>>>,
}

impl ParallelParser {
    /// Create a new parallel parser with specified thread count
    pub fn new(thread_count: usize) -> Self {
        let (results_tx, results_rx): (Sender<ParseResult>, Receiver<ParseResult>) = channel();
        let (work_tx, work_rx): (Sender<PathBuf>, Receiver<PathBuf>) = channel();
        let work_tx = Arc::new(Mutex::new(Some(work_tx)));
        let work_rx = Arc::new(Mutex::new(work_rx));

        // Create worker threads
        let mut workers = Vec::with_capacity(thread_count);
        for thread_id in 0..thread_count {
            let results_tx = results_tx.clone();
            let work_rx = Arc::clone(&work_rx);

            let handle = thread::spawn(move || {
                trace!("Worker thread {} started", thread_id);
                let worker = ParserWorker::new(results_tx);

                drain(&work_rx, |path| {
                    trace!("Worker {} processing file: {}", thread_id, path.display());
                    worker.process_file(path);
                });
                trace!("Worker thread {} exiting", thread_id);
            });

            workers.push(handle);
        }

        Self {
            thread_count,
            workers,
            results_rx,
            work_tx,
        }
    }

    /// Submit paths for parsing
    pub fn submit_paths(&self, paths: Vec<PathBuf>) -> Result<(), LiveSetError> {
        debug!("Submitting {} paths to worker threads", paths.len());
        if let Some(tx) = self.work_tx.lock().unwrap().as_ref() {
            for path in paths {
                trace!("Sending path to worker: {}", path.display());
                tx.send(path).map_err(|_| {
                    LiveSetError::InvalidProject("Failed to send path to worker thread".to_string())
                })?;
            }
            trace!("Finished submitting all paths");
            Ok(())
        } else {
            Err(LiveSetError::InvalidProject(
                "Worker threads are no longer available".to_string(),
            ))
        }
    }

    /// Get receiver for parsing results
    pub fn get_results_receiver(&self) -> &Receiver<ParseResult> {
        &self.results_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    /// The pool must actually run work in parallel.
    ///
    /// This is a silent failure mode: with the queue lock held across the body, the
    /// parser still produces correct results and still reports N threads -- it just
    /// runs them one at a time. Nothing but observed concurrency catches it, which is
    /// why the existing result-correctness tests missed it for the code's whole life.
    #[test]
    fn worker_loop_runs_bodies_concurrently() {
        const THREADS: usize = 4;
        const ITEMS: usize = 8;

        let (work_tx, work_rx) = channel::<usize>();
        let work_rx = Arc::new(Mutex::new(work_rx));

        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::with_capacity(THREADS);
        for _ in 0..THREADS {
            let work_rx = Arc::clone(&work_rx);
            let in_flight = Arc::clone(&in_flight);
            let max_in_flight = Arc::clone(&max_in_flight);

            handles.push(thread::spawn(move || {
                drain(&work_rx, |_item| {
                    // Stand-in for LiveSet::new() on a multi-megabyte file. The sleep
                    // guarantees an overlap window wide enough that genuine
                    // parallelism is observed rather than merely possible.
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    max_in_flight.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(30));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            }));
        }

        for i in 0..ITEMS {
            work_tx.send(i).unwrap();
        }
        drop(work_tx); // Closes the queue so the workers exit.

        for handle in handles {
            handle.join().unwrap();
        }

        let observed = max_in_flight.load(Ordering::SeqCst);
        // Deliberately not asserting the full THREADS: a loaded or single-core CI box
        // may not get all four overlapping, but anything above 1 proves the guard is
        // released. The bug pins this at exactly 1.
        assert!(
            observed > 1,
            "workers ran serially (max concurrent = {observed}); the queue lock is \
             being held across the loop body"
        );
    }

    /// The loop must terminate when the sender is dropped, or `ParallelParser::drop`
    /// would block forever joining its workers.
    #[test]
    fn drain_stops_when_the_sender_is_dropped() {
        let (work_tx, work_rx) = channel::<usize>();
        let work_rx = Arc::new(Mutex::new(work_rx));

        work_tx.send(1).unwrap();
        work_tx.send(2).unwrap();
        drop(work_tx);

        let mut seen = Vec::new();
        drain(&work_rx, |item| seen.push(item));

        assert_eq!(seen, vec![1, 2], "queued work must drain before exiting");
    }
}

impl Drop for ParallelParser {
    fn drop(&mut self) {
        trace!("ParallelParser being dropped, signaling workers to stop");
        // Drop work sender to signal workers to stop
        self.work_tx.lock().unwrap().take();

        trace!("Waiting for {} workers to complete", self.workers.len());
        // Wait for all workers to complete
        for (i, worker) in self.workers.drain(..).enumerate() {
            trace!("Waiting for worker {} to complete", i);
            let _ = worker.join();
            trace!("Worker {} completed", i);
        }
        debug!("All workers completed");
    }
}
