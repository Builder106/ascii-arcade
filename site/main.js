/*
 * Attract mode. The page runs its own loop of stations, the way an arcade
 * cabinet nobody is playing does, and the way the Mac app cycles scenes when
 * you walk away. The first key, click or tap takes over. Leave it alone and
 * the loop comes back; touch anything and it snaps back to what you chose.
 *
 * One requestAnimationFrame loop drives everything: the station clock, the
 * idle check, and the canvas.
 */

import { loadEngine, SceneSource } from "./engine.js";
import { Grid } from "./renderer.js";
import { loadRecording, loadDoom } from "./doom.js";
import { BootLog } from "./boot.js";

const IDLE_SECONDS = 10; // the app waits 90
const SCENE_SECONDS = 4; // the app gives each scene 20
const DISSOLVE = 0.9; // the app's own scene dissolve
const SNAP = 0.25;
const STILL = 6; // reduced motion paints every scene as it looks at 6 s
const BOOT_CAP = 2.4; // the power-on log never holds the page longer than this
const BOOT_HOLD = 0.35; // a beat on the finished log before the scene comes in
const SCENES = ["donut", "helix", "matrix", "life", "pipes"];
const STATIONS = [
  { id: "title", seconds: 9, scene: "donut" },
  { id: "demo", seconds: 7, scene: "doom" },
  { id: "scenes", seconds: SCENES.length * SCENE_SECONDS, scene: null },
  { id: "rules", seconds: 14, scene: "life" },
  { id: "install", seconds: 14, scene: "matrix" },
];

const root = document.documentElement;
const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
const coarse = matchMedia("(pointer: coarse)").matches;
const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

const stationEls = STATIONS.map((s) => document.getElementById(s.id));
const links = $$(".strip [data-station]");
const statusEl = $("#status");
const cycleButton = $("#cycle");
const playButton = $("#play-doom");
const doomStatus = $("#doom-status");

const wall = new Grid($("#wall"));
let grid = wall; // the grid the sources are sized for: the wall, or the cabinet's screen
let screenGrid = null;
let cabinet = null;
let view = "flat";
const sources = new Map();
let wasm = null;
let recording = null;
let doom = null;
let current = null;
let previous = null;
let fadeStart = 0;
let fadeLength = 0;
let ground = "#000";
let ink = "#fff";
let inkRgb = [255, 255, 255];
let now = performance.now() / 1000;
let last = now;
let lastPaint = 0;
let dirty = true;

const state = {
  mode: still ? "player" : "attract", // attract | player | doom
  pressStart: true, // only a key, click or tap ends this loop, not a pointer passing by
  station: 0,
  elapsed: 0,
  scene: null,
  chosen: { station: 0, scene: "donut" },
  autoCycle: !still,
  lastInput: now,
};

/* Frame sources and the wall */

function source(id) {
  if (sources.has(id)) return sources.get(id);
  let s = null;
  if (SCENES.includes(id) && wasm) s = new SceneSource(wasm, id);
  else if (id === "doom") s = recording;
  else if (id === "doom-live") s = doom;
  else if (id === "boot") s = bootLog;
  if (!s) return null;
  s.size(grid.cols, grid.rows, grid.cell);
  s.tint(inkRgb);
  if (still && s instanceof SceneSource) for (let t = 0; t < STILL; t += 1 / 15) s.frame(t);
  sources.set(id, s);
  return s;
}

function show(id, length = DISSOLVE) {
  if (id === state.scene) return;
  const next = source(id);
  if (!next) return;
  state.scene = id;
  previous = current;
  current = next;
  fadeStart = now;
  fadeLength = still ? 0 : length;
  root.dataset.scene = id;
  for (const b of $$("button[data-scene]")) b.setAttribute("aria-pressed", String(b.dataset.scene === id));
  dirty = true;
}

function paint() {
  dirty = false;
  if (!current) return grid.paint(null, null, 0, ground, ink);
  const t = still ? STILL : now;
  const p = fadeLength ? (now - fadeStart) / fadeLength : 1;
  if (p >= 1 || !previous) {
    previous = null;
    grid.paint(current.frame(t), null, 0, ground, ink);
  } else {
    dirty = true;
    grid.paint(previous.frame(t), current.frame(t), p, ground, ink);
  }
  if (grid !== wall) {
    drawScreenText(grid.ctx);
    cabinet.screenChanged();
  }
}

