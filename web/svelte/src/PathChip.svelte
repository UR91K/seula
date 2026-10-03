<script lang="ts">
  import { splitPath } from "../../shared/format";
  let { path }: { path: string } = $props();
  const parts = $derived(splitPath(path));

  function copy(e: MouseEvent) {
    navigator.clipboard?.writeText(path).catch(() => {});
    const el = e.currentTarget as HTMLElement;
    el.classList.add("copied");
    setTimeout(() => el.classList.remove("copied"), 1200);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<span class="path" title={path} onclick={copy}>
  <span class="head">{parts.head}</span><span class="tail">{parts.tail}</span>
</span>
