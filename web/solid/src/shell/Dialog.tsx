// The one dialog (ADR-0056): scrim, title bar, body, and a button row. It closes on Escape,
// on the close button and on a click on the scrim, and keeps Tab inside itself. A dialog
// supplies its body and its buttons and owns nothing else.

import { onCleanup, onMount, type JSX } from "solid-js";
import { Icon } from "./parts";

const FOCUSABLE = "input, textarea, select, button:not([disabled]), [tabindex]:not([tabindex='-1'])";

export function Dialog(props: {
  title: string; width?: number; onClose(): void; buttons: JSX.Element; children: JSX.Element;
}) {
  let box!: HTMLDivElement;
  // Closing must not leave focus on the page body, where the window's keys are not heard.
  const before = document.activeElement;
  onCleanup(() => (before instanceof HTMLElement && before.isConnected ? before : document.querySelector<HTMLElement>(".win"))?.focus());
  onMount(() => {
    // The first field, or failing that the first button after the close button.
    const first = box.querySelector<HTMLElement>(".db input, .db textarea") ?? box.querySelector<HTMLElement>(".df .btn");
    first?.focus();
  });
  const onKeyDown = (e: KeyboardEvent) => {
    e.stopPropagation();   // the window's own Escape handling must not also run
    if (e.key === "Escape") { props.onClose(); return; }
    if (e.key !== "Tab") return;
    const items = [...box.querySelectorAll<HTMLElement>(FOCUSABLE)];
    const [head, tail] = [items[0], items[items.length - 1]];
    if (e.shiftKey && document.activeElement === head) { e.preventDefault(); tail.focus(); }
    else if (!e.shiftKey && document.activeElement === tail) { e.preventDefault(); head.focus(); }
  };
  return (
    <div class="scrim" data-keep="" onMouseDown={(e) => { if (e.target === e.currentTarget) props.onClose(); }}>
      <div class="pop dialog" ref={box} role="dialog" aria-modal="true" aria-label={props.title}
        style={{ width: `calc(var(--u) * ${props.width ?? 170})` }} onKeyDown={onKeyDown}>
        <div class="dh">
          <span class="grow">{props.title}</span>
          <span class="x" title="Close" onClick={() => props.onClose()}><Icon name="close" /></span>
        </div>
        <div class="db">{props.children}</div>
        <div class="df">{props.buttons}</div>
      </div>
    </div>
  );
}
