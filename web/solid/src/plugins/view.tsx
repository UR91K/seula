import { Show, onMount } from "solid-js";
import { plural } from "../../../shared/projects";
import { Icon } from "../shell/parts";
import { lastScan, scan, shell } from "../shell/shell";
import type { View } from "../shell/view";
import { StatusDot } from "./parts";
import { Inspector } from "./Inspector";
import { ContextMenu, FormatMenu, GroupMenu, ScanMenu, StatesMenu, VendorPicker } from "./Popovers";
import { PluginTable } from "./Table";
import { ScanButton, Viewbar } from "./Viewbar";
import {
  clearFilters, filtered, load, loadError, loaded, plugins, pui, setQuery, stats, total,
} from "./state";
import { counts } from "../shell/shell";

function Empty() {
  return (
    <Show when={!loadError()}>
      <Show when={loaded()}>
        <Show when={plugins.length === 0 && !pui.query} fallback={
          <div class="empty">
            <Icon name="search_off" /><h2>No plugins match</h2>
            <p>{[pui.query && `“${pui.query}”`, pui.format, pui.vendor].filter(Boolean).join(" · ")}</p>
            <button class="btn" onClick={() => { clearFilters(); setQuery(""); }}>Clear filters</button>
          </div>}>
          <div class="empty">
            <Icon name="plug_connect" /><h2>No plugins yet</h2>
            <p>A plugin scan lists what is installed in your plugin folders. Plugins your projects use are added as the projects are scanned.</p>
            <ScanButton />
          </div>
        </Show>
      </Show>
    </Show>
  );
}

function Content() {
  onMount(load);
  return (
    <>
      <Show when={loadError()}><div class="error-banner">Cannot reach the daemon: {loadError()}</div></Show>
      <Show when={total() > 0} fallback={<Empty />}><PluginTable /></Show>
    </>
  );
}

function Status() {
  return (
    <Show when={loaded() && !(plugins.length === 0 && !pui.query)} fallback={<span>No plugins</span>}>
      <span>{filtered() ? `${total()} of ${counts.plugins ?? plugins.length} plugins` : plural(total(), "plugin")}</span>
      <span><StatusDot state="installed" /> {stats().installed} installed</span>
      <span><StatusDot state="absent" /> {stats().missing} missing</span>
      <Show when={stats().failed}><span><StatusDot state="failed" /> {stats().failed} failed</span></Show>
      <span><StatusDot state="unscanned" /> {stats().unscanned} not scanned</span>
      <span>{plural(stats().vendors, "vendor")}</span>
      <Show when={!scan() && lastScan()?.kind === "plugins" ? lastScan() : undefined}>{(l) => (
        <span class={l().event.status === "completed" ? "is-installed" : "is-absent"}>
          <Icon name={l().event.status === "completed" ? "task_alt" : "error"} /> {l().event.message}
        </span>
      )}</Show>
    </Show>
  );
}

function Popovers() {
  return (
    <>
      <Show when={shell.menu} keyed>{(m) => <ContextMenu menu={m} />}</Show>
      <Show when={shell.popover === "format"}><FormatMenu /></Show>
      <Show when={shell.popover === "vendor"}><VendorPicker /></Show>
      <Show when={shell.popover === "states"}><StatesMenu /></Show>
      <Show when={shell.popover === "group"}><GroupMenu /></Show>
      <Show when={shell.popover === "scan"}><ScanMenu /></Show>
    </>
  );
}

export const pluginsView: View = {
  Viewbar, Content, Inspector, Status, Popovers, keys: false,
  query: () => pui.query,
  setQuery,
};
