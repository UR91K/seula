// The stats page: the overview tiles, then four bands of panels (ADR-0045). Every figure is
// the response for the scope on screen; the charts draw the series as they come.

import { For, Show, type JSX } from "solid-js";
import { fmtLength } from "../../../shared/format";
import { plural } from "../../../shared/projects";
import { folderOf } from "../../../shared/samples";
import {
  dayMax, days, dec1, keyBars, monthAverage, monthColumns, overview, tagBars, taskColumns, tempoColumns, timeSignatureBars,
  versionBars, yearColumns, type DayKey,
} from "../../../shared/stats";
import type { Project, Statistics } from "../../../shared/types";
import { Icon } from "../shell/parts";
import { Cover } from "../collections/parts";
import { projectScope, shell } from "../shell/shell";
import { TagChip } from "../shell/parts";
import { Band, BarList, Columns, InlineBar, Panel, StatTile, taskStack } from "./charts";
import { setHover } from "./state";

/** A property sheet; a pair with no value is left out. */
function Facts(props: { rows: [string, JSX.Element | null | undefined | false][] }) {
  return (
    <dl class="props">
      <For each={props.rows.filter(([, v]) => v != null && v !== false && v !== "")}>{([k, v]) => <><dt>{k}</dt><dd>{v}</dd></>}</For>
    </dl>
  );
}

/** A project named on the page, marked when it is archived (the `all` scope). */
function ProjectName(props: { p: Pick<Project, "name" | "is_active"> }) {
  return (
    <>
      <span class="n">{props.p.name}</span>
      <Show when={!props.p.is_active}> <span class="faint" title="Archived"><Icon name="inventory_2" /></span></Show>
    </>
  );
}

const Faint = (props: { children: JSX.Element }) => <span class="faint">{props.children}</span>;

function Music(props: { s: Statistics }) {
  const keys = () => keyBars(props.s, shell.spelling);
  const l = () => props.s.longest_project;
  return (
    <Band title="Music">
      <Panel title="Tempo" span={4} count="BPM, 10 per bar"><Columns items={tempoColumns(props.s)} height={44} /></Panel>
      <Panel title="Length" span={2}>
        <Facts rows={[
          ["Average", fmtLength(props.s.average_project_duration_seconds)],
          ["Longest", l() && <><ProjectName p={l()!} /> <Faint>{fmtLength(l()!.duration_seconds)}</Faint></>],
          ["Under 40 s", plural(props.s.projects_under_40_seconds, "project")],
        ]} />
      </Panel>
      <Panel title="Keys" span={3} count={keys().keyed}>
        <BarList items={keys().bars} more={keys().more} moreLabel="more keys" labelW={46} />
      </Panel>
      <Panel title="Time signatures" span={3} count={props.s.time_signature_distribution.length}>
        <BarList items={timeSignatureBars(props.s)} labelW={20} />
      </Panel>
    </Band>
  );
}

const col = (w: number) => ({ width: `calc(var(--u) * ${w})` });

function PluginsAndSamples(props: { s: Statistics }) {
  const maxPlugin = () => Math.max(1, ...props.s.top_plugins.map((p) => p.usage_count));
  const maxVendor = () => Math.max(1, ...props.s.top_vendors.map((v) => v.usage_count));
  const maxSample = () => Math.max(1, ...props.s.top_samples.map((x) => x.usage_count));
  return (
    <Band title="Plugins and samples">
      <Panel title="Most used plugins" span={3}>
        <table class="grid st-table">
          <thead><tr><th>Plugin</th><th>Vendor</th><th class="num" style={col(60)}>Projects</th></tr></thead>
          <tbody><For each={props.s.top_plugins}>{(p) => (
            <tr><td><span class="n">{p.name}</span></td><td class="dim">{p.vendor}</td>
              <td class="num"><InlineBar n={p.usage_count} max={maxPlugin()} /></td></tr>
          )}</For></tbody>
        </table>
      </Panel>
      <Panel title="Top vendors" span={3}>
        <table class="grid st-table">
          <thead><tr><th>Vendor</th><th class="num" style={col(30)}>Plugins</th><th class="num" style={col(60)}>Uses</th></tr></thead>
          <tbody><For each={props.s.top_vendors}>{(v) => (
            <tr><td><span class="n">{v.vendor}</span></td><td class="num dim">{v.plugin_count}</td>
              <td class="num"><InlineBar n={v.usage_count} max={maxVendor()} /></td></tr>
          )}</For></tbody>
        </table>
      </Panel>
      <Panel title="Most used samples" span={4}>
        <table class="grid fixed st-table">
          <thead><tr><th style={col(110)}>Sample</th><th>Folder</th><th class="num" style={col(60)}>Projects</th></tr></thead>
          <tbody><For each={props.s.top_samples}>{(x) => {
            const folder = () => folderOf(x.path);
            const cut = () => folder().lastIndexOf("\\") + 1;
            return (
              <tr><td><span class="n">{x.name}</span></td>
                <td><span class="pathcell" title={folder()}>
                  <span class="head">{folder().slice(0, cut())}</span>
                  <span class="tail">{folder().slice(cut())}</span>
                </span></td>
                <td class="num"><InlineBar n={x.usage_count} max={maxSample()} /></td></tr>
            );
          }}</For></tbody>
        </table>
      </Panel>
      <Panel title="Averages" span={2}>
        <Facts rows={[
          ["Plugins", <>{dec1(props.s.average_plugins_per_project)} <Faint>per project</Faint></>],
          ["Samples", <>{dec1(props.s.average_samples_per_project)} <Faint>per project</Faint></>],
        ]} />
        <p class="faint st-foot">Over every project counted, including those with none.</p>
      </Panel>
    </Band>
  );
}

