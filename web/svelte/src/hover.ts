// The hover list closes shortly after the pointer leaves its cell, unless it has moved
// onto the list itself.
import { ui } from "./state.svelte";

let hideTimer: ReturnType<typeof setTimeout> | undefined;
export const holdHover = () => clearTimeout(hideTimer);
export const releaseHover = () => {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => { ui.hot = null; }, 150);
};
