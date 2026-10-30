// Register before loading WebAssembly so initial Wallpaper Engine events are retained.
let paused = false;
let fps = 30;
let resetClock = false;
window.wallpaperPropertyListener = {
  applyGeneralProperties(properties) {
    if (properties.fps && Number.isFinite(properties.fps.value) && properties.fps.value > 0) {
      fps = properties.fps.value;
    }
  },
  setPaused(value) {
    paused = value;
    resetClock = true;
  },
};
document.addEventListener("visibilitychange", () => { resetClock = true; });

function start() {
  const bytes = Uint8Array.from(atob(wallpaperWasmBase64), (char) => char.charCodeAt(0));
  wasm_bindgen.initSync({ module: bytes });
  const grid = new Grid(document.getElementById("wallpaper"));
  const scene = new wasm_bindgen.Engine("donut", 1, 1);
  const theme = JSON.parse(wasm_bindgen.themes_json())[0];
  const hex = (rgb) => `#${rgb.map((value) => value.toString(16).padStart(2, "0")).join("")}`;

  function resize() {
    grid.resize(window.innerWidth, window.innerHeight, 16);
    scene.set_grid(grid.cols, grid.rows);
  }
  window.addEventListener("resize", resize);
  resize();

  let elapsed = 0;
  let previous = null;
  let lastPaint = 0;
  function tick(now) {
    if (resetClock) {
      previous = null;
      resetClock = false;
    }
    if (paused || document.hidden) {
      previous = null;
    } else {
      if (previous !== null) elapsed += (now - previous) / 1000;
      previous = now;
      if (now - lastPaint >= 1000 / fps) {
        scene.render(elapsed);
        grid.paint({ glyphs: scene.glyphs(), colors: scene.colors() }, null, 0, hex(theme.background), hex(theme.text));
        lastPaint = now;
      }
    }
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

try {
  start();
} catch (error) {
  console.error("ASCII Arcade wallpaper could not start", error);
}
