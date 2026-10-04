# 0054. The window has no native frame; the top bar is the title bar

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-04, with the shell built and the native title bar sitting above
  a top bar that already has its own logo, menu and search
- **Decided:** 2026-10-04, accepted by the maintainer after running it (proposed the same
  day)
- **Recorded:** 2026-10-04
- **Confidence:** decided now; drafted by the assistant, then built and run by the
  maintainer on Windows 11, who reported the window, controls and snap layouts working.
  The places below marked unverified were written before that run and were not each
  recorded separately afterwards: treat them as working, not as individually measured
- **Evidence:** ADR-0048 (Tauri is a thin shell; IPC is for OS calls only);
  ADR-0032 (the frame: top bar, sidebar, status bar); ADR-0052 (the shell owns the frame);
  `web/solid/src-tauri/tauri.conf.json` (one window, default decorations, so a native
  title bar); `web/solid/src-tauri/capabilities/default.json` (no permissions granted);
  `Topbar` in `web/solid/src/shell/Frame.tsx` (ends in an empty spacer where controls
  would go)

## Context

The window currently has Windows' own title bar: an icon, the title "Seula (solid)", and
minimise, maximise and close. Directly under it is the app's top bar, which carries the
logo, the File/Edit/View/Tools/Help menu, search and settings. The result is two stacked
bars, and the native one shows nothing the app's own bar does not.

Turning decorations off removes the native bar and lets the top bar take its row. That
trades something for something: the app now owns what the OS did for free.

## Decision

**Decorations are off.** The window is created with `"decorations": false` in
`tauri.conf.json`. There is still one window (ADR-0048).

**The top bar is the drag region.** The empty parts of the top bar are marked with Tauri's
`data-tauri-drag-region`. Interactive children (logo if clickable, menu entries, search,
buttons) are not part of the region, so clicks reach them. Double-clicking the region
toggles maximise, which Tauri provides for drag regions (unverified on the pinned version).

**The app draws minimise, maximise/restore and close at the right end of the top bar.**
They replace the spacer `Topbar` already ends with. They are three buttons in `parts.tsx`
styled with the existing tokens. Close is the one with the danger hover, as on Windows.

**The controls call Tauri's window API.** This is Tauri IPC, but for the OS and not for
data, so it is inside the line ADR-0048 draws. `capabilities/default.json` grants exactly
what is used: minimise, toggle-maximise, close, start-dragging (the drag region needs
it), internal-toggle-maximise (double-click on the region), the is-maximized query so the
middle button can show restore, and `core:event:default` so the page can listen for
resizes. Nothing else is added. The calls go through the global (`withGlobalTauri` is already on), so no
new npm dependency is needed.

**The glyphs are Windows' own caption-button icons** (Segoe Fluent Icons, falling back to
Segoe MDL2 Assets), as VS Code shows on Windows, where its buttons are the OS's. They come
from a system font, so nothing is bundled. VS Code's own `chrome-*` codicons were the other
candidate: CC-BY-4.0, needing attribution and a bundled font, and no closer to what Windows
draws.

**Closing the window closes the window.** The daemon is a separate process (ADR-0048), so
the close button ends the Tauri app and leaves scans and watching running. It does not
quit the daemon. Quit stays in the tray menu.

**Resizing stays the OS's.** The window remains `resizable`, and the edges are expected to
keep working with decorations off on Windows (unverified). If they do not, an invisible
resize border is drawn by the app, which is a larger change and would be a reason to
revisit this ADR.

## Rejected alternatives

- **Keep the native title bar.** Free, correct everywhere, and it keeps Windows 11's snap
  layouts on the maximise button. Rejected because it duplicates the top bar and wastes a
  row of a layout where vertical space is the scarce thing (ADR-0032).
- **Overlay title bar (native buttons drawn over app content).** This is how VS Code
  keeps real snap layouts: Electron's `titleBarOverlay` leaves the OS's own caption buttons
  in place. Tauri 2 offers it on macOS only, and nothing on Windows. Not available.
- **`tauri-plugin-decorum`** (read in `references/tauri-plugin-decorum`, v1.1.1). It does
  not use native caption buttons. It sets `decorations(false)`, injects JavaScript with
  `eval` that builds its own button elements and drag region outside the framework's DOM,
  and fakes snap layouts: after the cursor rests on the maximise button for 620 ms it
  focuses the window and has `enigo` synthesise Win+Z, then taps Alt to hide the numbered
  hints. Rejected as a dependency because the part we would take is small and the part we
  would not want is the rest: injected DOM that Solid does not own (ADR-0052), a
  synthesised-keyboard-input crate in a shell meant to do three OS calls (ADR-0048), which
  is also the kind of behaviour antivirus dislikes while the executables are unsigned
  (ADR-0050), and the hover is a timer guess, not the OS's own hit test. Its maximise icon
  and `data-tauri-drag-region` handling are the same calls this ADR makes directly.
  Adoptable later for snap layouts alone: the snap trick is one Tauri command and about
  ten lines.
- **Make the native bar match the theme** (dark mode, accent colour). Considered and
  rejected as the half measure: it still costs a row and still can't hold the menu and
  search.