/* Messages on the cabinet's screen, the way an arcade machine talks:
   PRESS START during the demo, PLAYER 1 when you take over, and a
   CONTINUE? countdown before the loop comes back. */
const SCREEN_FONT = '"Pixelify Sans", "IBM Plex Mono", monospace';
let playerFlash = 0;

function drawScreenText(ctx) {
  let text = null;
  let size = 34;
  let y = 0.86;
  if (now < playerFlash) [text, size, y] = ["PLAYER 1", 72, 0.5];
  else if (state.mode === "attract") [text, size] = [now % 1.2 < 0.8 ? "PRESS START" : null, 44];
  else if (state.mode === "player" && state.autoCycle) {
    const left = IDLE_SECONDS - (now - state.lastInput);
    if (left <= 5) [text, size, y] = [`CONTINUE? ${Math.max(0, Math.ceil(left))}`, 64, 0.5];
  }
  if (!text) return;
  const { width, height } = ctx.canvas;
  ctx.font = `700 ${size}px ${SCREEN_FONT}`;
  const w = ctx.measureText(text).width + size;
  ctx.fillStyle = ground;
  ctx.fillRect((width - w) / 2, height * y - size * 0.8, w, size * 1.6);
  ctx.fillStyle = ink;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, width / 2, height * y);
  ctx.textAlign = "start";
  ctx.textBaseline = "alphabetic";
}

function useGrid(g) {
  grid = g;
  for (const s of sources.values()) s.size(grid.cols, grid.rows, grid.cell);
  dirty = true;
}

/* The cabinet is the stage. Each station has its own camera pose on the
   machine, and the loop and the snap-back move the camera between them. Only
   playing DOOM goes through the glass, into the flat page, because playing
   needs the whole window. */

let pose = "glass"; // the camera starts at the glass and pulls out
let lampsLit = false; // the cabinet's lamps flicker on with its first reveal
let moveSeconds = 1.6;

function syncView() {
  if (!cabinet || booting) return;
  const want = state.mode === "doom" ? "glass" : STATIONS[state.station].id;
  const seconds = moveSeconds;
  moveSeconds = 1.6;
  if (want === pose) return;
  const leaving = pose;
  pose = want;
  root.dataset.pose = want;
  if (want === "glass") {
    cabinet.frame("glass", 0.9, () => {
      if (pose !== "glass") return;
      view = "flat";
      root.dataset.view = "flat";
      useGrid(wall);
      updateMask();
      refreshHover();
    });
  } else {
    if (view !== "cabinet") {
      view = "cabinet";
      root.dataset.view = "cabinet";
      useGrid(screenGrid);
    }
    cabinet.frame(want, leaving === "glass" ? 2.2 : seconds, refreshHover);
    if (leaving === "glass" && !lampsLit) {
      lampsLit = true;
      cabinet.powerOn(1.4);
    }
  }
}

async function loadCabinet() {
  try {
    if (!document.createElement("canvas").getContext("webgl2")) return false;
    const { mountCabinet } = await import("./cabinet.js");
    const screenCanvas = document.createElement("canvas");
    screenGrid = new Grid(screenCanvas);
    screenGrid.resize(800, 600, 17, 1);
    cabinet = await mountCabinet(screenCanvas, { ground, ink });
    syncView();
    return true;
  } catch (err) {
    console.info("cabinet unavailable; the page stays flat", err);
    cabinet = null;
    return false;
  }
}

/* Power-on. The head script adds .booting on every load unless motion is
   reduced or the link goes straight to a station. The CRT turn-on is CSS and runs by itself; the
   boot log types the real loading steps into the grid; the loop and the
   cabinet wait until it finishes, hits its cap, or any input skips it. */

let booting = root.classList.contains("booting");
let bootLog = null;
let bootDoneAt = null;

function checkBoot() {
  if (!booting) return;
  const done = bootLog.finished(now);
  if (done) bootDoneAt ??= now;
  if ((done && now - bootDoneAt >= BOOT_HOLD) || now - bootLog.t0 >= BOOT_CAP) finishBoot(false);
}

