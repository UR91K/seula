// What a view hands the shell (ADR-0052). The shell draws the frame; a view supplies the
// regions that change with it. Only the active view is mounted, so a view loads its data
// when its `Content` mounts.

import type { Component } from "solid-js";

export interface View {
  /** Toolbar row: title, filters, scan button, pager. */
  Viewbar: Component;
  /** The main area. Loads the view's data on mount. */
  Content: Component;
  /** The inspector column, or null for a view with none. */
  Inspector: Component | null;
  /** The view's segments of the status bar, before the scan and watcher. */
  Status: Component;
  /** Menus and pickers, placed against the window. */
  Popovers: Component;
  /** Whether the view shows keys, and so carries the sharp/flat switch. */
  keys: boolean;
  /** The top bar's search box reads and writes the active view's query. */
  query(): string;
  setQuery(query: string): void;
  onKey?(e: KeyboardEvent): void;
}
