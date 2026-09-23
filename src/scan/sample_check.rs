//! Checking sample files on disk: whether each is still there, and how big it is
//! (ADR-0041).
//!
//! A library is tens of thousands of files, often on slow or network drives, and
//! asking for each file's metadata opens it. Samples cluster in folders, so the check
//! lists each folder once instead. On Windows a directory listing returns every
//! entry's size and times without opening any file. A folder holding only one or two
//! of the samples is cheaper to ask about file by file.
//!
//! Folders are shared out over a few threads, because the time goes to waiting on the
//! disk, not to the CPU. Each thread takes the next folder from an atomic counter; no
//! queue or lock is shared (compare ADR-0002).

use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::UNIX_EPOCH;

/// What the check found for one sample file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleFileState {
    pub size_bytes: u64,
    /// The file's modification time, epoch seconds, when the filesystem reports one.
    pub modified_at: Option<i64>,
}

/// Fewer samples than this in one folder are checked one by one rather than by
/// listing the folder, which could hold thousands of other files.
const LIST_THRESHOLD: usize = 3;

/// A thread count for an I/O-bound check: twice the cores, at most 16.
pub fn default_threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get() * 2).min(16)
}

/// Check every path, returning what was found for each, or `None` when it is not
/// there. `on_progress(folders_done, folders_total, folder)` runs on the calling
/// thread as each folder finishes, so it need not be `Send`.
pub fn check_sample_files(
    paths: &[String],
    threads: usize,
    on_progress: &mut dyn FnMut(usize, usize, &Path),
) -> HashMap<String, Option<SampleFileState>> {
    let mut by_folder: HashMap<PathBuf, Vec<&str>> = HashMap::new();
    for path in paths {
        let folder = Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default();
        by_folder.entry(folder).or_default().push(path);
    }
    let folders: Vec<(PathBuf, Vec<&str>)> = by_folder.into_iter().collect();
    let total = folders.len();

    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<(usize, Vec<(String, Option<SampleFileState>)>)>();
    let mut found = HashMap::with_capacity(paths.len());

    std::thread::scope(|scope| {
        for _ in 0..threads.clamp(1, total.max(1)) {
            let tx = tx.clone();
            let (next, folders) = (&next, &folders);
            scope.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((folder, files)) = folders.get(i) else { break };
                if tx.send((i, check_folder(folder, files))).is_err() {
                    break;
                }
            });
        }
        // The workers hold the only senders left, so the loop below ends when they do.
        drop(tx);
        for (done, (i, results)) in rx.iter().enumerate() {
            on_progress(done + 1, total, &folders[i].0);
            found.extend(results);
        }
    });
    found
}

fn check_folder(folder: &Path, files: &[&str]) -> Vec<(String, Option<SampleFileState>)> {
    let one_by_one = || files.iter().map(|p| (p.to_string(), stat(Path::new(p)))).collect();
    if files.len() < LIST_THRESHOLD {
        return one_by_one();
    }
    let Ok(entries) = fs::read_dir(folder) else {
        // Gone, or unreadable as a folder. Asking file by file tells the two apart.
        return one_by_one();
    };
    let listing: HashMap<String, SampleFileState> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            // A listing describes a link, not what it points at.
            let meta = if kind.is_symlink() { fs::metadata(entry.path()).ok()? } else { entry.metadata().ok()? };
            meta.is_file().then(|| (name_key(&entry.file_name().to_string_lossy()), state_of(&meta)))
        })
        .collect();
    files
        .iter()
        .map(|p| {
            let name = Path::new(p).file_name().map(|n| name_key(&n.to_string_lossy()));
            (p.to_string(), name.and_then(|n| listing.get(&n).copied()))
        })
        .collect()
}

fn stat(path: &Path) -> Option<SampleFileState> {
    fs::metadata(path).ok().filter(Metadata::is_file).map(|m| state_of(&m))
}

fn state_of(meta: &Metadata) -> SampleFileState {
    SampleFileState {
        size_bytes: meta.len(),
        modified_at: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
    }
}

/// File names compare the way the filesystem does: without case on Windows.
fn name_key(name: &str) -> String {
    if cfg!(windows) {
        name.to_lowercase()
    } else {
        name.to_string()
    }
}
