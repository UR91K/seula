<script lang="ts">
  import { plural } from "../../shared/projects";
  import ScanSegment from "./ScanSegment.svelte";
  import { data, ui, view } from "./state.svelte";

  const count = $derived(ui.query
    ? plural(view.total, "result")
    : `${view.total} ${ui.scope === "archived" ? "archived" : "projects"}`);
  const watcher = $derived.by(() => {
    const s = data.system;
    if (!s) return "";
    return s.watcher_active ? `Watching ${plural(s.watch_paths.length, "folder")}` : "Watcher off";
  });
</script>

<footer class="statusbar">
  <span>{count}</span>
  {#if view.selectedCount}<span>{view.selectedCount} selected</span>{/if}
  <ScanSegment />
  <span>{watcher}</span>
  <span class="ks">
    <span class="keyswitch">
      <button class:on={ui.spelling === "sharp"} title="Show keys with sharps" onclick={() => (ui.spelling = "sharp")}>♯</button>
      <button class:on={ui.spelling === "flat"} title="Show keys with flats" onclick={() => (ui.spelling = "flat")}>♭</button>
    </span>
  </span>
</footer>
