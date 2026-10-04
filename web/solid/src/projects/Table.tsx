// The projects table. Rows are keyed by project (`For`), so a sort or a page change moves
// the existing row nodes, and an edit to one project touches only its own cells.

import { For, Show, createMemo, onCleanup } from "solid-js";
import { fileName } from "../../../shared/format";
import { COLUMNS, columnById, missingPlugins, missingSamples, nextSort, type Column } from "../../../shared/projects";
import type { Project } from "../../../shared/types";
import { Cb, Icon, TagChip } from "../shell/parts";
import { setShell, shell } from "../shell/shell";
import {
  checkAll, isSelected, pageIds, pageRows, renameProject, rowCheck, rowClick, selectOnly, setUi, sortBy, ui,
} from "./state";

const cell = (n: number) => ({ width: `calc(var(--u) * ${n})` });

function SortTh(props: { id: string; label: string; w: number; num?: boolean }) {
  const sorted = () => ui.sort?.col === props.id;
  return (
    <th classList={{ num: props.num, sorted: sorted() }} style={cell(props.w)}
      onClick={() => sortBy(nextSort(ui.sort, props.id))}>
      {props.label}
      <Show when={sorted()}><Icon name={ui.sort!.desc ? "arrow_downward" : "arrow_upward"} /></Show>
      <span class="grip" />
    </th>
  );
}

function NameCell(props: { p: Project }) {
  const file = () => fileName(props.p.path);
  let done = false;   // Enter or Escape has settled it; the blur that follows must not commit again
  const finish = (commit: boolean, value: string) => {
    if (done) return;
    done = true;
    if (commit) renameProject(props.p.id, value);
    else setUi("renaming", null);
  };
  return (
    <span class="namecell">
      <Show when={ui.renaming === props.p.id}
        fallback={<>
          <span class="n">{props.p.name}</span>
          <Show when={file() !== props.p.name}><span class="f">{file()}</span></Show>
          <span class="edit" title="Rename (F2)"
            onClick={(e) => { e.stopPropagation(); selectOnly(props.p.id); setUi("renaming", props.p.id); }}>
            <Icon name="edit" />
          </span>
        </>}>
        <input value={props.p.name}
          ref={(el) => queueMicrotask(() => { el.focus(); el.select(); })}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            if (e.key === "Enter") finish(true, e.currentTarget.value);
            else if (e.key === "Escape") finish(false, "");
          }}
          onBlur={(e) => finish(true, e.currentTarget.value)} />
      </Show>
    </span>
  );
}

let hideTimer: ReturnType<typeof setTimeout> | undefined;
export const holdHover = () => clearTimeout(hideTimer);
export const releaseHover = () => { clearTimeout(hideTimer); hideTimer = setTimeout(() => setUi("hot", null), 150); };

function DataCell(props: { p: Project; c: Column }) {
  const missing = () => props.c.count === "plugins" ? missingPlugins(props.p) : props.c.count === "samples" ? missingSamples(props.p) : 0;
  const hot = () => ui.hot?.id === props.p.id && ui.hot.kind === props.c.count;
  return (
    <td classList={{ dim: props.c.dim, num: props.c.num, count: !!props.c.count, hot: hot() }}
      data-hover={props.c.count}
      onMouseEnter={() => { if (props.c.count && props.c.text(props.p, shell.spelling)) { holdHover(); setUi("hot", { id: props.p.id, kind: props.c.count }); } }}
      onMouseLeave={() => { if (props.c.count) releaseHover(); }}>
      {props.c.text(props.p, shell.spelling)}
      <Show when={missing()}><span class="miss" title={`${missing()} missing`}><Icon name="error" /></span></Show>
    </td>
  );
}

function Row(props: { p: Project; cols: Column[] }) {
  return (
    <tr data-id={props.p.id} classList={{ sel: isSelected(props.p.id) }}
      onClick={(e) => rowClick(props.p.id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey })}
      onContextMenu={(e) => {
        e.preventDefault();
        selectOnly(props.p.id);
        const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
        setUi("hot", null);
        setShell({ popover: null, menu: { id: props.p.id, x: e.clientX - win.left, y: e.clientY - win.top } });
      }}>
      <td class="lead check" onClick={(e) => { e.stopPropagation(); rowCheck(props.p.id); }}><Cb state={isSelected(props.p.id)} /></td>
      <td class="lead">
        <Show when={props.p.audio_file_id} fallback={<span class="addaudio" title="Add audition audio"><Icon name="add" /></span>}>
          <span class="play" title="Play audition audio"><Icon name="play_arrow" fill /></span>
        </Show>
      </td>
      <td><NameCell p={props.p} /></td>
      <td><For each={props.p.tags}>{(t) => <TagChip name={t.name} />}</For></td>
      <For each={props.cols}>{(c) => <DataCell p={props.p} c={c} />}</For>
    </tr>
  );
}

export function ProjectTable() {
  const cols = createMemo(() => ui.columns.map(columnById).sort((a, b) => COLUMNS.indexOf(a) - COLUMNS.indexOf(b)));
  const head = createMemo(() => {
    const ids = pageIds();
    const on = ids.filter(isSelected).length;
    return ids.length && on === ids.length ? true : on ? "mixed" : false;
  });
  onCleanup(() => clearTimeout(hideTimer));
  return (
    <table class="grid fixed">
      <thead>
        <tr>
          <th class="lead check" style={cell(11)} title="Select all on this page" onClick={checkAll}><Cb state={head()} /></th>
          <th class="lead" style={cell(10)} title="Audition audio" />
          <SortTh id="name" label="Name" w={100} />
          <SortTh id="tags" label="Tags" w={64} />
          <For each={cols()}>{(c) => <SortTh id={c.id} label={c.label} w={c.w} num={c.num} />}</For>
        </tr>
      </thead>
      <tbody>
        <For each={pageRows()}>{(p) => <Row p={p} cols={cols()} />}</For>
      </tbody>
    </table>
  );
}
