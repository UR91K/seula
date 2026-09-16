# 0023. Replace `log` with `tracing`

- **Status:** Accepted
- **Recognized:** the `log` choice predates `tracing` becoming the default; the cost only
  became concrete when the HTTP router (ADR-0024) was planned
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `Cargo.toml` (`log`, `env_logger`); `src/main.rs` (`init_logging`);
  `tests/common/mod.rs` (`setup`); 626 macro call sites across 39 files, surveyed
  2026-09-16; ADR-0024 (the surface that would otherwise have been written against the
  outgoing facade)

## Context

Seula logs through `log` + `env_logger`. That was the right choice when it was made —
`tracing` was not yet the ecosystem default, and `log` was the only facade with broad
adoption. The codebase is old enough that this predates the shift.

Two things have changed since.

First, the dependency tree is now split across both facades. `notify` emits `log`
records; `tonic`, `hyper` and `tower` emit `tracing`. `env_logger` sees the former and
none of the latter, so every span and event from the gRPC stack is currently invisible
with no indication that it is missing. This is already true — it is not a
consequence of the HTTP work.

Second, ADR-0024 adds an axum router. `axum` and `tower-http` instrument themselves with
`tracing` natively (`tower_http::trace::TraceLayer` emits spans, not log records), and
per-request correlation is the thing a request/response surface most wants from its
logging. Writing that module against `log` would mean writing new code against the
outgoing facade and then migrating through it a second time.

The survey that made this tractable: all 626 call sites use the plain macro forms
(`debug!`, `info!`, `warn!`, `error!`, `trace!`). None use `target:`, none use structured
key-value fields, and `crates/vst-meta` does not log at all. `tracing` exports macros
under the same five names with compatible syntax for these forms, so the body of every
call site is already valid `tracing`.

## Decision

Replace `log` with `tracing` and `env_logger` with `tracing-subscriber`, as a mechanical
port.

The mechanism is compiler-driven, and this is the reason it is worth doing as one change
rather than incrementally: remove `log` and `env_logger` from `Cargo.toml` first, then
let the build failures enumerate the work. Every site that needs touching is a compile
error, and the set of compile errors is complete by construction — there is no way to
miss one, and no need to trust a grep. The edits themselves are import swaps:
`use log::debug` becomes `use tracing::debug`, and fully-qualified `log::warn!(...)`
becomes `tracing::warn!(...)`. Both forms are in use.

`tracing-subscriber` takes the `env-filter` and `tracing-log` features. `env-filter`
preserves `RUST_LOG` as the control surface both init sites already use. `tracing-log`
installs the bridge that captures `log`-emitting dependencies — `notify` today, anything
else that has not migrated tomorrow — so the port does not silently drop records from
crates Seula does not control.

Two init sites change, and they are the only non-mechanical edits:
`init_logging` in `src/main.rs`, which maps a config string to a level filter and builds
`env_logger`, and `setup` in `tests/common/mod.rs`, which sets `RUST_LOG` and calls
`env_logger::try_init` once behind a `Once`.

**Spans are deliberately out of scope for this change.** The port converts events to
events and nothing more. Introducing spans is a design question per call site — what the
unit of work is, what fields belong on it — and answering it 626 times while also
performing a rename would make the diff unreviewable and hide any behavioral change
inside it. The first spans should be added where they are worth the most and can be
judged on their own: per-request instrumentation on the HTTP and gRPC surfaces.

## Rejected alternatives

- **Stay on `log`.** The facade still works and the code still compiles. Rejected on the
  dependency split: `tonic`/`hyper`/`tower` instrumentation is invisible today and axum's
  would join it, and the direction of ecosystem travel is one-way — the set of crates
  emitting `tracing` grows and the set emitting `log` does not. Staying means the bridge
  has to be installed eventually anyway, just in the direction that keeps the worse API.
- **Install `tracing-log`'s `LogTracer` and keep writing `log` macros.** This does route
  Seula's records into a `tracing` subscriber, and it is a genuinely smaller change.
  Rejected because it leaves two logging APIs in the tree permanently with nothing to
  indicate which one new code should use, and it does not make spans reachable — the
  `log` macros have nowhere to put them. It buys the output format without the thing the
  output format was wanted for.
- **Port incrementally, module by module, keeping both dependencies during the
  transition.** Rejected because it is strictly worse here than the big-bang: the port is
  mechanical and compiler-verified, so the usual argument for incrementalism (smaller
  independently-verifiable diffs) buys nothing, while the cost is real — a period where
  both facades are present and the correct one is ambiguous. Note this is the opposite
  call to ADR-0018, which migrated per-domain; the difference is that that refactor
  changed behavior at every step and this one cannot.
- **Add spans in the same pass.** Rejected: conflates a rename with a design change, and
  a 626-site diff that also contains judgment calls cannot be reviewed for either.

## Consequences

Log output format changes. `tracing-subscriber`'s `fmt` layer is not `env_logger`'s, so
anything parsing Seula's stderr — scripts, muscle memory — will see different lines.
`RUST_LOG` continues to work, but `EnvFilter` has its own directive syntax which is a
near-superset rather than an exact match; unusual existing filter strings may need
adjusting.

`log` remains in the dependency tree transitively via `notify`. Removing it from
`Cargo.toml` removes Seula's *direct* access to it — which is precisely what makes the
compiler enumerate the call sites — but does not remove the crate from the build. This
is intended, and the `tracing-log` bridge is what keeps those records visible.

The gRPC stack's own instrumentation becomes visible for the first time. This is a gain,
but it means `tonic`/`hyper` events will appear in output where they never have before,
and the default filter level may need tuning to keep that from being noise.

`crates/vst-meta` is untouched — it logs nothing today. The workspace consequently ends
up with logging in one crate only, which is the existing state, not a new one.

## Notes

Reversal is cheap in the same way the port is: the macro names are identical, so
reverting is the same mechanical import swap in the other direction. What would not
survive a reversal is any span added afterwards, which is one more reason to keep spans
out of this change and add them deliberately later.

The port should land before ADR-0024's router, so that the new module is written against
`tracing` from its first line rather than migrated through a second time.
