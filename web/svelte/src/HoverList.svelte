<script lang="ts">
  import { missingPlugins, missingSamples } from "../../shared/projects";
  import { holdHover, releaseHover } from "./hover";
  import Icon from "./Icon.svelte";
  import { placed } from "./popover";
  import { data } from "./state.svelte";

  let { hot }: { hot: { id: string; kind: "plugins" | "samples" } } = $props();
  const p = $derived(data.projects.find((x) => x.id === hot.id));
  const plugins = $derived(hot.kind === "plugins");
  const missing = $derived(p ? (plugins ? missingPlugins(p) : missingSamples(p)) : 0);
</script>

{#if p}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="pop hoverlist" onmouseenter={holdHover} onmouseleave={releaseHover}
    use:placed={() => {
      const cell = document.querySelector(`tr[data-id="${hot.id}"] td[data-hover="${hot.kind}"]`);
      return cell ? { anchor: cell, align: "right" } : null;
    }}>
    <div class="title">{plugins ? "Plugins" : "Samples"} · {plugins ? p.plugins.length : p.samples.length}
      {#if missing}<span class="miss">{missing} missing</span>{/if}</div>
    <ul class="plain">
      {#if plugins}
        {#each p.plugins as x (x.id)}
          <li>
            <span class="status-dot" class:is-ok={x.installed === true} class:is-missing={x.installed === false} class:is-unknown={x.installed == null}>
              <Icon fill name={x.installed === true ? "check_circle" : x.installed === false ? "error" : "help"} />
            </span>
            <span class="grow">{x.name}</span>
            {#if x.vendor}<span class="faint">{x.vendor}</span>{/if}
          </li>
        {/each}
      {:else}
        {#each p.samples as x (x.id)}
          <li>
            <span class="status-dot" class:is-ok={x.is_present} class:is-missing={!x.is_present}>
              <Icon fill name={x.is_present ? "check_circle" : "error"} />
            </span>
            <span class="grow" title={x.path}>{x.name}</span>
          </li>
        {/each}
      {/if}
    </ul>
  </div>
{/if}
