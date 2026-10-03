<script lang="ts">
  // One project selected, several, or none. Moving the selection from one project to
  // another changes what the nodes read rather than rebuilding them, so the notes box
  // keeps its focus and caret while a scan streams in.
  import Icon from "./Icon.svelte";
  import Multi from "./Multi.svelte";
  import Single from "./Single.svelte";
  import { view } from "./state.svelte";
</script>

<aside class="inspector">
  {#if view.selectedProjects.length === 1}
    <Single p={view.selectedProjects[0]} />
  {:else if view.selectedProjects.length > 1}
    <Multi projects={view.selectedProjects} />
  {:else}
    <div class="empty"><Icon name="right_panel_open" /><p>Select a project to see its details here.</p></div>
  {/if}
</aside>
