// The OS calls, the only use of Tauri's IPC (ADR-0048). Everything else is HTTP.

type Tauri = { core: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> } };

export const inTauri = () => "__TAURI__" in window;

export async function revealInExplorer(path: string): Promise<void> {
  const tauri = (window as unknown as { __TAURI__?: Tauri }).__TAURI__;
  if (!tauri) throw new Error("Show in Explorer needs the Tauri shell");
  await tauri.core.invoke("reveal_in_explorer", { path });
}

// The window itself (ADR-0054): the title bar is ours, so the buttons call these. They use
// Tauri's window API through the global, which the capability file grants one call at a time.

type TauriWindow = {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  isMaximized(): Promise<boolean>;
  onResized(handler: () => void): Promise<() => void>;
};

const appWindow = (): TauriWindow | null =>
  (window as unknown as { __TAURI__?: { window: { getCurrentWindow(): TauriWindow } } }).__TAURI__
    ?.window.getCurrentWindow() ?? null;

export const minimizeWindow = () => void appWindow()?.minimize();
export const toggleMaximizeWindow = () => void appWindow()?.toggleMaximize();
export const closeWindow = () => void appWindow()?.close();
export const isWindowMaximized = async () => (await appWindow()?.isMaximized()) ?? false;

/** Calls `handler` whenever the window is resized, which includes being maximised or restored
 * by any means. Returns a function that stops listening. */
export async function onWindowResized(handler: () => void): Promise<() => void> {
  return (await appWindow()?.onResized(handler)) ?? (() => {});
}

// Snap layouts (ADR-0054): a native overlay sits over the maximise button, so the page tells
// the shell where the button is and hears about hover from the shell.

type TauriEvents = {
  core: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> };
  event: { listen(event: string, handler: (e: { payload: unknown }) => void): Promise<() => void> };
};

const tauri = () => (window as unknown as { __TAURI__?: TauriEvents }).__TAURI__ ?? null;

/** The button's rectangle in physical pixels, relative to the window's client area. */
export const setSnapBounds = (x: number, y: number, width: number, height: number) =>
  void tauri()?.core.invoke("set_snap_bounds", { x, y, width, height }).catch(() => {});

/** Calls `handler(true)` when the cursor enters the overlay and `handler(false)` when it leaves. */
export async function onSnapHover(handler: (hovering: boolean) => void): Promise<() => void> {
  return (await tauri()?.event.listen("snap-hover", (e) => handler(e.payload === true))) ?? (() => {});
}
