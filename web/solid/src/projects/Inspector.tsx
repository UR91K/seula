// The project inspector. One project selected, several, or none. The single-project view is a
// non-keyed `Show`: moving the selection from one project to another changes what the
// nodes read rather than rebuilding them, so the notes box keeps its focus and caret
// while a scan streams in.

import { For, Show, type JSX } from "solid-js";
import { fmtDate, fmtKey, fmtLength, fmtTempo, fmtVersion } from "../../../shared/format";
import { collectionsOf, common, commonCollections, missingPlugins, missingSamples } from "../../../shared/projects";
import type { Plugin, Project, Sample } from "../../../shared/types";
import { Icon, PathChip, TagChip } from "../shell/parts";
import { collectionById, shell } from "../shell/shell";
import type { TableState } from "./tablestate";

const PluginDot = (p: { installed: boolean | null }) => (
  <span class="status-dot" classList={{ "is-ok": p.installed === true, "is-missing": p.installed === false, "is-unknown": p.installed == null }}>
    <Icon name={p.installed === true ? "check_circle" : p.installed === false ? "error" : "help"} fill={p.installed != null} />
  </span>
);
const SampleDot = (p: { present: boolean }) => (
  <span class="status-dot" classList={{ "is-ok": p.present, "is-missing": !p.present }}>
    <Icon name={p.present ? "check_circle" : "error"} fill />
  </span>
);

function Section(props: { title: string; count?: JSX.Element; children: JSX.Element }) {
  return <div class="insp-sec"><h3>{props.title} <span class="count">{props.count}</span></h3>{props.children}</div>;
}

/** Every item, scrolling once the list is taller than `max` rows. */
function Listing<T>(props: { items: T[]; max?: number; render: (x: T) => JSX.Element }) {
  const max = () => props.max ?? 8;
  return (
    <ul class="plain" style={{ "max-height": `calc(var(--row) * ${max()})`, "overflow-y": "auto" }}>
      <For each={props.items}>{(x) => props.render(x)}</For>
    </ul>
  );
}

function Single(props: { t: TableState; p: Project }) {
  const p = () => props.p;
  const done = () => p().tasks.filter((t) => t.completed).length;
  const cols = () => collectionsOf(p(), collectionById());
  return (
    <>
      <div class="insp-head"><div><h2>{p().name}</h2><PathChip path={p().path} /></div></div>
      <div class="insp-sec"><h3>Project</h3>
        <dl class="props">
          <dt>Tempo</dt><dd>{fmtTempo(p().tempo)} BPM</dd>
          <dt>Key</dt><dd>{fmtKey(p().key_signature, shell.spelling) || <span class="faint">None detected</span>}</dd>
          <dt>Time</dt><dd>{p().time_signature.numerator}/{p().time_signature.denominator}</dd>
          <dt>Length</dt><dd>{fmtLength(p().duration_seconds) || <span class="faint">Unknown</span>}</dd>
          <dt>Live</dt><dd>{fmtVersion(p().ableton_version)}</dd>
          <dt>Created</dt><dd>{fmtDate(p().created_at)}</dd>
          <dt>Modified</dt><dd>{fmtDate(p().modified_at)}</dd>
        </dl>
      </div>
      <div class="insp-sec"><h3>Tags</h3><div class="tags"><For each={p().tags}>{(t) => <TagChip name={t.name} />}</For></div></div>
      <div class="insp-sec"><h3>Notes</h3>
        <textarea class="notes" placeholder="Add notes" value={p().notes}
          onInput={(e) => props.t.editNotes(p().id, e.currentTarget.value)} />
      </div>
      <Section title="Tasks" count={p().tasks.length ? `${done()}/${p().tasks.length}` : ""}>
        <Show when={p().tasks.length}>
          <ul class="plain tasks">
            <For each={p().tasks}>{(t) => (
              <li classList={{ done: t.completed }}>
                <Icon name={t.completed ? "check_circle" : "radio_button_unchecked"} fill={t.completed} class="state" />
                <span class="grow">{t.description}</span>
              </li>
            )}</For>
          </ul>
        </Show>
      </Section>
      <Section title="Collections" count={cols().length || ""}>
        <Show when={cols().length} fallback={<span class="faint">In no collection</span>}>
          <Listing items={cols()} render={(c) => <li><Icon name="album" /><span class="grow">{c.name}</span></li>} />
        </Show>
      </Section>
      <div class="insp-sec"><h3>Plugins <span class="count">{p().plugins.length || ""}</span>
        <Show when={missingPlugins(p())}><span class="count is-missing">{missingPlugins(p())} missing</span></Show></h3>
        <Show when={p().plugins.length} fallback={<span class="faint">None</span>}>
          <Listing items={p().plugins} max={12} render={(x: Plugin) =>
            <li><PluginDot installed={x.installed} /><span class="grow">{x.name}</span><span class="faint">{x.vendor ?? ""}</span></li>} />
        </Show>
      </div>
      <div class="insp-sec"><h3>Samples <span class="count">{p().samples.length || ""}</span>
        <Show when={missingSamples(p())}><span class="count is-missing">{missingSamples(p())} missing</span></Show></h3>
        <Show when={p().samples.length} fallback={<span class="faint">None</span>}>
          <Listing items={p().samples} render={(x: Sample) =>
            <li><SampleDot present={x.is_present} /><span class="grow" title={x.path}>{x.name}</span></li>} />
        </Show>
      </div>
    </>
  );
}

