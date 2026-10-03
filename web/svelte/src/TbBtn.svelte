<script lang="ts">
  import Icon from "./Icon.svelte";
  let { icon, label, title, disabled = false, caret = false, on = false, keep = false, onclick }: {
    icon?: string; label?: string; title?: string; disabled?: boolean; caret?: boolean; on?: boolean; keep?: boolean;
    onclick?: (e: MouseEvent) => void;
  } = $props();
</script>

<button class="tb-btn" class:lbl={!!label} class:on title={title ?? label} {disabled} {onclick} data-keep={keep ? "" : undefined}>
  {#if icon}<Icon name={icon} />{/if}
  {#if label}<span>{label}</span>{/if}
  {#if caret}<Icon name="expand_more" class="caret" />{/if}
</button>
