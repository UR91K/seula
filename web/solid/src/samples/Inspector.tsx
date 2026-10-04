// The samples inspector. The head and the state line come from the list row, so they show
// at once; the file record and the projects using the sample follow when the detail request
// answers. A non-keyed `Show` keeps the nodes when the selection moves to another sample.

import { For, Show, type JSX } from "solid-js";
import { fmtBytes, fmtDate } from "../../../shared/format";
import { folderOf, formatOf } from "../../../shared/samples";
import type { Project, SampleFile, SampleRow } from "../../../shared/types";
import { Icon, PathChip } from "../shell/parts";
import { PresentDot } from "./parts";
import { detail, formats, selectedSample, showProjectsUsing } from "./state";

function Props(props: { rows: [string, JSX.Element | null | undefined | false][] }) {
  const shown = () => props.rows.filter(([, v]) => v != null && v !== "" && v !== false);
  return (
    <Show when={shown().length}>
      <dl class="props"><For each={shown()}>{([k, v]) => <><dt>{k}</dt><dd>{v}</dd></>}</For></dl>
    </Show>
  );
}

/** Whether the file is there, and when a check last said so. `file` is undefined while the
 *  detail request is out. */
function StateSection(props: { s: SampleRow; file: SampleFile | null | undefined }) {
  const when = () => (props.file ? fmtDate(props.file.checked_at) : null);
  return (
    <div class="insp-sec">
      <Show when={props.s.is_present} fallback={
        <>
          <div class="state-line is-absent"><PresentDot present={false} /><b>Missing</b></div>
          <Show when={props.file !== undefined}>
            <p class="faint">{when() ? `Last found by a check on ${when()}` : "No check has ever found it. Its projects refer to it by path only."}</p>
          </Show>
        </>}>
        <div class="state-line is-installed"><PresentDot present={true} /><b>Present</b></div>
        <Show when={props.file !== undefined}>
          <p class="faint">{when() ? `Found by the check on ${when()}` : "Found when its project was scanned. No check has measured it yet."}</p>
        </Show>
      </Show>
    </div>
  );
}

function UsedIn(props: { s: SampleRow; projects: Project[] }) {
  const recent = () => [...props.projects].sort((a, b) => b.modified_at - a.modified_at);
  return (
    <div class="insp-sec"><h3>Used in <span class="count">{props.projects.length || ""}</span></h3>
      <Show when={props.projects.length} fallback={<span class="faint">No project uses it</span>}>
        <ul class="plain">
          <For each={recent().slice(0, 10)}>{(x) => (
            <li><Icon name="audio_file" /><span class="grow">{x.name}</span><span class="faint">{fmtDate(x.modified_at).slice(0, 10)}</span></li>)}</For>
        </ul>
        <Show when={props.projects.length > 10}>
          <button class="linkbtn" onClick={() => showProjectsUsing(props.s)}>
            Show all {props.projects.length} in Projects <Icon name="arrow_forward" />
          </button>
        </Show>
      </Show>
    </div>
  );
}

function FileSection(props: { s: SampleRow; file: SampleFile | null }) {
  const fmt = () => formatOf(props.s.path, formats());
  const ext = () => props.s.path.split(".").pop()?.toLowerCase() ?? "";
  return (
    <div class="insp-sec"><h3>File<Show when={!props.s.is_present}> <span class="count">as last found</span></Show></h3>
      <Props rows={[
        ["Format", <>{fmt().name} <span class="faint">.{ext()}</span></>],
        ["Size", props.file && <>{fmtBytes(props.file.size_bytes)} <span class="faint">{props.file.size_bytes.toLocaleString("en-US")} bytes</span></>],
        ["Modified", props.file?.modified_at && fmtDate(props.file.modified_at)],
        ["Folder", <span title={folderOf(props.s.path)}>{folderOf(props.s.path).split(/[\\/]/).pop()}</span>],
      ]} />
      <Show when={!props.file}><p class="faint">No check has measured this file.</p></Show>
    </div>
  );
}

function Single(props: { s: SampleRow }) {
  const s = () => props.s;
  // The answer for this sample only: a stale one for the previous selection is not shown.
  const d = () => (detail()?.id === s().id ? detail() : undefined);
  return (
    <>
      <div class="insp-head">
        <span class="glyph"><Icon name="graphic_eq" /></span>
        <div>
          <h2>{s().name}</h2>
          <div class="faint">
            {formatOf(s().path, formats()).name}
            <Show when={s().size_bytes != null}> · {fmtBytes(s().size_bytes!)}</Show>
          </div>
          <PathChip path={s().path} />
        </div>
      </div>
      <StateSection s={s()} file={d()?.file} />
      <Show when={d()}>{(x) => (
        <>
          <UsedIn s={s()} projects={x().projects} />
          <FileSection s={s()} file={x().file} />
        </>
      )}</Show>
    </>
  );
}

export function Inspector() {
  return (
    <aside class="inspector">
      <Show when={selectedSample()} fallback={
        <div class="empty"><Icon name="right_panel_open" /><p>Select a sample to see its file and the projects that use it.</p></div>}>
        {(s) => <Single s={s()} />}
      </Show>
    </aside>
  );
}
