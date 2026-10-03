<script lang="ts">
  let { name, fill = false, class: cls = "" }: { name: string; fill?: boolean; class?: string } = $props();
</script>

<span class="ms {cls}" class:fill>{name}</span>
