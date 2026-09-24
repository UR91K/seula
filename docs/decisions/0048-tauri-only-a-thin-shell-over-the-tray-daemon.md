# 0048. The frontend ships in Tauri only, as a thin shell that talks to the tray daemon over HTTP

- **Status:** Accepted, not yet implemented
- **Recognized:** 2026-09-16, when ADR-0030 put Tauri on trial beside a browser target
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** ADR-0030 (the two build targets and the Tauri trial this revises);
  ADR-0028 (Tauri was tried once as the whole application and rejected as slow to open
  and heavy); ADR-0029 (serving was left undecided); ADR-0031 and ADR-0033 (both argued
  partly from having two targets); `src/http/server.rs` (the CORS origin check);
  `src/tray.rs`

## Context

ADR-0030 gave the frontend two build targets, browser and Tauri. Three features that need
the OS (Open in Ableton, Show in Explorer, importing from a path) were marked native-only
and hidden in the browser. Tauri was put on trial, with the browser as the fallback. In
practice, both targets would be built and kept in step from the start.

Tauri can be used in two ways. As a full application framework, the Rust side owns the
state and the webview calls it through Tauri's IPC (`invoke`, events, plugins). As a
shell, the webview is an ordinary page that talks to a server over HTTP, and Tauri adds
only a window and a few OS calls.

The maintainer's priorities, as stated: nothing happening in the background whose inner
workings are not understood, and no slow, bloated application that feels sluggish or
buggy and takes too long to open. The second is exactly why Tauri was rejected the
first time (ADR-0028).

## Decision

**Tauri is the only target.** The frontend is built for Tauri. A browser build is made
only if Tauri fails the trial below, and it is not maintained alongside.

**The Tauri app is a thin shell.** All data travels over HTTP to the axum API from
ADR-0024: the same routes and the same SSE streams a browser would use. The frontend
does not use Tauri's IPC for data, state or events, and the Tauri side holds no
application state.

The page calls the daemon itself, with `fetch` and `EventSource`, as it would in a
browser tab. The Tauri process's Rust code does two things: it hosts the webview and
it runs the OS calls below. It never proxies the API, and it has no knowledge of the API.
Two settings let the page reach the daemon across origins: the daemon's CORS check
(ADR-0024) accepts the webview's origin, and Tauri's content security policy lists the
daemon's address in `connect-src`.

**Tauri's IPC is used for the OS calls only.** Opening a project in Ableton, revealing a
file in Explorer, playing a sample from the local disk and picking a path to import are
Tauri commands. These are the features ADR-0030 listed as needing native access. Each
is a small explicit call and none of them touches the database. The daemon does not
grow endpoints that act on the OS.

**The daemon stays a separate tray application.** The axum API runs in the tray process,
as today, and owns the database, the scanner and the watcher. The Tauri app is a client
of it. The maintainer gave three reasons:

- **The work outlives the window.** Scans, sample checks and folder watching keep running
  in the background, and the frontend can be opened and closed at will without stopping
  or restarting any of them.
- **A frontend failure stays in the frontend.** A JavaScript error or a webview crash
  takes down the window, not the program. The database, and any scan that is writing to
  it, are in another process.
- **The client does not have to be local.** A daemon that is reached over HTTP can later
  be reached over a network, so the Tauri app could run on another machine against a
  library it does not host. This is not planned. It is future-proofing, and the first two
  reasons come with it for free. ADR-0033 already kept media on HTTP for the same
  reason.

**The trial still happens, and now decides whether a browser build is ever needed.** It
is the one ADR-0030 described: a Tauri shell that loads the approved mockups and exposes
one command that reveals a file in Explorer. It passes if it opens fast, packages small
and does not feel sluggish, which are the properties the first attempt failed on. No
numeric thresholds were set. The maintainer judges the result, and it is recorded in
`docs/status.md`.

**Serving is settled.** ADR-0029 left open whether the daemon serves the frontend's
assets. It does not: they are bundled into the Tauri app. The daemon serves only the API
and stored media (ADR-0033).

## Rejected alternatives

- **Browser and Tauri targets side by side (ADR-0030).** Rejected because every feature
  would be tested twice, and three of them would be hidden in one target and not the
  other, an inconsistency ADR-0030 itself said the user would notice.
- **Tauri as the full application, with the backend behind Tauri's IPC.** Rejected
  because data would then move through a framework-owned channel with its own
  serialisation, event system and permissions model. That is the background machinery
  the maintainer does not want to depend on without understanding it. Plain HTTP can be
  read in devtools and replayed with `curl`. It is also the surface that already exists
  and is tested, and the first Tauri attempt, as the whole application, is the one that
  failed.
- **Tauri commands for everything, with HTTP kept open for other frontends.** The Tauri
  app would host the backend and use its IPC for all data, and the axum API would stay
  for any other client. Rejected on all three reasons above. Scanning and watching
  would stop when the window closed. The UI and the database would share a process, so
  a webview crash would take the backend with it. A remote Tauri app would need the
  HTTP path anyway, so the IPC path would be a second way to reach every service, kept
  in step with the first for no benefit. It would also return this project to three
  adapters over one service layer, which ADR-0046 has just removed.
- **Daemon endpoints for the OS calls, and Tauri as a bare webview.** ADR-0030's fallback,
  and still the fallback if the trial fails. Rejected as the plan because a server
  endpoint that launches programs is attack surface on the loopback port, open to any
  local process. A Tauri command is reachable only from the app's own page.

## Consequences

One target to build, test and review. No feature is hidden anywhere, and "native-only"
stops being a category in `docs/architecture/frontend.md`.

Two processes must both be running. The Tauri app has to find the daemon's `http_port`,
and say plainly when the daemon is not running rather than show an empty library. The
tray menu is the natural place to open the app from. Neither is designed yet.

The Tauri webview is a different origin from the daemon, so CORS stays load-bearing
(ADR-0024). The origin check had a gap: it did not accept `http://tauri.localhost`,
Tauri 2's default origin on Windows, so every request from the shell would have been
refused. Fixed with this record (`docs/status.md`).

Two earlier ADRs lose part of their argument, but not their conclusions:

- **ADR-0031** kept preferences out of `localStorage` partly because two targets meant
  two stores. With one target, its other reason carries it: preferences belong to the
  user's database, which they back up, not to a webview profile.
- **ADR-0033** kept media on HTTP partly because `file://` fails in the browser target.
  With one target, the thin-shell rule carries it: data reaches the page over HTTP, and
  a Tauri asset protocol would be a second channel.

Remote use is kept possible, not built. Three things stand between here and it, and each
needs its own decision when the time comes:

- **Trust.** The daemon binds to loopback and has no authentication. ADR-0024 named any
  change to the bind address as the trigger for adding auth, and that still holds.
- **The OS calls.** Open in Ableton, Show in Explorer and playing a sample act on paths
  on the daemon's machine. A remote client cannot run them locally, so they would be
  hidden or work differently there.
- **Finding the daemon.** `http_port` on localhost becomes an address the app is given.

## Notes

If the trial fails, the fallback is ADR-0030's: a browser build, with the OS features
either as daemon endpoints with a written trust argument or cut. That would be a new ADR
superseding this one.

The thin-shell rule is the part most likely to erode. Each convenience Tauri offers, such
as a store plugin, an event bus or a command that reads the database, moves the
application a little further from HTTP. Any Tauri command beyond the OS calls above
needs its own ADR.
