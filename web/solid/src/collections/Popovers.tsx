// The collections view's card menu and its Sort dropdown, placed against the window once
// they are in the DOM (shared/place.ts).

import { For, Show, onMount } from "solid-js";
import { COLLECTION_SORTS, sameSort } from "../../../shared/collections";
import { place, type Placement } from "../../../shared/place";
import { Icon } from "../shell/parts";
import { setShell } from "../shell/shell";
import { collections, cui, openCollection, openDialog, setCui, setSort, showProjectsIn } from "./state";

function placed(at: () => Placement | null) {
  return (el: HTMLElement) => onMount(() => {
    const win = el.closest(".win") as HTMLElement;
    const p = at();
    if (p) place(win, el, p);
  });
}

export function ContextMenu(props: { menu: { id: string; x: number; y: number } }) {
  const c = () => collections.find((x) => x.id === props.menu.id);
  const item = (icon: string, label: string, act: () => void, kbd?: string) => (
    <div class="mi" onClick={() => { setShell("menu", null); act(); }}>
      <Icon name={icon} /><span class="lbl">{label}</span>
      <Show when={kbd}><span class="kbd">{kbd}</span></Show>
    </div>
  );
  return (
    <Show when={c()}>{(x) => (
      <div class="pop menu" ref={placed(() => ({ x: props.menu.x, y: props.menu.y }))}>
        {item("open_in_full", "Open", () => openCollection(x().id), "Enter")}
        {item("audio_file", "Show in Projects", () => showProjectsIn(x()))}
        <div class="sep" />
        {item("edit", "Rename", () => setCui("renaming", x().id), "F2")}
        {item("edit_note", "Edit details…", () => openDialog("edit", x().id))}
        {item("content_copy", "Duplicate…", () => openDialog("duplicate", x().id))}
        <div class="sep" />
        {item("delete", "Delete…", () => openDialog("delete", x().id), "Del")}
      </div>
    )}</Show>
  );
}

/** Sort ▾ for the grid: what each choice asks the server for. */
export function SortMenu() {
  const anchor = () => {
    const el = document.querySelector('.viewbar [data-pop="sort"]');
    return el ? { anchor: el } : null;
  };
  return (
    <div class="pop picker pickmenu" style={{ width: "calc(var(--u) * 80)" }} ref={placed(anchor)}>
      <div class="list" style={{ "max-height": "none", "padding-top": "var(--u)" }}>
        <For each={COLLECTION_SORTS}>{(s) => {
          const on = () => sameSort(s.sort, cui.sort);
          return (
            <div class="it" classList={{ hot: on() }} onClick={() => setSort(s.sort)}>
              <Icon name="check" class={on() ? "" : "blank"} /><span class="grow">{s.label}</span>
            </div>
          );
        }}</For>
      </div>
    </div>
  );
}
