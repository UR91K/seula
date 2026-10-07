// The plugins inspector. The head and the state line come from the list row, so they show
// at once; what a scan recorded and the projects using the plugin follow when the detail
// request answers. A non-keyed `Show` keeps the nodes when the selection moves to another
// plugin, and the scan's counts update in place.

import { For, Show, type JSX } from "solid-js";
import { fmtDate } from "../../../shared/format";
import { plural } from "../../../shared/projects";
import type { PluginDetails, PluginRow, Project } from "../../../shared/types";
import { Icon, PathChip } from "../shell/parts";
import { StatusDot } from "./parts";
import { detail, scanPlugins, selectedPlugin, showProjectsUsing } from "./state";

const yesNo = (b: boolean | null) => (b == null ? null : b ? "Yes" : "No");

function Props(props: { rows: [string, JSX.Element | null | undefined][] }) {
  const shown = () => props.rows.filter(([, v]) => v != null && v !== "");
  return (
    <Show when={shown().length}>
      <dl class="props"><For each={shown()}>{([k, v]) => <><dt>{k}</dt><dd>{v}</dd></>}</For></dl>
    </Show>
  );
}

function List<T>(props: { items: T[]; max?: number; render: (x: T) => JSX.Element }) {
  const max = () => props.max ?? 8;
  return (
    <ul class="plain">
      <For each={props.items.slice(0, max())}>{(x) => props.render(x)}</For>
      <Show when={props.items.length > max()}><li class="faint">and {props.items.length - max()} more</li></Show>
    </ul>
  );
}

/** What the installed flag means for this plugin, and when a scan last said so. */
function StateSection(props: { p: PluginRow; d?: PluginDetails }) {
  const when = () => (props.d?.last_scanned_at ? fmtDate(props.d.last_scanned_at) : null);
  return (
    <div class="insp-sec">
      <Show when={props.p.installed === true}>
        <div class="state-line is-installed"><StatusDot installed={true} /><b>Installed</b></div>
        <Show when={when()}><p class="faint">Found by the scan on {when()}</p></Show>
      </Show>
      <Show when={props.p.installed === false}>
        <div class="state-line is-absent"><StatusDot installed={false} /><b>Missing</b></div>
        <Show when={when()}><p class="faint">Not found by the scan on {when()}</p></Show>
        <Show when={props.d} fallback={null}>
          <Show when={props.d!.path} fallback={<p class="faint">No scan has ever found it on this machine. Projects know it by name only.</p>}>
            <p class="faint" style={{ "margin-top": "calc(var(--u) * 2)" }}>Last found at</p>
            <PathChip path={props.d!.path!} />
          </Show>
        </Show>
      </Show>
      <Show when={props.p.installed == null}>
        <div class="state-line is-unscanned"><StatusDot installed={null} /><b>Not scanned</b></div>
        <p class="faint">No plugin scan has looked for it yet, so it may be installed or not. Projects know it by name only.</p>
        <button class="btn" onClick={scanPlugins}><Icon name="radar" />Scan plugins</button>
      </Show>
    </div>
  );
}

function audio(d: PluginDetails) {
  if (d.audio_in_channels == null && d.audio_out_channels == null) return null;
  const buses = d.audio_in_buses != null
    ? <> <span class="faint">({plural(d.audio_in_buses, "bus", "buses")} in, {d.audio_out_buses} out)</span></> : null;
  return <>{d.audio_in_channels} in · {d.audio_out_channels} out{buses}</>;
}

function midi(d: PluginDetails) {
  if (d.has_midi_input == null && d.has_midi_output == null) return null;
  return d.has_midi_input && d.has_midi_output ? "In and out" : d.has_midi_input ? "In" : d.has_midi_output ? "Out" : "None";
}

