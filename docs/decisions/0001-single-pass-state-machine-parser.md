# 0001. Parse `.als` in a single pass with a state machine

- **Status:** Accepted (retrospective)
- **Recognized:** ~mid-2024
- **Decided:** 2025-01-10 (`8ee0622`) through 2025-01-13 (`d68cdde`)
- **Recorded:** 2026-09-15
- **Confidence:** remembered, corroborated by git
- **Evidence:** deleted `utils/tempo.rs`, `utils/version.rs`, `utils/xml_parsing.rs`
  (recoverable via `git show 8ee0622^:src/utils/tempo.rs`); `ScanOptions` introduced in
  `fffc4f6`; `ParserState` introduced in `d68cdde`

## Context

The original parser had one extractor per datum, each opening its own reader over the
same bytes. The recovered `find_post_10_tempo` from `utils/tempo.rs` is representative:

```rust
pub(crate) fn find_post_10_tempo(xml_data: &[u8]) -> Result<f64, TempoError> {
    let mut reader = Reader::from_reader(xml_data);
    // ... walks the entire file to return one f64
}
```

Tempo, version, plugins, samples, time signature and the rest each cost a full traversal
of a multi-megabyte XML document. With thousands of projects, the file was being walked
seven or eight times over.

The problem was understood well before it was fixed. Three separate commits in
2024 — `a75d7bd` "Optimised finding plugins", `516c452` "Sample parsing: optimized",
`5157dd9` "Plugin parsing: optimized" — tune individual passes without changing the
structure. In 2024-12 (`fffc4f6`) `ScanOptions` appeared:

```rust
pub struct ScanOptions {
    pub scan_plugins: bool, pub scan_samples: bool, pub scan_tempo: bool,
    pub scan_time_signature: bool, pub scan_midi: bool, /* ... */
}
```

A boolean per datum is only necessary if each datum costs a pass. That struct is a
mitigation, not a feature — and the same commit deletes a commented-out copy of it, so
the idea had been sitting unexecuted in the source for some time before it was made
real.

The delay was partly other priorities, and partly that the design had not crystallized:
it was not clear what a single-pass parser needed to look like until it was. The second
half of that is deferral done correctly.

## Decision

One traversal. `Scanner` in `src/scan/parser.rs` holds a `ParserState` enum plus depth
tracking, and accumulates every datum as the events go past. Per-datum extractors and
`ScanOptions` were both deleted.

## Rejected alternatives

- **Keep multiple passes, optimise each.** Tried for roughly eight months across three
  commits. Constant-factor gains against a structural problem.
- **`ScanOptions` — let callers switch off passes they cannot afford.** Shipped as a
  stopgap two weeks before the rewrite, then removed. It made the cost configurable
  rather than absent, and pushed the decision onto every caller.
- **Load the whole document into a DOM and query it.** `elementtree` is still in
  `Cargo.toml`. Rejected on memory: these files reach tens of megabytes decompressed,
  and thousands are parsed per scan.

## Consequences

The single largest performance win in the project; the ~160–270 MB/s figure in the
README depends on it.

The cost is real and visible. `parser.rs` is long, stateful, and hard to follow.
Correctness now depends on guard flags whose purpose is not locally obvious —
`plugin_info_processed` stops a second name overwriting the first, `in_vst3_preset`
stops a preset's `<Name>` being read as the plugin's. Both encode bugs found the hard
way. The author recalls the rewrite as annoying to write, and it sat recognized for
roughly sixteen months before being attempted, which is itself evidence of the cost.

## Notes

**Do not split this back into per-datum extractors.** It will look like an obvious
simplification. It is the design this replaced, the refactor is expensive, and the
regression is silent — results stay correct, only throughput collapses.

If the state machine needs extending, add states and guards rather than adding passes.
