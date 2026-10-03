<script lang="ts">
  import { onMount } from "svelte";
  import ColumnChooser from "./ColumnChooser.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import HoverList from "./HoverList.svelte";
  import Icon from "./Icon.svelte";
  import Inspector from "./Inspector.svelte";
  import Sidebar from "./Sidebar.svelte";
  import Statusbar from "./Statusbar.svelte";
  import Table from "./Table.svelte";
  import TbBtn from "./TbBtn.svelte";
  import Topbar from "./Topbar.svelte";
  import Viewbar from "./Viewbar.svelte";
  import { data, load, loadSidecars, ui, view } from "./state.svelte";

  onMount(() => { load(); loadSidecars(); });

  // The list follows the scope and the search, nothing else.
  let first = true;
  $effect(() => {
    ui.scope; ui.query;   // the two reads this effect depends on
    if (first) { first = false; return; }
    load();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "F2" && ui.renaming == null && view.selectedProjects.length === 1) {
      e.preventDefault();
      ui.renaming = view.selectedProjects[0].id;
    }
    if (e.key === "Escape") { ui.menu = null; ui.popover = null; }
  }

  // A click outside any popover closes the open ones.
  function onclick(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest(".pop, [data-keep]")) { ui.menu = null; ui.popover = null; }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="win" class:sidebar-collapsed={ui.sidebarCollapsed} class:no-inspector={!ui.inspectorOpen}
  {onkeydown} {onclick} tabindex="-1">
  <Topbar />
  <div class="cols">
    <Sidebar />
    <section class="main">
      <div class="viewbar">
        <Viewbar />
        <span class="grow"></span>
        <TbBtn icon={ui.inspectorOpen ? "right_panel_close" : "right_panel_open"} title="Inspector" on={ui.inspectorOpen}
          onclick={() => (ui.inspectorOpen = !ui.inspectorOpen)} />
      </div>
      <div class="content">
        {#if data.loadError}<div class="error-banner">Cannot reach the daemon: {data.loadError}</div>{/if}
        {#if view.total > 0}
          <Table />
        {:else if !data.loadError}
          <div class="empty">
            <Icon name={ui.query ? "search_off" : "inventory_2"} />
            <h2>{ui.query ? `No projects match “${ui.query}”` : ui.scope === "archived" ? "No archived projects" : "No projects yet"}</h2>
          </div>
        {/if}
      </div>
    </section>
    {#if ui.inspectorOpen}<Inspector />{/if}
  </div>
  <Statusbar />
  <!-- Keyed on the object, so a new menu or hover target is a fresh popover, placed anew. -->
  {#key ui.menu}{#if ui.menu}<ContextMenu menu={ui.menu} />{/if}{/key}
  {#key ui.hot}{#if ui.hot}<HoverList hot={ui.hot} />{/if}{/key}
  {#if ui.popover === "columns"}<ColumnChooser />{/if}
</div>
