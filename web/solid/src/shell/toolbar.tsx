// Pieces of a view's toolbar row that every list view uses the same way.

import { For, Show, batch, onMount } from "solid-js";
import { place } from "../../../shared/place";
import { Icon, TbBtn } from "./parts";
import { PAGE_SIZES, closePopups, pageSize, setPageSize, setShell, shell } from "./shell";

/** A filter dropdown. `data-keep` keeps the window's click-away handler from closing the
 *  popover this very click opens. */
export function FilterSelect(props: { name: string; label: string; value: string | null; onClear(): void }) {
  const open = () => shell.popover === props.name;
  return (
    <span class="select" classList={{ set: !!props.value, open: open() }} data-keep="" data-pop={props.name}
      onClick={() => setShell({ menu: null, popover: open() ? null : props.name })}>
      <span class="k">{props.label}</span> {props.value ?? "All"}{" "}
      <Show when={props.value} fallback={<Icon name="expand_more" />}>
        <span class="x" title="Clear" onClick={(e) => { e.stopPropagation(); props.onClear(); }}><Icon name="close" /></span>
      </Show>
    </span>
  );
}

/** Paging, and the rows-per-page choice. `onSize` is the view's chance to go back to its
 *  first page, which the new size may have left past the end. */
export function Pager(props: { page: number; total: number; pages: number; onPage(delta: number): void; onSize(): void }) {
  const from = () => (props.total ? props.page * pageSize() + 1 : 0);
  const to = () => Math.min(props.total, (props.page + 1) * pageSize());
  const open = () => shell.popover === "pagesize";
  const choose = (n: number) => batch(() => { setPageSize(n); props.onSize(); closePopups(); });
  return (
    <span class="pager">
      {from()}–{to()} of {props.total}
      <TbBtn icon="chevron_left" title="Previous page" disabled={props.page === 0} onClick={() => props.onPage(-1)} />
      <TbBtn icon="chevron_right" title="Next page" disabled={props.page >= props.pages - 1} onClick={() => props.onPage(1)} />
      <span class="select" classList={{ open: open() }} data-keep="" data-pop="pagesize"
        onClick={() => setShell({ menu: null, popover: open() ? null : "pagesize" })}>
        <span class="k">Show</span> {pageSize()} <Icon name="expand_more" />
      </span>
      <Show when={open()}>
        <div class="pop picker pickmenu" style={{ width: "calc(var(--u) * 50)" }} ref={(el: HTMLElement) => onMount(() => {
          const anchor = document.querySelector('.viewbar [data-pop="pagesize"]');
          if (anchor) place(el.closest(".win") as HTMLElement, el, { anchor });
        })}>
          <div class="list" style={{ "padding-top": "var(--u)" }}>
            <For each={PAGE_SIZES}>{(n) => (
              <div class="it" classList={{ hot: n === pageSize() }} onClick={() => choose(n)}>
                <Icon name="check" class={n === pageSize() ? "" : "blank"} /><span class="grow">{n}</span>
              </div>
            )}</For>
          </div>
        </div>
      </Show>
    </span>
  );
}
