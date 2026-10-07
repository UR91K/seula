// The collections view's toolbar row: sort, new collection, and the layout switch.

import { Show } from "solid-js";
import { COLLECTION_SORTS, sameSort } from "../../../shared/collections";
import { columnById, plural } from "../../../shared/projects";
import { Icon, TbBtn } from "../shell/parts";
import { setShell, shell } from "../shell/shell";
import {
  closeCollection, collectionById, collections, cui, loaded, openDialog, removeSelected, setLayout, setQuery, tracks,
} from "./state";

function NewButton() {
  return <TbBtn icon="library_add" label="New collection" onClick={() => openDialog("new")} />;
}

/** An opened collection: back, the path, what is selected, and the column chooser. */
function OpenedViewbar() {
  const name = () => collectionById(cui.open!)?.name ?? "";
  const sortHint = () => {
    const s = tracks.sort();
    const label = s && (s.col === "name" || s.col === "tags" ? s.col : columnById(s.col).label.toLowerCase());
    return label ? `Sorted by ${label}. Sort by # to drag.` : null;
  };
  return (
    <>
      <TbBtn icon="arrow_back" title="Back to collections" onClick={closeCollection} />
      <span class="crumb" onClick={closeCollection}>Collections</span>
      <Icon name="chevron_right" class="crumbsep" />
      <h1>{name()}</h1>
      <Show when={tracks.selectedCount()}>
        <span class="sep" />
        <span class="count-sel">{tracks.selectedCount()} selected</span>
        <TbBtn icon="close" title="Clear selection" onClick={() => tracks.clearSelection()} />
        <TbBtn icon="playlist_remove" label="Remove from collection" onClick={removeSelected} />
      </Show>
      <span class="sep" />
      <TbBtn icon="view_column" title="Columns" keep on={shell.popover === "columns"}
        onClick={() => setShell("popover", shell.popover === "columns" ? null : "columns")} />
      <Show when={sortHint()}>{(h) => <span class="muted">{h()}</span>}</Show>
    </>
  );
}

export function Viewbar() {
  return <Show when={cui.open} fallback={<ListViewbar />}><OpenedViewbar /></Show>;
}

function ListViewbar() {
  const sortLabel = () => (COLLECTION_SORTS.find((s) => sameSort(s.sort, cui.sort)) ?? COLLECTION_SORTS[0]).label;
  // A library with no collections has nothing to sort or switch.
  const empty = () => loaded() && collections.length === 0 && !cui.query;
  const open = () => shell.popover === "sort";
  return (
    <>
      <h1>Collections</h1>
      <Show when={!empty()} fallback={<NewButton />}>
        <Show when={cui.query} fallback={
          // The details table sorts by its headers, so the dropdown is the grid's alone.
          <Show when={cui.layout === "grid"}>
            <span class="select" classList={{ open: open() }} data-keep="" data-pop="sort"
              onClick={() => setShell({ menu: null, popover: open() ? null : "sort" })}>
              <span class="k">Sort</span> {sortLabel()} <Icon name="expand_more" />
            </span>
          </Show>}>
          <span class="chip">
            <Icon name="search" />“{cui.query}” · {plural(collections.length, "result")}
            <span onClick={() => setQuery("")}><Icon name="close" /></span>
          </span>
        </Show>
        <span class="sep" />
        <NewButton />
        <span class="grow" />
        <span class="layoutswitch">
          <TbBtn icon="grid_view" title="Grid" on={cui.layout === "grid"} onClick={() => setLayout("grid")} />
          <TbBtn icon="view_list" title="Details" on={cui.layout === "details"} onClick={() => setLayout("details")} />
        </span>
      </Show>
    </>
  );
}
