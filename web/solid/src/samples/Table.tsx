// The samples table. Rows are the store's own proxies, so `For` keeps a row's nodes across
// a sort, a page change or a check.

import { For, Show } from "solid-js";
import { fmtBytes, splitPath } from "../../../shared/format";
import { SAMPLE_COLUMNS, folderOf, formatOf, presenceLabel } from "../../../shared/samples";
import type { SampleRow } from "../../../shared/types";
import { Grip, createWidths } from "../shell/columns";
import { Icon } from "../shell/parts";
import { setShell } from "../shell/shell";
import { PresentDot } from "./parts";
import { formats, pageRows, select, sortBy, sui } from "./state";

/** Column widths (ADR-0066), kept while the app runs. */
const widths = createWidths();
const last = (id: string) => SAMPLE_COLUMNS[SAMPLE_COLUMNS.length - 1].id === id;

function Head() {
  return (
    <thead>
      <tr>
        <For each={SAMPLE_COLUMNS}>{(c) => {
          const sorted = () => sui.sort?.col === c.id;
          return (
            <th classList={{ num: c.num, sorted: sorted() }} style={widths.th(c, last(c.id))} onClick={() => sortBy(c.id)}>
              {c.label}
              <Show when={sorted()}><Icon name={sui.sort!.desc ? "arrow_downward" : "arrow_upward"} /></Show>
              <Grip widths={widths} col={SAMPLE_COLUMNS[SAMPLE_COLUMNS.indexOf(c) - 1]} />
            </th>
          );
        }}</For>
      </tr>
    </thead>
  );
}

/** A folder truncated in the middle, so the drive and the folder's own name both stay. */
function Folder(props: { path: string }) {
  const folder = () => folderOf(props.path);
  const parts = () => splitPath(folder());
  return (
    <span class="pathcell" title={folder()}>
      <span class="head">{parts().head}</span><span class="tail">{parts().tail}</span>
    </span>
  );
}

/** A missing sample shows the size it last had, dimmed; one no check has found shows none. */
function Size(props: { s: SampleRow }) {
  return (
    <Show when={props.s.size_bytes != null}>
      <Show when={props.s.is_present} fallback={
        <span class="faint" title="Its size when a check last found it">{fmtBytes(props.s.size_bytes!)}</span>}>
        {fmtBytes(props.s.size_bytes!)}
      </Show>
    </Show>
  );
}

function Row(props: { s: SampleRow }) {
  return (
    <tr data-id={props.s.id} classList={{ sel: sui.selected === props.s.id }}
      onClick={() => select(props.s.id)}
      onContextMenu={(e) => {
        e.preventDefault();
        select(props.s.id);
        const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
        setShell({ popover: null, menu: { id: props.s.id, x: e.clientX - win.left, y: e.clientY - win.top } });
      }}>
      <td><span class="n">{props.s.name}</span></td>
      <td>
        <span class="pstate" classList={{ "is-installed": props.s.is_present, "is-absent": !props.s.is_present }}>
          <PresentDot present={props.s.is_present} />{presenceLabel(props.s.is_present)}
        </span>
      </td>
      <td><Folder path={props.s.path} /></td>
      <td class="dim">{formatOf(props.s.path, formats()).name}</td>
      <td class="num"><Size s={props.s} /></td>
      <td class="num">{props.s.project_count || ""}</td>
    </tr>
  );
}

export function SampleTable() {
  return (
    <table class="grid fixed samples" style={widths.table(SAMPLE_COLUMNS)}>
      <Head />
      <tbody><For each={pageRows()}>{(s) => <Row s={s} />}</For></tbody>
    </table>
  );
}
