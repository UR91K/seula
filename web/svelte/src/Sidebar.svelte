<script lang="ts">
  import Icon from "./Icon.svelte";
  import { data, ui } from "./state.svelte";

  const VIEWS = [
    { id: "projects", label: "Projects", icon: "audio_file" },
    { id: "collections", label: "Collections", icon: "album" },
    { id: "plugins", label: "Plugins", icon: "plug_connect" },
    { id: "samples", label: "Samples", icon: "earthquake" },
    { id: "stats", label: "Stats", icon: "bar_chart" },
  ];
</script>

<aside class="sidebar">
  {#each VIEWS as v (v.id)}
    <div class="nav-item" class:active={v.id === "projects"} title={v.label}>
      <Icon name={v.icon} fill={v.id === "projects"} />
      <span class="label">{v.label}</span>
      <span class="count">{v.id === "projects" ? (data.activeTotal ?? "") : ""}</span>
    </div>
  {/each}
  <div class="spacer"></div>
  <button class="tb-btn collapse" title="{ui.sidebarCollapsed ? 'Expand' : 'Collapse'} sidebar"
    onclick={() => (ui.sidebarCollapsed = !ui.sidebarCollapsed)}>
    <Icon name={ui.sidebarCollapsed ? "left_panel_open" : "left_panel_close"} />
  </button>
</aside>