/** The last thirty days as two strips, created and modified: small multiples on one day
 *  axis, so neither needs a legend. */
function Strips(props: { s: Statistics }) {
  const list = () => days(props.s);
  const max = () => dayMax(list());
  const strip = (key: DayKey, label: string) => (
    <div class="st-strip">
      <span class="st-sl">{label} <Faint>{list().reduce((a, d) => a + d[key], 0)}</Faint></span>
      <For each={list()}>{(d) => (
        <span class="st-cell" classList={{ zero: !d[key] }} data-hid={`${key}:${d.hid}`} data-tip={d.tip}>
          <i style={{ height: `${(100 * d[key]) / max()}%` }} />
        </span>
      )}</For>
    </div>
  );
  return (
    <>
      {strip("created", "Created")}
      {strip("modified", "Modified")}
      <div class="st-strip axis"><span class="st-sl" /><For each={list()}>{(d) => <span class="st-cell">{d.tick}</span>}</For></div>
    </>
  );
}

function Activity(props: { s: Statistics }) {
  return (
    <Band title="Activity">
      <Panel title="Created per month" span={4} count="last 12 months">
        <Columns items={monthColumns(props.s)} height={44} avg={monthAverage(props.s)} />
      </Panel>
      <Panel title="Created per year" span={2}><Columns items={yearColumns(props.s)} height={44} /></Panel>
      <Panel title="Last 30 days" span={6}><Strips s={props.s} /></Panel>
    </Band>
  );
}

function Library(props: { s: Statistics }) {
  const versions = () => versionBars(props.s);
  const c = () => props.s.largest_collection;
  return (
    <Band title="Library">
      <Panel title="Most complex projects" span={3} count="plugins + samples">
        <table class="grid st-table">
          <thead><tr><th>Project</th><th class="num" style={col(30)}>Plugins</th><th class="num" style={col(30)}>Samples</th><th class="num" style={col(26)}>Total</th></tr></thead>
          <tbody><For each={props.s.most_complex_projects}>{(m) => (
            <tr classList={{ archived: !m.project.is_active }}>
              <td><ProjectName p={m.project} /></td>
              <td class="num dim">{m.plugin_count}</td><td class="num dim">{m.sample_count}</td><td class="num">{m.complexity_score}</td>
            </tr>
          )}</For></tbody>
        </table>
      </Panel>
      <Panel title="Ableton versions" span={3} count={props.s.ableton_versions.length}>
        <BarList items={versions().bars} more={versions().more} moreLabel="older versions" labelW={30} />
      </Panel>
      <Panel title="Top tags" span={2}>
        <BarList items={tagBars(props.s)} labelW={40} label={(b) => <TagChip name={b.label} />} />
      </Panel>
      <Panel title="Collections" span={2}>
        <Facts rows={[["Average", <>{dec1(props.s.average_projects_per_collection)} <Faint>projects each</Faint></>]]} />
        <div class="st-sub">Largest</div>
        <Show when={c()} fallback={<Faint>No collection holds a project</Faint>}>
          <div class="st-largest">
            <Cover c={c()!} />
            <div>
              <div class="n">{c()!.name}</div>
              <div class="faint">{plural(c()!.project_count, "project")} · {fmtLength(c()!.total_duration_seconds)}</div>
            </div>
          </div>
        </Show>
      </Panel>
      <Panel title="Tasks per month" span={2} count="by month created">
        <div class="st-legend"><span><i class="is-installed" />Completed</span><span><i class="is-quiet" />Pending</span></div>
        <Columns items={taskColumns(props.s)} height={30} narrow stacked={taskStack} />
      </Panel>
    </Band>
  );
}

export function StatsPage(props: { s: Statistics }) {
  return (
    <div class="st-page" data-scope={projectScope()}
      onMouseOver={(e) => setHover((e.target as HTMLElement).closest<HTMLElement>("[data-hid]"))}
      onMouseLeave={() => setHover(null)}>
      <div class="st-tiles"><For each={overview(props.s, projectScope())}>{(t) => <StatTile tile={t} />}</For></div>
      <Music s={props.s} />
      <PluginsAndSamples s={props.s} />
      <Activity s={props.s} />
      <Library s={props.s} />
    </div>
  );
}
