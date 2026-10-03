// The projects view's toolbar row: filters and paging, or batch actions at >1 selected.

import { Show } from "solid-js";
import { plural } from "../../shared/projects";
import { Icon, TbBtn } from "./parts";
import {
  clearSelection, goPage, pageCount, runScan, scan, selectedCount, setQuery, setScope, setUi, total, ui,
} from "./state";
import { PAGE_SIZE } from "../../shared/projects";

function Pager() {
  const from = () => (total() ? ui.page * PAGE_SIZE + 1 : 0);
  const to = () => Math.min(total(), (ui.page + 1) * PAGE_SIZE);
  return (
    <span class="pager">
      {from()}–{to()} of {total()}
      <TbBtn icon="chevron_left" title="Previous page" disabled={ui.page === 0} onClick={() => goPage(-1)} />
      <TbBtn icon="chevron_right" title="Next page" disabled={ui.page >= pageCount() - 1} onClick={() => goPage(1)} />
      <span class="select"><span class="k">Show</span> {PAGE_SIZE} <Icon name="expand_more" /></span>
    </span>
  );
}

function Batch() {
  // Tag, collection and archive edits are not part of this comparison; the buttons are
  // the mockup's, disabled.
  return (
    <>
      <span class="count-sel">{selectedCount()} selected</span>
      <TbBtn icon="close" title="Clear selection" onClick={clearSelection} />
      <span class="sep" />
      <TbBtn icon="sell" label="Tags" caret disabled />
      <TbBtn icon="album" label="Collection" caret disabled />
      <TbBtn icon="archive" label="Archive" disabled />
    </>
  );
}

function Filters() {
  return (
    <Show when={ui.query} fallback={
      <>
        <span class="select" onClick={() => setScope(ui.scope === "archived" ? "active" : "archived")}>
          <span class="k">Show</span> {ui.scope === "archived" ? "Archived" : "Active"} <Icon name="expand_more" />
        </span>
        <span class="select"><span class="k">Tag</span> Any <Icon name="expand_more" /></span>
      </>}>
      <span class="chip">
        <Icon name="search" />“{ui.query}” · {plural(total(), "result")}
        <span onClick={() => setQuery("")}><Icon name="close" /></span>
      </span>
      <span class="muted">{ui.sort ? "" : "by relevance"}</span>
    </Show>
  );
}

export function Viewbar() {
  return (
    <>
      <Show when={selectedCount() > 1} fallback={
        <>
          <h1>Projects</h1>
          <Filters />
          <span class="sep" />
          <TbBtn icon="view_column" title="Columns" keep on={ui.popover === "columns"}
            onClick={() => setUi("popover", ui.popover === "columns" ? null : "columns")} />
          <TbBtn icon="refresh" label="Scan" title="Scan the project folders" disabled={!!scan()} onClick={() => runScan(false)} />
          <TbBtn icon="speed" label="Simulate" title="A stand-in scan of 600 events at 40 a second" disabled={!!scan()} onClick={() => runScan(true)} />
          <span class="grow" />
          <Pager />
        </>}>
        <Batch />
      </Show>
    </>
  );
}
