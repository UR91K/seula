<script lang="ts">
  // The projects table. Rows are keyed by project, so a sort or a page change moves the
  // existing row nodes, and an edit to one project touches only its own cells.
  import { COLUMNS, columnById, nextSort } from "../../shared/projects";
  import Cb from "./Cb.svelte";
  import Icon from "./Icon.svelte";
  import Row from "./Row.svelte";
  import { checkAll, isSelected, sortBy, ui, view } from "./state.svelte";

  const cols = $derived(ui.columns.map(columnById).sort((a, b) => COLUMNS.indexOf(a) - COLUMNS.indexOf(b)));
  const head = $derived.by(() => {
    const ids = view.pageIds;
    const on = ids.filter(isSelected).length;
    return ids.length && on === ids.length ? true : on ? ("mixed" as const) : false;
  });
  const w = (n: number) => `width: calc(var(--u) * ${n})`;
</script>

{#snippet th(id: string, label: string, width: number, num = false)}
  {@const sorted = ui.sort?.col === id}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <th class:num class:sorted style={w(width)} onclick={() => sortBy(nextSort(ui.sort, id))}>
    {label}
    {#if sorted}<Icon name={ui.sort!.desc ? "arrow_downward" : "arrow_upward"} />{/if}
    <span class="grip"></span>
  </th>
{/snippet}

<table class="grid fixed">
  <thead>
    <tr>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <th class="lead check" style={w(11)} title="Select all on this page" onclick={checkAll}><Cb state={head} /></th>
      <th class="lead" style={w(10)} title="Audition audio"></th>
      {@render th("name", "Name", 100)}
      {@render th("tags", "Tags", 64)}
      {#each cols as c (c.id)}{@render th(c.id, c.label, c.w, c.num)}{/each}
    </tr>
  </thead>
  <tbody>
    {#each view.pageRows as p (p.id)}
      <Row {p} {cols} />
    {/each}
  </tbody>
</table>
