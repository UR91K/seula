import { Show, onMount } from "solid-js";
import { fmtTotal } from "../../../shared/collections";
import { plural } from "../../../shared/projects";
import { ColumnChooser, ContextMenu as ProjectMenu, HoverList } from "../projects/Popovers";
import { Icon } from "../shell/parts";
import { counts, loadCollectionNames, setShell, shell } from "../shell/shell";
import type { View } from "../shell/view";
import { Dialogs } from "./Dialogs";
import { Inspector } from "./Inspector";
import { Details, Grid } from "./List";
import { Opened } from "./Opened";
import { ContextMenu, SortMenu } from "./Popovers";
import { Viewbar } from "./Viewbar";
import {
  collections, cui, load, loadError, loaded, openCollection, openDialog, removeSelected, setCui, setQuery,
  tracks,
} from "./state";

function Empty() {
  return (
    <Show when={loaded() && !loadError()}>
      <Show when={cui.query} fallback={
        <div class="empty">
          <Icon name="album" /><h2>No collections yet</h2>
          <p>A collection is an ordered list of projects: an EP, a set, a batch of ideas. Select projects in Projects and choose Collection ▾ › New collection from selection, or start an empty one.</p>
          <button class="btn primary" onClick={() => openDialog("new")}><Icon name="library_add" />New collection</button>
        </div>}>
        <div class="empty">
          <Icon name="search_off" /><h2>No collections match “{cui.query}”</h2>
          <p>Search covers collection names, descriptions and notes. To find projects, search from Projects.</p>
        </div>
      </Show>
    </Show>
  );
}

function List() {
  return (
    <Show when={collections.length > 0} fallback={<Empty />}>
      <Show when={cui.layout === "grid"} fallback={<Details />}><Grid /></Show>
    </Show>
  );
}

function Content() {
  onMount(() => { load(); loadCollectionNames(); });
  return (
    <>
      <Show when={loadError()}><div class="error-banner">Cannot reach the daemon: {loadError()}</div></Show>
      <Show when={cui.open} fallback={<List />}><Opened /></Show>
    </>
  );
}

function Status() {
  const secs = () => tracks.sorted().reduce((n, p) => n + (p.duration_seconds ?? 0), 0);
  return (
    <Show when={cui.open} fallback={
      <Show when={loaded() && collections.length + (counts.collections ?? 0) > 0} fallback={<span>No collections</span>}>
        <span>
          {cui.query ? `${collections.length} of ${counts.collections ?? collections.length} collections` : plural(collections.length, "collection")}
        </span>
      </Show>}>
      <span>{plural(tracks.total(), "project")}</span>
      <Show when={tracks.total() > 0}><span>{fmtTotal(secs())}</span></Show>
      <Show when={tracks.selectedCount()}><span>{tracks.selectedCount()} selected</span></Show>
    </Show>
  );
}

/** A tracklist row's menu: the project menu, plus taking it out of this collection. */
function TrackMenu(props: { menu: { id: string; x: number; y: number } }) {
  return (
    <ProjectMenu t={tracks} menu={props.menu} extra={
      <div class="mi" onClick={() => { setShell("menu", null); removeSelected(); }}>
        <Icon name="playlist_remove" /><span class="lbl">Remove from collection</span>
      </div>} />
  );
}

function Popovers() {
  return (
    <>
      <Show when={shell.menu} keyed>{(m) => (
        <Show when={cui.open} fallback={<ContextMenu menu={m} />}><TrackMenu menu={m} /></Show>)}
      </Show>
      <Show when={cui.open ? tracks.hot() : null} keyed>{(h) => <HoverList t={tracks} hot={h} />}</Show>
      <Show when={shell.popover === "sort"}><SortMenu /></Show>
      <Show when={cui.open && shell.popover === "columns"}><ColumnChooser t={tracks} /></Show>
      <Dialogs />
    </>
  );
}

export const collectionsView: View = {
  Viewbar, Content, Inspector, Status, Popovers, keys: true,
  query: () => cui.query,
  setQuery,
  onKey(e) {
    const t = e.target as HTMLElement;
    if (t.closest("input, textarea") || cui.dialog) return;
    if (cui.open) {
      if (e.key === "F2" && tracks.selectedProjects().length === 1) { e.preventDefault(); tracks.setRenaming(tracks.selectedProjects()[0].id); }
      return;
    }
    if (!cui.selected) return;
    if (e.key === "Enter") { e.preventDefault(); openCollection(cui.selected); }
    else if (e.key === "F2") { e.preventDefault(); setCui("renaming", cui.selected); }
    else if (e.key === "Delete") { e.preventDefault(); openDialog("delete", cui.selected); }
  },
};
