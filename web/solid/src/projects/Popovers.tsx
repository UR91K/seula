// Popovers: the row context menu, the plugins/samples hover list and the column chooser.
// Each is placed against the window once it is in the DOM (shared/place.ts).

import { For, Show, onMount, type JSX } from "solid-js";
import { COLUMNS, missingPlugins, missingSamples } from "../../../shared/projects";
import { place, type Placement } from "../../../shared/place";
import type { Project } from "../../../shared/types";
import { Cb, Icon } from "../shell/parts";
import { setShell, showInExplorer } from "../shell/shell";
import type { TableState } from "./tablestate";

/** Mount-time placement: find the window and put the popover where it belongs. */
function placed(at: () => Placement | null) {
  return (el: HTMLElement) => onMount(() => {
    const win = el.closest(".win") as HTMLElement;
    const p = at();
    if (p) place(win, el, p);
  });
}

/** A project's row menu. `extra` adds entries of the caller's own, after the rename group. */
export function ContextMenu(props: { t: TableState; menu: { id: string; x: number; y: number }; extra?: JSX.Element }) {
  const m = () => props.menu;
  const many = () => props.t.selectedProjects().length > 1;
  const project = () => props.t.find(m().id);
  const item = (icon: string, label: string, opts: { off?: boolean; kbd?: string; act?: () => void } = {}) => (
    <div class="mi" classList={{ off: opts.off }} onClick={() => { opts.act?.(); setShell("menu", null); }}>
      <Icon name={icon} /><span class="lbl">{label}</span>
      <Show when={opts.kbd}><span class="kbd">{opts.kbd}</span></Show>
    </div>
  );
  return (
    <div class="pop menu" ref={placed(() => ({ x: m().x, y: m().y }))}>
      {item("open_in_new", "Open in Ableton", { off: true, kbd: "Enter" })}
      {item("folder_open", "Show in Explorer", { off: many(), act: () => { const p = project(); if (p) showInExplorer(p.path); } })}
      <div class="sep" />
      {item("sell", "Tags", { off: true })}
      {item("album", "Add to collection", { off: true })}
      <div class="sep" />
      {item("edit", "Rename", { off: many(), kbd: "F2", act: () => props.t.setRenaming(m().id) })}
      {item("music_note_add", "Add audition audio…", { off: true })}
      {props.extra}
      <div class="sep" />
      {item("archive", many() ? "Archive projects" : "Archive", { off: true, kbd: "Del" })}
    </div>
  );
}

export function HoverList(props: { t: TableState; hot: { id: string; kind: "plugins" | "samples" } }) {
  const hot = () => props.hot;
  const p = (): Project | undefined => props.t.find(hot().id);
  return (
    <Show when={p()}>{(proj) => {
      const plugins = () => hot().kind === "plugins";
      const items = () => (plugins() ? proj().plugins : proj().samples);
      const missing = () => (plugins() ? missingPlugins(proj()) : missingSamples(proj()));
      return (
        <div class="pop hoverlist" onMouseEnter={props.t.holdHover} onMouseLeave={props.t.releaseHover}
          ref={placed(() => {
            const cell = document.querySelector(`tr[data-id="${hot().id}"] td[data-hover="${hot().kind}"]`);
            return cell ? { anchor: cell, align: "right" } : null;
          })}>
          <div class="title">{plugins() ? "Plugins" : "Samples"} · {items().length}
            <Show when={missing()}><span class="miss">{missing()} missing</span></Show></div>
          <ul class="plain">
            <For each={items()}>{(x) => (
              <li>
                <span class="status-dot" classList={{
                  "is-ok": "installed" in x ? x.installed === true : x.is_present,
                  "is-missing": "installed" in x ? x.installed === false : !x.is_present,
                  "is-unknown": "installed" in x && x.installed == null,
                }}><Icon fill name={"installed" in x ? (x.installed === true ? "check_circle" : x.installed === false ? "error" : "help") : x.is_present ? "check_circle" : "error"} /></span>
                <span class="grow">{x.name}</span>
                <Show when={"vendor" in x && x.vendor}><span class="faint">{"vendor" in x ? x.vendor : ""}</span></Show>
              </li>
            )}</For>
          </ul>
        </div>
      );
    }}</Show>
  );
}

export function ColumnChooser(props: { t: TableState }) {
  return (
    <div class="pop picker" ref={placed(() => {
      const btn = document.querySelector('.viewbar .tb-btn[title="Columns"]');
      return btn ? { anchor: btn } : null;
    })}>
      <div class="list" style={{ "max-height": "none", "padding-top": "var(--u)" }}>
        <For each={["Audition", "Name", "Tags"]}>{(l) => (
          <div class="it" style={{ color: "var(--text-3)" }}><Cb state={true} /><span class="grow">{l}</span><Icon name="lock" /></div>
        )}</For>
        <div class="menu"><div class="sep" /></div>
        <For each={COLUMNS}>{(c) => (
          <div class="it" onClick={() => props.t.toggleColumn(c.id)}>
            <Cb state={props.t.columns().includes(c.id)} /><span class="grow">{c.label}</span>
          </div>
        )}</For>
      </div>
    </div>
  );
}
