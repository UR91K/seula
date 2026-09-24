# 0052. A project's id is assigned once, and files are linked to it

- **Status:** Accepted — not yet implemented
- **Recognized:** 2026-09-24, while answering whether the batch insert does all the
  batching it can
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** `src/project.rs` (`Uuid::new_v4()` on every parse); `src/database/batch.rs`
  (`INSERT OR REPLACE INTO projects`); `src/database/schema.sql` (`projects.path` is
  `UNIQUE`; six tables cascade from `projects`); `src/database/helpers.rs` (the same
  cascade trap, already avoided for plugins); `src/services/system.rs` (the watcher's
  `Renamed` event changes nothing in the database); `src/database/projects.rs`
  (`is_active = false` means both "deleted" and "archived"); ADR-0011 (a schema bump
  discards the database)

## Context

Every parse gave a project a new id, and the batch insert wrote it with `INSERT OR
REPLACE`. Because `path` is unique, a rescan of an edited project deleted the old row,
and the cascade took its tags, collections, tasks and audio files with it. A rename or
move was no better: the next scan saw an unknown path and made a new project, leaving
the old one pointing at a file that no longer existed.

What users do to `.als` files:

- **Save** (Ctrl+S): same path, new content.
- **Rename or move in Explorer**: same file and bytes, new path.
- **Save As to a new name, then delete the original**: Live cannot rename a set, so this
  is how a rename happens from inside Live. The new file is a fresh write: new path, new
  file-system id, and content that differs at least slightly.
- **Save As to a new name, keep the original**: how many people version a song
  (*Song v1*, *Song v2*). Both files stay, and both are real.

Keeping a project stable through two of these at once is hard. The goal is best effort.

## Decision

**A project's id is assigned once, when it is first seen, and is never derived from its
file.** A path, a content hash or a file-system id is evidence about which project a
file belongs to, not the project's identity. Each scan becomes a matching problem, and a
new id is minted only when nothing matches.

**A project can have several files.** `projects.path` stays the primary file, the one
the project shows and opens. A new `project_files` table lists every file linked to the
project, primary included. This follows `project_audio_files` next to
`projects.audio_file_id`: additive, so no `SCHEMA_VERSION` bump.

**The primary file is the newest linked file, unless the user has chosen one.** A choice
pins it: from then on, a newly linked file does not replace it, until the user picks
"use newest file", which unpins it and makes the newest primary again. The pin is a
flag per project, kept on its `project_files` row rather than as a column on
`projects`, for the same reason as the table.

**Links are made automatically only on exact evidence.** Each scan matches all its files
at once, in this order:

1. **A known path**: the file belongs to the project already linked to that path. This
   covers Save and rescans.
2. **A content hash or file-system id matching an orphan**: a project whose linked file
   has disappeared. This is a rename or move in Explorer. Only orphans are candidates: if
   the original file still exists, a same-hash file is a copy and gets its own project.
3. **Anything else** is a new project with a new id.

Following a rename or move in Explorer automatically is a feature, and the reason the
watcher exists. Its `Renamed` event updates the linked path as it happens, and the
scan's matching catches whatever happened while the watcher was not running.

A linked file that has disappeared is marked missing on its `project_files` row. It is
never deleted. A project whose files are all missing is shown as missing, which is
separate from archived. It keeps its tags and collections, and a later match brings it
back.

**Save As is suggested, never linked automatically.** A heuristic proposes "this file
belongs to that project", and the user confirms or rejects it. The same heuristic covers
Save As then delete (the candidate is an orphan) and Save As keeping the original (the
candidate is still present, so the new file would be linked as a version). Its signals,
strongest first:

- **Internal similarity**: track count, track names and colours in order, tempo, time
  signature, and overlap in plugins and samples. A Save As starts out as the same
  document, so these begin nearly identical.
- **A similar file name**: a low edit distance, or the same stem with something appended
  (*Song* → *Song v2*, *Song final*).
- **The same project folder**, as a weak signal only: some folders hold several `.als`
  files that are entirely different songs.

Until the user confirms, the new file is its own project. A rejected suggestion is
remembered, so it is not offered again.

**Linking and unlinking can always be done by hand**, for whatever the heuristic misses.

## Rejected alternatives

- **An id derived from the path.** It changes on every rename and move, which is the
  case this decision exists for.
- **An id derived from the content hash.** It changes on every save.
- **Linking Save As matches automatically.** A wrong link puts one song's tags,
  collections and tasks on another, silently. A missed link only leaves them on a
  missing project, where a manual link recovers them. The costs are not symmetrical.
- **One file per project.** It cannot express versions, and a Save As that keeps the
  original would have to become an unrelated project.
- **New columns on `projects`** for the file-system id or a missing flag. That is a
  change to an existing table and needs a `SCHEMA_VERSION` bump, which discards the
  user's database (ADR-0011), including the tags this decision exists to protect.

## Consequences

- The parser records track names and colours in order, in its single pass (ADR-0001).
- The file-system id (the NTFS file ID; inode and device elsewhere) needs a platform call.
  It survives a rename or move within a volume, and a Save followed by a rename before
  the next scan, if Live saves in place. Whether Live saves in place has not been
  checked.
- The frontend needs a way to review suggestions, and a manual link and unlink action.
- Search, statistics and every "per project" count have to decide what a project with
  several files means for them. Most count the project once, from its primary file.
- The watcher's `Renamed` event, which today changes nothing in the database, has to
  update the linked path. The scan's matching must still work alone, because the
  watcher does not always run.
- A heuristic can be wrong in both directions. Suggesting rather than linking bounds the
  damage to a dismissed popup.

## Notes

Suggested build order, each step useful alone:

1. Keep the id on a known path: the fix for the rescan that loses tags.
2. `project_files`, with the missing state separate from archived.
3. Automatic orphan matching by content hash, then by file-system id.
4. Manual link and unlink.
5. Track names and colours in the parser, then the suggestion heuristic and its review UI.
