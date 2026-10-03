# 0050. Seula ships as three parts: the tray daemon, the frontend, and a CLI client that talks to the daemon over HTTP

- **Status:** Accepted, not yet implemented. Supersedes the CLI half of 0046
- **Recognized:** 2026-10-03, in conversation, while bringing the README up to date
- **Decided:** 2026-10-03
- **Recorded:** 2026-10-03
- **Confidence:** decided now
- **Evidence:** ADR-0046 (the no-CLI end state this reverses, and its rejection of a thin
  HTTP-client CLI); ADR-0048 (the daemon and the Tauri shell, which this keeps);
  ADR-0029 (the frontend lives in this repository); ADR-0049 (the CLI still opens its
  own database connection); `src/cli/`; `src/tray.rs`

## Context

ADR-0046 settled that HTTP is the only surface and that no CLI survives, not even a thin
HTTP client. Its reason was that such a client would be "a second client to keep in step
with the DTOs, for a single user who will have a GUI." Scripting was listed as a cost and
pointed at `curl`.

That premise no longer holds. Seula is becoming an application that other people install,
not a backend for one user's GUI. In the maintainer's words, a command-line client is
wanted for three kinds of caller:

- **programs** that need to talk to Seula
- **people who prefer commands** to a GUI
- **AI agents**

None of these is well served by `curl` against JSON routes.

The other half of the motivation is architectural. Today the CLI is a second way into the
data: it opens the database itself, beside the daemon. The goal is a CLI that exists
without that, so that **every client reaches the data the same way, through the daemon
over HTTP.**

## Decision

**The shipped application has three parts, and only one of them touches the data.**

1. **The daemon.** It is always running in the tray, owns the database and serves the
   HTTP API. Its tray menu is kept minimal (below). Otherwise it is unchanged from
   ADR-0048.
2. **The frontend.** It lives in this repository (ADR-0029) and is launched from the tray
   menu or a shortcut. It is a thin Tauri shell over HTTP (ADR-0048), and it can be
   started and stopped without the daemon noticing or caring.
3. **The CLI client.** It is a small, separate program meant to sit on the `PATH`, and it
   can be called whenever. Each call is one HTTP request, or a few, to the running daemon,
   over the same API the frontend uses. It holds no database connection and no logic of
   its own. **It has no interactive mode.**

**The executables are `seula-service` (the daemon) and `seula` (the CLI).** The CLI
gets the short name because it is the one people type. The daemon is started from the
tray, a shortcut, login or a client, and nobody types its name.

**When the daemon is not running, a client starts it.** The frontend and the CLI behave
identically here, ideally through one shared implementation. A client that says "daemon
not running" is a dead end for a normal user and friction for everyone else. Starting it
means:

- **Find the daemon next to the client,** in the same install folder. Never search the
  `PATH` for it.
- **Start it detached,** so it outlives the CLI call that started it.
- **Wait until it is ready, with a timeout.** Poll a cheap route until it answers. This
  needs the daemon to serve HTTP before any scan begins, so the first-run plugin scan
  (ADR-0013), which takes minutes, never holds a client up. That scan runs in the
  background.
- **Say so on stderr,** for example "Started seula-service", never on stdout, so a
  script or agent parsing JSON output is not broken.
- **Let a caller opt out.** A flag such as `--no-start` makes the CLI fail fast with a
  distinct exit code, for scripts and tests that must not start things.
- **Guard against two daemons.** Two clients starting at once, or the GUI and a CLI call
  together, could each start one. The daemon takes a single-instance lock (a named mutex
  on Windows) before it opens the database, and a second instance exits quietly. When
  this was recorded there was no such guard: a second daemon only failed when it tried
  to bind the HTTP port, after it had already opened the database.

If starting the daemon from the CLI trips antivirus or SmartScreen before the
executables are signed, that ability is disabled until they are. The CLI then falls back
to reporting that the daemon is not running.

**The tray menu stays minimal.** An action belongs in the tray only if it needs no input
and its result fits in a notification. Everything else goes in the GUI.

- **Open Seula.** Also the double-click action, and the main reason the tray exists.
- **Rescan projects.** Progress shows in the tray tooltip, with a notification when the
  scan finishes.
- **Quit.** Says plainly that it stops the service, and that the GUI or CLI will start it
  again the next time they are used.

- **Start with Windows.** A checkable item, because it fits an always-on service. It is
  also in the GUI's settings pane. Both must show the same state, so they read and
  write one underlying setting, the Windows startup entry itself, rather than each
  keeping a copy.

One more is optional:

- **Open logs folder,** while Seula is in alpha, for testers reporting bugs. Remove it
  once there are releases.

Left out on purpose:

- **Plugin rescan.** It takes minutes, and its results need the GUI to read.
- **Pausing the watcher.**
- **Scan status as menu items.** The tooltip covers it.

When this was recorded, the menu had only Quit (`src/tray.rs`).

**ADR-0046 stands for everything except the CLI.** gRPC is still removed, and
`SystemService` still folds into `Services`. The current CLI subcommands and interactive
mode still go.

