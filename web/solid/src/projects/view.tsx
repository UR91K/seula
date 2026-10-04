import { Show, createEffect, on, onMount } from "solid-js";
import { plural } from "../../../shared/projects";
import { Icon } from "../shell/parts";
import { shell } from "../shell/shell";
import type { View } from "../shell/view";
import { Inspector } from "./Inspector";
import { ColumnChooser, ContextMenu, HoverList } from "./Popovers";
import { ProjectTable } from "./Table";
import { Viewbar } from "./Viewbar";
import { load, loadCollections, loadError, selectedCount, selectedProjects, setQuery, setUi, total, ui } from "./state";

function Content() {
  onMount(() => { load(); loadCollections(); });
  // The list follows the scope and the search, nothing else.
  createEffect(on(() => [ui.scope, ui.query], () => { load(); }, { defer: true }));
  return (
    <>
      <Show when={loadError()}><div class="error-banner">Cannot reach the daemon: {loadError()}</div></Show>
      <Show when={total() > 0} fallback={
        <Show when={!loadError()}>
          <div class="empty">
            <Icon name={ui.query ? "search_off" : "inventory_2"} />
            <h2>{ui.query ? `No projects match “${ui.query}”` : ui.scope === "archived" ? "No archived projects" : "No projects yet"}</h2>
          </div>
        </Show>}>
        <ProjectTable />
      </Show>
    </>
  );
}

function Status() {
  const count = () => ui.query ? plural(total(), "result") : `${total()} ${ui.scope === "archived" ? "archived" : "projects"}`;
  return (
    <>
      <span>{count()}</span>
      <Show when={selectedCount()}><span>{selectedCount()} selected</span></Show>
    </>
  );
}

function Popovers() {
  return (
    <>
      <Show when={shell.menu} keyed>{(m) => <ContextMenu menu={m} />}</Show>
      <Show when={ui.hot} keyed>{(h) => <HoverList hot={h} />}</Show>
      <Show when={shell.popover === "columns"}><ColumnChooser /></Show>
    </>
  );
}

export const projectsView: View = {
  Viewbar, Content, Inspector, Status, Popovers, keys: true,
  query: () => ui.query,
  setQuery,
  onKey(e) {
    if (e.key === "F2" && ui.renaming == null) {
      const sel = selectedProjects();
      if (sel.length === 1) { e.preventDefault(); setUi("renaming", sel[0].id); }
    }
  },
};
