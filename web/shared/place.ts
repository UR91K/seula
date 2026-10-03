/** Position a popover inside the window, below (or beside) an anchor, or at a point, and
 *  keep it on screen. Popovers are measured, so these are px set at runtime: geometry,
 *  not a stylesheet measure (the mockup's `place`). Call after the popover is in the DOM. */
export type Placement =
  | { x: number; y: number }                                              // window-relative point
  | { anchor: Element; align?: "left" | "right"; beside?: boolean };

export function place(win: HTMLElement, pop: HTMLElement, at: Placement): void {
  const w = win.getBoundingClientRect();
  let left: number, top: number;
  if ("anchor" in at) {
    const a = at.anchor.getBoundingClientRect();
    if (at.beside) { left = a.right - w.left; top = a.top - w.top - 3; }
    else {
      left = (at.align === "right" ? a.right : a.left) - w.left - (at.align === "right" ? pop.offsetWidth : 0);
      top = a.bottom - w.top;
    }
  } else { left = at.x; top = at.y; }
  pop.style.left = `${Math.max(0, Math.min(left, win.offsetWidth - pop.offsetWidth - 2))}px`;
  pop.style.top = `${Math.max(0, Math.min(top, win.offsetHeight - pop.offsetHeight - 2))}px`;
}

/** A point inside the window, from a mouse event. */
export function pointIn(win: HTMLElement, e: MouseEvent) {
  const w = win.getBoundingClientRect();
  return { x: e.clientX - w.left, y: e.clientY - w.top };
}
