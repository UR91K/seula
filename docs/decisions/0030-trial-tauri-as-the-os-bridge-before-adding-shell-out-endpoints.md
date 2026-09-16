# 0030. Trial Tauri as the OS bridge in a branch before the server grows shell-out endpoints

- **Status:** Accepted, with the trial outcome pending
- **Recognized:** 2026-09-16, while reading the frontend sketch against the HTTP surface
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `docs/archive/FRONTEND_SPEC.md` (the three features that need OS access);
  `src/http/handlers/system.rs` (`add_project` takes a filesystem path); ADR-0024 (the
  trust model: loopback, single user); ADR-0028 (the Tauri history)

## Context

Three features in the frontend sketch need something a browser page cannot do:

- **Open in Ableton** launches an external program on a file.
- **Show in Explorer** reveals a file in the OS file manager.
- **Import a project from outside the watched directories**, including by drag and drop.
  The existing add-project endpoint takes a path, and a browser drop event yields file
  bytes and a bare filename, never a path.

There are three ways to get these: a native shell around the page (Tauri) that exposes
OS calls to it; endpoints on the daemon that shell out on the page's behalf, which the
loopback trust model from ADR-0024 would permit; or dropping the features.

Tauri was tried once before as the whole application and rejected as too slow to open
and too heavy (ADR-0028's history). The maintainer's view on 2026-09-16 is that this
was a while ago, the framework has likely improved, and it is worth a fresh try, this
time as a shell around an HTTP consumer rather than as the app itself.

## Decision

Try Tauri in a branch as the OS bridge. Until that trial has an outcome, the daemon does
not grow endpoints that launch programs, reveal files or otherwise act on the OS. The
mockups and the early React work treat the three features as present but mark them as
native-only, so the browser build degrades to hiding them rather than failing.

The outcome is recorded in `docs/status.md`, and whichever way it lands gets a new ADR:
either Tauri is the shell and the three features are Tauri commands, or Tauri is
rejected again and the features become daemon endpoints with a written trust argument,
or they are cut.

## Rejected alternatives

- **Add shell-out endpoints to the daemon now.** Rejected for sequencing, not on the
  merits. They are cheap to write and the trust model permits them, but if Tauri works
  they are dead code with an attack surface, and the mockups do not need them to exist.
  If the trial fails this becomes the obvious next move.
- **Drop the three features.** Rejected because they are the features that make the
  tool a project manager rather than a database viewer, and the sketch has kept them for
  a year. Dropping is the fallback of last resort and would be its own decision.
- **Decide Tauri now, without a trial.** Rejected because the previous attempt failed on
  properties (startup time, bundle weight) that only an attempt can measure.

## Consequences

The frontend has two build targets from the start: browser and Tauri. Every feature in
the sketch is tagged as one or both, and the architecture doc carries that tag. A
feature that exists in only one target is an inconsistency the user will notice, so the
number of native-only features should stay at three or fall.

Import by drag and drop in the browser target is limited to files under the watched
directories, or to uploading bytes to a server-side inbox, neither of which the sketch
asked for. That gap stays open until the trial resolves.

## Notes

The trial is the cheapest possible test: a Tauri shell that loads the mockup index and
exposes one command that reveals a file in Explorer. If that opens fast and packages
small, the question is answered. If it does not, the answer is also clear and costs a
branch.
