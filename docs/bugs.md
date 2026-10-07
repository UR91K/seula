# Bugs

One line per bug, newest last. `just bug <name> <description>` appends one; `just bugs`
lists the open ones. Indented lines under an entry are for anything more to say.

- **Format:** ``- [ ] 2026-10-07 `name` description``. The name is short kebab-case and unique:
  commits and `//` comments cite it. No numbers, so two branches cannot pick the same one.
- **Fixing:** tick it (`[x]`) and add the commit, as `Fixed 1a2b3c4.` The cause and the
  fix belong in that commit's message, not here.
- **Not a bug after all:** tick it and say so, as `Not a bug: …` or `Won't fix: …`.

Accepted limitations that will not be fixed live in `docs/status.md` under Known issues.
Bugs fixed before this list existed are written up there under Resolved. See ADR-0065.

- [ ] 2026-10-07 `appledouble-sidecars-scanned` The scan picks up macOS `._*.als` AppleDouble sidecars as projects; they fail to parse, and `test_process_projects_integration` fails on them. tests/integration/scanning.rs
  Moved from `docs/status.md`, found before 2026-09-16. Environment-dependent: it needs
  sidecars in the configured project folders. Fixing it means deciding whether the scanner
  skips `._` files.
- [ ] 2026-10-07 `real-folder-tests-collide` `test_process_projects_with_progress` fails when run alongside `test_process_projects_integration`: both scan the real configured folders into the same real database at once. tests/integration/scanning.rs
  Moved from `docs/status.md`. Passes serially (`--test-threads=1`) and alone. Both are
  `#[ignore]`d, so it only shows under `cargo test --workspace --tests -- --ignored`.
- [ ] 2026-10-07 `list-default-limit-1000` Most list routes answer 1000 rows when no `limit` is given (`limit.unwrap_or(1000)` across src/database/), while projects and search answer every row.
  Unclear whether 1000 was deliberate; asked 2026-10-07. The frontend no longer depends on
  it: it pages to the route's `total_count`.
- [x] 2026-10-07 `headers-rendering-over-sidebar` Headers render on top of sidebar at window widths lower than 1144px
  Fixed 58a9f95.
