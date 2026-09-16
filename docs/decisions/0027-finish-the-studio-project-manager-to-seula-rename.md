# 0027. Finish the "Studio Project Manager" → "Seula" rename

- **Status:** Accepted
- **Recognized:** 2026-09-16, when a commit message and code comments this session
  reintroduced `StudioProjectManagerServer` and `STUDIO_PROJECT_MANAGER_*` from habit
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `df4ed4b` ("chore: rename all instances/variants of \"Studio Project
  Manager\" to \"Seula\""); `src/grpc/server.rs` (`SeulaServer`); `src/config/mod.rs`
  (`SEULA_GRPC_PORT`, `SEULA_HTTP_PORT`, `SEULA_LOG_LEVEL`, `SEULA_DATABASE_PATH`,
  `SEULA_CONFIG`); `docs/architecture/overview.md`; `docs/codebase_patterns.md`

## Context

The product was already named Seula everywhere user-facing — the README's title, the
CLI's help text, every doc written about it. But the identifier that predated that name,
`StudioProjectManagerServer`, was still the gRPC server struct, and five environment
variables (`STUDIO_PROJECT_MANAGER_GRPC_PORT`, `_HTTP_PORT`, `_LOG_LEVEL`,
`_DATABASE_PATH`, `_CONFIG`) still carried it. `docs/architecture/overview.md` had a line
acknowledging this directly: the env vars "predate the rename and still work" — a
conscious decision, at the time, to leave a legacy prefix in place rather than force a
config-file rewrite for something with no users yet to disrupt.

That gap is also exactly the kind of thing a knowledge-cutoff model reproduces by habit:
this session wrote `StudioProjectManagerServer` and `STUDIO_PROJECT_MANAGER_HTTP_PORT`
into new code and a commit message (`640b33e`) days after they should have been retired,
not because either was correct, but because the old name is what the training data and
the surrounding code both still said. The maintainer caught it and swept the codebase.

The product has no users yet — it is alpha/testing-stage software — so there is no
external cost to changing environment variable names. That fact is load-bearing for the
decision below: the same rename attempted after a real deployment exists would need a
transition period (read both prefixes, warn on the old one) that isn't worth building for
a rename with nobody on the other end of it.

## Decision

Rename every remaining instance of "Studio Project Manager" / `StudioProjectManager` to
"Seula" / `Seula`, with no backward-compatible alias kept:

- `StudioProjectManagerServer` → `SeulaServer` (`src/grpc/server.rs`, its trait impls,
  and every reference in `src/main.rs`, `src/http/state.rs`, `src/grpc/mod.rs`,
  `tests/grpc/server_setup.rs`, `tests/grpc/collections.rs`)
- `STUDIO_PROJECT_MANAGER_GRPC_PORT` → `SEULA_GRPC_PORT`
- `STUDIO_PROJECT_MANAGER_HTTP_PORT` → `SEULA_HTTP_PORT`
- `STUDIO_PROJECT_MANAGER_LOG_LEVEL` → `SEULA_LOG_LEVEL`
- `STUDIO_PROJECT_MANAGER_DATABASE_PATH` → `SEULA_DATABASE_PATH`
- `STUDIO_PROJECT_MANAGER_CONFIG` → `SEULA_CONFIG`
- Every doc mention (`docs/architecture/overview.md`, `docs/codebase_patterns.md`,
  `docs/archive/.kiro/steering/tech.md`)

No compatibility shim reads the old env var names. A shim is the right tool when real
configurations exist that would otherwise silently stop working; here there are none, so
a shim would be pure surface area for a problem that doesn't exist.

## Rejected alternatives

- **Keep reading both env var prefixes, preferring the new one.** This is the standard
  deprecation pattern and would be the right call once Seula has an actual deployed
  config somewhere. Rejected now because there is nothing depending on the old names —
  the earlier "predate the rename and still work" note in `docs/architecture/overview.md`
  was itself made when the same reasoning didn't yet clearly apply; carrying it forward
  indefinitely would mean never finishing a rename that has no one left to break.
- **Leave the internal struct name (`StudioProjectManagerServer`) alone since it's not
  user-facing, and rename only the env vars.** Rejected: the whole point of finishing the
  rename is that a name lingering in code is exactly what gets copied into new code by
  habit, as this session's own commit did. An internal identifier is not lower-risk than a
  public one for that failure mode — arguably higher, since nothing external ever flags it
  as wrong.

## Consequences

Any config, script, or shell profile already setting `STUDIO_PROJECT_MANAGER_*` stops
working silently — the new code simply doesn't look for those names, so a set-but-ignored
env var produces no error, just the default value. Acceptable here because nothing sets
them yet; would not be acceptable to redo the same way post-launch.

`docs/architecture/overview.md`'s configuration section needed its prose corrected
alongside the identifier, not just the identifier itself — the sentence "environment
overrides (`SEULA_*`) that predate the rename" is true of the old prefix and false of the
new one; a mechanical find-and-replace on the token doesn't fix a sentence whose meaning
depends on which name it's talking about.

## Notes

If `STUDIO_PROJECT_MANAGER` or `StudioProjectManager` reappears anywhere in this codebase
— in a comment, a commit message, a variable name — it is a regression to fix on sight,
not a deliberate choice to ask about. This ADR is the record of why.
