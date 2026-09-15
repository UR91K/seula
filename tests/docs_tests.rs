//! Keeps the documentation honest about the code it describes.
//!
//! The most common way docs rot is silently: a file moves or a module is renamed, and
//! every document that pointed at it becomes wrong with nothing to signal it. Docs that
//! name real paths are greppable and reviewable; docs that name paths which no longer
//! exist are worse than none, because they are confidently wrong.
//!
//! This walks the Markdown under `docs/` plus `CLAUDE.md` and checks that every
//! repo-relative source path it mentions still resolves. It deliberately does not check
//! prose, line numbers, or anything else subjective.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Documents that are checked. `docs/archive/` is excluded on purpose: it is a record
/// of superseded plans, and its paths are *expected* to be stale.
fn documents(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![root.join("CLAUDE.md")];
    collect_markdown(&root.join("docs"), &mut found);
    found.retain(|p| !p.components().any(|c| c.as_os_str() == "archive"));
    found
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// Pull out anything that looks like a repo-relative path into source we control.
///
/// Scoped to the directories that actually exist in this repo so that prose like
/// "config.toml" or a plugin path from example output is not mistaken for a claim about
/// the tree.
fn referenced_paths(text: &str) -> BTreeSet<String> {
    const ROOTS: [&str; 5] = ["src/", "crates/", "tests/", "examples/", "proto/"];

    let mut found = BTreeSet::new();
    for raw in text.split(|c: char| c.is_whitespace() || "()[]`\"'*,;<>|".contains(c)) {
        // `8ee0622^:src/utils/tempo.rs` -- git's rev:path syntax names a file *as it
        // was*, so a path that no longer exists is the whole point.
        if raw.contains("^:") {
            continue;
        }
        // Earliest match wins, not first-in-list: otherwise `crates/vst-meta/src/x.rs`
        // matches on the inner `src/` and gets truncated to a path that never existed.
        let Some(start) = ROOTS.iter().filter_map(|r| raw.find(r)).min() else {
            continue;
        };
        let candidate = &raw[start..];

        // Trim locators and trailing punctuation: `file.rs:922`, `file.rs#L40`, "file.rs."
        let candidate = candidate
            .split([':', '#'])
            .next()
            .unwrap_or(candidate)
            .trim_end_matches(['.', ',', ')', '`']);

        // Only check concrete files. Directory mentions are prose; globs and
        // `{placeholder}` templates describe a shape rather than naming a file.
        if candidate.contains('*')
            || candidate.contains('{')
            || candidate.contains('}')
            || !candidate.contains('.')
        {
            continue;
        }
        found.insert(candidate.to_string());
    }
    found
}

#[test]
fn documented_paths_still_exist() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut broken: Vec<String> = Vec::new();

    for doc in documents(&root) {
        let Ok(text) = fs::read_to_string(&doc) else {
            continue;
        };
        let rel_doc = doc.strip_prefix(&root).unwrap_or(&doc).display().to_string();

        for path in referenced_paths(&text) {
            if !root.join(&path).exists() {
                broken.push(format!("  {rel_doc} -> {path}"));
            }
        }
    }

    assert!(
        broken.is_empty(),
        "Documentation references paths that no longer exist:\n{}\n\n\
         Update the doc, or move the file back. If a path is intentionally historical, \
         describe it in prose rather than as a path.",
        broken.join("\n")
    );
}

#[test]
fn every_adr_declares_a_status_and_confidence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let decisions = root.join("docs").join("decisions");

    let mut checked = 0;
    let mut problems = Vec::new();

    for entry in fs::read_dir(&decisions).expect("docs/decisions must exist").flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("0000") {
                continue; // the template
            }
            let text = fs::read_to_string(&path).unwrap();
            // Both fields are what make a retrospective ADR trustworthy: one says
            // whether the decision still stands, the other how much weight its stated
            // rationale can bear.
            if !text.contains("**Status:**") {
                problems.push(format!("  {name}: no **Status:**"));
            }
            if !text.contains("**Confidence:**") {
                problems.push(format!("  {name}: no **Confidence:**"));
            }
            checked += 1;
        }
    }

    assert!(checked > 0, "no ADRs found in docs/decisions");
    assert!(problems.is_empty(), "Malformed ADRs:\n{}", problems.join("\n"));
}
