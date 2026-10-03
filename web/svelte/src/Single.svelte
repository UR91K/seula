<script lang="ts">
  import { fmtDate, fmtKey, fmtLength, fmtTempo, fmtVersion } from "../../shared/format";
  import { missingPlugins, missingSamples } from "../../shared/projects";
  import type { Project } from "../../shared/types";
  import Icon from "./Icon.svelte";
  import PathChip from "./PathChip.svelte";
  import { collectionsFor, editNotes, ui } from "./state.svelte";

  let { p }: { p: Project } = $props();
  const done = $derived(p.tasks.filter((t) => t.completed).length);
  const cols = $derived(collectionsFor(p));
</script>

<div class="insp-head"><div><h2>{p.name}</h2><PathChip path={p.path} /></div></div>
<div class="insp-sec"><h3>Project</h3>
  <dl class="props">
    <dt>Tempo</dt><dd>{fmtTempo(p.tempo)} BPM</dd>
    <dt>Key</dt><dd>{fmtKey(p.key_signature, ui.spelling) || ""}{#if !p.key_signature}<span class="faint">None detected</span>{/if}</dd>
    <dt>Time</dt><dd>{p.time_signature.numerator}/{p.time_signature.denominator}</dd>
    <dt>Length</dt><dd>{fmtLength(p.duration_seconds)}{#if p.duration_seconds == null}<span class="faint">Unknown</span>{/if}</dd>
    <dt>Live</dt><dd>{fmtVersion(p.ableton_version)}</dd>
    <dt>Created</dt><dd>{fmtDate(p.created_at)}</dd>
    <dt>Modified</dt><dd>{fmtDate(p.modified_at)}</dd>
  </dl>
</div>
<div class="insp-sec"><h3>Tags</h3>
  <div class="tags">{#each p.tags as t (t.id)}<span class="tag">{t.name}</span>{/each}</div>
</div>
<div class="insp-sec"><h3>Notes</h3>
  <textarea class="notes" placeholder="Add notes" value={p.notes}
    oninput={(e) => editNotes(p.id, e.currentTarget.value)}></textarea>
</div>
<div class="insp-sec"><h3>Tasks <span class="count">{p.tasks.length ? `${done}/${p.tasks.length}` : ""}</span></h3>
  {#if p.tasks.length}
    <ul class="plain tasks">
      {#each p.tasks as t (t.id)}
        <li class:done={t.completed}>
          <Icon name={t.completed ? "check_circle" : "radio_button_unchecked"} fill={t.completed} class="state" />
          <span class="grow">{t.description}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>
<div class="insp-sec"><h3>Collections <span class="count">{cols.length || ""}</span></h3>
  {#if cols.length}
    <ul class="plain">
      {#each cols.slice(0, 8) as c (c.id)}<li><Icon name="album" /><span class="grow">{c.name}</span></li>{/each}
      {#if cols.length > 8}<li class="faint">and {cols.length - 8} more</li>{/if}
    </ul>
  {:else}<span class="faint">In no collection</span>{/if}
</div>
<div class="insp-sec"><h3>Plugins <span class="count">{p.plugins.length || ""}</span>
  {#if missingPlugins(p)}<span class="count is-missing">{missingPlugins(p)} missing</span>{/if}</h3>
  {#if p.plugins.length}
    <ul class="plain">
      {#each p.plugins.slice(0, 12) as x (x.id)}
        <li>
          <span class="status-dot" class:is-ok={x.installed === true} class:is-missing={x.installed === false} class:is-unknown={x.installed == null}>
            <Icon name={x.installed === true ? "check_circle" : x.installed === false ? "error" : "help"} fill={x.installed != null} />
          </span>
          <span class="grow">{x.name}</span><span class="faint">{x.vendor ?? ""}</span>
        </li>
      {/each}
      {#if p.plugins.length > 12}<li class="faint">and {p.plugins.length - 12} more</li>{/if}
    </ul>
  {:else}<span class="faint">None</span>{/if}
</div>
<div class="insp-sec"><h3>Samples <span class="count">{p.samples.length || ""}</span>
  {#if missingSamples(p)}<span class="count is-missing">{missingSamples(p)} missing</span>{/if}</h3>
  {#if p.samples.length}
    <ul class="plain">
      {#each p.samples.slice(0, 8) as x (x.id)}
        <li>
          <span class="status-dot" class:is-ok={x.is_present} class:is-missing={!x.is_present}>
            <Icon name={x.is_present ? "check_circle" : "error"} fill />
          </span>
          <span class="grow" title={x.path}>{x.name}</span>
        </li>
      {/each}
      {#if p.samples.length > 8}<li class="faint">and {p.samples.length - 8} more</li>{/if}
    </ul>
  {:else}<span class="faint">None</span>{/if}
</div>
