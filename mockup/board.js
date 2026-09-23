// The review board (ADR-0036). A board page calls `board({ title, intro, columns })`
// and nothing else: this owns the canvas, the columns, the frame labels and the three
// board-wide controls (theme, root font, zoom).
//
// A frame is { id, label, note, live?, kind: "screen" | "component", render(el) }.
// render() fills the element and is called again by refreshBoard(), so a board-wide
// change such as the sharp/flat switch redraws every frame from its own state.

const BOARD_FRAMES = [];

function refreshBoard() {
  for (const f of BOARD_FRAMES) f.render(f.el);
}

const boardPrefs = (() => {
  // Per-viewer convenience only; the board works without it.
  const read = () => { try { return JSON.parse(localStorage.getItem("seula-board") || "{}"); } catch { return {}; } };
  const write = (v) => { try { localStorage.setItem("seula-board", JSON.stringify(v)); } catch { /* ignore */ } };
  const prefs = { theme: "dark", pt: 9, zoom: 1, ...read() };
  return { get: (k) => prefs[k], set: (k, v) => { prefs[k] = v; write(prefs); } };
})();

function board({ title, intro, columns }) {
  const seg = (name, options, current) => `<span class="seg" data-seg="${name}">${
    options.map(([v, label]) => `<button data-v="${v}" class="${String(v) === String(current) ? "on" : ""}">${label}</button>`).join("")
  }</span>`;

  document.body.innerHTML = `
    <div class="board-bar">
      <h1>${title}</h1>
      <a href="shell.html">shell</a><a href="projects.html">projects</a><a href="plugins.html">plugins</a><a href="samples.html">samples</a>
      <span class="grow"></span>
      <span class="ctl">Theme ${seg("theme", [["dark", "Dark"], ["light", "Light"]], boardPrefs.get("theme"))}</span>
      <span class="ctl">Root font ${seg("pt", [[8, "8pt"], [9, "9pt"], [10, "10pt"], [11, "11pt"]], boardPrefs.get("pt"))}</span>
      <span class="ctl">Zoom ${seg("zoom", [[0.5, "50%"], [0.75, "75%"], [1, "100%"]], boardPrefs.get("zoom"))}</span>
    </div>
    ${intro ? `<div class="board-intro">${intro}</div>` : ""}
    <div class="canvas"></div>`;

  const canvas = document.querySelector(".canvas");
  for (const col of columns) {
    const colEl = document.createElement("section");
    colEl.className = "board-col";
    colEl.innerHTML = `<h2>${col.title}</h2>`;
    for (const f of col.frames) {
      const wrap = document.createElement("div");
      wrap.className = "frame-wrap";
      wrap.innerHTML = `<div class="frame-label">${f.id} · ${f.label}${f.note ? ` <span class="note">— ${f.note}</span>` : ""}${f.live ? `<span class="live">live</span>` : ""}</div>`;
      const el = document.createElement("div");
      el.className = `frame ${f.kind || "component"}`;
      wrap.appendChild(el);
      colEl.appendChild(wrap);
      f.el = el;
      BOARD_FRAMES.push(f);
    }
    canvas.appendChild(colEl);
  }

  const apply = () => {
    document.documentElement.dataset.theme = boardPrefs.get("theme");
    document.documentElement.style.fontSize = `${boardPrefs.get("pt")}pt`;
    canvas.style.zoom = boardPrefs.get("zoom");
  };
  document.querySelector(".board-bar").addEventListener("click", (e) => {
    const b = e.target.closest("[data-seg] button");
    if (!b) return;
    const name = b.parentElement.dataset.seg;
    boardPrefs.set(name, name === "theme" ? b.dataset.v : Number(b.dataset.v));
    b.parentElement.querySelectorAll("button").forEach((x) => x.classList.toggle("on", x === b));
    apply();
    // Popovers are placed from measured geometry, so a size change needs a redraw.
    if (name !== "theme") refreshBoard();
  });
  apply();
  refreshBoard();
}
