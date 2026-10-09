// The plugins table. Plugin rows are the store's own proxies, so `For` keeps a row's
// nodes across a sort, a page change or a scan; only a group header is rebuilt, and only
// when the rows change.

import { For, Match, Show, Switch } from "solid-js";
import {
  GROUPS, PLUGIN_COLUMNS, isHeader, rollupFor, stateLabel, stateOf, type GroupHeader,
} from "../../../shared/plugins";
import { plural } from "../../../shared/projects";
import type { PluginRow } from "../../../shared/types";
import { Grip, createWidths } from "../shell/columns";
import { Icon } from "../shell/parts";
import { setShell } from "../shell/shell";
import { StatusDot } from "./parts";
import { entries, formats, pui, select, sortBy, toggleGroup, vendors } from "./state";

/** Column widths (ADR-0066), kept while the app runs. */
const widths = createWidths();
const last = (id: string) => PLUGIN_COLUMNS[PLUGIN_COLUMNS.length - 1].id === id;

function Head() {
  return (
    <thead>
      <tr>
        <For each={PLUGIN_COLUMNS}>{(c) => {
          const sorted = () => pui.sort?.col === c.id;
          return (
            <th classList={{ num: c.num, sorted: sorted() }} style={widths.th(c, last(c.id))} onClick={() => sortBy(c.id)}>
              {c.label}
              <Show when={sorted()}><Icon name={pui.sort!.desc ? "arrow_downward" : "arrow_upward"} /></Show>
              <Grip widths={widths} col={PLUGIN_COLUMNS[PLUGIN_COLUMNS.indexOf(c) - 1]} />
            </th>
          );
        }}</For>
      </tr>
    </thead>
  );
}

function Header(props: { g: GroupHeader }) {
  const open = () => !pui.collapsed[String(props.g.key)];
  const roll = () => (pui.group ? rollupFor(pui.group, props.g.key, vendors(), formats()) : undefined);
  const bits = () => {
    const r = roll();
    if (!r) return [plural(props.g.shown, "plugin")];
    return [
      plural(r.plugin_count, "plugin"),
      r.installed_plugins ? `${r.installed_plugins} installed` : "",
      r.missing_plugins ? `${r.missing_plugins} missing` : "",
      r.unknown_plugins ? `${r.unknown_plugins} not scanned` : "",
      `used in ${plural(r.unique_projects_using, "project")}`,
    ].filter(Boolean);
  };
  return (
    <tr class="group" onClick={() => toggleGroup(String(props.g.key))}>
      <td colspan={PLUGIN_COLUMNS.length}>
        <Icon name={open() ? "expand_more" : "chevron_right"} class="caret" />
        <span class="gname">{GROUPS[pui.group!].name(props.g.key)}</span>
        <Show when={roll() && props.g.shown !== roll()!.plugin_count}><span class="shown">{props.g.shown} shown</span></Show>
        <span class="gstats">{bits().join(" · ")}</span>
      </td>
    </tr>
  );
}

function Row(props: { p: PluginRow }) {
  return (
    <tr data-id={props.p.id} classList={{ sel: pui.selected === props.p.id }}
      onClick={() => select(props.p.id)}
      onContextMenu={(e) => {
        e.preventDefault();
        select(props.p.id);
        const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
        setShell({ popover: null, menu: { id: props.p.id, x: e.clientX - win.left, y: e.clientY - win.top } });
      }}>
      <td><span class="n">{props.p.name}</span></td>
      <td>
        <span class="pstate" classList={{ [`is-${stateOf(props.p)}`]: true }}>
          <StatusDot state={stateOf(props.p)} />{stateLabel(props.p)}
        </span>
      </td>
      <td class="dim"><Show when={props.p.vendor} fallback={<span class="faint">No vendor</span>}>{props.p.vendor}</Show></td>
      <td class="dim">{props.p.format}</td>
      <td class="dim">{props.p.version ?? ""}</td>
      <td class="num">{props.p.project_count || ""}</td>
    </tr>
  );
}

export function PluginTable() {
  return (
    <table class="grid fixed plugins" style={widths.table(PLUGIN_COLUMNS)}>
      <Head />
      <tbody>
        <For each={entries()}>{(e) => (
          <Switch>
            <Match when={isHeader(e) && e}>{(g) => <Header g={g()} />}</Match>
            <Match when={!isHeader(e) && (e as PluginRow)}>{(p) => <Row p={p()} />}</Match>
          </Switch>
        )}</For>
      </tbody>
    </table>
  );
}
