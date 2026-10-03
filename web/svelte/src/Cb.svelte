<script lang="ts">
  import Icon from "./Icon.svelte";
  let { state }: { state: boolean | "mixed" } = $props();
</script>

<span class="cb" class:on={state === true} class:mixed={state === "mixed"}>
  {#if state === "mixed"}<Icon name="remove" />{:else if state}<Icon name="check" />{/if}
</span>