function finishBoot(skipped) {
  if (!booting) return;
  booting = false;
  root.classList.remove("booting");
  root.classList.add("entered");
  setTimeout(() => root.classList.remove("entered"), 1200);
  $("#power")?.remove();
  const scene = state.mode === "attract" ? STATIONS[state.station].scene ?? SCENES[0] : state.chosen.scene;
  show(scene, skipped ? SNAP : DISSOLVE);
  if (state.scene === "boot") {
    // No engine to dissolve into: clear the log rather than leave it up.
    current = null;
    state.scene = null;
    dirty = true;
  }
  state.elapsed = 0;
  state.lastInput = now;
  render();
}

function fit() {
  const fontPx = Math.max(10, Math.min(15, innerWidth / 110));
  wall.resize(innerWidth, innerHeight, fontPx);
  if (grid === wall) useGrid(wall);
  updateMask();
}

// Copy sits in a hole cut out of the grid, snapped to whole cells.
function updateMask() {
  const pad = (r, d) => ({ left: r.left - d, right: r.right + d, top: r.top - d, bottom: r.bottom + d, width: r.width, height: r.height });
  // While powering on, only the headline is on screen, so only it gets a hole.
  if (booting) wall.setMask([pad($("#title h1").getBoundingClientRect(), 12)]);
  else {
    wall.setMask([
      stationEls[state.station].getBoundingClientRect(),
      $(".strip").getBoundingClientRect(),
      ...$$(".top a").map((a) => pad(a.getBoundingClientRect(), 6)),
    ]);
  }
  dirty = true;
}

function readTheme() {
  const css = getComputedStyle(root);
  ground = css.getPropertyValue("--ground").trim();
  ink = css.getPropertyValue("--ink").trim();
  const n = parseInt(ink.slice(1), 16);
  inkRgb = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  for (const s of sources.values()) s.tint(inkRgb);
  cabinet?.setTheme({ ground, ink });
  for (const b of $$("[data-theme-name]")) b.setAttribute("aria-pressed", String(b.dataset.themeName === root.dataset.theme));
  $('meta[name="theme-color"]').content = ground;
  dirty = true;
}

function setTheme(name) {
  root.dataset.theme = name;
  try {
    localStorage.setItem("aa-theme", name);
  } catch {}
  readTheme();
}

/* The loop */

function tick(ms) {
  requestAnimationFrame(tick);
  now = ms / 1000;
  const elapsed = now - last;
  const dt = Math.min(elapsed, 0.1);
  last = now;
  checkBoot();
  if (state.mode === "attract" && !booting) advance(dt);
  else if (state.mode === "player" && state.autoCycle) {
    const idle = now - state.lastInput;
    if (idle >= IDLE_SECONDS) resume();
    else setStatus(idle > IDLE_SECONDS - 5 && !cabinet ? `Loop in ${Math.ceil(IDLE_SECONDS - idle)}.` : statusText());
  }
  // The camera runs on real time, so a slow device still finishes a move on
  // schedule instead of stretching it out.
  if (cabinet) root.style.setProperty("--glass", cabinet.update(Math.min(elapsed, 0.5)).toFixed(3));
  if (still ? !dirty : now - lastPaint < 1 / 30) return;
  lastPaint = now;
  paint();
}

function advance(dt) {
  state.elapsed += dt;
  const st = STATIONS[state.station];
  if (st.id === "scenes") show(SCENES[Math.min(SCENES.length - 1, Math.floor(state.elapsed / SCENE_SECONDS))]);
  if (state.elapsed >= st.seconds) goTo((state.station + 1) % STATIONS.length);
  else links[state.station].querySelector(".bar").style.scale = `${state.elapsed / st.seconds} 1`;
}

function goTo(i) {
  state.station = i;
  state.elapsed = 0;
  show(STATIONS[i].scene ?? SCENES[0]);
  render();
}

/* Modes */

function takeOver() {
  state.mode = "player";
  playerFlash = now + 1.2;
  cabinet?.press(1);
  state.chosen = { station: state.station, scene: state.scene ?? SCENES[0] };
  // The loop never touches the address, so record where the visitor took
  // over, and Back returns here. A plain address stays plain, which keeps
  // the power-on on reload.
  const id = STATIONS[state.station].id;
  if (location.hash) history.replaceState({ station: id }, "", `#${id}`);
  else history.replaceState({ station: id }, "");
  render();
}

function snapBack() {
  state.mode = "player";
  moveSeconds = 0.7;
  state.station = state.chosen.station;
  show(state.chosen.scene, SNAP);
  render();
}