## Consequences

Things the native bar gave for free that are lost or must be rebuilt:

- **Snap layouts.** Hovering the maximise button no longer offers Windows 11's layout
  picker, because the button is not the OS's. Win+Z and dragging to a screen edge still
  work (unverified). It can be approximated as decorum does (a hover timer that sends
  Win+Z), at the cost described under that alternative. Not part of this decision; the
  first version ships without it.
- **The window menu.** Right-clicking the title area, and Alt+Space, may not show
  Restore/Move/Size/Close. Not provided unless built.
- **Keyboard and accessibility.** The three buttons need real labels and focus order. A
  screen reader no longer finds a title bar.
- **Looks and drop shadow.** Whether the borderless window keeps its shadow and rounded
  corners on Windows 11 is unverified. The first implementation should check both and
  record the answer here or in `docs/status.md`.
- **Maximised state.** The middle button's icon must follow the window being maximised by
  other means (drag to top, keyboard), so it listens for resize and re-queries rather than
  tracking its own clicks.
- **Dev experience.** The webview in `tauri dev` and in the packaged app both lose the
  native bar. Nothing else changes, since the page never ran in a browser tab (ADR-0048).

The top bar's existing menu is placeholder text. Its dropdowns will sit in the draggable
row, so menu entries must be outside the region, or the first click starts a drag.

macOS and Linux are untested (CLAUDE.md). Borderless windows behave differently there
(traffic-light position, no native shadow on some Linux compositors). This ADR is decided
for Windows only.

## Notes

**Real snap layouts later, if wanted.** VS Code gets them from Electron's
`titleBarOverlay` (`windows.ts` in `references/vscode`: `titleBarStyle: 'hidden'`,
`frame: false`, then `titleBarOverlay` with a height and colours), which leaves Chromium's
own caption buttons in place, and the page reserves their space with
`env(titlebar-area-*)` (`titlebarpart.css`). That is Chromium window code, so it does not
exist under WebView2. The cause is that Windows offers the flyout only to a window that
answers `WM_NCHITTEST` with `HTMAXBUTTON`, and WebView2's child window covers the client
area, so the page cannot answer. The workaround several Tauri plugins use, per their
descriptions and not read by us, is a transparent native overlay window placed exactly over
the maximise button that does answer `HTMAXBUTTON`. Three plugins do this; all were read
in `references/` (read, not run):

- **`tauri-plugin-snap-layout`** (1.0.9, about 660 lines of Rust). Overlay only. It draws
  nothing: the page names its own button by element id, reports the button's bounds, and
  the plugin keeps a transparent child window over it that answers `HTMAXBUTTON` and emits
  enter and leave events so the page can style the hover. Our Solid button stays ours,
  themed with our tokens.
- **`tauri-plugin-frame`** (1.1.8, about 990 lines). The same overlay, plus injected
  buttons and drag region like decorum's. Takes the overlay and brings the injected DOM
  with it, which this ADR rejects above.
- **`tauri-plugin-window-controls`** (0.1.0, about 1000 lines, mostly one 914-line GDI
  file). Draws the whole caption, buttons and glyphs, as native layered windows above
  WebView2. The most VS Code-like result: nothing in the page, real buttons. But the
  pixels are grey-scale white or black chosen by the *system* theme, and the app's theme
  is its own (`data-theme`, dark by default, ADR-0032), so a light-mode system with the
  dark app gives dark glyphs on a dark bar unless the window's theme is forced to match.
  Fixed 46-pixel buttons, needs an app manifest, the youngest and least exercised of the
  three, and the README's install line says 0.2 while the crate says 0.1.0.

Direction, agreed in conversation 2026-10-04, written the same day and not yet run: **the
overlay is our own, modelled on `snap-layout`'s `snap.rs`** (`web/solid/src-tauri/src/snap.rs`,
one command `set_snap_bounds`, one `snap-hover` event), added after the plain buttons.
Known limit: the overlay is native and sits above the webview, so a dialog scrim or popup
drawn over the maximise button does not cover it, and a click there still maximises. `snap-layout`'s division of labour is the right one (the page owns the button, the
overlay owns only the hit test, and handles the click itself since it covers the button).
All three plugins are real native hit-testing, unlike decorum's Win+Z keystroke, and all are
Win32 code in the shell that tracks the button on every resize. Writing it ourselves is a
window class, a subclass procedure and one `SetWindowPos` in `src-tauri`, reached by one
command that the page calls with the button's bounds, so the Win32 is ours and understood
(ADR-0048) and no JS is injected. `snap-layout` is MIT; if code is taken from it, say so
at the site. The fallback is to use `snap-layout` itself. Not
verified: that any of them works with the pinned Tauri (2.12.1) and WebView2, that the
flyout shows, that the overlay survives maximise and DPI changes. Whoever adopts one
records that as its own ADR. The HTML button this ADR draws is what it would sit over, so
nothing here has to be undone.

If this is accepted, implementing it is one config key, one capability file, one small
component and the attribute on `Topbar`. It is small enough that it can be tried and
reverted before the ADR is accepted, and the first things to look at are the three
unverified items: resize edges, shadow and corners, and double-click to maximise.
