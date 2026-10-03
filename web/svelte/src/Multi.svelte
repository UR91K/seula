<script lang="ts">
  import { fmtLength, fmtTempo } from "../../shared/format";
  import { common, commonCollections, missingPlugins } from "../../shared/projects";
  import type { Project } from "../../shared/types";
  import Icon from "./Icon.svelte";
  import { collectionMap } from "./state.svelte";

  let { projects }: { projects: Project[] } = $props();
  const n = $derived(projects.length);
  const secs = $derived(projects.reduce((s, p) => s + (p.duration_seconds ?? 0), 0));
  const tempos = $derived(projects.map((p) => p.tempo));
  const tags = $derived(common(projects, (p) => p.tags));
  const cols = $derived(commonCollections(projects, collectionMap()));
  const pm = $derived(projects.reduce((s, p) => s + missingPlugins(p), 0));
  const distinctPlugins = $derived(new Set(projects.flatMap((p) => p.plugins.map((x) => x.id))).size);
  const distinctSamples = $derived(new Set(projects.flatMap((p) => p.samples.map((x) => x.id))).size);
</script>

<div class="insp-head insp-multi"><div><h2>{n} projects</h2><div class="faint">Selected</div></div></div>
<div class="insp-sec"><h3>Together</h3>
  <dl class="props">
    <dt>Length</dt><dd>{fmtLength(secs)}</dd>
    <dt>Tempo</dt><dd>{fmtTempo(Math.min(...tempos))}–{fmtTempo(Math.max(...tempos))} BPM</dd>
    <dt>Plugins</dt><dd>{distinctPlugins} distinct{#if pm} · <span class="is-missing">{pm} missing</span>{/if}</dd>
    <dt>Samples</dt><dd>{distinctSamples} distinct</dd>
  </dl>
</div>
<div class="insp-sec"><h3>Tags on all {n}</h3>
  {#if tags.length}
    <div class="tags">{#each tags as t (t.id)}<span class="tag">{t.name}</span>{/each}</div>
  {:else}<span class="faint">None in common</span>{/if}
</div>
<div class="insp-sec"><h3>Collections holding all {n}</h3>
  {#if cols.length}
    <ul class="plain">
      {#each cols.slice(0, 8) as c (c.id)}<li><Icon name="album" /><span class="grow">{c.name}</span></li>{/each}
      {#if cols.length > 8}<li class="faint">and {cols.length - 8} more</li>{/if}
    </ul>
  {:else}<span class="faint">None in common</span>{/if}
</div>
<div class="insp-sec"><h3>Projects</h3>
  <ul class="plain">
    {#each projects.slice(0, 12) as p (p.id)}<li><Icon name="audio_file" /><span class="grow">{p.name}</span></li>{/each}
    {#if projects.length > 12}<li class="faint">and {projects.length - 12} more</li>{/if}
  </ul>
</div>
