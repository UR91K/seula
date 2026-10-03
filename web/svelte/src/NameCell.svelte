<script lang="ts">
  import { fileName } from "../../shared/format";
  import type { Project } from "../../shared/types";
  import Icon from "./Icon.svelte";
  import { renameProject, selectOnly, ui } from "./state.svelte";

  let { p }: { p: Project } = $props();
  const file = $derived(fileName(p.path));
  let done = false;   // Enter or Escape has settled it; the blur that follows must not commit again

  function finish(commit: boolean, value: string) {
    if (done) return;
    done = true;
    if (commit) renameProject(p.id, value);
    else ui.renaming = null;
  }

  function focusSelect(el: HTMLInputElement) {
    done = false;
    queueMicrotask(() => { el.focus(); el.select(); });
  }
</script>

<span class="namecell">
  {#if ui.renaming === p.id}
    <input value={p.name} use:focusSelect
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key === "Enter") finish(true, e.currentTarget.value);
        else if (e.key === "Escape") finish(false, "");
      }}
      onblur={(e) => finish(true, e.currentTarget.value)} />
  {:else}
    <span class="n">{p.name}</span>
    {#if file !== p.name}<span class="f">{file}</span>{/if}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span class="edit" title="Rename (F2)"
      onclick={(e) => { e.stopPropagation(); selectOnly(p.id); ui.renaming = p.id; }}>
      <Icon name="edit" />
    </span>
  {/if}
</span>
