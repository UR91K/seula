<script lang="ts">
  import Icon from "./Icon.svelte";
  import { placed } from "./popover";
  import { data, showInExplorer, ui, view } from "./state.svelte";

  let { menu }: { menu: { id: string; x: number; y: number } } = $props();
  const many = $derived(view.selectedProjects.length > 1);
  const project = $derived(data.projects.find((p) => p.id === menu.id));

  function act(f?: () => void) { f?.(); ui.menu = null; }
</script>

{#snippet item(icon: string, label: string, off = false, kbd = "", f?: () => void)}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="mi" class:off onclick={() => act(f)}>
    <Icon name={icon} /><span class="lbl">{label}</span>
    {#if kbd}<span class="kbd">{kbd}</span>{/if}
  </div>
{/snippet}

<div class="pop menu" use:placed={() => ({ x: menu.x, y: menu.y })}>
  {@render item("open_in_new", "Open in Ableton", true, "Enter")}
  {@render item("folder_open", "Show in Explorer", many, "", () => { if (project) showInExplorer(project.path); })}
  <div class="sep"></div>
  {@render item("sell", "Tags", true)}
  {@render item("album", "Add to collection", true)}
  <div class="sep"></div>
  {@render item("edit", "Rename", many, "F2", () => (ui.renaming = menu.id))}
  {@render item("music_note_add", "Add audition audio…", true)}
  <div class="sep"></div>
  {@render item("archive", many ? "Archive projects" : "Archive", true, "Del")}
</div>
