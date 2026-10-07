import { Icon } from "../shell/parts";

/** The tri-state install marker (ADR-0012): icon and colour, never the colour alone. */
export function StatusDot(props: { installed: boolean | null }) {
  return (
    <span class="status-dot" classList={{
      "is-ok": props.installed === true, "is-missing": props.installed === false, "is-unknown": props.installed == null,
    }}>
      <Icon name={props.installed === true ? "check_circle" : props.installed === false ? "error" : "help"} fill={props.installed != null} />
    </span>
  );
}