// Like the app: the moment idle runs out, move on to the next thing.
function resume() {
  state.mode = "attract";
  state.pressStart = false;
  goTo((state.station + 1) % STATIONS.length);
}

// Always the visitor's doing (a link, or the back button), so it counts as input.
function navigate(i) {
  state.lastInput = now;
  if (state.mode === "doom") releaseDoom();
  state.mode = "player";
  state.station = i;
  const scene = STATIONS[i].scene ?? (SCENES.includes(state.scene) ? state.scene : SCENES[0]);
  show(scene);
  state.chosen = { station: i, scene };
  render();
}

function statusText() {
  if (cabinet && state.mode === "player") return coarse ? "Swipe, or tap part of the cabinet." : "Drag the joystick, or press ← →. Or click the cabinet.";
  if (state.mode === "doom") return "DOOM has the keyboard. Esc gives it back.";
  if (state.mode === "attract") {
    if (state.pressStart) return coarse ? "Demo running. Tap to play." : "Demo running. Press any key or click to play.";
    return coarse ? "You left, so the loop came back. Tap to return." : "You left, so the loop came back. Move the mouse to return.";
  }
  if (still && !state.autoCycle) return "Reduced motion: the loop starts off.";
  return state.autoCycle ? `Your turn. Idle ${IDLE_SECONDS} s and the loop returns.` : "Your turn. The loop is off.";
}

function setStatus(text) {
  if (statusEl.textContent !== text) statusEl.textContent = text;
}

function render() {
  root.dataset.mode = state.mode;
  root.dataset.station = STATIONS[state.station].id;
  stationEls.forEach((el, i) => el.classList.toggle("is-current", i === state.station));
  links.forEach((a, i) => {
    a.querySelector(".bar").style.scale = "";
    if (i === state.station) a.setAttribute("aria-current", "location");
    else a.removeAttribute("aria-current");
  });
  cycleButton.setAttribute("aria-pressed", String(state.autoCycle));
  setStatus(statusText());
  updateMask();
  syncView();
}

/* Input */

function onInput() {
  state.lastInput = now;
  if (booting) return finishBoot(true);
  if (state.mode !== "attract") return;
  if (state.pressStart) takeOver();
  else snapBack();
}

addEventListener("keydown", onInput, true);
addEventListener("pointerdown", onInput, true);
addEventListener("wheel", onInput, { capture: true, passive: true });
addEventListener("pointermove", (e) => {
  if (e.pointerType !== "mouse") return;
  state.lastInput = now;
  if (state.mode === "attract" && !state.pressStart) snapBack();
});

document.addEventListener("click", (e) => {
  const a = e.target.closest('a[href^="#"]');
  if (!a || a.classList.contains("skip")) return;
  const id = a.getAttribute("href").slice(1);
  const i = STATIONS.findIndex((s) => s.id === id);
  if (i < 0) return;
  e.preventDefault();
  if (location.hash !== `#${id}`) history.pushState(null, "", `#${id}`);
  navigate(i);
  stationEls[i].focus({ preventScroll: true });
});

addEventListener("popstate", route);

/* The cabinet's controls. ← → (or A and D) work the joystick, and so does
   dragging the stick itself with a mouse; a swipe does the same on a phone.
   Clicking a part of the cabinet goes to its station, and a coin in the slot
   starts the download. The strip stays as the plain
   route through the same stations. */

function step(dir) {
  cabinet?.tilt(dir);
  const i = (state.station + dir + STATIONS.length) % STATIONS.length;
  history.pushState(null, "", `#${STATIONS[i].id}`);
  navigate(i);
  focusStation(i);
}

// Focus moves to the new station when it would otherwise be lost inside the
// one just hidden, so keyboard and screen reader users land on what changed.
function focusStation(i) {
  const a = document.activeElement;
  if (a && a !== document.body && !a.closest(".station")) return;
  stationEls[i].focus({ preventScroll: true });
}

