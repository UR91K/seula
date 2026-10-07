// Column widths, as in Explorer's details view (ADR-0066). Every column keeps its own
// width whatever the pane's width: the table scrolls sideways rather than squeeze them.
// The last column has no width of its own and takes the rest of the row; its default
// still counts towards the table's minimum, so it never collapses to nothing. A header's
// grip drags its column's width. Widths are kept in memory, per table, until the
// preferences endpoint exists (as the collections layout is, ADR-0062).

import { createStore } from "solid-js/store";

/** A column's id, and its default width in `--u`. */
export interface Col { id: string; w: number }

/** The narrowest a column can be dragged, in px. */
const MIN = 24;

export function createWidths() {
  /** Dragged widths, in px. A column that was never dragged keeps its default. */
  const [px, setPx] = createStore<Record<string, number>>({});
  const width = (c: Col) => (px[c.id] != null ? `${px[c.id]}px` : `calc(var(--u) * ${c.w})`);

  return {
    /** A header cell's style. The last column's is empty: it takes what is left. */
    th: (c: Col, last: boolean) => (last ? {} : { width: width(c) }),

    /** The table's style: as wide as the pane, and never narrower than its columns. */
    table: (cols: Col[]) => ({ "min-width": `calc(${cols.map(width).join(" + ")})` }),

    /** Drag `c`'s width from its grip. It starts from the column's drawn width, so the
     *  last column, drawn wider than its default, does not jump when grabbed. */
    grab(c: Col, e: PointerEvent) {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      const grip = e.currentTarget as HTMLElement;
      const start = grip.closest("th")!.getBoundingClientRect().width;
      const x0 = e.clientX;
      grip.setPointerCapture(e.pointerId);
      const move = (ev: PointerEvent) => setPx(c.id, Math.max(MIN, Math.round(start + ev.clientX - x0)));
      const end = () => {
        grip.removeEventListener("pointermove", move);
        grip.removeEventListener("pointerup", end);
        grip.removeEventListener("pointercancel", end);
      };
      grip.addEventListener("pointermove", move);
      grip.addEventListener("pointerup", end);
      grip.addEventListener("pointercancel", end);
    },
  };
}

export type Widths = ReturnType<typeof createWidths>;

/** A header's resize handle. It swallows its own click, so a drag never sorts. */
export function Grip(props: { widths: Widths; col: Col }) {
  return (
    <span class="grip"
      onPointerDown={(e) => props.widths.grab(props.col, e)}
      onClick={(e) => e.stopPropagation()} />
  );
}
