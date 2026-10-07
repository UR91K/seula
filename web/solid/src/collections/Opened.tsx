// An opened collection (ADR-0044): a header band, then the projects table over its
// projects in collection order, with the place column and drag handles (ADR-0055,
// ADR-0057). Sorting here reorders the view only; the stored order is kept.

import { For, Show } from "solid-js";
import { collectionFacts } from "../../../shared/collections";
import { ProjectTable } from "../projects/Table";
import { Icon } from "../shell/parts";
import { shell } from "../shell/shell";
import { Cover } from "./parts";
import { cui, collectionById, detail, reorderTracks, tracks } from "./state";

function Header(props: { id: string }) {
  const c = () => collectionById(props.id);
  // The facts of this collection only: a stale answer for another is not shown.
  const d = () => (detail()?.id === props.id ? detail() : undefined);
  return (
    <Show when={c()}>{(x) => (
      <div class="colhead">
        <Cover c={x()} class="big" />
        <div class="txt">
          <h2>{x().name}</h2>
          <Show when={x().description}><p class="desc">{x().description}</p></Show>
          <Show when={d()}>{(dd) => (
            <p class="facts">
              <For each={collectionFacts(dd().stats,
                { total_tasks: dd().totalTasks, completed_tasks: dd().tasks.filter((t) => t.completed).length }, shell.spelling)}>
                {(f) => <span>{f}</span>}
              </For>
            </p>
          )}</Show>
        </div>
      </div>
    )}</Show>
  );
}

export function Opened() {
  return (
    <Show when={cui.open}>{(id) => (
      <>
        <Header id={id()} />
        <Show when={tracks.total() > 0} fallback={
          <Show when={detail()?.id === id()}>
            <div class="empty tracks-empty">
              <Icon name="playlist_add" /><h2>Nothing in this collection yet</h2>
              <p>Add projects from the Projects view: select them, then Collection ▾ › Add to collection.</p>
            </div>
          </Show>}>
          <ProjectTable t={tracks} tracks={{ onReorder: reorderTracks }} />
        </Show>
      </>
    )}</Show>
  );
}
