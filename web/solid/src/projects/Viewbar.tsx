// The projects view's toolbar row: filters and paging, or batch actions at >1 selected.

import { Show } from "solid-js";
import { plural } from "../../../shared/projects";
import { Icon, TbBtn } from "../shell/parts";
import { runScan, scan, setShell, shell } from "../shell/shell";
import { Pager } from "../shell/toolbar";
import { load, setQuery, setScope, table as t, ui } from "./state";



function Batch() {
  // Tag, collection and archive edits are not part of this comparison; the buttons are
  // the mockup's, disabled.
  return (
    <>
      <span class="count-sel">{t.selectedCount()} selected</span>
      <TbBtn icon="close" title="Clear selection" onClick={t.clearSelection} />
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
        <Icon name="search" />“{ui.query}” · {plural(t.total(), "result")}
        <span onClick={() => setQuery("")}><Icon name="close" /></span>
      </span>
      <span class="muted">{t.sort() ? "" : "by relevance"}</span>
    </Show>
  );
}

export function Viewbar() {
  return (
    <>
      <Show when={t.selectedCount() > 1} fallback={
        <>
          <h1>Projects</h1>
          <Filters />
          <span class="sep" />
          <TbBtn icon="view_column" title="Columns" keep on={shell.popover === "columns"}
            onClick={() => setShell("popover", shell.popover === "columns" ? null : "columns")} />
          <TbBtn icon="refresh" label="Scan" title="Scan the project folders" disabled={!!scan()} onClick={() => runScan("projects", load)} />
          <TbBtn icon="speed" label="Simulate" title="A stand-in scan of 600 events at 40 a second" disabled={!!scan()} onClick={() => runScan("simulated", load)} />
          <span class="grow" />
          <Pager page={t.page()} total={t.total()} pages={t.pageCount()} onPage={t.goPage} onSize={t.firstPage} />
        </>}>
        <Batch />
      </Show>
    </>
  );
}
