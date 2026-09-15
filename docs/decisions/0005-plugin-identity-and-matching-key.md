# 0005. Match plugins on `(format, uid)`, derived from Ableton's `dev_identifier`

- **Status:** Accepted
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now; the byte-order claim is empirically verified
- **Evidence:** fixtures in `tests/scan/parser/plugins.rs`; live scan output from
  `crates/vst-meta`

## Context

A project file references a plugin by a string Ableton invents, e.g.
`device:vst3:audiofx:72c4db71-7a4d-459a-b97e-51745d84b39d`. Plugin binaries know
nothing about that format, so it looked like matching scanned plugins to project
references would need fuzzy name-and-vendor heuristics.

It does not. The `dev_identifier` is an Ableton-specific *encoding* of the plugin's own
native identifier, and the payload inside is standard.

**VST3.** The fixture `device:vst3:audiofx:13b117f4-1b21-3a38-7923-ff895d3b3131` has a
suffix which, with dashes stripped, is exactly the VST3 class ID. The same `.als` also
carries it independently:

```xml
<Uid>
  <Fields.0 Value="330373108" />   <!-- 0x13b117f4 -->
  <Fields.1 Value="455162424" />   <!-- 0x1b213a38 -->
  <Fields.2 Value="2032402313" />  <!-- 0x7923ff89 -->
  <Fields.3 Value="1564160305" />  <!-- 0x5d3b3131 -->
</Uid>
```

Four `u32`s concatenated big-endian: `13b117f41b213a387923ff895d3b3131`.

**VST2.** `device:vst:audiofx:1096184373?n=Altiverb%207` — `1096184373` is `0x41567235`,
ASCII `"AVr5"`, Altiverb's four-character code.

Both were confirmed against live scanner output. FabFilter Pro-Q 3 scans as
`72C4DB717A4D459AB97E51745D84B39D`, byte-for-byte the fixture's `dev_identifier`, which
settles the open question of whether VST3 class IDs needed the COM-style byte swap of
the first three groups: **they do not.** Altiverb scans as `uid: "41567235"` with
`fourcc: "AVr5"`.

## Decision

Strip the `device:<kind>:<category>:` prefix and any `?n=` suffix, then join on
`(format, uid)` — `PluginMeta.uid`, which is `UNIQUE (format, uid)` in the scanner's
schema.

```rust
match kind {
    "vst"  => PluginKey::Vst2(id.parse::<i64>().ok()? as u32),
    "vst3" => PluginKey::Vst3(hex_16(&id.replace('-', "").to_lowercase())?),
    _ => None,
}
```

Parse VST2 via `i64` before the `as u32`: `unique_id` is a signed `i32` in the VST2
spec, and Ableton writes it as decimal without a guaranteed sign convention, so this
accepts both `-1094795586` and `3200171710`.

## Rejected alternatives

- **Fuzzy match on name and vendor.** Names differ between the XML and the binary, the
  database, and the vendor's own branding. Unnecessary given an exact key exists.
- **Treat `dev_identifier` as opaque and keep asking Ableton to resolve it.** That is
  ADR-0006, and the whole point is to stop.
- **Include version in the key.** The uid is deliberately stable across plugin updates —
  a project referencing Serum should still match after Serum ships an update. Version is
  descriptive only and must never participate in matching.

## Consequences

Matching is exact, cheap, and needs no heuristics. The identity is the plugin's own, so
it works for plugins Ableton never indexed — which is what makes ADR-0006 possible.

## Notes

Four cases to handle when this is implemented:

- **Multi-class VST3 bundles.** A bundle's factory exports several classes with distinct
  IDs, and Ableton's `dev_identifier` points at the *processor class the user
  instantiated*, which need not be the top-level `uid`. The lookup must also search
  `plugin_classes.class_id` and resolve to the parent row. Dexed, for example, exports
  three classes with three different IDs. Without this, multi-class bundles silently
  fail to match.
- **VST2 shells.** One `.dll` containing many logical plugins. Ableton stores the
  *child's* unique id, never the container's, so `is_shell` container rows must be
  excluded from matching entirely — nothing will ever reference them.
- **`instr` vs `audiofx` in the `dev_identifier` is Ableton's classification**, not the
  plugin's, and the two disagree in practice. Use it as a soft signal or a mismatch
  warning; never as part of the key.
- The parser currently reads `<Name>` and discards `<Uid>`. Capturing those four fields
  gives a second, independent copy of the class ID — useful when a `dev_identifier` is
  malformed, and the only identity available when `BranchSourceContext` is missing.
