/*
 * aa-core, the app's own Rust scene engine, compiled to WebAssembly by
 * scripts/build-wasm.sh. The page runs the shipping scenes rather than a
 * JavaScript copy of them, so the two cannot drift apart.
 */

export async function loadEngine() {
  try {
    const mod = await import("./pkg/aa_wasm.js");
    await mod.default();
    return mod;
  } catch (err) {
    console.info("aa-core WebAssembly unavailable; the wallpaper stays blank", err);
    return null;
  }
}

/** One aa-core scene as a frame source for the grid. */
export class SceneSource {
  constructor(wasm, id) {
    this.engine = new wasm.Engine(id, 1, 1);
  }

  size(cols, rows) {
    this.engine.set_grid(cols, rows);
  }

  tint([r, g, b]) {
    this.engine.apply_base_color(r, g, b);
  }

  frame(t) {
    this.engine.render(t);
    return { glyphs: this.engine.glyphs(), colors: this.engine.colors() };
  }
}
