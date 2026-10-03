import { Show, createEffect, on, onMount } from "solid-js";
import { Icon, TbBtn } from "./parts";
import { Inspector } from "./Inspector";
import { ColumnChooser, ContextMenu, HoverList } from "./Popovers";
import { Sidebar, Statusbar, Topbar } from "./Shell";
import { ProjectTable } from "./Table";
import { Viewbar } from "./Viewbar";
import { load, loadError, loadSidecars, setUi, total, ui, selectedProjects } from "./state";

export function App() {
  onMount(() => { load(); loadSidecars(); });
  // The list follows the scope and the search, nothing else.
  createEffect(on(() => [ui.scope, ui.query], () => { load(); }, { defer: true }));

  const shown = () => ui.inspectorOpen;

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "F2" && ui.renaming == null) {
      const sel = selectedProjects();
      if (sel.length === 1) { e.preventDefault(); setUi("renaming", sel[0].id); }
    }
    if (e.key === "Escape") setUi({ menu: null, popover: null });
  };

  return (
    <div class="win" classList={{ "sidebar-collapsed": ui.sidebarCollapsed, "no-inspector": !shown() }}
      onKeyDown={onKey} tabIndex={-1}
      onClick={(e) => {
        // A click outside any popover closes the open ones.
        if (!(e.target as HTMLElement).closest(".pop, [data-keep]")) setUi({ menu: null, popover: null });
      }}>
      <Topbar />
      <div class="cols">
        <Sidebar />
        <section class="main">
          <div class="viewbar">
            <Viewbar />
            <span class="grow" />
            <TbBtn icon={shown() ? "right_panel_close" : "right_panel_open"} title="Inspector" on={shown()}
              onClick={() => setUi("inspectorOpen", (o) => !o)} />
          </div>
          <div class="content">
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
          </div>
        </section>
        <Show when={shown()}><Inspector /></Show>
      </div>
      <Statusbar />
      <Show when={ui.menu} keyed>{(m) => <ContextMenu menu={m} />}</Show>
      <Show when={ui.hot} keyed>{(h) => <HoverList hot={h} />}</Show>
      <Show when={ui.popover === "columns"}><ColumnChooser /></Show>
    </div>
  );
}
