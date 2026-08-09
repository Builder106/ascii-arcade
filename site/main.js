/*
 * Wiring only. The loop runs when the tab is visible and stops otherwise.
 */
import { Renderer } from "./renderer.js";
import { loadEngine, SceneDriver } from "./engine.js";
import { initMotion, updateScrollProgress } from "./motion.js";
import { initEnhancements } from "./enhance.js";
import { mountDoom } from "./doom.js";

const FONT_PX = 13;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Palette buttons are built from the engine's own Theme::ALL rather than a
 * hardcoded list, so the page cannot drift from the app's palettes.
 */
function buildPalette(driver, mount, extraDrivers = []) {
  if (!mount || driver.themes.length === 0) return;

  for (const theme of driver.themes) {
    const id = theme.name.toLowerCase();
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "swatch";
    btn.textContent = theme.name;
    btn.setAttribute("aria-pressed", String(id === document.documentElement.dataset.theme));
    btn.style.setProperty("--swatch-fg", `rgb(${theme.text.join(",")})`);
    btn.style.setProperty("--swatch-bg", `rgb(${theme.background.join(",")})`);

    btn.addEventListener("click", () => {
      document.documentElement.dataset.theme = id;
      driver.setTheme(theme.name);
      for (const gd of extraDrivers) gd.setTheme(theme.name);
      for (const other of mount.querySelectorAll("button")) {
        other.setAttribute("aria-pressed", String(other === btn));
      }
    });

    mount.append(btn);
  }
}

/**
 * Scene selection buttons for the hero section allowing interactive previewing
 * of the WASM scenes.
 */
const SCENES = [
  { id: "donut", label: "Donut" },
  { id: "helix", label: "Helix" },
  { id: "matrix", label: "Matrix" },
  { id: "pipes", label: "Pipes" },
  { id: "life", label: "Life" },
];

function buildScenePicker(driver, mount) {
  if (!mount || !driver.wasm || driver.themes.length === 0) return null;

  const label = document.createElement("span");
  label.className = "open__scenes-label";
  label.textContent = "Live Scene:";
  mount.append(label);

  const updateActive = (activeId) => {
    for (const btn of mount.querySelectorAll("button")) {
      btn.setAttribute("aria-pressed", String(btn.dataset.sceneId === activeId));
    }
  };

  for (const s of SCENES) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "scene-btn";
    btn.textContent = s.label;
    btn.dataset.sceneId = s.id;
    btn.setAttribute("aria-pressed", String(s.id === (driver.currentId || "donut")));

    btn.addEventListener("click", () => {
      driver.setScene(s.id);
    });

    mount.append(btn);
  }

  return updateActive;
}

/**
 * Five independent SceneDriver/Renderer pairs sharing the one loaded WASM
 * module, so every tile in the gallery is a genuinely live scene rather than
 * a screenshot or a shared canvas cropped five ways. Cheap to run: each tile
 * is a small grid (font size below), and the caller only ticks these while
 * the gallery section is actually on screen (see boot()'s galleryVisible
 * gate) — the same "don't draw what nobody sees" discipline the .fine copy
 * in #layer claims for the desktop app itself.
 */
const GALLERY_FONT_PX = 7;

/**
 * Renderer.resize() sets an explicit pixel style.width/style.height on the
 * canvas (correct for the full-viewport background, where CSS can't size it
 * any other way). For a grid tile that CSS *does* size (width:100% plus
 * aspect-ratio in .gallery__canvas), that same explicit style permanently
 * pins the box — including during the tiles' own construction loop, where
 * each canvas gets measured before its later siblings exist and the grid
 * hasn't reached its final column count yet. Clearing the inline size
 * afterwards hands sizing back to CSS, so a later re-measurement (fit())
 * reads the grid's real size instead of reading back its own earlier guess.
 */
function resizeGalleryCanvas(renderer, rect) {
  renderer.resize(rect.width, rect.height, GALLERY_FONT_PX);
  renderer.canvas.style.width = "";
  renderer.canvas.style.height = "";
}

function buildSceneGallery(wasm, mount, mainDriver) {
  if (!mount || !wasm) return { drivers: [], updateActive: null };

  const drivers = [];
  const cards = [];

  for (const s of SCENES) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "gallery__card";
    card.dataset.sceneId = s.id;
    card.setAttribute("aria-pressed", "false");
    card.setAttribute("aria-label", `Preview ${s.label} and set it as the background`);

    const canvas = document.createElement("canvas");
    canvas.className = "gallery__canvas";
    canvas.setAttribute("aria-hidden", "true");

    const cap = document.createElement("span");
    cap.className = "gallery__label";
    cap.textContent = s.label;

    card.append(canvas, cap);
    mount.append(card);
    cards.push(card);

    const renderer = new Renderer(canvas);
    resizeGalleryCanvas(renderer, canvas.getBoundingClientRect());

    const driver = new SceneDriver(renderer, wasm);
    const currentTheme = mainDriver.themes.find(
      (t) => t.name.toLowerCase() === document.documentElement.dataset.theme,
    );
    driver.setTheme(currentTheme?.name ?? mainDriver.themes[0]?.name);
    driver.setScene(s.id);
    drivers.push({ driver, renderer });

    card.addEventListener("click", () => mainDriver.setScene(s.id));
  }

  const updateActive = (activeId) => {
    for (const card of cards) {
      card.setAttribute("aria-pressed", String(card.dataset.sceneId === activeId));
    }
  };

  return { drivers, updateActive };
}

