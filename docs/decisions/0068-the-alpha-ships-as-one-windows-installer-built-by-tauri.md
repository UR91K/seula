# 0068. The alpha ships as one Windows installer built by Tauri

- **Status:** Accepted
- **Recognized:** 2026-10-08, when the first release of the rewrite was being planned
- **Decided:** 2026-10-08
- **Recorded:** 2026-10-08
- **Confidence:** decided now
- **Evidence:** ADR-0048 (the shell and the daemon are separate programs); ADR-0004
  (`vst-meta` is a subprocess found beside the executable); ADR-0011 (a schema bump
  discards the database); ADR-0054 (AGPL, with MPL scanner crates);
  `src/scan/plugins/spawner.rs` (`locate_worker`); `web/solid/src-tauri/tauri.conf.json`;
  the repository's tags and releases before this record (`1.0-alpha`, `1.0.1-alpha`,
  a CLI and TUI build)

## Context

Nothing packages the application yet. The justfile is for development, and the Tauri
configuration is still the trial one: product name "Seula (solid)", version `0.0.0`,
`"targets": "all"`. The only releases are two alphas of the old CLI and TUI program,
which the rewrite replaces (ADR-0046, ADR-0048).

The first testers will not have Rust or Node.js. They need one file that installs
everything. Three executables have to arrive together: the daemon (`seula.exe`), the
Tauri shell, and the plugin scanner worker. The worker has to sit next to the daemon,
because `locate_worker` only looks beside `current_exe()` and calls the file
`vst-meta.exe`.

The old releases were also named wrongly. The maintainer meant them to be `0.1.0` alphas.
That was fixed on 2026-10-08 by creating `0.1.0-alpha.1` and `0.1.0-alpha.2` on the same
commits, repointing the two GitHub releases and deleting the old tags. Three testers had
downloaded the old build.

## Decision

**One installer, built by Tauri's bundler.** `tauri build` produces an NSIS setup
executable, and that file is the release. `bundle.targets` is `["nsis"]`, so MSI is not
built. The install is per-user, which needs no administrator prompt.

**The daemon and the worker are sidecars.** They are listed in `bundle.externalBin`, built
with `cargo build --release` before `tauri build`, and installed in the same folder as the
shell. The worker keeps the name `vst-meta.exe` once installed, which is what the spawner
looks for. The target-triple suffix `externalBin` needs on the source files is not part of
the installed name.

**The runtime is the installer's job, not the user's.** The C runtime is linked statically
for the daemon and the worker (`+crt-static`), so no Visual C++ redistributable is
needed. WebView2 is handled by the installer's `webviewInstallMode`, chosen for the
Windows 10 case, since Windows 11 already has it.

**The version is `0.2.0-alpha.1`.** The label is alpha: features are still arriving and
the database may be discarded between builds. Later alphas count up as `-alpha.2`,
`-alpha.3`. The stable release is `1.0.0`, after a beta. A version is never reused. The
version is set in four places that must agree: the root `Cargo.toml`, `tauri.conf.json`,
`web/solid/package.json` and `web/solid/src-tauri/Cargo.toml`. The GitHub release is
marked as a pre-release.

**The product is named "Seula".** The "(solid)" suffix belonged to the trial. The identifier
is `app.seula.shell`.

**The alpha is unsigned.** Testers are told to expect a SmartScreen warning.

**The alpha does not wait for these, but they are known and listed in `docs/status.md`:**
the shell starting the daemon, a single-instance guard, "Start with Windows", log files,
the installer stopping the daemon on upgrade and uninstall, and the licence files and a
source link for the AGPL.

## Rejected alternatives

- **A second packaging tool (WiX, Inno Setup).** Tauri's bundler is already in the
  toolchain and produces the installer. A second tool would be a second place for the
  layout to be defined, for no feature the alpha needs.
- **`bundle.resources` for the daemon and the worker.** It copies files but does not treat
  them as executables. `externalBin` is the mechanism meant for sidecars and puts them
  beside the main executable, which is where the spawner looks.
- **MSI as well as NSIS.** An MSI requires a purely numeric pre-release identifier, which
  `-alpha.1` is not, and the alpha has no need of Group Policy deployment.
- **Restarting the numbers at `0.1.0-alpha.1`.** That version sorts below the old
  `1.0.1-alpha` tag, which was then still Latest, so an updater would have seen the old
  CLI build as the newer one. The tags were renamed instead of being left, and the
  rewrite became `0.2.0-alpha.1`.
- **Calling the rewrite `2.0.0-alpha.1`.** It was chosen first, because the old tags said
  `1.0`, and dropped once they were renamed. It implied a stable 1.x that never existed.
- **Waiting for a beta before packaging.** The installer is how the alpha gets tested on
  machines that are not the maintainer's, which is where missing DLLs and path
  assumptions show up.
- **Per-machine install.** It needs administrator rights and writes to Program Files for
  no gain. User data lives under the user's profile either way.

## Consequences

Testers install by running one file. Node and Rust are build-time only.

The cost is that the shell and the daemon are packaged together but are still two
processes (ADR-0048), and the packaging does not make them cooperate. Until the shell
starts the daemon, an installed alpha needs the daemon started by hand or from a
shortcut, and the frontend's default address still points at the mock daemon's port
(`web/shared/api.ts`). Both are on the path to a working alpha and are not solved by this
record.

An unsigned installer is the likelier thing to scare a tester away, and ADR-0050 already
expects antivirus trouble for unsigned executables. The alpha accepts that.

A schema bump discards testers' databases (ADR-0011). That is acceptable in an alpha only
if testers are told. A backup copy before the discard is the cheap fix, and is not part
of this record.

Distributing binaries brings the AGPL's source offer with it (ADR-0054). The installer
must ship the licence files, and the application needs a visible source link.

## Notes

Windows only. macOS and Linux have different packaging (notarisation, `.dmg`, AppImage)
and the code is untested there.

Reversing any part is cheap, since it is configuration. The version scheme is the
exception: a published version cannot be taken back, which is why it is written down
here.
