/*
 * Wiring only. The loop runs when the tab is visible and stops otherwise.
 */
import { Renderer } from "./renderer.js";
import { loadEngine, SceneDriver } from "./engine.js";
import { initMotion, updateScrollProgress } from "./motion.js";
import { initEnhancements } from "./enhance.js";

const FONT_PX = 13;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Palette buttons are built from the engine's own Theme::ALL rather than a
 * hardcoded list, so the page cannot drift from the app's palettes.
 */
function buildPalette(driver, mount) {
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
      for (const other of mount.querySelectorAll("button")) {
        other.setAttribute("aria-pressed", String(other === btn));
      }
    });

    mount.append(btn);
  }
}

async function boot() {
  const canvas = document.getElementById("grid");
  if (!canvas) return;

  const renderer = new Renderer(canvas);
  const wasm = await loadEngine();
  const driver = new SceneDriver(renderer, wasm);

  const fit = () => {
    renderer.resize(innerWidth, innerHeight, FONT_PX);
    driver.resize();
  };
  fit();
  addEventListener("resize", fit);

  driver.setTheme("Hacker");
  driver.setScene("donut");
  buildPalette(driver, document.getElementById("palette"));

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

  const start = performance.now();
  let running = true;

  const frame = () => {
    if (!running) return;
    if (document.visibilityState === "visible") {
      updateScrollProgress();
      driver.tick((performance.now() - start) / 1000);
    }
    requestAnimationFrame(frame);
  };

  // Under reduced motion, paint one frame and stop.
  if (reduced) {
    driver.tick(1.2);
  } else {
    requestAnimationFrame(frame);
  }

  addEventListener("pagehide", () => {
    running = false;
  });

  // Both are enhancements. A failure in either leaves the page as it was.
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

  window.__aaReady = true;
}

boot();