/** The scan's own record of the plugin, and the format-specific part of it. */
function Scanned(props: { p: PluginRow; d: PluginDetails }) {
  const d = () => props.d;
  return (
    <>
      <div class="insp-sec"><h3>Plugin<Show when={props.p.installed === false}> <span class="count">as last scanned</span></Show></h3>
        <Props rows={[
          ["Category", d().category?.replace(/\|/g, " · ")],
          ["Type", d().is_instrument == null ? null : d().is_instrument ? "Instrument" : "Effect"],
          ["Audio", audio(d())],
          ["MIDI", midi(d())],
          ["Latency", d().latency_samples == null ? null : d().latency_samples ? `${d().latency_samples} samples` : "None"],
          ["Presets", d().presets],
          ["Parameters", d().parameters],
          ["Editor", yesNo(d().has_gui)],
          ["Website", d().vendor_url && <a class="link" title={d().vendor_url!}>{d().vendor_url!.replace(/^https?:\/\//, "")}</a>],
        ]} />
      </div>
      <Show when={d().plugin_kind === "VST3"} fallback={
        <div class="insp-sec"><h3>VST2</h3>
          <Props rows={[
            ["Unique ID", d().fourcc && <code>{d().fourcc}</code>],
            ["MIDI channels", d().midi_in_channels ? `${d().midi_in_channels} in` : "None"],
            ["Preset chunks", yesNo(d().preset_chunks)],
            ["64-bit audio", yesNo(d().f64_precision)],
            ["Silent when stopped", yesNo(d().silent_when_stopped)],
          ]} />
        </div>}>
        <div class="insp-sec"><h3>Classes <span class="count">{d().classes.length || ""}</span></h3>
          <Show when={d().classes.length} fallback={<span class="faint">None reported</span>}>
            <List items={d().classes} render={(c) => (
              <li><Icon name={c.category === "Audio Module Class" ? "graphic_eq" : "tune"} /><span class="grow">{c.name}</span>
                <span class="faint">{c.category.replace(/ Class$/, "")}</span></li>)} />
          </Show>
        </div>
        <div class="insp-sec"><h3>Buses <span class="count">{d().buses.length || ""}</span></h3>
          <Show when={d().buses.length} fallback={<span class="faint">None reported</span>}>
            <List max={10} items={d().buses} render={(b) => (
              <li><Icon name={b.direction === "input" ? "input" : "output"} /><span class="grow">{b.name}</span>
                <span class="faint">{b.media === "event" ? "MIDI" : `${b.channel_count} ch`}{b.bus_type === 1 ? " · aux" : ""}</span></li>)} />
          </Show>
        </div>
      </Show>
    </>
  );
}

function UsedIn(props: { p: PluginRow; projects: Project[] }) {
  const recent = () => [...props.projects].sort((a, b) => b.modified_at - a.modified_at);
  return (
    <div class="insp-sec"><h3>Used in <span class="count">{props.projects.length || ""}</span></h3>
      <Show when={props.projects.length} fallback={<span class="faint">No project uses it</span>}>
        <List max={10} items={recent()} render={(x) => (
          <li><Icon name="audio_file" /><span class="grow">{x.name}</span><span class="faint">{fmtDate(x.modified_at).slice(0, 10)}</span></li>)} />
        <Show when={props.projects.length > 10}>
          <button class="linkbtn" onClick={() => showProjectsUsing(props.p)}>
            Show all {props.projects.length} in Projects <Icon name="arrow_forward" />
          </button>
        </Show>
      </Show>
    </div>
  );
}

function Single(props: { p: PluginRow }) {
  const p = () => props.p;
  // The answer for this plugin only: a stale one for the previous selection is not shown.
  const d = () => (detail()?.id === p().id ? detail() : undefined);
  return (
    <>
      <div class="insp-head">
        <span class="glyph"><Icon name={p().format.includes("Instrument") ? "piano" : "graphic_eq"} /></span>
        <div>
          <h2>{p().name}</h2>
          <div class="faint">{p().vendor ?? "No vendor"} · {p().format}{p().version ? ` · ${p().version}` : ""}</div>
          <Show when={p().installed === true && d()?.details.path}>{(path) => <PathChip path={path()} />}</Show>
        </div>
      </div>
      <StateSection p={p()} d={d()?.details} />
      <Show when={d()}>{(x) => (
        <>
          <UsedIn p={p()} projects={x().projects} />
          <Show when={x().details.path != null}><Scanned p={p()} d={x().details} /></Show>
          <div class="insp-sec"><h3>Identity</h3>
            <Props rows={[
              ["Kind", x().details.plugin_kind],
              ["UID", <code class="uid" title={`${x().details.uid} · click to copy`}
                onClick={(e) => { navigator.clipboard?.writeText(x().details.uid).catch(() => {}); e.currentTarget.classList.add("copied"); }}>{x().details.uid}</code>],
            ]} />
            <p class="faint" style={{ "margin-top": "var(--u)" }}>Projects refer to it as</p>
            <List items={x().details.references} render={(r) => (
              <li><Icon name="link" /><span class="grow" title={r.dev_identifier}>{r.ableton_name ?? r.dev_identifier}</span>
                <span class="faint">{r.ableton_format === "instr" ? "instrument" : "effect"} · matched by {r.resolved_via === "created" ? "name only" : r.resolved_via.replace("_", " ")}</span></li>)} />
          </div>
        </>
      )}</Show>
    </>
  );
}

export function Inspector() {
  return (
    <aside class="inspector">
      <Show when={selectedPlugin()} fallback={
        <div class="empty"><Icon name="right_panel_open" /><p>Select a plugin to see what the scan recorded and which projects use it.</p></div>}>
        {(p) => <Single p={p()} />}
      </Show>
    </aside>
  );
}
