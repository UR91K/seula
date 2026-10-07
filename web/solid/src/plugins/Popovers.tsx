// The plugins view's menus and pickers: the row context menu and the four toolbar
// dropdowns. Each is placed against the window once it is in the DOM (shared/place.ts).

import { For, Show, onMount, type JSX } from "solid-js";
import { INSTALL_STATES, type Group, type InstallState } from "../../../shared/plugins";
import { place, type Placement } from "../../../shared/place";
import { Cb, Icon } from "../shell/parts";
import { api, setShell, showInExplorer } from "../shell/shell";
import { StatusDot } from "./parts";
import {
  copyName, formats, plugins, pui, scanPlugins, setFormat, setGroup, setPui, setVendor, showProjectsUsing, stats,
  toggleState, vendors,
} from "./state";

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

function Item(props: { hot?: boolean; onClick(): void; children: JSX.Element }) {
  return (
    <div class="it" classList={{ hot: props.hot }} onClick={() => props.onClick()}>
      <Icon name="check" class={props.hot ? "" : "blank"} />
      {props.children}
    </div>
  );
}

export function ContextMenu(props: { menu: { id: string; x: number; y: number } }) {
  const p = () => plugins.find((x) => x.id === props.menu.id);
  const item = (icon: string, label: string, opts: { off?: boolean; act?: () => void } = {}) => (
    <div class="mi" classList={{ off: opts.off }} onClick={() => { opts.act?.(); setShell("menu", null); }}>
      <Icon name={icon} /><span class="lbl">{label}</span>
    </div>
  );
  const explorer = async () => {
    const x = p();
    if (!x) return;
    // A failed file's row has its path; a plugin's is in what the scan recorded.
    const path = x.file?.path ?? (await api.pluginDetails(x.id)).path;
    if (path) showInExplorer(path);
  };
  return (
    <Show when={p()}>{(x) => (
      <div class="pop menu" ref={placed(() => ({ x: props.menu.x, y: props.menu.y }))}>
        {item("folder_open", "Show in Explorer", { off: x().installed !== true && !x().file, act: explorer })}
        {item("audio_file", `Show the ${x().project_count} ${x().project_count === 1 ? "project" : "projects"} using it`,
          { off: !x().project_count, act: () => showProjectsUsing(x()) })}
        <div class="sep" />
        {item("content_copy", "Copy name", { act: () => copyName(x()) })}
      </div>
    )}</Show>
  );
}

/** Under the scan button's caret: the two scans (ADR-0067). */
export function ScanMenu() {
  const item = (icon: string, label: string, title: string, mode: "changes" | "all") => (
    <div class="mi" title={title} onClick={() => { setShell("popover", null); scanPlugins(mode); }}>
      <Icon name={icon} /><span class="lbl">{label}</span>
    </div>
  );
  return (
    <div class="pop menu" ref={placed(anchorFor("scan"))}>
      {item("radar", "Scan for changes", "Load only plugin files that are new or have changed. Files that failed are left as they are.", "changes")}
      {item("refresh", "Rescan all", "Load every plugin file again, retrying the ones that failed. Takes minutes.", "all")}
    </div>
  );
}

/** Format ▾: every format, with its plugin count from /plugins/formats. */
export function FormatMenu() {
  return (
    <div class="pop picker pickmenu" ref={placed(anchorFor("format"))}>
      <div class="list">
        <Item hot={pui.format == null} onClick={() => setFormat(null)}>
          <span class="grow">All formats</span><span class="n">{plugins.length}</span>
        </Item>
        <For each={formats()}>{(f) => (
          <Item hot={pui.format === f.format} onClick={() => setFormat(f.format)}>
            <span class="grow">{f.format}</span><span class="n">{f.plugin_count}</span>
          </Item>
        )}</For>
      </div>
    </div>
  );
}

/** Vendor ▾: many and growing, so it filters as you type, like the tag picker. */
export function VendorPicker() {
  const hits = () => {
    const q = pui.vendorQuery.toLowerCase();
    return [...vendors()].sort((a, b) => a.vendor.localeCompare(b.vendor)).filter((v) => v.vendor.toLowerCase().includes(q));
  };
  const mark = (name: string) => {
    const q = pui.vendorQuery;
    const i = q ? name.toLowerCase().indexOf(q.toLowerCase()) : -1;
    return i < 0 ? name : <>{name.slice(0, i)}<b>{name.slice(i, i + q.length)}</b>{name.slice(i + q.length)}</>;
  };
  return (
    <div class="pop picker pickmenu" ref={placed(anchorFor("vendor"))}>
      <label class="field focus">
        <Icon name="search" />
        <input placeholder="Find a vendor" value={pui.vendorQuery}
          ref={(el) => queueMicrotask(() => el.focus())}
          onInput={(e) => setPui("vendorQuery", e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && hits().length === 1) setVendor(hits()[0].vendor); }} />
      </label>
      <div class="list">
        <Show when={!pui.vendorQuery}>
          <Item hot={pui.vendor == null} onClick={() => setVendor(null)}><span class="grow">All vendors</span></Item>
        </Show>
        <For each={hits()} fallback={<div class="hint">No vendor matches</div>}>{(v) => (
          <Item hot={pui.vendor === v.vendor} onClick={() => setVendor(v.vendor)}>
            <span class="grow">{mark(v.vendor)}</span><span class="n">{v.plugin_count}</span>
          </Item>
        )}</For>
      </div>
    </div>
  );
}

/** Status ▾: any of the four states (ADR-0025, ADR-0067), each with its library count. */
export function StatesMenu() {
  const n = (id: InstallState) => {
    const s = stats();
    return { installed: s.installed, absent: s.missing, failed: s.failed, unscanned: s.unscanned }[id];
  };
  return (
    <div class="pop picker pickmenu" ref={placed(anchorFor("states"))}>
      <div class="list" style={{ "padding-top": "var(--u)" }}>
        <For each={INSTALL_STATES}>{(s) => (
          <div class="it" onClick={() => toggleState(s.id)}>
            <Cb state={pui.states.includes(s.id)} /><StatusDot state={s.id} />
            <span class="grow">{s.label}</span><span class="n">{n(s.id)}</span>
          </div>
        )}</For>
      </div>
      <div class="hint">None ticked shows all. Not scanned is not missing: no scan has looked yet. Failed is there but did not load.</div>
    </div>
  );
}

export function GroupMenu() {
  const opt = (v: Group | null, label: string) => (
    <Item hot={pui.group === v} onClick={() => setGroup(v)}><span class="grow">{label}</span></Item>
  );
  return (
    <div class="pop picker pickmenu" style={{ width: "calc(var(--u) * 70)" }} ref={placed(anchorFor("group"))}>
      <div class="list" style={{ "padding-top": "var(--u)" }}>
        {opt(null, "No grouping")}{opt("vendor", "By vendor")}{opt("format", "By format")}
      </div>
    </div>
  );
}