**The new client lands before the old subcommands are deleted.** The old subcommands
are the reference for the new ones, and many can probably be ported across almost
directly: the argument shapes, and the table, JSON and CSV output in
`src/cli/output.rs` (ADR-0003). There are no releases to protect, only an early alpha
CLI preview for testing, so the order is about having a reference to port from. It is
not about compatibility.

## Rejected alternatives

- **ADR-0046's end state: no CLI, with `curl` for scripting.** Rejected because it leaves
  all three callers above with raw HTTP routes and JSON bodies, and with no `--help` to
  discover them from.
- **Keep the current in-process CLI.** Rejected because it is a second path to the data
  beside the daemon, which is exactly what this decision removes. ADR-0049 made the same
  change inside the daemon, moving its scan onto the shared database handle.

- **Bring interactive mode back in the client.** Rejected on experience, not speculation.
  The existing interactive mode was annoying to work on, and it was extra work on top
  of keeping both the CLI and the GUI going. It also offers little that running the
  commands directly or opening the GUI doesn't already give.

- **The CLI as subcommands on the daemon's binary, one program instead of two.**
  Rejected for five reasons, all confirmed by the maintainer:
  1. *On Windows, one program cannot be both.* A tray app should be built as a windowless
     program (`#![windows_subsystem = "windows"]`), so launching it from a shortcut or
     at login opens no console. A CLI must be a console program to print. One program
     cannot do both without hacks. Checkable: when this was recorded, nothing in `src/`
     set `windows_subsystem`, so the daemon opened a console window on launch.
  2. *It makes the one-path rule structural.* A client that never links SQLite or the
     database code cannot touch the database. The dependency graph enforces the
     Decision, so nobody has to remember it.
  3. *Size and startup time.* Scripts and agents call the CLI many times in a row. It
     should start instantly, and not carry tokio's full runtime, axum, tray-icon,
     rusqlite, notify and the scanner on every call.
  4. *Different lifecycles.* The daemon is a single long-running process; the CLI runs
     and exits. In one program, bare `seula` would have to mean either "start the
     service" or "show help".
  5. *Only the CLI belongs on the `PATH`.* The daemon is started by the tray, a
     shortcut, login or the clients. Nobody needs to type its name.

  Downsides of two programs were weighed and judged minor:
  - *They must be installed together.* The CLI finds the daemon next to itself, so
    copying the CLI on its own breaks starting the daemon. Most programs that ship
    several executables break the same way. That is the user's doing, not a defect.
  - *Two unsigned executables to trust.* Both can trigger SmartScreen or antivirus
    prompts, and a CLI that starts a background process is a pattern some heuristics
    dislike. Signing fixes both, and one certificate covers every executable. If it
    causes trouble before the executables are signed, the CLI's ability to start the
    daemon can be disabled until they are.
  - *Duplicated code on disk.* Both statically link serde, the DTOs and so on. A few MB
    at most, which only matters on very constrained machines.

  Two costs that look like they come from the split come from the HTTP decision instead.
  Every command needs the daemon, and nothing can inspect the data while the daemon
  cannot start. Neither would change with one program.

## Consequences

The cost ADR-0046 refused is now accepted. There are two HTTP clients to keep in step
with the API: the frontend's TypeScript types and the CLI. The CLI is Rust, in this
workspace, so it can probably deserialize the `src/http/dto/` types directly rather than
mirror them. That would make the second client cheap to keep in step. This is a likely
mitigation, not a decision. If the DTOs live in the main crate, sharing them may mean
moving them into a small crate of their own, so the client doesn't link the daemon.

As a separate program, the client need not link the daemon's dependencies (tray, axum,
SQLite, the scanner). It can stay small, as long as the shared DTOs don't drag them in.

Every CLI command needs the daemon. That is the price of having a single path to the
data, and starting the daemon on demand is what makes it bearable. Until the executables
are signed, it may only be bearable for the GUI, if the CLI's start ability has to be
disabled.

Starting on demand puts two requirements on the daemon. First, it must serve HTTP before
it does any scanning. That already holds: when this was recorded, `src/main.rs` started
no scan at startup, and scans ran only on request. It has to stay that way if a startup
scan is ever added. Second, it must refuse to run twice. That is not true yet.

A client and the daemon can be different versions. This happens however the programs
are split: the daemon runs for a long time, so after an update a new client can easily
be talking to an old daemon that is still running. The daemon should report its version
on a cheap route, and clients should check it before relying on the API.

For a while the old in-process CLI and the new client both exist. That overlap is
deliberate (see Decision) and ends when the old subcommands are deleted.

## Notes

Renaming the daemon is mechanical. When this was recorded, its executable took the
package name, because `Cargo.toml` had no `[[bin]]` section. Renaming it needs one, and
the library crate can stay `seula`, so `use seula::...` is unaffected.
`mockup/data/generate.py` hardcodes the executable path and has to follow.

Nothing was left open when this was recorded.
