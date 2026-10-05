import { For, Show, createEffect, onCleanup } from "solid-js";
import { isEmpty, SCOPES, statusFor } from "../../../shared/stats";
import type { StatsScope } from "../../../shared/types";
import { place } from "../../../shared/place";
import { Icon, TbBtn } from "../shell/parts";
import { projectScope, setShell, shell } from "../shell/shell";
import type { View } from "../shell/view";
import { StatsPage } from "./Page";
import { exportCsv, hover, setHover, setScope, statistics } from "./state";

const SCOPE_IDS: StatsScope[] = ["active", "all"];

function Viewbar() {
  const open = () => shell.popover === "scope";
  return (
    <>
      <h1>Stats</h1>
      <Show when={statistics() && !isEmpty(statistics()!)}>
        <span class="select" classList={{ open: open() }} data-keep="" data-pop="scope"
          title="The project scope, shared with Plugins, Samples and Collections"
          onClick={() => setShell({ menu: null, popover: open() ? null : "scope" })}>
          <span class="k">Counting</span> {SCOPES[projectScope()]} <Icon name="expand_more" />
        </span>
        <span class="sep" />
        <TbBtn icon="download" label="Export CSV" onClick={exportCsv}
          title={`Every figure on this page, as a CSV file (${SCOPES[projectScope()].toLowerCase()})`} />
      </Show>
    </>
  );
}

function Content() {
  return (
    <>
      <Show when={statistics.error}>
        <div class="error-banner">Cannot reach the daemon: {String(statistics.error)}</div>
      </Show>
      <Show when={statistics()}>{(s) => (
        <Show when={!isEmpty(s())} fallback={
          <div class="empty">
            <Icon name="bar_chart" /><h2>Nothing to count yet</h2>
            <p>Statistics fill in as your projects are scanned.</p>
          </div>}>
          <StatsPage s={s()} />
        </Show>
      )}</Show>
    </>
  );
}

function Status() {
  return (
    <Show when={statistics()} fallback={<span>&nbsp;</span>}>{(s) => (
      <Show when={!isEmpty(s())} fallback={<span>No projects</span>}>
        <span>{statusFor(s(), projectScope()).count}</span>
        <Show when={statusFor(s(), projectScope()).archived} fallback={<span class="faint">{statusFor(s(), projectScope()).note}</span>}>
          <span><Icon name="inventory_2" class="sb" /> {statusFor(s(), projectScope()).note}</span>
        </Show>
      </Show>
    )}</Show>
  );
}

/** Counting ▾: the scope, with how many projects each counts. */
function ScopeMenu() {
  let el!: HTMLDivElement;
  createEffect(() => {
    const anchor = document.querySelector('.viewbar [data-pop="scope"]');
    if (anchor) place(el.closest(".win") as HTMLElement, el, { anchor });
  });
  const n = (scope: StatsScope) => (scope === "all" ? statistics()?.projects.total : statistics()?.projects.active);
  return (
    <div class="pop picker pickmenu" ref={el} style={{ width: "calc(var(--u) * 110)" }}>
      <div class="list" style={{ "padding-top": "var(--u)", "max-height": "none" }}>
        <For each={SCOPE_IDS}>{(id) => (
          <div class="it" classList={{ hot: id === projectScope() }} onClick={() => setScope(id)}>
            <Icon name="check" class={id === projectScope() ? "" : "blank"} />
            <span class="grow">{id === "all" ? "All projects, archived too" : SCOPES[id]}</span>
            <span class="n">{n(id)}</span>
          </div>
        )}</For>
      </div>
      <div class="hint">The same setting as in Plugins, Samples and Collections</div>
    </div>
  );
}

/** The tooltip of the mark under the pointer, above it and centred, where it does not cover
 *  what it describes. */
function Tip() {
  let el!: HTMLDivElement;
  const mark = hover;
  createEffect(() => {
    const m = mark();
    if (!m) return;
    m.classList.add("hot");
    onCleanup(() => m.classList.remove("hot"));
    const win = el.closest(".win") as HTMLElement;
    const zoom = win.getBoundingClientRect().width / win.offsetWidth || 1;
    const a = m.getBoundingClientRect(), w = win.getBoundingClientRect();
    const left = (a.left + a.width / 2 - w.left) / zoom - el.offsetWidth / 2;
    const top = (a.top - w.top) / zoom - el.offsetHeight - 4;
    el.style.left = `${Math.max(0, Math.min(left, win.offsetWidth - el.offsetWidth - 2))}px`;
    el.style.top = `${Math.max(0, top)}px`;
  });
  return <div class="pop tip" ref={el} style={{ visibility: mark() ? "visible" : "hidden" }}>{mark()?.dataset.tip}</div>;
}

function Popovers() {
  onCleanup(() => setHover(null));
  return (
    <>
      <Show when={shell.popover === "scope"}><ScopeMenu /></Show>
      <Show when={hover()}><Tip /></Show>
    </>
  );
}

export const statsView: View = {
  Viewbar, Content, Inspector: null, Status, Popovers, keys: true,
  // Stats has nothing to search: the box is left as it is and ignored.
  query: () => "",
  setQuery: () => {},
};
