// The plugins view's toolbar row: search chip, the three filters, grouping, the scan
// button and the pager.

import { Show, type JSX } from "solid-js";
import { GROUPS, INSTALL_STATES } from "../../../shared/plugins";
import { plural } from "../../../shared/projects";
import { Icon, TbBtn } from "../shell/parts";
import { scan, setShell, shell } from "../shell/shell";
import { FilterSelect, Pager } from "../shell/toolbar";
import {
  goPage, loaded, pageCount, pageSize, plugins, pui, scanPlugins, setFormat, setPui, setQuery, setVendor, total,
} from "./state";

export function ScanButton(): JSX.Element {
  return (
    <TbBtn icon={scan() ? "hourglass_top" : "radar"} label={scan() ? "Scanning…" : "Scan plugins"}
      title={scan() ? "A scan is running" : "Rescan the plugin folders"} disabled={!!scan()} onClick={scanPlugins} />
  );
}

export function Viewbar() {
  const states = () => (pui.states.length
    ? INSTALL_STATES.filter((s) => pui.states.includes(s.id)).map((s) => s.label).join(", ") : null);
  // A library no scan has filled has nothing to filter or page yet.
  const empty = () => loaded() && plugins.length === 0 && !pui.query;
  return (
    <>
      <h1>Plugins</h1>
      <Show when={!empty()} fallback={<ScanButton />}>
        <Show when={pui.query}>
          <span class="chip">
            <Icon name="search" />“{pui.query}” · {plural(total(), "result")}
            <span onClick={() => setQuery("")}><Icon name="close" /></span>
          </span>
        </Show>
        <FilterSelect name="format" label="Format" value={pui.format} onClear={() => setFormat(null)} />
        <FilterSelect name="vendor" label="Vendor" value={pui.vendor} onClear={() => setVendor(null)} />
        <FilterSelect name="states" label="Status" value={states()} onClear={() => { setPui("states", []); setPui("page", 0); }} />
        <span class="select" classList={{ open: shell.popover === "group" }} data-keep="" data-pop="group"
          onClick={() => setShell({ menu: null, popover: shell.popover === "group" ? null : "group" })}>
          <span class="k">Group</span> {pui.group ? GROUPS[pui.group].label : "None"} <Icon name="expand_more" />
        </span>
        <span class="sep" />
        <ScanButton />
        <span class="grow" />
        <Pager page={pui.page} total={total()} size={pageSize} pages={pageCount()} onPage={goPage} />
      </Show>
    </>
  );
}
