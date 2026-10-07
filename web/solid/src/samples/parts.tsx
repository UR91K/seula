import { Icon } from "../shell/parts";

/** Present or missing: icon and colour, never the colour alone. */
export function PresentDot(props: { present: boolean }) {
  return (
    <span class="status-dot" classList={{ "is-ok": props.present, "is-missing": !props.present }}>
      <Icon name={props.present ? "check_circle" : "error"} fill />
    </span>
  );
}
