<script lang="ts">
  // Its own component so a progress tick re-runs this and nothing else.
  import { data, push } from "./state.svelte";
</script>

<span class="grow">
  {#if push.scan}
    <span class="progress"><i style:width="{(100 * (push.scan.total ? push.scan.completed / push.scan.total : 0)).toFixed(1)}%"></i></span>
    {push.scan.message}
  {:else if data.notice || data.loadError}
    <span style="color: var(--danger)">{data.notice ?? data.loadError}</span>
  {/if}
</span>
