// Small shared pieces: icon, checkbox, toolbar button, path chip, tag chip.

import { Show } from "solid-js";
import { splitPath } from "../../shared/format";

export function Icon(props: { name: string; fill?: boolean; class?: string }) {
  return <span class={`ms ${props.fill ? "fill" : ""} ${props.class ?? ""}`}>{props.name}</span>;
}

export function Cb(props: { state: boolean | "mixed" }) {
  return (
    <span class="cb" classList={{ on: props.state === true, mixed: props.state === "mixed" }}>
      <Show when={props.state === "mixed"} fallback={<Show when={props.state}><Icon name="check" /></Show>}>
        <Icon name="remove" />
      </Show>
    </span>
  );
}

export function TbBtn(props: {
  icon?: string; label?: string; title?: string; disabled?: boolean; caret?: boolean; on?: boolean; keep?: boolean;
  onClick?: (e: MouseEvent) => void;
}) {
  return (
    <button class="tb-btn" classList={{ lbl: !!props.label, on: props.on }} title={props.title ?? props.label}
      disabled={props.disabled} data-keep={props.keep ? "" : undefined} onClick={(e) => props.onClick?.(e)}>
      <Show when={props.icon}><Icon name={props.icon!} /></Show>
      <Show when={props.label}><span>{props.label}</span></Show>
      <Show when={props.caret}><Icon name="expand_more" class="caret" /></Show>
    </button>
  );
}

export const TagChip = (props: { name: string }) => <span class="tag">{props.name}</span>;

/** A path truncated in the middle, so the drive and the file name both stay visible. */
export function PathChip(props: { path: string }) {
  const parts = () => splitPath(props.path);
  const copy = (e: MouseEvent) => {
    navigator.clipboard?.writeText(props.path).catch(() => {});
    const el = e.currentTarget as HTMLElement;
    el.classList.add("copied");
    setTimeout(() => el.classList.remove("copied"), 1200);
  };
  return (
    <span class="path" title={props.path} onClick={copy}>
      <span class="head">{parts().head}</span><span class="tail">{parts().tail}</span>
    </span>
  );
}
