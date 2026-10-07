# 0067. Plugin scans skip unchanged files and remember failures

- **Status:** Accepted
- **Recognized:** 2026-10-07, when the maintainer asked for a quick rescan like the
  project scanner's, and for a Failed status beside Installed, Missing and Not scanned
- **Decided:** 2026-10-07, by the maintainer: both kinds of Failed below, and a scan for
  changes does not retry failures
- **Recorded:** 2026-10-07
- **Confidence:** decided now; measured on the maintainer's machine against a copy of
  their database: a rescan of all 334 plugin files took 151 s, and a scan for changes
  straight after took 100 ms
- **Evidence:** `plugin_files` and `plugin_file_plugins` in `src/database/schema.sql`;
  `persist_plugin_scan_of`, `unchanged_plugin_files`, `plugin_scan_errors` and
  `failed_plugin_files` in `src/database/plugin_scan.rs`; `scan_files` and `ScanMode` in
  `src/scan/plugins/mod.rs`; `stat` in `src/scan/plugins/discovery.rs`; the tests in
  `tests/database/plugin_scan.rs` from `only_new_and_changed_files_need_loading` on

## Context

Every plugin scan loaded every plugin file in the worker (ADR-0004), and every one cost
its full load time. The worst cost the most: a binary that hangs costs the whole timeout,
30 s by default, and one that crashes costs a worker restart. The maintainer's machine
has 334 plugin files; 44 of them fail every time, 42 because they are not plugins at all.
A scan took two and a half minutes, so the cost of "did I install anything?" was the same
as a first scan.

A scan could not skip a file, because nothing remembered files. `plugins` is one row per
identity (ADR-0010), and `plugins.path` is one location of possibly several; ADR-0010
itself warns that a staleness check against it reasons about one arbitrary copy. A
plugin's identity only comes from loading its binary, so without a record per file there
was no way to know which plugin an unloaded file was. Failures were not recorded at all.

That also made a failing plugin look missing. If a plugin that scanned fine started to
crash the worker after an update, the scan never saw its uid and marked it not installed,
while its file was still there.

## Decision

**A record per file.** `plugin_files` holds each file a scan loaded or tried to: its size
and mtime, when, and how it failed if it did. For a VST3 bundle directory the size is the
total of the files inside it and the mtime the newest, because an installer can replace
the binary deep inside without touching the directory. `plugin_file_plugins` links a file
to the plugins it yielded when it last loaded: several, for a shell plugin. A file that
then fails keeps its links. Both are new tables, so `SCHEMA_VERSION` is unchanged.

**Two modes.** A scan for changes, the default everywhere except the gRPC API, still walks
every folder and stats every file, which loads nothing. It then loads only files that are
new or whose size or mtime changed. Unchanged files keep what they gave last time, their
failures included: a failed file is not retried until it changes or the user asks.
Rescan all loads every file. A file whose mtime cannot be read never counts as unchanged.

**The sweep is per file.** A full scan forgets the files it no longer found, then marks
not installed every plugin no remaining file yields. A scan for changes still finds every
file, so it can still sweep (ADR-0012). A partial scan neither forgets nor sweeps, as
before.

**Failed, two ways.**

- A plugin whose every file failed its last scan, having loaded in an earlier one, stays
  `installed = 1`, since its file is there. The HTTP API adds a `scan_error` to it from
  `plugin_files`, and the plugins view shows it as Failed rather than Installed. Failed is
  not a new value of `installed`. That column keeps its meaning, and changing an existing
  column would discard every database (ADR-0011).
- A file that failed and has never loaded has no identity, so no plugin row. The parser
  does not keep plugin file names from projects, so it cannot be matched to a project's
  plugin either. `GET /api/v1/plugins/failed-files` lists these, and the plugins view
  shows each as a Failed row named after its file. Files that failed as `invalid_format`
  are left out: they are not plugins, such as installer helper DLLs.

**Where the modes are offered.** The plugins view's Scan button runs a scan for changes,
and its caret menu offers both. `POST /api/v1/plugins/scan?mode=all`, and the same on
`refresh-installation-status`; `seula plugin refresh --all`. The first-run scan
(ADR-0013) loads everything, since nothing is recorded, and records it. The gRPC
`RefreshPluginInstallationStatus` always loads everything: that API has no way to ask
for less and is being retired (ADR-0046).

## Rejected alternatives

- **Staleness against `plugins.path`.** Rejected for ADR-0010's reason: it is one copy
  of possibly several, and a file that never loaded has no row to hold it.
- **Retrying failures on every scan for changes.** Rejected by the maintainer: failures
  are the slowest files, so retrying them keeps the scan slow. Retrying only timeouts,
  which can be one-off, was offered and not chosen. Rescan all, or the file changing, is
  the retry.
- **A Failed value in `plugins.installed`.** Rejected: changing an existing column means
  a `SCHEMA_VERSION` bump, which discards the user's database (ADR-0011). It would also
  lose what Installed means, that the file is there.
- **Matching never-loaded files to project plugins by file name.** Not possible today:
  the parser does not keep a plugin's file name or path from the project. It could be
  added, and would then let a Failed file row stand in for a Missing plugin row.
- **Rescanning when a plugin folder changes**, through the file watcher. Not done: it
  loads third-party binaries without the user asking, which deserves its own decision.
  The per-file record makes it possible later.

## Consequences

A scan for changes after an install loads just the new plugin. Failing plugins no longer
read as missing, and failures stop costing time on every scan.

The costs:

- Size and mtime are a heuristic. A file replaced by one of the same size and mtime is
  not reloaded; Rescan all is the way out.
- The completion counts for a scan for changes cover only what it loaded. Its "failed to
  load" leaves out files skipped as unchanged, so zero counts are left out of the
  message rather than read as "nothing failed".
- `plugin_files` now records every location of a plugin, the information ADR-0010 chose
  not to keep. Only the scan reads it so far. A "you have two copies" view, which ADR-0010
  said needed rescanning, could now read it instead.

## Notes

Reversing is cheap: the two tables can be dropped, and `persist_plugin_scan` without file
information behaves like the old scan, because a file with no mtime is never unchanged.
