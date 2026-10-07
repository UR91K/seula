import type { InstallState } from "../../../shared/plugins";
import { Icon } from "../shell/parts";

const ICONS: Record<InstallState, string> = { installed: "check_circle", absent: "error", failed: "warning", unscanned: "help" };
const CLASSES: Record<InstallState, string> = { installed: "is-ok", absent: "is-missing", failed: "is-failed", unscanned: "is-unknown" };

/** The status marker (ADR-0012, ADR-0067): icon and colour, never the colour alone. */
export function StatusDot(props: { state: InstallState }) {
  return (
    <span class={`status-dot ${CLASSES[props.state]}`}>
      <Icon name={ICONS[props.state]} fill={props.state !== "unscanned"} />
    </span>
  );
}
