// The projects view's state. The table's own state (rows, sort, page, selection, columns)
// is a `createTableState` (ADR-0058); what is left here is what only this view has: the
// scope and the search, and loading the list for them. The window's state and the scan's
// are in ../shell/shell.ts.

import { batch, createSignal, untrack } from "solid-js";
import { createStore } from "solid-js/store";
import { DEFAULT_SORT } from "../../../shared/projects";
import type { Scope } from "../../../shared/types";
import { api, closePopups, onProjectsSearch, setCounts } from "../shell/shell";
import { createTableState } from "./tablestate";

export const table = createTableState({ sort: DEFAULT_SORT, paged: true });

export const [ui, setUi] = createStore({
  scope: "active" as Scope,
  query: "",
});

// ---------------------------------------------------------------- data

export const [loadError, setLoadError] = createSignal<string | null>(null);

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const list = ui.query ? await api.search(ui.query) : await api.projects(ui.scope);
    if (token !== loadToken) return;   // a newer load has started
    batch(() => {
      table.replace(list);
      if (!ui.query && ui.scope === "active") setCounts("projects", list.length);
      setLoadError(null);
    });
  } catch (e) {
    if (token === loadToken) setLoadError(String(e));
  }
}

// ---------------------------------------------------------------- actions

function resetView() {
  batch(() => { table.reset(); closePopups(); });
}

export function setScope(scope: Scope) {
  batch(() => { setUi("scope", scope); resetView(); });
}

export function setQuery(query: string) {
  if (query === ui.query) return;
  batch(() => {
    setUi("query", query);
    table.sortBy(query ? null : DEFAULT_SORT);   // a search answers by relevance
    resetView();
  });
}

// Other views reach this through the shell (ADR-0055); `untrack` because they may call it
// from inside a tracked scope, and a search must not subscribe that scope to our query.
onProjectsSearch((query) => untrack(() => setQuery(query)));
