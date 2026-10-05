// The collections inspector. The head comes from the list row, so it shows at once; the
// facts, the tracklist and the tasks follow when the detail requests answer. Tasks are
// read-only for now: the Solid project inspector has no task toggling to share.

import { For, Show, type JSX } from "solid-js";
import { fmtDate, fmtKey, fmtLength } from "../../../shared/format";
import { fmtTotal, tasksDone } from "../../../shared/collections";
import type { CollectionRow } from "../../../shared/types";
import { Icon } from "../shell/parts";
import { shell } from "../shell/shell";
import { Cover } from "./parts";
import { SelectionInspector } from "../projects/Inspector";
import { cui, detail, selectedCollection, tracks, type Detail } from "./state";

function Props(props: { rows: [string, JSX.Element | null | undefined | false][] }) {
  const shown = () => props.rows.filter(([, v]) => v != null && v !== "" && v !== false);
  return (
    <Show when={shown().length}>
      <dl class="props"><For each={shown()}>{([k, v]) => <><dt>{k}</dt><dd>{v}</dd></>}</For></dl>
    </Show>
  );
}

function Facts(props: { c: CollectionRow; d: Detail }) {
  const s = () => props.d.stats;
  const archived = () => props.d.tracks.filter((p) => !p.is_active).length;
  return (
    <div class="insp-sec"><h3>Collection</h3>
      <Props rows={[
        ["Projects", s().project_count
          ? <>{s().project_count}{archived() > 0 && <> <span class="faint">({archived()} archived)</span></>}</>
          : <span class="faint">None yet</span>],
        ["Length", fmtTotal(s().total_duration_seconds)],
        ["Tempo", s().average_tempo != null && <>{Math.round(s().average_tempo!)} BPM <span class="faint">average</span></>],
        ["Key", s().most_common_key && <>{fmtKey(s().most_common_key, shell.spelling)} <span class="faint">most common</span></>],
        ["Time", s().most_common_time_signature && <>{s().most_common_time_signature} <span class="faint">most common</span></>],
        ["Plugins", s().total_plugins || null],
        ["Samples", s().total_samples || null],
        ["Tags", s().total_tags || null],
        ["Created", fmtDate(props.c.created_at)],
        ["Modified", fmtDate(props.c.modified_at)],
      ]} />
    </div>
  );
}

function Tracklist(props: { d: Detail }) {
  return (
    <div class="insp-sec"><h3>Tracklist <span class="count">{props.d.tracks.length || ""}</span></h3>
      <Show when={props.d.tracks.length} fallback={<span class="faint">No projects yet</span>}>
        <ul class="plain tracklist">
          <For each={props.d.tracks.slice(0, 10)}>{(p, i) => (
            <li classList={{ archived: !p.is_active }}>
              <span class="num">{i() + 1}</span><span class="grow">{p.name}</span>
              <Show when={!p.is_active}><span class="faint" title="Archived"><Icon name="inventory_2" /></span></Show>
              <span class="faint">{fmtLength(p.duration_seconds)}</span>
            </li>
          )}</For>
        </ul>
      </Show>
    </div>
  );
}

function Tasks(props: { d: Detail }) {
  return (
    <div class="insp-sec"><h3>Tasks <span class="count">{props.d.tasks.length ? `${tasksDone(props.d.tasks)}/${props.d.tasks.length}` : ""}</span></h3>
      <Show when={props.d.tasks.length} fallback={<span class="faint">No project here has tasks</span>}>
        <ul class="plain tasks">
          <For each={props.d.tasks}>{(t) => (
            <li classList={{ done: t.completed }}>
              <Icon name={t.completed ? "check_circle" : "radio_button_unchecked"} class={`state ${t.completed ? "fill" : ""}`} />
              <span class="grow">{t.description}</span>
              <span class="faint proj" title={t.project_name}>{t.project_name}</span>
            </li>
          )}</For>
        </ul>
      </Show>
    </div>
  );
}

function Single(props: { c: CollectionRow; opened: boolean }) {
  // The answer for this collection only: a stale one for the previous selection is not shown.
  const d = () => (detail()?.id === props.c.id ? detail() : undefined);
  return (
    <>
      <div class="insp-head insp-col">
        <Cover c={props.c} class="cover-lg" />
        <div>
          <h2>{props.c.name}</h2>
          <Show when={props.c.description}><p class="desc">{props.c.description}</p></Show>
        </div>
      </div>
      <Show when={d()}>{(x) => (
        <>
          <Facts c={props.c} d={x()} />
          {/* Opened, the tracklist is already on screen. */}
          <Show when={!props.opened}><Tracklist d={x()} /></Show>
          <Tasks d={x()} />
        </>
      )}</Show>
    </>
  );
}

/** The collection itself: the one selected in the list, or the opened one. */
function CollectionPanel() {
  return (
    <Show when={selectedCollection()} fallback={
      <div class="empty"><Icon name="right_panel_open" /><p>Select a collection to see its tracklist and tasks here. Double-click to open it.</p></div>}>
      {(c) => <Single c={c()} opened={cui.open != null} />}
    </Show>
  );
}

/** Opened, a selected project (or several) is shown as in Projects; with none selected,
 *  the collection is. */
export function Inspector() {
  return (
    <aside class="inspector">
      <Show when={cui.open} fallback={<CollectionPanel />}>
        <SelectionInspector t={tracks} fallback={<CollectionPanel />} />
      </Show>
    </aside>
  );
}