const STEPS = { ArrowRight: 1, KeyD: 1, ArrowLeft: -1, KeyA: -1 };
addEventListener("keydown", (e) => {
  const dir = STEPS[e.code];
  if (!dir || state.mode === "doom" || e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
  // A focused command line scrolls with the arrows instead.
  if (e.target.closest?.("input, textarea, [contenteditable], .cmd code")) return;
  e.preventDefault();
  // Holding the key steps once, like pushing the stick and holding it.
  if (!e.repeat) step(dir);
});

let swipe = null;
addEventListener(
  "touchstart",
  (e) => {
    const t = e.touches[0];
    swipe = e.target.closest?.(".cmd, .strip") ? null : { x: t.clientX, y: t.clientY };
  },
  { passive: true },
);
addEventListener(
  "touchend",
  (e) => {
    if (!swipe || state.mode === "doom") return;
    const t = e.changedTouches[0];
    const dx = t.clientX - swipe.x;
    const dy = t.clientY - swipe.y;
    swipe = null;
    if (Math.abs(dx) > 60 && Math.abs(dx) > 1.5 * Math.abs(dy)) step(dx < 0 ? 1 : -1);
  },
  { passive: true },
);

const onPage = (e) => e.target.closest?.("a, button, code, .station, .strip, .top");

// The stick: hovering leans it toward the pointer, pressing grabs it, and
// pushing it past its notch steps one station, like an arcade menu. It has
// to come back toward centre before the next push counts. The push is
// measured in pointer travel, not the stick's angle on screen, so it takes
// the same deliberate move however small the stick is drawn.
const TRAVEL = 60; // pixels of travel to the stick's stop
const NOTCH = 36; // pixels of travel that step a station
let grip = null; // { x0, armed } while the stick is held
let lastPointer = null; // where the mouse last was, so hover can catch up when the camera moves
let swallowClick = false; // the click that ends a drag isn't a click on the cabinet

// Works out what's under the mouse. Runs on every move, and again when the
// camera lands somewhere new or the view goes flat, so the cursor and the
// glow never describe a part that's moved away.
function refreshHover() {
  if (!cabinet || grip) return;
  const tag = view === "cabinet" && lastPointer && !lastPointer.onPage ? cabinet.pick(lastPointer.x, lastPointer.y) : null;
  cabinet.hover(tag);
  if (tag === "stick") cabinet.leanStick(lastPointer.x);
  else cabinet.releaseStick();
  document.body.style.cursor = tag === "stick" ? "grab" : tag ? "pointer" : "";
}

addEventListener("pointerdown", (e) => {
  if (!cabinet || view !== "cabinet" || e.pointerType === "touch" || e.button !== 0 || onPage(e)) return;
  if (cabinet.pick(e.clientX, e.clientY) !== "stick") return;
  e.preventDefault();
  grip = { x0: e.clientX, armed: true };
  cabinet.holdStick(0);
  document.body.style.cursor = "grabbing";
});

addEventListener("pointermove", (e) => {
  if (e.pointerType === "touch") return;
  lastPointer = { x: e.clientX, y: e.clientY, onPage: Boolean(onPage(e)) };
  if (!cabinet) return;
  if (!grip) return refreshHover();
  const travel = e.clientX - grip.x0;
  cabinet.holdStick((Math.max(-TRAVEL, Math.min(TRAVEL, travel)) / TRAVEL) * cabinet.stickStop);
  if (grip.armed && Math.abs(travel) >= NOTCH) {
    grip.armed = false;
    step(Math.sign(travel));
  } else if (Math.abs(travel) < NOTCH / 2) grip.armed = true;
});

// The mouse left the page.
document.addEventListener("pointerout", (e) => {
  if (e.relatedTarget || e.pointerType === "touch") return;
  lastPointer = null;
  refreshHover();
});

function letGo() {
  if (!grip) return;
  grip = null;
  swallowClick = true;
  setTimeout(() => (swallowClick = false));
  cabinet?.releaseStick();
  refreshHover();
}
addEventListener("pointerup", letGo);
addEventListener("pointercancel", letGo);
addEventListener("blur", letGo);

addEventListener("click", (e) => {
  if (!cabinet || view !== "cabinet" || onPage(e) || swallowClick) return;
  const tag = cabinet.pick(e.clientX, e.clientY);
  if (tag === "coin") return cabinet.insertCoin(() => $(".strip .cta").click());
  const i = STATIONS.findIndex((s) => s.id === tag);
  if (i < 0 || i === state.station) return;
  history.pushState(null, "", `#${tag}`);
  navigate(i);
  focusStation(i);
});

// The station the address names: its fragment, or for a plain address, the
// station recorded when the visitor took over there.
function stationFromUrl() {
  const id = location.hash ? location.hash.slice(1) : history.state?.station;
  return STATIONS.findIndex((s) => s.id === id);
}

function route() {
  const i = stationFromUrl();
  // A fragment that isn't a station, like the skip link's, leaves the
  // station where it is.
  if (i < 0 && location.hash) return;
  navigate(Math.max(i, 0));
}

for (const b of $$("button[data-scene]")) {
  b.addEventListener("click", () => {
    if (state.mode === "doom") releaseDoom();
    state.mode = "player";
    show(b.dataset.scene);
    cabinet?.press(0);
    state.chosen = { station: state.station, scene: b.dataset.scene };
    render();
  });
}

for (const b of $$("[data-theme-name]")) b.addEventListener("click", () => setTheme(b.dataset.themeName));

for (const b of $$("[data-copy]")) {
  b.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(document.getElementById(b.dataset.copy).textContent);
      b.textContent = "Copied";
    } catch {
      b.textContent = "Select it";
    }
    setTimeout(() => (b.textContent = "Copy"), 1600);
  });
}

