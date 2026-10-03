// The OS calls, the only use of Tauri's IPC (ADR-0048). Everything else is HTTP.

type Tauri = { core: { invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> } };

export const inTauri = () => "__TAURI__" in window;

export async function revealInExplorer(path: string): Promise<void> {
  const tauri = (window as unknown as { __TAURI__?: Tauri }).__TAURI__;
  if (!tauri) throw new Error("Show in Explorer needs the Tauri shell");
  await tauri.core.invoke("reveal_in_explorer", { path });
}
