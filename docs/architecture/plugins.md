# Plugins

The most intricate area in the codebase, and the one mid-migration. Read
`docs/status.md` for what is actually wired up today.

## The four plugin types

| Type | Location | Means |
|---|---|---|
| `PluginInfo` | `src/models.rs` | Raw — exactly what an `.als` yields |
| `PluginKey` | `src/models.rs` | The join key: `(format, uid)` as an enum (ADR-0005) |
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

The scanner, and only the scanner. A reference that matches nothing installed keeps its
name and format from the `.als` and nothing else — vendor and version stay NULL, which
is honest: we have never seen the binary.

Ableton's own database was the source until 2026-09-15 and is now gone entirely, along
with the five columns that only it populated. See ADR-0006.

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

`seula plugin scan-system` prints results without touching the database — the
diagnostic view. `seula plugin refresh` scans and persists.

`seula scan` runs the same scan automatically when one has never completed, before
discovering projects, and reports each plugin as it is attempted under a
`scanning_plugins` phase. The trigger is a recorded fact in `app_state`, not an empty
plugins table — a machine whose plugins all fail to load would leave that table empty and
rescan forever. See ADR-0013.

## Persisting, and matching

```
plugin refresh ──► scan ──► persist_plugin_scan     (src/database/plugin_scan.rs)
                              upsert on (plugin_kind, uid), installed = 1
                              replace plugin_classes / plugin_buses
                              sweep unseen -> installed = 0   (full scans only)
                              reconcile phantoms

project scan  ──► parser emits bare references
                  BatchTransaction resolves them        (src/database/batch.rs)
                    1. (plugin_kind, uid)
                    2. plugin_classes.class_id -> owning bundle
                    3. otherwise create a row, installed unknown
                  writes plugin_refs recording which branch won
```

The parser no longer touches a database at all. It used to open Ableton's SQLite file
inside `finalize_result` — once per project, on every worker thread.

**Only the scan writes `installed`**, and it is tri-state: `NULL` means no scan has
looked, which is different from having looked and not found it (ADR-0012). A partial
scan — narrowed with `--paths`, or cut short by the restart budget — skips the sweep,
because it has no grounds to call anything missing.

**Phantom reconciliation** is what makes "install the missing plugin, run refresh" work.
A project can reference a bundle's non-primary class before that bundle is ever scanned;
the reference is a real identity, so it gets its own row. Once the bundle turns up, that
row is a duplicate, and the merge repoints `plugin_refs` and `project_plugins` at the
real bundle before deleting it.

## Schema

| Table | Key | A row means |
|---|---|---|
| `plugins` | `(plugin_kind, uid)` | a plugin — installed, referenced, or both (ADR-0007) |
| `plugin_refs` | `dev_identifier` | an Ableton reference string → the plugin it resolves to (ADR-0009) |
| `plugin_classes` | — | a class a VST3 bundle's factory exports; `class_id` is the matching fallback |
| `plugin_buses` | — | a bundle's bus layout |
| `project_plugins` | — | which projects use which plugin, unchanged |

`plugins.format` keeps its four display values (`"VST3 Instrument"`, …). Identity is
`plugin_kind` + `uid`, deliberately separate, because `format` bakes in Ableton's
instr/audiofx call — which ADR-0005 keeps out of identity.

`plugins.dev_identifier` is a representative reference kept for display; `plugin_refs`
is the authoritative and exhaustive mapping.

Uids and `class_id`s are stored normalised through `PluginKey::uid_hex()` — lowercase,
undashed. Ableton writes lowercase and the scanner uppercase, so without that every
lookup would miss.

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