// Mirrors the app's menu item. Off is also this page's pause control.
cycleButton.addEventListener("click", () => {
  state.autoCycle = !state.autoCycle;
  if (state.mode === "attract") takeOver();
  render();
});

/* DOOM */

playButton.addEventListener("click", async () => {
  if (!doom) {
    playButton.disabled = true;
    doomStatus.textContent = "Loading 28 MB.";
    try {
      doom = await loadDoom();
    } catch (err) {
      console.info("doom-wasm unavailable", err);
      doomStatus.textContent = "DOOM didn’t load here. It runs in the app.";
      playButton.disabled = false;
      return;
    }
    playButton.disabled = false;
    // The visitor may have moved on while it downloaded. Don't pull them back.
    if (state.station !== 1 || state.mode === "attract") {
      doomStatus.textContent = "DOOM is ready. Play to take the keys.";
      return;
    }
  }
  state.mode = "doom";
  show("doom-live", SNAP);
  state.chosen = { station: 1, scene: "doom-live" };
  doomStatus.textContent = "Arrows or WASD move, Ctrl fires, Space opens, Esc lets go.";
  playButton.blur();
  render();
});

function releaseDoom() {
  state.mode = "player";
  doomStatus.textContent = "DOOM keeps running. Play again to take the keys.";
  render();
}

addEventListener("keydown", (e) => {
  if (state.mode !== "doom") return;
  if (e.code === "Escape") return releaseDoom();
  if (doom.press(e.code, true)) e.preventDefault();
});
addEventListener("keyup", (e) => {
  if (state.mode === "doom" && doom.press(e.code, false)) e.preventDefault();
});

/* Start */

for (const el of stationEls) el.tabIndex = -1;
addEventListener("resize", fit);
new ResizeObserver(updateMask).observe($("#stations"));
readTheme();

await document.fonts.load('400 13px "IBM Plex Mono"').catch(() => {});
fit();
// Only the fragment counts on load: a recorded takeover on a plain address
// is for Back, and a reload starts the loop afresh.
if (location.hash && stationFromUrl() >= 0) route();
else render();
let bootEngine = null;
let bootScenes = null;
let bootCabinet = null;
if (booting) {
  bootLog = new BootLog(now + 0.25); // starts typing as the CRT opens
  show("boot", 0);
  bootEngine = bootLog.step("aa-core");
  bootScenes = bootLog.step("scenes");
  bootCabinet = bootLog.step("cabinet");
}
requestAnimationFrame(tick);

wasm = await loadEngine();
bootEngine?.(wasm ? "ok" : "off");
bootScenes?.(wasm ? String(wasm.scene_ids().length) : "off");
if (!booting) {
  show(state.mode === "attract" ? STATIONS[state.station].scene ?? SCENES[0] : state.chosen.scene, 0);
  if (!state.scene) show(SCENES[0], 0);
}
(still ? Promise.resolve(false) : loadCabinet()).then((ok) => bootCabinet?.(ok ? "ok" : "flat"));

loadRecording("/assets/doom-attract.json")
  .then((s) => {
    recording = s;
    const wanted = state.mode === "attract" ? STATIONS[state.station].scene : state.chosen.scene;
    if (wanted === "doom") show("doom");
  })
  .catch((err) => console.info("DOOM recording unavailable", err));
