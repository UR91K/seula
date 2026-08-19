//! Finding plugin binaries on disk.
//!
//! This runs in the main process on purpose. Walking directories is ordinary, safe
//! file I/O -- nothing here opens a plugin -- and the parent needs the full candidate
//! list up front anyway so the supervisor can resume past a plugin that kills the
//! worker.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::defaults::default_vst_search_paths;

/// How deep to descend into a search root.
///
/// Vendors nest one or two levels (`Common Files/VST3/FabFilter/…`), and a VST3
/// "bundle" is itself a directory we must not walk into. A shallow cap keeps a
/// misconfigured root -- someone pointing this at `C:\` -- from turning into a
/// full-disk crawl.
const MAX_DEPTH: usize = 6;

/// Extensions that identify a plugin. `.vst3` may be either a bundle directory or a
/// flat file depending on the vendor and platform; both are handled.
const PLUGIN_EXTENSIONS: [&str; 4] = ["vst3", "dll", "vst", "so"];

/// Collect candidate plugin paths from the configured search roots.
///
/// `roots` empty means "use the platform defaults". Roots that do not exist are
/// skipped silently -- the default list names every conventional location, and most
/// machines have only some of them.
pub fn discover(roots: &[PathBuf]) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = if roots.is_empty() {
        default_vst_search_paths()
    } else {
        roots.to_vec()
    };

    // The default roots overlap on some setups, and a user may list a directory twice.
    // Dedupe so a plugin is never scanned (or reported) more than once.
    let mut seen = HashSet::new();
    let mut found = Vec::new();

    for root in roots {
        if !root.is_dir() {
            log::debug!(
                "Skipping VST search path (not a directory): {}",
                root.display()
            );
            continue;
        }

        let mut walker = WalkDir::new(&root)
            .max_depth(MAX_DEPTH)
            .follow_links(false)
            .into_iter();

        while let Some(entry) = walker.next() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // An unreadable subdirectory is common enough on a system drive
                    // that it should not abort the walk.
                    log::debug!("Skipping unreadable entry during plugin discovery: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            let is_dir = entry.file_type().is_dir();

            // A VST3 "bundle" is a directory that *is* the plugin. Take it whole and
            // do not descend, or its internal binary would surface as a second,
            // bogus candidate for the same plugin.
            if is_dir {
                if has_extension(path, "vst3") {
                    if seen.insert(path.to_path_buf()) {
                        found.push(path.to_path_buf());
                    }
                    walker.skip_current_dir();
                }
                continue;
            }

            if PLUGIN_EXTENSIONS.iter().any(|ext| has_extension(path, ext))
                && seen.insert(path.to_path_buf())
            {
                found.push(path.to_path_buf());
            }
        }
    }

    found.sort();
    found
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_plugins_and_ignores_other_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Thing.dll"), b"x").unwrap();
        fs::write(dir.path().join("readme.txt"), b"x").unwrap();
        fs::write(dir.path().join("Preset.fxp"), b"x").unwrap();

        let found = discover(&[dir.path().to_path_buf()]);

        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("Thing.dll"));
    }

    #[test]
    fn treats_a_vst3_bundle_as_one_candidate() {
        // A real bundle nests the actual binary several levels down, under a name
        // that would otherwise match on its own.
        let dir = TempDir::new().unwrap();
        let bundle = dir.path().join("Serum.vst3");
        let inner = bundle.join("Contents").join("x86_64-win");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("Serum.vst3"), b"x").unwrap();

        let found = discover(&[dir.path().to_path_buf()]);

        assert_eq!(found.len(), 1, "bundle should yield exactly one candidate");
        assert_eq!(found[0], bundle);
    }

    #[test]
    fn descends_into_vendor_subdirectories() {
        let dir = TempDir::new().unwrap();
        let vendor = dir.path().join("FabFilter");
        fs::create_dir_all(&vendor).unwrap();
        fs::write(vendor.join("Pro-Q 3.dll"), b"x").unwrap();

        let found = discover(&[dir.path().to_path_buf()]);

        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("Pro-Q 3.dll"));
    }

    #[test]
    fn dedupes_overlapping_roots() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Thing.dll"), b"x").unwrap();

        let root = dir.path().to_path_buf();
        let found = discover(&[root.clone(), root]);

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn missing_roots_are_skipped_not_fatal() {
        let found = discover(&[PathBuf::from("/definitely/not/a/real/path")]);
        assert!(found.is_empty());
    }
}
