<script lang="ts">
  import { LOGO_PATH, LOGO_TRANSFORM, LOGO_VIEWBOX } from "../../shared/logo";
  import Icon from "./Icon.svelte";
  import TbBtn from "./TbBtn.svelte";
  import { setQuery, ui } from "./state.svelte";

  let timer: ReturnType<typeof setTimeout> | undefined;
  // Search waits for typing to pause, then asks the server (FTS5, ADR-0024).
  function onInput(value: string) {
    clearTimeout(timer);
    timer = setTimeout(() => setQuery(value.trim()), 250);
  }
</script>

<header class="topbar">
  <svg class="logo" viewBox={LOGO_VIEWBOX} role="img" aria-label="Seula">
    <path fill="currentColor" transform={LOGO_TRANSFORM} d={LOGO_PATH} />
  </svg>
  <nav class="menubar"><span>File</span><span>Edit</span><span>View</span><span>Tools</span><span>Help</span></nav>
  <label class="search">
    <Icon name="search" />
    <input placeholder="Search projects, plugins, samples, tags" value={ui.query}
      oninput={(e) => onInput(e.currentTarget.value)} />
  </label>
  <TbBtn icon="settings" title="Settings" />
  <span style="width: calc(var(--u) * 2)"></span>
</header>
