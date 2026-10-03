<script lang="ts">
  import { missingPlugins, missingSamples, type Column } from "../../shared/projects";
  import type { Project } from "../../shared/types";
  import Cb from "./Cb.svelte";
  import { holdHover, releaseHover } from "./hover";
  import Icon from "./Icon.svelte";
  import NameCell from "./NameCell.svelte";
  import { isSelected, rowCheck, rowClick, selectOnly, ui } from "./state.svelte";

  let { p, cols }: { p: Project; cols: Column[] } = $props();

  const missing = (c: Column) =>
    c.count === "plugins" ? missingPlugins(p) : c.count === "samples" ? missingSamples(p) : 0;

  function menu(e: MouseEvent) {
    e.preventDefault();
    selectOnly(p.id);
    const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
    ui.hot = null; ui.popover = null;
    ui.menu = { id: p.id, x: e.clientX - win.left, y: e.clientY - win.top };
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<tr data-id={p.id} class:sel={isSelected(p.id)}
  onclick={(e) => rowClick(p.id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey })}
  oncontextmenu={menu}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <td class="lead check" onclick={(e) => { e.stopPropagation(); rowCheck(p.id); }}><Cb state={isSelected(p.id)} /></td>
  <td class="lead">
    {#if p.audio_file_id}
      <span class="play" title="Play audition audio"><Icon name="play_arrow" fill /></span>
    {:else}
      <span class="addaudio" title="Add audition audio"><Icon name="add" /></span>
    {/if}
  </td>
  <td><NameCell {p} /></td>
  <td>{#each p.tags as t (t.id)}<span class="tag">{t.name}</span>{/each}</td>
  {#each cols as c (c.id)}
    <td class:dim={c.dim} class:num={c.num} class:count={!!c.count}
      class:hot={ui.hot?.id === p.id && ui.hot.kind === c.count}
      data-hover={c.count}
      onmouseenter={() => { if (c.count && c.text(p, ui.spelling)) { holdHover(); ui.hot = { id: p.id, kind: c.count }; } }}
      onmouseleave={() => { if (c.count) releaseHover(); }}>
      {c.text(p, ui.spelling)}
      {#if missing(c)}<span class="miss" title="{missing(c)} missing"><Icon name="error" /></span>{/if}
    </td>
  {/each}
</tr>
