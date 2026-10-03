// A Svelte action that places a popover against the window once it is mounted
// (shared/place.ts does the measuring).
import { place, type Placement } from "../../shared/place";

export function placed(el: HTMLElement, at: () => Placement | null) {
  const win = el.closest(".win") as HTMLElement;
  const p = at();
  if (p) place(win, el, p);
}
