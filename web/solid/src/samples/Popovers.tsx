// The samples view's row context menu and its two toolbar dropdowns. Each is placed against
// the window once it is in the DOM (shared/place.ts).

import { For, Show, onMount, type JSX } from "solid-js";
import { PRESENCE } from "../../../shared/samples";
import { place, type Placement } from "../../../shared/place";
import { inTauri } from "../../../shared/os";
import { Icon } from "../shell/parts";
import { setShell, showInExplorer } from "../shell/shell";
import { PresentDot } from "./parts";
import { copyPath, formats, samples, setFormat, setPresence, showProjectsUsing, sui } from "./state";

/** Mount-time placement: find the window and put the popover where it belongs. */
function placed(at: () => Placement | null) {
  return (el: HTMLElement) => onMount(() => {
    const win = el.closest(".win") as HTMLElement;
    const p = at();
    if (p) place(win, el, p);
  });
}

/** Placed under the toolbar dropdown that opened it. */
const anchorFor = (name: string) => () => {
  const el = document.querySelector(`.viewbar [data-pop="${name}"]`);
  return el ? { anchor: el } : null;
};

function Item(props: { hot?: boolean; title?: string; off?: boolean; onClick(): void; children: JSX.Element }) {
  return (
    <div class="it" classList={{ hot: props.hot, off: props.off }} title={props.title} onClick={() => props.onClick()}>
      <Icon name="check" class={props.hot ? "" : "blank"} />
      {props.children}
    </div>
  );
}

export function ContextMenu(props: { menu: { id: string; x: number; y: number } }) {
  const s = () => samples.find((x) => x.id === props.menu.id);
  const item = (icon: string, label: string, opts: { off?: boolean; act?: () => void } = {}) => (
    <div class="mi" classList={{ off: opts.off }} onClick={() => { opts.act?.(); setShell("menu", null); }}>
      <Icon name={icon} /><span class="lbl">{label}</span>
    </div>
  );
  return (
    <Show when={s()}>{(x) => (
      <div class="pop menu" ref={placed(() => ({ x: props.menu.x, y: props.menu.y }))}>
        {/* Play needs a Tauri command that does not exist yet (ADR-0048), so it is not offered. */}
        <Show when={inTauri()}>
          {item("folder_open", "Show in Explorer", { off: !x().is_present, act: () => showInExplorer(x().path) })}
          <div class="sep" />
        </Show>
        {item("audio_file", `Show the ${x().project_count} ${x().project_count === 1 ? "project" : "projects"} using it`,
          { off: !x().project_count, act: () => showProjectsUsing(x()) })}
        {item("content_copy", "Copy path", { act: () => copyPath(x()) })}
      </div>
    )}</Show>
  );
}

/** Format ▾: every format Live loads, with its count; AIFF covers .aif, .aiff and .aifc (ADR-0039). */
export function FormatMenu() {
  const exts = (e: string[]) => e.map((x) => `.${x}`).join(" ");
  return (
    <div class="pop picker pickmenu" ref={placed(anchorFor("format"))}>
      <div class="list" style={{ "padding-top": "var(--u)" }}>
        <Item hot={sui.format == null} onClick={() => setFormat(null)}>
          <span class="grow">All formats</span><span class="n">{samples.length}</span>
        </Item>
        <For each={formats()}>{(f) => (
          <Item hot={sui.format === f.format} off={!f.count} title={exts(f.extensions)} onClick={() => setFormat(f.format)}>
            <span class="grow">{f.name} <span class="faint">{exts(f.extensions)}</span></span><span class="n">{f.count}</span>
          </Item>
        )}</For>
      </div>
    </div>
  );
}

/** Status ▾: present or missing, with the counts of the whole list. */
export function PresenceMenu() {
  const n = (present: boolean) => samples.filter((s) => s.is_present === present).length;
  return (
    <div class="pop picker pickmenu" style={{ width: "calc(var(--u) * 80)" }} ref={placed(anchorFor("presence"))}>
      <div class="list" style={{ "padding-top": "var(--u)" }}>
        <Item hot={sui.presence == null} onClick={() => setPresence(null)}>
          <span class="grow">All samples</span><span class="n">{samples.length}</span>
        </Item>
        <For each={PRESENCE}>{(p) => (
          <Item hot={sui.presence === p.id} onClick={() => setPresence(p.id)}>
            <PresentDot present={p.value} /><span class="grow">{p.label}</span><span class="n">{n(p.value)}</span>
          </Item>
        )}</For>
      </div>
    </div>
  );
}
