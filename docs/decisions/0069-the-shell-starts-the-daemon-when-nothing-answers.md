# 0069. The shell starts the daemon when nothing answers

- **Status:** Accepted
- **Recognized:** 2026-10-09, installing the first alpha installer in Windows Sandbox: the
  window opened to "Cannot reach the daemon" on every view
- **Decided:** 2026-10-09
- **Recorded:** 2026-10-09
- **Confidence:** decided now
- **Evidence:** ADR-0048 (the daemon is a separate tray program; "the Tauri app has to find
  the daemon's `http_port`, and say plainly when the daemon is not running", not designed
  then); ADR-0068 (the daemon ships beside the shell as a sidecar); `src/main.rs`
  (`run_tray_mode`); `web/solid/src-tauri/src/main.rs`; the Sandbox run

## Context

ADR-0048 made the daemon its own process so scans, sample checks and folder watching
outlive the window, and left open how it gets started. ADR-0068 installs both executables
in one folder. Nothing starts the daemon, so a fresh install opens a window with nothing
behind it, and a user has no reason to know a second program exists. The views report
"Cannot reach the daemon" when a load fails and do not retry, so the daemon has to be
answering before the page makes its first request.

ADR-0048 limits the Tauri side to hosting the webview and a few OS calls, and asks for an
ADR before anything else is added.

## Decision

**At startup, before the window loads its page, the shell checks the daemon and starts it
if nothing answers.** In Tauri's `setup` step it makes one `GET /health` to
`127.0.0.1:50052`. A `200` means a daemon is running and nothing more happens. Otherwise
the shell starts `seula.exe` from its own folder, then polls `/health` for up to fifteen
seconds before letting the window continue.

**It starts the daemon in tray mode, with no arguments.** The tray icon is the user's
way to see that the daemon is running and to quit it. `--server` has neither. A daemon
running with no visible sign of it is what the maintainer does not want.

**The daemon outlives the window.** It is started detached, with no console window and no
tie to the shell's process group, so closing the shell leaves scans and the watcher
running (ADR-0048). The next launch of the shell finds it answering and starts nothing.

**The check is an HTTP request, not a port test,** so another program on the port is not
taken for the daemon.

**This is startup behaviour, not an API.** The shell adds no Tauri command and the page
cannot ask it to start or stop anything. It does not read the daemon's config or proxy any
request, so ADR-0048's thin-shell rule holds.

**A failure is left for the page to show.** If the daemon is missing from the folder, will
not start, or does not answer in time, the shell logs it to standard error and opens the
window anyway. The views then show "Cannot reach the daemon", as they would in any
other case of it not running.

## Rejected alternatives

- **The installer starts the daemon, and the shell never does.** It covers the first run
  only. After a reboot, or after the user quits from the tray, the shell would open to the
  same error.
- **Autostart with Windows, as the way the daemon gets started.** ADR-0050 plans it as an
  opt-in, and it stays one. As the only mechanism it would run a background program on
  login for every user who installs the app, which they have not agreed to.
- **A Tauri command the page calls to start the daemon.** It would put an action that
  launches programs behind the page's own code, which ADR-0048 keeps out of the
  webview's reach, and it would need the page to retry.
- **`--server` instead of tray mode.** It would hide the process. The user could then
  stop it only through Task Manager.
- **A port test.** Anything listening on 50052 would count as the daemon.
- **Reading `http_port` from `config.toml`.** The page's default address is the same
  fixed port (`web/shared/api.ts`), so a daemon moved to another port is unreachable
  anyway. Both change together when the address is made configurable.

## Consequences

An installed app opens straight to a working library. A second launch of the shell costs
one local request.

The shell now knows one fact about the daemon: its file name and port. A change to either
has to be made in both places.

A daemon that is already running but unhealthy, or an old one left over after an update,
is taken for a good one. Checking the daemon's version against the shell's is future work
and is needed before an updater.

A user who quits from the tray, then opens the shell, gets the daemon started again. That
matches what quitting a tray program means here, and a setting for it can come with the
tray menu in ADR-0050.

The installer still has to stop a running daemon on upgrade and uninstall, since the shell
now leaves one running. That is separate work and is not solved by this record.

## Notes

The wait is bounded so a daemon that never answers cannot leave the user with no window.
Reversing is cheap: it is one function in `web/solid/src-tauri/src/main.rs`.
