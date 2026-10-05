import { Show, createEffect, on, onMount } from "solid-js";
import { plural } from "../../../shared/projects";
import { Icon } from "../shell/parts";
import { loadCollectionNames, shell } from "../shell/shell";
import type { View } from "../shell/view";
import { SelectionInspector } from "./Inspector";
import { ColumnChooser, ContextMenu, HoverList } from "./Popovers";
import { ProjectTable } from "./Table";
import { Viewbar } from "./Viewbar";
import { load, loadError, setQuery, table as t, ui } from "./state";

function Content() {
  onMount(() => { load(); loadCollectionNames(); });
  // The list follows the scope and the search, nothing else.
  createEffect(on(() => [ui.scope, ui.query], () => { load(); }, { defer: true }));
  return (
    <>
      <Show when={loadError()}><div class="error-banner">Cannot reach the daemon: {loadError()}</div></Show>
      <Show when={t.total() > 0} fallback={
        <Show when={!loadError()}>
          <div class="empty">
            <Icon name={ui.query ? "search_off" : "inventory_2"} />
            <h2>{ui.query ? `No projects match “${ui.query}”` : ui.scope === "archived" ? "No archived projects" : "No projects yet"}</h2>
          </div>
        </Show>}>
        <ProjectTable t={t} />
      </Show>
    </>
  );
}

function Inspector() {
  return (
    <aside class="inspector">
      <SelectionInspector t={t} fallback={
        <div class="empty"><Icon name="right_panel_open" /><p>Select a project to see its details here.</p></div>} />
    </aside>
  );
}

function Status() {
  const count = () => ui.query ? plural(t.total(), "result") : `${t.total()} ${ui.scope === "archived" ? "archived" : "projects"}`;
  return (
    <>
      <span>{count()}</span>
      <Show when={t.selectedCount()}><span>{t.selectedCount()} selected</span></Show>
    </>
  );
}

function Popovers() {
  return (
    <>
      <Show when={shell.menu} keyed>{(m) => <ContextMenu t={t} menu={m} />}</Show>
      <Show when={t.hot()} keyed>{(h) => <HoverList t={t} hot={h} />}</Show>
      <Show when={shell.popover === "columns"}><ColumnChooser t={t} /></Show>
    </>
  );
}

export const projectsView: View = {
  Viewbar, Content, Inspector, Status, Popovers, keys: true,
  query: () => ui.query,
  setQuery,
  onKey(e) {
    if (e.key === "F2" && t.renaming() == null) {
      const sel = t.selectedProjects();
      if (sel.length === 1) { e.preventDefault(); t.setRenaming(sel[0].id); }
    }
  },
};
