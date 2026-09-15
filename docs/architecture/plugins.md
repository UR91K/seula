# Plugins

The most intricate area in the codebase, and the one mid-migration. Read
`docs/status.md` for what is actually wired up today.

## The four plugin types

| Type | Location | Means |
|---|---|---|
| `PluginInfo` | `src/models.rs:731` | Raw — exactly what an `.als` yields |
| `DbPlugin` | `src/ableton_db.rs:9` | A row from Ableton's own database (being retired) |
| `Plugin` | `src/models.rs:587` | The merged record, stored in Seula's database |
| `GrpcPlugin` | `src/models.rs:617` | `Plugin` + usage counts, for API responses |
| `PluginMeta` | `crates/vst-meta/src/meta.rs` | What the scanner extracts from a binary |

## What the project file actually contains

Only two pieces of information, despite three fields:

```rust
pub struct PluginInfo {
    pub name: String,            // <Name>/<PlugName> in Vst3PluginInfo/VstPluginInfo
    pub dev_identifier: String,  // the enclosing device's ID
    pub plugin_format: PluginFormat, // *derived* from dev_identifier, not read
}
```

`parser.rs` walks `PluginDesc` → `Vst3PluginInfo`/`VstPluginInfo` and takes the first
name it sees, with two guards that both encode bugs found the hard way:
`plugin_info_processed` stops a second name overwriting the first, and `in_vst3_preset`
stops a *preset's* `<Name>` being read as the plugin's.

Format comes from the `dev_identifier` prefix (`utils/plugins.rs:114`). An identifier
matching none of the four known prefixes is dropped — that check is the gate for "is
this thing a plugin at all".

Results accumulate into `plugin_info_tags: HashMap<String, PluginInfo>` keyed by
`dev_identifier`, so a plugin used on ten tracks collapses to one entry.

## Where everything else comes from

Today: Ableton's database, via `make_plugin` (`parser.rs:922`). Vendor, version, SDK
version, Ableton's row ids and scan bookkeeping, and `installed` — which means only
"the lookup hit". On a miss, the XML name and format survive and everything optional is
`None`.

Being replaced by the scanner. See ADR-0006.

## Identity

`dev_identifier` is an Ableton-specific *encoding* of the plugin's own native ID:

```
device:vst3:audiofx:13b117f4-1b21-3a38-7923-ff895d3b3131
                    └── VST3 class ID, dashes stripped

device:vst:audiofx:1096184373?n=Altiverb%207
                   └── 0x41567235 = fourcc "AVr5"
```

Both map directly onto `PluginMeta.uid`. The join key is `(format, uid)` — never name,
never version. Full derivation, verification, and the four cases that will bite
(multi-class bundles, VST2 shells, Ableton's own instr/audiofx classification, version
stability) are in **ADR-0005**.

## Scanning installed plugins

```
discovery.rs ──► spawner.rs ──[spawn]──► vst-meta (subprocess)
  candidate       supervises,             loads the actual binary
  paths           resumes past            emits NDJSON per path
                  crashes
```

- **Discovery** stays in the parent (ADR-0008). Bundles are one candidate; the walk is
  depth-capped and deduplicated.
- **The supervisor** runs one worker per batch and restarts past whatever kills it,
  attributing the failure via the worker's `begin` line. Timeout for hangs, bounded
  restart budget.
- **The worker** is the only thing that ever loads a plugin (ADR-0004). The main process
  depends on `vst-meta` with `default-features = false` so the VST hosts are not even
  linked here.

Exercise it with `seula plugin scan-system`. Results are printed, **not persisted** —
that is the next piece of work.

## VST2 shells

One `.dll` containing many logical plugins (Waves, some Kontakt builds). The worker
enumerates them via `effShellGetNextPlugin`, then re-instantiates the binary once per
sub-plugin, answering the `CurrentId` host callback with the target id so the plugin
materialises as that sub-plugin (`crates/vst-meta/src/shell.rs`).

One `.als` reference never points at a container — Ableton stores the child's id — so
`is_shell` rows must be excluded from matching. This is why a scan `result` carries a
`Vec<PluginMeta>` rather than one record.

## VST3 multi-class bundles

A bundle's factory exports several classes with distinct IDs — processor, controller,
compatibility. Ableton's `dev_identifier` names the *processor class the user
instantiated*, which need not equal the bundle's top-level `uid`. Matching must also
search `plugin_classes.class_id`. See ADR-0005.
