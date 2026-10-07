// The samples view's toolbar row: search chip, the two filters, the check button and the
// pager.

import { Show, type JSX } from "solid-js";
import { PRESENCE } from "../../../shared/samples";
import { plural } from "../../../shared/projects";
import { Icon, TbBtn } from "../shell/parts";
import { scan } from "../shell/shell";
import { FilterSelect, Pager } from "../shell/toolbar";
import {
  checkSamples, firstPage, formats, goPage, loaded, pageCount, samples, setFormat, setPresence, setQuery, sui, total,
} from "./state";

export function CheckButton(): JSX.Element {
  return (
    <TbBtn icon={scan() ? "hourglass_top" : "fact_check"} label={scan() ? "Checking…" : "Check samples"}
      title={scan() ? "A scan is running" : "Check each sample file is still there, and measure it"}
      disabled={!!scan()} onClick={checkSamples} />
  );
}

export function Viewbar() {
  const format = () => formats().find((f) => f.format === sui.format)?.name ?? sui.format;
  const presence = () => PRESENCE.find((p) => p.id === sui.presence)?.label ?? null;
  // A library no project scan has filled has nothing to filter or page yet.
  const empty = () => loaded() && samples.length === 0 && !sui.query;
  return (
    <>
      <h1>Samples</h1>
      <Show when={!empty()} fallback={<CheckButton />}>
        <Show when={sui.query}>
          <span class="chip">
            <Icon name="search" />“{sui.query}” · {plural(total(), "result")}
            <span onClick={() => setQuery("")}><Icon name="close" /></span>
          </span>
        </Show>
        <FilterSelect name="format" label="Format" value={format()} onClear={() => setFormat(null)} />
        <FilterSelect name="presence" label="Status" value={presence()} onClear={() => setPresence(null)} />
        <span class="sep" />
        <CheckButton />
        <span class="grow" />
        <Pager page={sui.page} total={total()} pages={pageCount()} onPage={goPage} onSize={firstPage} />
      </Show>
    </>
  );
}
