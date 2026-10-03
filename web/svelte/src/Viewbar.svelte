<script lang="ts">
  // The projects view's toolbar row: filters and paging, or batch actions at >1 selected.
  import { PAGE_SIZE, plural } from "../../shared/projects";
  import Icon from "./Icon.svelte";
  import TbBtn from "./TbBtn.svelte";
  import { clearSelection, goPage, push, runScan, setQuery, setScope, ui, view } from "./state.svelte";

  const from = $derived(view.total ? ui.page * PAGE_SIZE + 1 : 0);
  const to = $derived(Math.min(view.total, (ui.page + 1) * PAGE_SIZE));
</script>

{#if view.selectedCount > 1}
  <!-- Tag, collection and archive edits are not part of this comparison; the buttons are
       the mockup's, disabled. -->
  <span class="count-sel">{view.selectedCount} selected</span>
  <TbBtn icon="close" title="Clear selection" onclick={clearSelection} />
  <span class="sep"></span>
  <TbBtn icon="sell" label="Tags" caret disabled />
  <TbBtn icon="album" label="Collection" caret disabled />
  <TbBtn icon="archive" label="Archive" disabled />
{:else}
  <h1>Projects</h1>
  {#if ui.query}
    <span class="chip">
      <Icon name="search" />“{ui.query}” · {plural(view.total, "result")}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <span onclick={() => setQuery("")}><Icon name="close" /></span>
    </span>
    <span class="muted">{ui.sort ? "" : "by relevance"}</span>
  {:else}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span class="select" onclick={() => setScope(ui.scope === "archived" ? "active" : "archived")}>
      <span class="k">Show</span> {ui.scope === "archived" ? "Archived" : "Active"} <Icon name="expand_more" />
    </span>
    <span class="select"><span class="k">Tag</span> Any <Icon name="expand_more" /></span>
  {/if}
  <span class="sep"></span>
  <TbBtn icon="view_column" title="Columns" keep on={ui.popover === "columns"}
    onclick={() => (ui.popover = ui.popover === "columns" ? null : "columns")} />
  <TbBtn icon="refresh" label="Scan" title="Scan the project folders" disabled={!!push.scan} onclick={() => runScan(false)} />
  <TbBtn icon="speed" label="Simulate" title="A stand-in scan of 600 events at 40 a second" disabled={!!push.scan} onclick={() => runScan(true)} />
  <span class="grow"></span>
  <span class="pager">
    {from}–{to} of {view.total}
    <TbBtn icon="chevron_left" title="Previous page" disabled={ui.page === 0} onclick={() => goPage(-1)} />
    <TbBtn icon="chevron_right" title="Next page" disabled={ui.page >= view.pageCount - 1} onclick={() => goPage(1)} />
    <span class="select"><span class="k">Show</span> {PAGE_SIZE} <Icon name="expand_more" /></span>
  </span>
{/if}
