# 0042. Browse samples as a folder tree

- **Status:** Proposed — not scheduled
- **Recognized:** 2026-09-23, planning the samples board
- **Decided:** not yet
- **Recorded:** 2026-09-23
- **Confidence:** decided now (that it is worth recording, not that it will be built)
- **Evidence:** the planning discussion for the samples board; `samples.path`, the only
  structure a sample has

## Context

The plugins view groups by vendor or format, from rollup routes that already existed.
For samples, grouping by format was on offer, and the maintainer did not find it useful.
What would be useful is the folder a sample lives in: a sample pack, a recording folder,
a project's own `Samples` folder. A sample's path is the only structure it has, and it
is a tree.

The maintainer's view: not needed for the first pass, but worth recording, as a tree
rather than a grouping.

## Proposal

A folder tree beside or in place of the flat sample list, in the manner of Explorer's
navigation pane:

- Each node is a folder that holds samples, directly or below it, with its sample count,
  how many are missing, and its measured size (ADR-0041).
- Selecting a node lists that folder's samples, optionally including subfolders.
- Chains of folders that hold nothing but one subfolder collapse into one node
  (`C:\Users\producer\Music\Samples` rather than four levels), so the tree starts where
  the samples do.

What the API would need, roughly: a route that returns the folder rollup for a prefix,
one level at a time (`GET /api/v1/samples/folders?parent=...`), and a `path_prefix`
filter on the list, search and stats routes. Both can come from `samples.path` with
prefix matching; nothing new needs storing.

## Open questions

- Windows paths from more than one drive, and paths from another machine when projects are
  shared: one tree with a root per drive, or per machine?
- Whether a folder's samples include its subfolders by default.
- Whether the tree replaces the format filter or sits alongside it.

## Consequences

None until built. The flat list with a format filter (ADR-0039) is the first pass.