async function boot() {
  const canvas = document.getElementById("grid");
  if (!canvas) return;

  const renderer = new Renderer(canvas);
  const wasm = await loadEngine();
  const driver = new SceneDriver(renderer, wasm);

  driver.setTheme("Hacker");
  driver.setScene("donut");

  const { drivers: galleryDrivers, updateActive: galleryUpdateActive } = buildSceneGallery(
    wasm,
    document.getElementById("sceneGallery"),
    driver,
  );

  const fit = () => {
    renderer.resize(innerWidth, innerHeight, FONT_PX);
    driver.resize();
    for (const { renderer: gr, driver: gd } of galleryDrivers) {
      const rect = gr.canvas.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        resizeGalleryCanvas(gr, rect);
        gd.resize();
      }
    }
  };
  fit();
  addEventListener("resize", fit);

  buildPalette(
    driver,
    document.getElementById("palette"),
    galleryDrivers.map((g) => g.driver),
  );
  const heroUpdateActive = buildScenePicker(driver, document.getElementById("heroScenePicker"));
  driver.onSceneChange = (id) => {
    heroUpdateActive?.(id);
    galleryUpdateActive?.(id);
  };
  galleryUpdateActive?.(driver.currentId);

  // Sections declare which scene belongs to them. Half-visible wins, so the
  // scene changes once per section rather than fighting on boundaries.
  const sceneWatcher = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting && e.intersectionRatio > 0.5) {
          driver.setScene(e.target.dataset.scene);
        }
      }
    },
    { threshold: [0.5] },
  );
  for (const s of document.querySelectorAll("[data-scene]")) {
    sceneWatcher.observe(s);
  }

  // The gallery's five extra engines only cost anything while their section
  // is actually on screen — same "don't draw what nobody sees" rule the
  // desktop app itself follows (see the .fine copy in #layer).
  let galleryVisible = false;
  const galleryEl = document.getElementById("gallery");
  if (galleryEl) {
    new IntersectionObserver(([e]) => { galleryVisible = e.isIntersecting; }).observe(galleryEl);
  }

  const start = performance.now();
  let running = true;

  let ambientPaused = false;

  const frame = () => {
    if (!running) return;
    if (document.visibilityState === "visible") {
      updateScrollProgress();
      const t = (performance.now() - start) / 1000;
      if (!ambientPaused) {
        driver.tick(t);
        if (galleryVisible) {
          for (const { driver: gd } of galleryDrivers) gd.tick(t);
        }
      }
    }
    requestAnimationFrame(frame);
  };

  // Under reduced motion, paint one frame and stop.
  if (reduced) {
    driver.tick(1.2);
    for (const { driver: gd } of galleryDrivers) gd.tick(1.2);
  } else {
    requestAnimationFrame(frame);
  }

  addEventListener("pagehide", () => {
    running = false;
  });

  const doomFrame = document.getElementById("doomFrame");
  const playDoom = document.getElementById("playDoom");
  const stopDoom = document.getElementById("stopDoom");
  if (doomFrame && playDoom) {
    mountDoom(doomFrame, playDoom, stopDoom, {
      pauseAmbient: () => {
        ambientPaused = true;
      },
      resumeAmbient: () => {
        ambientPaused = false;
      },
    });
  }

  // All three are enhancements. A failure in any leaves the page as it was.
  try {
    initMotion({ reduced });
  } catch (err) {
    console.warn("motion unavailable; static layout stands", err);
  }
  try {
    initEnhancements();
  } catch (err) {
    console.warn("enhancements unavailable; page still works", err);
  }

  // Preload courtesy: fetch the ~27MB doom-wasm bundle in the background
  // once the page has settled, so a later click on "Play it" is a near-
  // instant start rather than the visitor's first-ever wait on it. Skipped
  // on a metered connection — costing every visitor this weight regardless
  // of whether they ever press play would be the wrong tradeoff there.
  const saveData =
    navigator.connection?.saveData || matchMedia("(prefers-reduced-data: reduce)").matches;
  if (!saveData && "requestIdleCallback" in window) {
    requestIdleCallback(() => {
      import("./doom-wasm/doom.js").catch((err) => {
        console.warn("doom-wasm preload failed; will retry on Play click", err);
      });
    });
  }

  window.__aaReady = true;
}

boot();
