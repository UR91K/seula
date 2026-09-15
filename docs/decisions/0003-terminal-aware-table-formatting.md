# 0003. Replace `comfy-table` with terminal-aware `SimpleTable`

- **Status:** Accepted (retrospective)
- **Decided:** 2025-08-28 (`92b3ab8`), Unicode correctness followed in `3d0bcef`
- **Recorded:** 2026-09-15
- **Confidence:** reconstructed from git
- **Evidence:** `92b3ab8` commit message and diff; `src/cli/output.rs`

## Context

CLI output used `comfy-table` for every listing. Bordered tables wrapped or corrupted on
narrow terminals, and the borders themselves consumed width that the data needed.

## Decision

`SimpleTable` in `src/cli/output.rs`: detect terminal width via `terminal_size`, size
columns proportionally, truncate cell content with an ellipsis, and drop borders
entirely in favour of a single header rule.

Every `TableDisplay` implementation across the ten command modules renders through it,
so table/JSON/CSV output stays uniform for free.

## Rejected alternatives

- **Keep `comfy-table`, configure it harder.** Its width handling was the problem, not
  its defaults.
- **Fixed-width columns.** Simple, but degrades badly in both directions — wasteful on
  wide terminals, unreadable on narrow ones.

## Consequences

Output is legible at any terminal width and no longer spends columns on box drawing.

The cost is owning the formatting code, including its edge cases. One surfaced
immediately: width was measured with `.len()`, which counts bytes, and truncation
sliced on a byte index — so any non-ASCII plugin or project name would panic rather
than render narrow. Fixed in `3d0bcef` by measuring with `unicode-width` and truncating
on character boundaries, accounting for CJK and emoji double-width and zero-width
combining marks.

Note that `comfy-table` is still declared in `Cargo.toml` despite the commit message
saying it was replaced. Unused; safe to drop.

## Notes

New output types should implement `TableDisplay` rather than printing directly —
that is what keeps `--format json` and `--format csv` working without per-command
effort. See `SystemScanDisplay` in `src/cli/commands/plugin.rs` for a recent example,
including the case where a summary line is printed as a *message* rather than a table
row, because table column widths will truncate it.
