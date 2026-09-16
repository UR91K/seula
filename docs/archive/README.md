# Archive

Superseded planning documents. **Nothing here is current, and some of it is actively
wrong.** Do not read these for orientation — start from `/CLAUDE.md`.

Kept only so existing links do not break. Git history preserves them either way, so
deleting this folder is safe.

| File | Why it is here |
|---|---|
| `plugin_scanner_plan.md` | Raw AI chat transcript (1007 lines, contains `Ran tool` / `Search files...` markers). Proposes a **C++ binary** for plugin scanning. Superseded by `crates/vst-meta` in Rust — see ADR-0004. Actively misleading. |
| `generalisation_plan.md` | Raw AI chat transcript (476 lines). Explores decoupling the codebase from Ableton-specific models. Partly overtaken by ADR-0006; no current plan depends on it. |
| `overall_generalisation.md` | Was 0 bytes. Deleted rather than archived. |
| `.kiro/` | Steering files plus a spec for the plugin subprocess that `crates/vst-meta` superseded (ADR-0004). `steering/structure.md` is stale (predates `src/scan/plugins/`). Moved here 2026-09-16, per `docs/status.md`. |

These are the reason `docs/README.md` insists on one home per fact. They were written as
planning artefacts, never updated, and then outlived the plans they described.
