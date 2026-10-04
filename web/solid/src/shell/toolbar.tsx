// Pieces of a view's toolbar row that every list view uses the same way.

import { Show } from "solid-js";
import { Icon, TbBtn } from "./parts";
import { setShell, shell } from "./shell";

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

export function Pager(props: { page: number; total: number; size: number; pages: number; onPage(delta: number): void }) {
  const from = () => (props.total ? props.page * props.size + 1 : 0);
  const to = () => Math.min(props.total, (props.page + 1) * props.size);
  return (
    <span class="pager">
      {from()}–{to()} of {props.total}
      <TbBtn icon="chevron_left" title="Previous page" disabled={props.page === 0} onClick={() => props.onPage(-1)} />
      <TbBtn icon="chevron_right" title="Next page" disabled={props.page >= props.pages - 1} onClick={() => props.onPage(1)} />
      <span class="select"><span class="k">Show</span> {props.size} <Icon name="expand_more" /></span>
    </span>
  );
}