function Multi(props: { projects: Project[] }) {
  const n = () => props.projects.length;
  const secs = () => props.projects.reduce((s, p) => s + (p.duration_seconds ?? 0), 0);
  const tempos = () => props.projects.map((p) => p.tempo);
  const tags = () => common(props.projects, (p) => p.tags);
  const cols = () => commonCollections(props.projects, collectionById());
  const pm = () => props.projects.reduce((s, p) => s + missingPlugins(p), 0);
  return (
    <>
      <div class="insp-head insp-multi"><div><h2>{n()} projects</h2><div class="faint">Selected</div></div></div>
      <div class="insp-sec"><h3>Together</h3>
        <dl class="props">
          <dt>Length</dt><dd>{fmtLength(secs())}</dd>
          <dt>Tempo</dt><dd>{fmtTempo(Math.min(...tempos()))}–{fmtTempo(Math.max(...tempos()))} BPM</dd>
          <dt>Plugins</dt><dd>{new Set(props.projects.flatMap((p) => p.plugins.map((x) => x.id))).size} distinct
            <Show when={pm()}> · <span class="is-missing">{pm()} missing</span></Show></dd>
          <dt>Samples</dt><dd>{new Set(props.projects.flatMap((p) => p.samples.map((x) => x.id))).size} distinct</dd>
        </dl>
      </div>
      <div class="insp-sec"><h3>Tags on all {n()}</h3>
        <Show when={tags().length} fallback={<span class="faint">None in common</span>}>
          <div class="tags"><For each={tags()}>{(t) => <TagChip name={t.name} />}</For></div>
        </Show>
      </div>
      <div class="insp-sec"><h3>Collections holding all {n()}</h3>
        <Show when={cols().length} fallback={<span class="faint">None in common</span>}>
          <Listing items={cols()} render={(c) => <li><Icon name="album" /><span class="grow">{c.name}</span></li>} />
        </Show>
      </div>
      <div class="insp-sec"><h3>Projects</h3>
        <Listing items={props.projects} max={12} render={(p) => <li><Icon name="audio_file" /><span class="grow">{p.name}</span></li>} />
      </div>
    </>
  );
}

/** The inspector's body for a table's selection, or `fallback` when nothing is selected.
 *  The projects view and a collection's tracklist share it. */
export function SelectionInspector(props: { t: TableState; fallback: JSX.Element }) {
  const one = () => (props.t.selectedProjects().length === 1 ? props.t.selectedProjects()[0] : undefined);
  const many = () => (props.t.selectedProjects().length > 1 ? props.t.selectedProjects() : undefined);
  return (
    <Show when={one()} fallback={
      <Show when={many()} fallback={props.fallback}>
        {(ps) => <Multi projects={ps()} />}
      </Show>}>
      {(p) => <Single t={props.t} p={p()} />}
    </Show>
  );
}
