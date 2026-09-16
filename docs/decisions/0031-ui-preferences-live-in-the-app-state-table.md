# 0031. UI preferences persist in the database's `app_state` table, not in the browser

- **Status:** Accepted
- **Recognized:** 2026-09-16
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/database/core.rs` (the `app_state` key-value table, added
  additively so it needed no `SCHEMA_VERSION` bump); `src/database/plugin_scan.rs`
  (its only current use, the first-run scan marker); ADR-0024 (the single-user,
  single-machine trust model that makes this safe); ADR-0030 (the two build targets
  that make this necessary)

## Context

The frontend sketch wants column widths, column order and visibility, items per page,
and the details-panel toggle to persist. A browser has `localStorage` for exactly this,
and it would be the default choice for a web app.

Seula is not a web app in that sense. It is a single-user tool on one machine, with no
accounts, and ADR-0030 gives it two possible frontends over the same daemon: a browser
tab and a Tauri shell. Those are two different origins with two different
`localStorage` stores. A user who resizes a column in one and opens the other sees it
snap back.

## Decision

UI preferences are stored server-side in the existing `app_state` table, under a
namespaced key prefix, and reached through the HTTP surface. The browser holds them
only as a cache for the current session.

The daemon does not interpret the values. It stores whatever the frontend writes under
a key, and the frontend owns the schema of what it writes. That keeps the daemon from
needing a release every time the frontend adds a column.

## Rejected alternatives

- **`localStorage` in the browser.** Rejected because of the two-origin problem above,
  and because it ties preferences to a browser profile rather than to the user's
  database, which is the thing they back up. It remains the right place for things that
  are genuinely per-tab, such as an unsent search string.
- **A typed `preferences` table with one column per setting.** Rejected because the
  daemon would then need to know the frontend's settings and change with them, and
  because adding a column to an existing table is exactly the change that forces a
  `SCHEMA_VERSION` bump, which discards the user's database (ADR-0011). The key-value
  table exists to avoid that.
- **A config file next to `config.toml`.** Rejected because `config.toml` is
  hand-edited and validated, and UI state written by a program at high frequency does
  not belong in a file the user is invited to edit.

## Consequences

One small addition to the HTTP surface: get and set for a preferences blob. It is not
a business-logic change and goes through the service layer like everything else.

Preferences now die with the database on a `SCHEMA_VERSION` bump. That is consistent
with everything else user-authored and is already the documented cost of ADR-0011.

The frontend must tolerate an empty or unparseable stored value and fall back to
defaults, since the daemon does not validate what it stores.

## Notes

Cheap to reverse: the frontend reads and writes one blob through one pair of calls. If
a per-tab store turns out to be wanted for some setting, it can be added alongside
without touching this.
