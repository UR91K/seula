import { Show, onMount } from "solid-js";
import { fmtBytes } from "../../../shared/format";
import { plural } from "../../../shared/projects";
import { Icon } from "../shell/parts";
import { counts, lastScan, scan, shell } from "../shell/shell";
import type { View } from "../shell/view";
import { Inspector } from "./Inspector";
import { PresentDot } from "./parts";
import { ContextMenu, FormatMenu, PresenceMenu } from "./Popovers";
import { SampleTable } from "./Table";
import { CheckButton, Viewbar } from "./Viewbar";
import { clearFilters, filtered, load, loadError, loaded, samples, setQuery, stats, sui, total, formats } from "./state";

function Empty() {
  return (
    <Show when={!loadError()}>
      <Show when={loaded()}>
        <Show when={samples.length === 0 && !sui.query} fallback={
          <div class="empty">
            <Icon name="search_off" /><h2>No samples match</h2>
            <p>{[sui.query && `“${sui.query}”`, formats().find((f) => f.format === sui.format)?.name,
              sui.presence].filter(Boolean).join(" · ")}</p>
            <button class="btn" onClick={() => { clearFilters(); setQuery(""); }}>Clear filters</button>
          </div>}>
          <div class="empty">
            <Icon name="earthquake" /><h2>No samples yet</h2>
            <p>Samples are listed as the projects that use them are scanned.</p>
            <CheckButton />
          </div>
        </Show>
      </Show>
    </Show>
  );
}

function Content() {
  onMount(load);
  return (
    <>
      <Show when={loadError()}><div class="error-banner">Cannot reach the daemon: {loadError()}</div></Show>
      <Show when={total() > 0} fallback={<Empty />}><SampleTable /></Show>
    </>
  );
}

/** Until a check has measured the files, the size says so rather than showing zero. A size
 *  over fewer samples than are present says how many it covers. */
function Size() {
  return (
    <span>
      <Icon name="hard_drive" class="sb" />{" "}
      <Show when={stats().measured > 0 || stats().present === 0}
        fallback={<span class="faint">size not measured yet</span>}>
        {fmtBytes(stats().sizeBytes)}
        <Show when={stats().measured < stats().present}>
          {" "}<span class="faint">({stats().measured} of {stats().present} measured)</span>
        </Show>
      </Show>
    </span>
  );
}

function Status() {
  return (
    <Show when={loaded() && !(samples.length === 0 && !sui.query)} fallback={<span>No samples</span>}>
      <span>{filtered() ? `${total()} of ${counts.samples ?? samples.length} samples` : plural(total(), "sample")}</span>
      <span><PresentDot present={true} /> {stats().present} present</span>
      <span><PresentDot present={false} /> {stats().missing} missing</span>
      <Size />
      <Show when={!scan() && lastScan()?.kind === "samples" ? lastScan() : undefined}>{(l) => (
        <span class={l().event.status === "completed" ? "is-installed" : "is-absent"}>
          <Icon name={l().event.status === "completed" ? "task_alt" : "error"} /> {l().event.message}
        </span>
      )}</Show>
    </Show>
  );
}

function Popovers() {
  return (
    <>
      <Show when={shell.menu} keyed>{(m) => <ContextMenu menu={m} />}</Show>
      <Show when={shell.popover === "format"}><FormatMenu /></Show>
      <Show when={shell.popover === "presence"}><PresenceMenu /></Show>
    </>
  );
}

export const samplesView: View = {
  Viewbar, Content, Inspector, Status, Popovers, keys: false,
  query: () => sui.query,
  setQuery,
};
