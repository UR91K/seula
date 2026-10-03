<script lang="ts">
  import { COLUMNS } from "../../shared/projects";
  import Cb from "./Cb.svelte";
  import Icon from "./Icon.svelte";
  import { placed } from "./popover";
  import { toggleColumn, ui } from "./state.svelte";
</script>

<div class="pop picker" use:placed={() => {
  const btn = document.querySelector('.viewbar .tb-btn[title="Columns"]');
  return btn ? { anchor: btn } : null;
}}>
  <div class="list" style="max-height: none; padding-top: var(--u)">
    {#each ["Audition", "Name", "Tags"] as l (l)}
      <div class="it" style="color: var(--text-3)"><Cb state={true} /><span class="grow">{l}</span><Icon name="lock" /></div>
    {/each}
    <div class="menu"><div class="sep"></div></div>
    {#each COLUMNS as c (c.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="it" onclick={() => toggleColumn(c.id)}>
        <Cb state={ui.columns.includes(c.id)} /><span class="grow">{c.label}</span>
      </div>
    {/each}
  </div>
</div>
