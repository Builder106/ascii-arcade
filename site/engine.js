/*
 * Loads aa-core (as WebAssembly) and drives the renderer from it.
 *
 * Every failure here is survivable: the canvas is decoration, so a missing
 * module costs the page its wallpaper and nothing else.
 */
import { makeThresholds, blend } from "./dissolve.js";

const TRANSITION_MS = 900;

export async function loadEngine() {
  try {
    const mod = await import("./pkg/aa_wasm.js");
    await mod.default();
    return mod;
  } catch (err) {
    console.warn("aa-core WebAssembly unavailable; grid stays empty", err);
    return null;
  }
}

export class SceneDriver {
  constructor(renderer, wasm) {
    this.renderer = renderer;
    this.wasm = wasm;
    this.current = null;
    this.next = null;
    this.currentId = null;
    this.transitionStart = 0;
    this.thresholds = new Float32Array(0);
    this.theme = { r: 48, g: 209, b: 88 };
    this.themes = wasm ? JSON.parse(wasm.themes_json()) : [];
  }

  setTheme(name) {
    const t = this.themes.find((x) => x.name === name);
    if (!t) return;
    this.theme = { r: t.text[0], g: t.text[1], b: t.text[2] };

    // Background comes from the page, not from Theme::background. Hacker's is
    // literally #000000, and painting that behind a tinted --bg leaves a seam
    // where the two blacks meet as the scrim fades out.
    const css = getComputedStyle(document.documentElement)
      .getPropertyValue("--bg")
      .trim();
    this.renderer.background =
      css || `rgb(${t.background[0]},${t.background[1]},${t.background[2]})`;
    for (const e of [this.current, this.next]) {
      if (e) e.apply_base_color(this.theme.r, this.theme.g, this.theme.b);
    }
  }

  setScene(id) {
    if (!this.wasm || id === this.currentId) return;

    const { cols, rows } = this.renderer;
    let engine;
    try {
      engine = new this.wasm.Engine(id, cols, rows);
    } catch (err) {
      console.warn(`scene ${id} unavailable`, err);
      return;
    }
    engine.apply_base_color(this.theme.r, this.theme.g, this.theme.b);
    this.currentId = id;
    if (typeof this.onSceneChange === "function") {
      this.onSceneChange(id);
    }

    if (!this.current) {
      this.current = engine;
      return;
    }
    this.next = engine;
    this.transitionStart = performance.now();
    this.thresholds = makeThresholds(cols * rows, cols * 31 + rows);
  }

  resize() {
    const { cols, rows } = this.renderer;
    for (const e of [this.current, this.next]) {
      if (e) e.set_grid(cols, rows);
    }
    this.thresholds = makeThresholds(cols * rows, cols * 31 + rows);
  }

  tick(tSeconds) {
    if (!this.current) return;
    this.current.render(tSeconds);

    if (!this.next) {
      this.renderer.paint(this.current.glyphs(), this.current.colors(), this.theme);
      return;
    }

    this.next.render(tSeconds);
    const progress = Math.min(
      1,
      (performance.now() - this.transitionStart) / TRANSITION_MS,
    );
    const mixed = blend(
      this.current.glyphs(),
      this.next.glyphs(),
      this.current.colors(),
      this.next.colors(),
      this.thresholds,
      progress,
    );
    this.renderer.paint(mixed.glyphs, mixed.colors, this.theme);

    if (progress >= 1) {
      this.current = this.next;
      this.next = null;
    }
  }
}
