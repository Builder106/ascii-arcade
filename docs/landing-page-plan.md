# Landing page implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the ASCII Arcade marketing site in `site/`, driven by `aa-core` compiled to WebAssembly, per `docs/landing-page-design.md`.

**Architecture:** A new `aa-wasm` crate wraps `aa-core` with `wasm-bindgen` so the page runs the shipping scene engine rather than a JavaScript port. One canvas character grid is fixed to the viewport and its scene state is driven by scroll; sections float over it. The static, no-JavaScript layout is built and tested first, then motion is layered on top.

**Tech Stack:** Rust + `wasm-bindgen`, vanilla ES modules, Motion (motion.dev) vendored, CSS scroll-driven animations, Playwright + `@axe-core/playwright`, IBM Plex Mono subset.

## Global constraints

Every task inherits these. Values are copied from the design document.

- **All builds run on `ampere-dev`, never on the Mac.** Anything producing `target/`, `node_modules/`, or `.venv/` goes through `verify-on-vm`. Use full path `/Users/yinkavaughan/bin/verify-on-vm` if the bare command is not found.
- **Budget:** HTML, CSS, JavaScript and the WebAssembly module together stay under 150 kB. DOOM payloads sit outside this budget and load only on request.
- **Contrast:** body text holds 4.5:1 on all four palettes, Ghost included. Enforced by test, not by eye.
- **No pure `#000000`.** The Hacker background is tinted fractionally toward the phosphor hue.
- **Section index is the luminance ramp** `.,-~:;=!*#$@`. Numbered section labels (`01 —`), eyebrow labels above headings, and runs of headings sharing one grammatical shape are all prohibited.
- **Font:** IBM Plex Mono, subset to printable ASCII plus exactly seven glyphs: `█ ─ │ ┌ ┐ └ ┘`.
- **Motion is vendored as a committed file**, never installed. No `package.json` in `site/`.
- **No pulsing dots, live badges, or status indicators.**
- **The canvas is `aria-hidden`.** Every fact it conveys also exists in text.
- Rust edition 2021, licence GPL-2.0, matching the workspace.
- Commits use the repository's Conventional Commits style (`feat:`, `docs:`, `ci:`, `test:`). Never add an AI co-author trailer.

## File structure

```text
crates/aa-wasm/            new: wasm-bindgen wrapper over aa-core
  Cargo.toml
  src/lib.rs
scripts/build-wasm.sh      new: cargo + wasm-bindgen -> site/pkg/
scripts/subset-font.sh     new: fonttools subset -> site/fonts/
scripts/record-doom.mjs    new: capture attract-mode ANSI frames
site/
  index.html               structure and copy
  styles.css               tokens, layout, type
  main.js                  entry point, wiring
  renderer.js              canvas character-grid painter
  dissolve.js              per-cell transition thresholds
  engine.js                WebAssembly loading and scene state
  doom.js                  DoomSource contract + RecordedDoom
  motion.js                scroll choreography
  vendor/motion.min.js     committed, not installed
  fonts/                   IBMPlexMono-subset.woff2
  pkg/                     generated wasm-bindgen output
e2e/tests/site/            new Playwright specs
.github/workflows/pages.yml new: deploy
```

---

### Task 1: `aa-wasm` crate

Wraps `aa-core` for the browser. Later tasks depend only on the interface this produces.

**Files:**

- Create: `crates/aa-wasm/Cargo.toml`
- Create: `crates/aa-wasm/src/lib.rs`
- Create: `scripts/build-wasm.sh`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**

- Consumes: `aa_core::{scenes, RgbColor, Scene, Theme}`, all existing.
- Produces, for every later task:
  - `new Engine(id: string, cols: number, rows: number)` where `id` is one of `donut`, `helix`, `matrix`, `pipes`, `life`
  - `engine.set_grid(cols: number, rows: number): void`
  - `engine.apply_base_color(r: number, g: number, b: number): void`
  - `engine.render(t: number): void` — advances to time `t` in seconds and caches one frame
  - `engine.glyphs(): string` — row-major, exactly `cols * rows` characters, no newlines
  - `engine.colors(): Uint32Array` — row-major, one entry per cell. `0` means "paint in the theme colour"; anything else is `0xFF_RR_GG_BB`
  - `scene_ids(): string[]`
  - `themes_json(): string` — `[{"name":"Hacker","text":[r,g,b],"background":[r,g,b]}, ...]`

- [ ] **Step 1: Write the failing test**

Create `crates/aa-wasm/src/lib.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_renders_a_full_grid() {
        let mut e = Engine::new("donut", 40, 20).expect("donut exists");
        e.render(1.0);
        assert_eq!(e.glyphs().chars().count(), 800);
        assert_eq!(e.colors().len(), 800);
    }

    #[test]
    fn unknown_scene_is_an_error() {
        assert!(Engine::new("nope", 10, 10).is_err());
    }

    #[test]
    fn uncoloured_cells_use_zero_sentinel() {
        let mut e = Engine::new("donut", 20, 10).expect("donut exists");
        e.render(0.5);
        assert!(e.colors().iter().any(|&c| c == 0), "donut is monochrome");
    }

    #[test]
    fn themes_json_lists_all_four() {
        let json = themes_json();
        for name in ["Hacker", "Amber", "Ice", "Ghost"] {
            assert!(json.contains(name), "missing {name}");
        }
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade "cargo test -p aa-wasm"`

Expected: FAIL, the crate is not a workspace member and `Engine` does not exist.

- [ ] **Step 3: Add the crate to the workspace**

In `Cargo.toml`, add `"crates/aa-wasm",` to `[workspace] members`, after `"crates/aa-ffi",`.

Create `crates/aa-wasm/Cargo.toml`:

```toml
[package]
name = "aa-wasm"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "wasm-bindgen wrapper over aa-core for the marketing site."

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
aa-core = { workspace = true }
wasm-bindgen = "0.2"
```

- [ ] **Step 4: Write the implementation**

Prepend to `crates/aa-wasm/src/lib.rs`, above the test module:

```rust
//! Browser bindings for `aa-core`. The marketing site runs the shipping scene
//! engine through this rather than a JavaScript port, so the two cannot drift.

use aa_core::{scenes, RgbColor, Scene, Theme};
use wasm_bindgen::prelude::*;

/// One live scene plus its most recent frame, flattened for JavaScript.
#[wasm_bindgen]
pub struct Engine {
    scene: Box<dyn Scene + Send>,
    glyphs: String,
    colors: Vec<u32>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, cols: usize, rows: usize) -> Result<Engine, JsValue> {
        let mut scene = scenes::make(id)
            .ok_or_else(|| JsValue::from_str(&format!("unknown scene: {id}")))?;
        scene.set_grid(cols, rows);
        Ok(Engine { scene, glyphs: String::new(), colors: Vec::new() })
    }

    pub fn set_grid(&mut self, cols: usize, rows: usize) {
        self.scene.set_grid(cols, rows);
    }

    pub fn apply_base_color(&mut self, r: u8, g: u8, b: u8) {
        self.scene.apply_base_color(RgbColor::new(r, g, b));
    }

    /// Advance to `t` seconds and cache the frame. Call before the getters.
    pub fn render(&mut self, t: f64) {
        let frame = self.scene.frame(t);
        self.glyphs.clear();
        self.colors.clear();
        self.colors.reserve(frame.cells.len());
        for cell in &frame.cells {
            self.glyphs.push(cell.ch);
            // Packed explicitly rather than via RgbColor::to_argb so that a
            // genuinely black cell can never collide with the zero sentinel.
            self.colors.push(match cell.color {
                Some(c) => {
                    0xFF00_0000
                        | (u32::from(c.r) << 16)
                        | (u32::from(c.g) << 8)
                        | u32::from(c.b)
                }
                None => 0,
            });
        }
    }

    /// Row-major glyphs, `cols * rows` characters, no newlines.
    pub fn glyphs(&self) -> String {
        self.glyphs.clone()
    }

    /// Row-major colour, one entry per cell. Zero means "use the theme colour".
    pub fn colors(&self) -> Vec<u32> {
        self.colors.clone()
    }
}

/// Built-in scene ids, in the order the site presents them.
#[wasm_bindgen]
pub fn scene_ids() -> Vec<String> {
    scenes::BUILTIN_IDS.iter().map(|s| (*s).to_string()).collect()
}

/// The four palettes, so the page never keeps a second copy that can drift.
/// Hand-rolled rather than pulling in serde, which would cost more than it saves.
#[wasm_bindgen]
pub fn themes_json() -> String {
    let entries: Vec<String> = Theme::ALL
        .iter()
        .map(|t| {
            format!(
                r#"{{"name":"{}","text":[{},{},{}],"background":[{},{},{}]}}"#,
                t.name,
                t.text.r, t.text.g, t.text.b,
                t.background.r, t.background.g, t.background.b
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade "cargo test -p aa-wasm"`

Expected: PASS, four tests.

- [ ] **Step 6: Write the build script**

Create `scripts/build-wasm.sh`, mode `755`:

```bash
#!/usr/bin/env bash
# Build aa-wasm for the browser into site/pkg/.
#
# Runs on ampere-dev, never on the Mac: it produces target/ and needs the
# wasm32 toolchain. See docs/landing-page-design.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/pkg"

rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked 2>/dev/null || true

cargo build -p aa-wasm --release --target wasm32-unknown-unknown

wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir "$OUT" \
  "$ROOT/target/wasm32-unknown-unknown/release/aa_wasm.wasm"

echo "built: $OUT"
ls -lh "$OUT"
```

- [ ] **Step 7: Build the WebAssembly module and record its size**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade "bash scripts/build-wasm.sh"`

Expected: `site/pkg/aa_wasm_bg.wasm` and `site/pkg/aa_wasm.js` exist. Note the `.wasm` size; it counts against the 150 kB budget checked in Task 9. If it exceeds 100 kB, add `[profile.release] opt-level = "z"` and `lto = true` to the workspace `Cargo.toml` and rebuild before continuing.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/aa-wasm scripts/build-wasm.sh
git commit -m "feat: add aa-wasm, a wasm-bindgen wrapper over aa-core"
```

---

### Task 2: IBM Plex Mono subset

**Files:**

- Create: `scripts/subset-font.sh`
- Create: `site/fonts/OFL.txt`
- Generated: `site/fonts/IBMPlexMono-subset.woff2`

**Interfaces:**

- Produces: `site/fonts/IBMPlexMono-subset.woff2`, referenced by `styles.css` in Task 6 as `font-family: "IBM Plex Mono"`.

- [ ] **Step 1: Write the subset script**

Create `scripts/subset-font.sh`, mode `755`:

```bash
#!/usr/bin/env bash
# Subset IBM Plex Mono to the glyphs the site actually draws.
#
# Printable ASCII covers the luminance ramp, the Matrix alphabet and all page
# copy. The seven extras are Life's block and Pipes' box drawing; without them
# those two scenes render as gaps. See docs/landing-page-design.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/fonts"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$OUT"

curl -fsSL -o "$WORK/plex.zip" \
  https://github.com/IBM/plex/releases/download/%40ibm%2Fplex-mono%401.1.0/ibm-plex-mono.zip
unzip -q "$WORK/plex.zip" -d "$WORK"

SRC="$(find "$WORK" -name 'IBMPlexMono-Regular.ttf' -o -name 'IBMPlexMono-Regular.otf' | head -1)"
if [ -z "$SRC" ]; then
  echo "IBMPlexMono-Regular not found in the release archive" >&2
  exit 1
fi

uv tool run --from fonttools pyftsubset "$SRC" \
  --output-file="$OUT/IBMPlexMono-subset.woff2" \
  --flavor=woff2 \
  --layout-features='' \
  --unicodes='U+0020-007E,U+2588,U+2500,U+2502,U+250C,U+2510,U+2514,U+2518'

ls -lh "$OUT/IBMPlexMono-subset.woff2"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade "bash scripts/subset-font.sh"`

Expected: it may fail on the pinned release URL. If it 404s, list available releases with `curl -fsSL https://api.github.com/repos/IBM/plex/releases | grep browser_download_url | grep -i mono`, pick the current `ibm-plex-mono` asset, and update the URL in the script. Do not switch to a different typeface.

- [ ] **Step 3: Verify glyph coverage**

Run:

```bash
verify-on-vm ~/CS/projects/personal/ascii-arcade "uv tool run --from fonttools ttx -o - -t cmap site/fonts/IBMPlexMono-subset.woff2 | grep -c 'code=' "
```

Expected: at least 102. Then confirm each of the seven extras individually:

```bash
verify-on-vm ~/CS/projects/personal/ascii-arcade "uv tool run --from fonttools ttx -o - -t cmap site/fonts/IBMPlexMono-subset.woff2 | grep -E '0x2588|0x2500|0x2502|0x250c|0x2510|0x2514|0x2518'"
```

Expected: seven matching lines. A missing one means Pipes or Life will render as gaps.

- [ ] **Step 4: Add the licence**

IBM Plex is OFL 1.1 and the licence text must ship with the font. Copy `LICENSE.txt` from the downloaded archive to `site/fonts/OFL.txt`, or fetch it:

```bash
curl -fsSL -o site/fonts/OFL.txt https://raw.githubusercontent.com/IBM/plex/master/LICENSE.txt
```

- [ ] **Step 5: Commit**

```bash
git add scripts/subset-font.sh site/fonts/
git commit -m "feat: subset IBM Plex Mono to the glyphs the site draws"
```

---

### Task 3: Canvas renderer

Paints a character grid. Knows nothing about scenes, scroll, or WebAssembly.

**Files:**

- Create: `site/renderer.js`
- Create: `e2e/tests/site/renderer.spec.ts`

**Interfaces:**

- Produces:
  - `measureCell(ctx, fontPx): { w: number, h: number }`
  - `gridSize(pxW, pxH, cell): { cols: number, rows: number }`
  - `new Renderer(canvas)` with `resize(cssW, cssH, fontPx)`, `paint(glyphs, colors, themeColor)`, and readonly `cols` / `rows`
  - `themeColor` is `{ r, g, b }`; `colors` entries of `0` fall back to it, matching Task 1's sentinel

- [ ] **Step 1: Write the failing test**

Create `e2e/tests/site/renderer.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import { gridSize } from "../../../site/renderer.js";

test("gridSize floors to whole cells", () => {
  expect(gridSize(800, 600, { w: 8, h: 16 })).toEqual({ cols: 100, rows: 37 });
});

test("gridSize never returns a zero dimension", () => {
  expect(gridSize(2, 2, { w: 8, h: 16 })).toEqual({ cols: 1, rows: 1 });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/renderer.spec.ts"`

Expected: FAIL, `site/renderer.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `site/renderer.js`:

```javascript
/*
 * Canvas character-grid painter.
 *
 * Colour is why this is canvas and not a <pre>: Matrix has bright heads with
 * fading trails and Pipes carries per-pipe hue, so per-cell colour in the DOM
 * would mean thousands of spans a frame. Runs of one colour are batched into a
 * single fillText call to keep that affordable.
 */

/** Cell metrics for the current font, measured rather than assumed. */
export function measureCell(ctx, fontPx) {
  const m = ctx.measureText("M");
  return { w: m.width, h: fontPx * 1.2 };
}

/** How many whole cells fit, never zero. */
export function gridSize(pxW, pxH, cell) {
  return {
    cols: Math.max(1, Math.floor(pxW / cell.w)),
    rows: Math.max(1, Math.floor(pxH / cell.h)),
  };
}

export class Renderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d", { alpha: false });
    this.cols = 0;
    this.rows = 0;
    this.cell = { w: 8, h: 16 };
    this.background = "#000";
  }

  /** Device pixel ratio is capped at 2; beyond that costs fill rate for nothing. */
  resize(cssW, cssH, fontPx) {
    const dpr = Math.min(2, globalThis.devicePixelRatio || 1);
    this.canvas.width = Math.floor(cssW * dpr);
    this.canvas.height = Math.floor(cssH * dpr);
    this.canvas.style.width = `${cssW}px`;
    this.canvas.style.height = `${cssH}px`;

    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.font = `${fontPx}px "IBM Plex Mono", monospace`;
    this.ctx.textBaseline = "top";

    this.cell = measureCell(this.ctx, fontPx);
    const g = gridSize(cssW, cssH, this.cell);
    this.cols = g.cols;
    this.rows = g.rows;
    return g;
  }

  /**
   * `glyphs` is row-major and `cols * rows` long. A `colors` entry of 0 means
   * paint in `themeColor`, matching the sentinel aa-wasm packs.
   */
  paint(glyphs, colors, themeColor) {
    const { ctx, cols, rows, cell } = this;
    ctx.fillStyle = this.background;
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

    const fallback = `rgb(${themeColor.r},${themeColor.g},${themeColor.b})`;

    for (let y = 0; y < rows; y++) {
      let run = "";
      let runStart = 0;
      let runColor = null;

      for (let x = 0; x <= cols; x++) {
        const i = y * cols + x;
        const ch = x < cols ? glyphs[i] : null;
        const packed = x < cols ? colors[i] : -1;
        const color = packed === 0 ? fallback : cssFromPacked(packed);

        if (ch === null || color !== runColor) {
          if (run.trim().length > 0) {
            ctx.fillStyle = runColor;
            ctx.fillText(run, runStart * cell.w, y * cell.h);
          }
          if (ch === null) break;
          run = ch;
          runStart = x;
          runColor = color;
        } else {
          run += ch;
        }
      }
    }
  }
}

function cssFromPacked(packed) {
  const r = (packed >>> 16) & 0xff;
  const g = (packed >>> 8) & 0xff;
  const b = packed & 0xff;
  return `rgb(${r},${g},${b})`;
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/renderer.spec.ts"`

Expected: PASS, two tests.

- [ ] **Step 5: Commit**

```bash
git add site/renderer.js e2e/tests/site/renderer.spec.ts
git commit -m "feat: add the canvas character-grid renderer"
```

---

### Task 4: Dissolve transitions

Scene changes swap glyph by glyph rather than crossfading. Pure logic, no canvas.

**Files:**

- Create: `site/dissolve.js`
- Create: `e2e/tests/site/dissolve.spec.ts`

**Interfaces:**

- Produces:
  - `makeThresholds(count, seed): Float32Array` — stable for a given seed
  - `blend(fromGlyphs, toGlyphs, fromColors, toColors, thresholds, progress): { glyphs: string[], colors: Uint32Array }`
  - `progress` of `0` yields the `from` frame exactly, `1` yields `to` exactly

- [ ] **Step 1: Write the failing test**

Create `e2e/tests/site/dissolve.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import { makeThresholds, blend } from "../../../site/dissolve.js";

test("thresholds are stable for a seed", () => {
  expect(Array.from(makeThresholds(64, 7))).toEqual(
    Array.from(makeThresholds(64, 7)),
  );
});

test("progress 0 is the from frame and 1 is the to frame", () => {
  const from = "aaaa".split("");
  const to = "bbbb".split("");
  const fc = new Uint32Array([0, 0, 0, 0]);
  const tc = new Uint32Array([1, 1, 1, 1]);
  const th = makeThresholds(4, 1);

  expect(blend(from, to, fc, tc, th, 0).glyphs.join("")).toBe("aaaa");
  expect(blend(from, to, fc, tc, th, 1).glyphs.join("")).toBe("bbbb");
});

test("a partial dissolve is a mix of both frames", () => {
  const n = 400;
  const from = Array(n).fill("a");
  const to = Array(n).fill("b");
  const fc = new Uint32Array(n);
  const tc = new Uint32Array(n).fill(1);
  const out = blend(from, to, fc, tc, makeThresholds(n, 3), 0.5).glyphs.join("");

  expect(out).toContain("a");
  expect(out).toContain("b");
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/dissolve.spec.ts"`

Expected: FAIL, `site/dissolve.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `site/dissolve.js`:

```javascript
/*
 * Scene transitions dissolve character by character instead of crossfading
 * opacity. Each cell gets a fixed threshold; as progress sweeps 0 to 1 a cell
 * flips the moment progress passes its threshold. One array and one comparison
 * per cell, and it is something images cannot do.
 */

/**
 * Deterministic thresholds. Seeded so a resize or a scrub backwards reproduces
 * the same dissolve rather than reshuffling under the reader.
 */
export function makeThresholds(count, seed) {
  const out = new Float32Array(count);
  let s = seed >>> 0 || 1;
  for (let i = 0; i < count; i++) {
    // xorshift32: small, fast, and good enough for scattering cells.
    s ^= s << 13;
    s >>>= 0;
    s ^= s >>> 17;
    s ^= s << 5;
    s >>>= 0;
    out[i] = s / 0xffffffff;
  }
  return out;
}

export function blend(fromGlyphs, toGlyphs, fromColors, toColors, thresholds, progress) {
  const n = thresholds.length;
  const glyphs = new Array(n);
  const colors = new Uint32Array(n);

  for (let i = 0; i < n; i++) {
    const flipped = progress >= 1 || (progress > 0 && progress > thresholds[i]);
    glyphs[i] = flipped ? toGlyphs[i] : fromGlyphs[i];
    colors[i] = flipped ? toColors[i] : fromColors[i];
  }
  return { glyphs, colors };
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/dissolve.spec.ts"`

Expected: PASS, three tests.

- [ ] **Step 5: Commit**

```bash
git add site/dissolve.js e2e/tests/site/dissolve.spec.ts
git commit -m "feat: add character-dissolve scene transitions"
```

---

### Task 5: Engine glue

Loads the WebAssembly module and drives the renderer. First point where a scene actually appears.

**Files:**

- Create: `site/engine.js`
- Create: `site/main.js`
- Create: `e2e/tests/site/engine.spec.ts`

**Interfaces:**

- Consumes: Task 1's `Engine`, `scene_ids`, `themes_json`; Task 3's `Renderer`; Task 4's `makeThresholds`, `blend`.
- Produces:
  - `loadEngine(): Promise<{ Engine, scene_ids, themes_json }>` — resolves `null` if the module fails to load
  - `new SceneDriver(renderer, wasm)` with `setScene(id)`, `setTheme(name)`, `tick(tSeconds)`, `stop()`
  - `SceneDriver` no-ops safely when `wasm` is `null`, which is the WebAssembly-absent path the design requires

- [ ] **Step 1: Write the failing test**

Create `e2e/tests/site/engine.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("the donut paints a non-empty grid", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, {
    timeout: 15000,
  });

  const painted = await page.evaluate(() => {
    const c = document.getElementById("grid");
    const ctx = c.getContext("2d");
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    for (let i = 0; i < d.length; i += 4) {
      if (d[i] || d[i + 1] || d[i + 2]) return true;
    }
    return false;
  });

  expect(painted).toBe(true);
});

test("the page still renders when WebAssembly fails to load", async ({ page }) => {
  await page.route("**/pkg/**", (r) => r.abort());
  await page.goto("/site/");

  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(page.getByRole("link", { name: /download/i }).first()).toBeVisible();
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/engine.spec.ts"`

Expected: FAIL, there is no `site/index.html` yet. This test stays red until Task 6 adds the page; that is expected and Task 6 re-runs it.

- [ ] **Step 3: Write the engine glue**

Create `site/engine.js`:

```javascript
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
    this.transitionStart = 0;
    this.thresholds = new Float32Array(0);
    this.theme = { r: 48, g: 209, b: 88 };
    this.themes = wasm ? JSON.parse(wasm.themes_json()) : [];
  }

  setTheme(name) {
    const t = this.themes.find((x) => x.name === name);
    if (!t) return;
    this.theme = { r: t.text[0], g: t.text[1], b: t.text[2] };
    for (const e of [this.current, this.next]) {
      if (e) e.apply_base_color(this.theme.r, this.theme.g, this.theme.b);
    }
  }

  setScene(id) {
    if (!this.wasm) return;
    const { cols, rows } = this.renderer;
    let engine;
    try {
      engine = new this.wasm.Engine(id, cols, rows);
    } catch (err) {
      console.warn(`scene ${id} unavailable`, err);
      return;
    }
    engine.apply_base_color(this.theme.r, this.theme.g, this.theme.b);

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
```

- [ ] **Step 4: Write the entry point**

Create `site/main.js`:

```javascript
/*
 * Wiring only. The loop runs when the grid is on screen and the tab is
 * visible, and stops otherwise.
 */
import { Renderer } from "./renderer.js";
import { loadEngine, SceneDriver } from "./engine.js";

const FONT_PX = 13;
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

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

  driver.setScene("donut");

  // Sections declare which scene belongs to them. One observer, half-visible
  // wins, so the scene changes once per section rather than fighting on
  // boundaries.
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
      driver.tick((performance.now() - start) / 1000);
    }
    if (!reduced) requestAnimationFrame(frame);
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

  window.__aaReady = true;
}

boot();
```

- [ ] **Step 5: Commit**

```bash
git add site/engine.js site/main.js e2e/tests/site/engine.spec.ts
git commit -m "feat: drive the canvas grid from aa-core WebAssembly"
```

---

### Task 6: Page structure, copy and styles

The complete static page. This is also the reduced-motion and WebAssembly-absent baseline, so it is built and tested before any motion exists.

**Files:**

- Create: `site/index.html`
- Create: `site/styles.css`
- Create: `e2e/tests/site/content.spec.ts`
- Create: `e2e/tests/site/a11y.spec.ts`
- Modify: `e2e/package.json`
- Modify: `e2e/playwright.config.ts` (create if absent)

**Interfaces:**

- Consumes: Task 2's font, Task 5's `main.js` and the `#grid` canvas, and `data-scene` attributes on sections.
- Produces: section ids `stack`, `layer`, `scenes`, `palette`, `surfaces`, `install`, consumed by Task 7's choreography.

- [ ] **Step 1: Add the accessibility dependency and Playwright config**

In `e2e/package.json`, add to `devDependencies`:

```json
"@axe-core/playwright": "^4.10.0"
```

Create `e2e/playwright.config.ts`:

```typescript
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  webServer: {
    command: "python3 -m http.server 8899 --directory ..",
    port: 8899,
    reuseExistingServer: true,
  },
  use: { baseURL: "http://127.0.0.1:8899" },
});
```

- [ ] **Step 2: Write the failing tests**

Create `e2e/tests/site/content.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

const HEADINGS = [
  "macOS can't screenshot this",
  "It lives under everything",
  "DOOM was the sideshow",
  "Every glyph keys off one colour",
  "It doesn't need a desktop",
  "Put it on yours",
];

test("every section heading is present", async ({ page }) => {
  await page.goto("/site/");
  for (const h of HEADINGS) {
    await expect(page.getByRole("heading", { name: h })).toBeVisible();
  }
});

test("no numbered section labels", async ({ page }) => {
  await page.goto("/site/");
  const body = await page.locator("body").innerText();
  expect(body).not.toMatch(/^\s*0[1-6]\s*[—-]/m);
});

test("Gatekeeper is explained rather than buried", async ({ page }) => {
  await page.goto("/site/");
  await expect(page.getByText(/gatekeeper/i)).toBeVisible();
  await expect(page.getByText("xattr -dr com.apple.quarantine")).toBeVisible();
});

test("the depth rail exposes accessible names, not bare glyphs", async ({ page }) => {
  await page.goto("/site/");
  const rail = page.getByRole("navigation", { name: /sections/i });
  for (const h of HEADINGS) {
    await expect(rail.getByRole("link", { name: h })).toBeAttached();
  }
});
```

Create `e2e/tests/site/a11y.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const THEMES = ["hacker", "amber", "ice", "ghost"];

for (const theme of THEMES) {
  test(`no accessibility violations under the ${theme} palette`, async ({ page }) => {
    await page.goto("/site/");
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
    }, theme);

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();

    expect(results.violations).toEqual([]);
  });
}

test("the canvas is hidden from assistive technology", async ({ page }) => {
  await page.goto("/site/");
  await expect(page.locator("#grid")).toHaveAttribute("aria-hidden", "true");
});
```

- [ ] **Step 3: Run them to verify they fail**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npm install && npx playwright test tests/site/"`

Expected: FAIL, `site/index.html` does not exist.

- [ ] **Step 4: Write the page**

Create `site/index.html`. Section order and headings are fixed by the design; the ramp glyph is the only label.

```html
<!DOCTYPE html>
<html lang="en" data-theme="hacker">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ASCII Arcade — live ASCII wallpapers for macOS</title>
<meta name="description" content="A macOS live wallpaper that renders ASCII scenes onto your desktop, behind your windows. Donut, Matrix rain, Game of Life, pipes, and playable text-mode DOOM.">
<link rel="icon" href="../assets/logo.svg" type="image/svg+xml">
<link rel="stylesheet" href="styles.css">
</head>
<body>

<a class="skip" href="#main">Skip to content</a>

<canvas id="grid" aria-hidden="true"></canvas>

<nav class="rail" aria-label="Sections">
  <a href="#stack"    data-mark="."><span class="vh">macOS can't screenshot this</span></a>
  <a href="#layer"    data-mark="~"><span class="vh">It lives under everything</span></a>
  <a href="#scenes"   data-mark=":"><span class="vh">DOOM was the sideshow</span></a>
  <a href="#palette"  data-mark="="><span class="vh">Every glyph keys off one colour</span></a>
  <a href="#surfaces" data-mark="*"><span class="vh">It doesn't need a desktop</span></a>
  <a href="#install"  data-mark="@"><span class="vh">Put it on yours</span></a>
</nav>

<section class="open" id="open">
  <pre class="open__doom" id="doomFrame" aria-hidden="true"></pre>
  <h1 class="open__claim">This is a desktop background.</h1>
  <p class="open__sub">It is also DOOM, and you can play it.</p>
  <div class="open__acts">
    <button class="btn btn--go" id="playDoom" type="button">Play it</button>
    <a class="btn" href="https://github.com/Builder106/ascii-arcade/releases">Download</a>
  </div>
  <p class="open__note">Recorded. Everything below this is running live in your browser.</p>
</section>

<main id="main">

<section class="sec" id="stack" data-scene="matrix">
  <h2>macOS can't screenshot this</h2>
  <p>
    ASCII Arcade draws into the desktop layer, underneath every window you have
    open. <kbd>⌘</kbd><kbd>⇧</kbd><kbd>3</kbd> samples the wallpaper compositor
    and skips that window entirely, so the app carries its own capture path
    built on <code>CGWindowListCreateImage</code>. Press
    <kbd>⌘</kbd><kbd>⌥</kbd><kbd>S</kbd> and you get a PNG on the Desktop and a
    copy on the clipboard.
  </p>
</section>

<section class="sec" id="layer" data-scene="donut">
  <h2>It lives under everything</h2>
  <p>
    Not a screensaver that takes over, and not a still image. A live process
    painting characters behind your work. The torus above is Andy Sloane's
    original donut maths, the same code the app runs on your desktop, compiled
    to WebAssembly and drawing in this page.
  </p>
  <a class="btn btn--go" href="https://github.com/Builder106/ascii-arcade/releases">Download the DMG</a>
</section>

<section class="sec" id="scenes" data-scene="life">
  <h2>DOOM was the sideshow</h2>
  <p>
    There are six scenes and DOOM is the one that is off by default. The donut
    and its precessing helix variant are pure projection maths. Matrix rain
    keeps bright heads with trails fading behind them. Conway's Game of Life
    reseeds itself whenever it stalls. Pipes is the old screensaver in
    box-drawing glyphs.
  </p>
  <p>
    A playable shooter is not what everyone wants greeting them on a shared or
    school machine, so DOOM stays out of the scene list, out of
    <kbd>⌘</kbd><kbd>⌥</kbd><kbd>C</kbd> cycling and out of the idle rotation
    until you turn it on.
  </p>
  <a class="btn btn--go" href="https://github.com/Builder106/ascii-arcade/releases">Download the DMG</a>
</section>

<section class="sec" id="palette" data-scene="pipes">
  <h2>Every glyph keys off one colour</h2>
  <p>
    Matrix, Life and the maths scenes take their colour from the theme's text
    colour, so the rain turns amber under Amber. DOOM keeps its own palette.
    Pick one and this page retints with it, grid included.
  </p>
  <div class="palette" id="palette" role="group" aria-label="Choose a palette"></div>
</section>

<section class="sec" id="surfaces" data-scene="helix">
  <h2>It doesn't need a desktop</h2>
  <p>
    The scenes are a Rust core with more than one front end, which is handy for
    sharing or for machines with no wallpaper to draw on.
  </p>
  <pre class="code"><code>cargo run -p aa -- play donut</code></pre>
  <pre class="code"><code>cargo run -p aa-web</code></pre>
  <p>
    The first plays any scene straight into your terminal. The second streams
    ANSI truecolor frames at 30 fps over a WebSocket into an xterm.js terminal
    at <code>127.0.0.1:8788</code>.
  </p>
</section>

<section class="sec" id="install" data-scene="matrix">
  <h2>Put it on yours</h2>
  <p>
    Grab <code>ASCII-Arcade.dmg</code> from the releases page, open it, and drag
    ASCII Arcade onto Applications. Look for the <code>◎</code> in your menu bar.
  </p>
  <p>
    There is no paid Apple Developer account behind this, so the app is not
    notarized and Gatekeeper blocks the first launch. Control-click it, choose
    Open, then confirm. macOS remembers from then on.
  </p>
  <pre class="code"><code>xattr -dr com.apple.quarantine "/Applications/ASCII Arcade.app"</code></pre>
  <a class="btn btn--go" href="https://github.com/Builder106/ascii-arcade/releases">Download the DMG</a>
  <p class="fine">macOS 13 or newer. GPL-2.0. No account, no telemetry.</p>
</section>

</main>

<footer class="foot">
  <nav aria-label="Footer">
    <a href="https://github.com/Builder106/ascii-arcade">GitHub</a>
    <a href="https://github.com/Builder106/ascii-arcade/releases">Releases</a>
    <a href="https://github.com/Builder106/ascii-arcade/blob/main/LICENSE">Licence</a>
  </nav>
  <p>Started as two projects, <code>donut</code> and <code>DOOM</code>, that turned out to want the same renderer.</p>
</footer>

<script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 5: Write the styles**

Create `site/styles.css`:

```css
/*
 * Tokens come from aa-core's Theme::ALL, so the palettes here are the app's.
 * Backgrounds are tinted rather than pure black: a flat #000 reads synthetic.
 */
@font-face {
  font-family: "IBM Plex Mono";
  src: url("fonts/IBMPlexMono-subset.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}

:root {
  --fg: #30d158;
  --bg: oklch(3% 0.015 145);
  --muted: color-mix(in oklab, var(--fg) 80%, var(--bg));
  --dim: color-mix(in oklab, var(--fg) 66%, var(--bg));
  --rule: color-mix(in oklab, var(--fg) 24%, var(--bg));
}
:root[data-theme="amber"] { --fg: #ffa600; --bg: #1a0800; }
:root[data-theme="ice"]   { --fg: #00ffff; --bg: #000d1a; }
:root[data-theme="ghost"] { --fg: #1c1c1e; --bg: #f5f5f5; }

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 400 1rem/1.6 "IBM Plex Mono", ui-monospace, monospace;
}

#grid {
  position: fixed;
  inset: 0;
  z-index: 0;
  pointer-events: none;
}

.skip {
  position: absolute;
  left: -9999px;
}
.skip:focus {
  left: 1rem;
  top: 1rem;
  z-index: 10;
  background: var(--bg);
  padding: 0.5rem 1rem;
}

.vh {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.rail {
  position: fixed;
  left: 1rem;
  top: 50%;
  translate: 0 -50%;
  z-index: 3;
  display: grid;
  gap: 0.75rem;
}
.rail a {
  color: var(--dim);
  text-decoration: none;
  line-height: 1;
}
.rail a::before { content: attr(data-mark); }
.rail a:hover,
.rail a:focus-visible { color: var(--fg); }

.open,
.sec {
  position: relative;
  z-index: 1;
  min-height: 100svh;
  display: grid;
  align-content: center;
  gap: 1rem;
  padding: 4rem clamp(1.5rem, 8vw, 10rem);
  max-width: 62rem;
}

.open__doom { margin: 0; color: var(--muted); font-size: 0.7rem; }
.open__claim { margin: 0; font-size: clamp(2rem, 6vw, 4rem); line-height: 1.05; }
.open__sub { margin: 0; color: var(--muted); }
.open__note { color: var(--dim); font-size: 0.85rem; }
.open__acts { display: flex; gap: 0.75rem; flex-wrap: wrap; }

h2 { font-size: clamp(1.6rem, 4vw, 2.6rem); margin: 0; line-height: 1.1; }
p { margin: 0; color: var(--muted); max-width: 60ch; }

.btn {
  display: inline-block;
  border: 1px solid var(--fg);
  color: var(--fg);
  background: none;
  padding: 0.6rem 1.1rem;
  text-decoration: none;
  font: inherit;
  cursor: pointer;
  justify-self: start;
}
.btn--go { background: var(--fg); color: var(--bg); }
.btn:focus-visible { outline: 2px solid var(--fg); outline-offset: 3px; }

.code {
  border: 1px solid var(--rule);
  padding: 0.75rem 1rem;
  overflow-x: auto;
  margin: 0;
}
kbd { border: 1px solid var(--rule); padding: 0 0.3rem; }
.fine { color: var(--dim); font-size: 0.85rem; }

.palette { display: flex; gap: 0.75rem; flex-wrap: wrap; }

.foot {
  position: relative;
  z-index: 1;
  padding: 3rem clamp(1.5rem, 8vw, 10rem);
  border-top: 1px solid var(--rule);
  color: var(--dim);
}
.foot nav { display: flex; gap: 1.25rem; flex-wrap: wrap; margin-bottom: 1rem; }
.foot a { color: var(--muted); }

@media (prefers-reduced-motion: reduce) {
  * { animation: none !important; transition: none !important; }
  html { scroll-behavior: auto; }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/"`

Expected: PASS. If an axe contrast violation appears, raise `--muted` or `--dim` toward `--fg` until it clears on all four palettes. Do not silence the rule.

- [ ] **Step 7: Commit**

```bash
git add site/index.html site/styles.css e2e/
git commit -m "feat: add the static landing page, copy and palettes"
```

---

### Task 7: Motion and progressive enhancements

Layered onto a page that already works without any of it. Covers the reveals, the
scroll scrub, the copy buttons and the persistent affordance, all of which are
additions to markup that already reads on its own.

**Files:**

- Create: `site/vendor/motion.min.js`
- Create: `site/motion.js`
- Create: `site/enhance.js`
- Modify: `site/main.js`
- Modify: `site/index.html`
- Modify: `site/styles.css`
- Create: `e2e/tests/site/motion.spec.ts`
- Create: `e2e/tests/site/enhance.spec.ts`

**Interfaces:**

- Consumes: Task 6's section ids, Task 5's `SceneDriver`.
- Produces:
  - `initMotion({ reduced })`, called once from `main.js`, a no-op when `reduced` is true
  - `scrollProgress()` inside `motion.js` — a lerped 0-to-1 value updated on the existing `rAF` loop, which is where scrub smoothing comes from since native scroll timelines are strictly one-to-one with scroll position
  - `initEnhancements()` in `enhance.js` — copy buttons on `pre.code`, and the fixed depth-glyph affordance

- [ ] **Step 1: Vendor Motion**

```bash
curl -fsSL -o site/vendor/motion.min.js https://cdn.jsdelivr.net/npm/motion@11/+esm
```

Confirm it is an ES module (`grep -c "export" site/vendor/motion.min.js` returns non-zero) and record the version in a comment at the top of `site/motion.js`. Motion is MIT licensed, which is why it is here rather than GSAP.

- [ ] **Step 2: Write the failing test**

Create `e2e/tests/site/motion.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("scrolling reaches every section", async ({ page }) => {
  await page.goto("/site/");
  for (const id of ["stack", "layer", "scenes", "palette", "surfaces", "install"]) {
    await page.locator(`#${id}`).scrollIntoViewIfNeeded();
    await expect(page.locator(`#${id}`)).toBeInViewport();
  }
});

test("reduced motion still renders the whole page", async ({ browser }) => {
  const ctx = await browser.newContext({ reducedMotion: "reduce" });
  const page = await ctx.newPage();
  await page.goto("/site/");

  await expect(page.getByRole("heading", { name: "Put it on yours" })).toBeAttached();
  const opacity = await page
    .locator("#install")
    .evaluate((el) => getComputedStyle(el).opacity);
  expect(Number(opacity)).toBe(1);

  await ctx.close();
});
```

- [ ] **Step 3: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/motion.spec.ts"`

Expected: the reduced-motion assertion fails once reveals are added in Step 4 without a guard. Run it again after Step 4.

- [ ] **Step 4: Write the choreography**

Create `site/motion.js`:

```javascript
/*
 * Motion 11 (MIT), vendored rather than installed. GSAP has the better pinning
 * story but ships under a licence that does not sit with GPL-2.0.
 *
 * Reveals are added here rather than in the markup so the page still reads if
 * this module never loads.
 */
import { animate, inView } from "./vendor/motion.min.js";

export function initMotion({ reduced }) {
  if (reduced) return;

  for (const sec of document.querySelectorAll(".sec")) {
    sec.style.opacity = "0";
    sec.style.transform = "translateY(1.5rem)";

    inView(
      sec,
      (el) => {
        animate(
          el,
          { opacity: 1, transform: "translateY(0)" },
          { duration: 0.5, easing: [0.22, 1, 0.36, 1] },
        );
      },
      { amount: 0.25 },
    );
  }
}
```

- [ ] **Step 5: Add the scroll-linked rail and the pinned pull-back in CSS**

Append to `site/styles.css`:

```css
/* Native scroll timelines where the browser has them; everything below is
   enhancement, so a browser without them simply shows the static page. */
@supports (animation-timeline: view()) {
  .rail a {
    animation: rail-lit linear both;
    animation-timeline: view();
    animation-range: cover 0% cover 100%;
  }
  @keyframes rail-lit {
    0%, 100% { color: var(--dim); }
    50% { color: var(--fg); }
  }

  .open {
    position: sticky;
    top: 0;
  }
  .open__doom {
    animation: pull-back linear both;
    animation-timeline: scroll();
    animation-range: 0 100svh;
  }
  @keyframes pull-back {
    from { scale: 1; filter: none; }
    to { scale: 0.72; filter: blur(2px); }
  }
}

@media (prefers-reduced-motion: reduce) {
  .sec { opacity: 1 !important; transform: none !important; }
  .open { position: static; }
}
```

- [ ] **Step 6: Call it from the entry point**

In `site/main.js`, add the import at the top:

```javascript
import { initMotion } from "./motion.js";
```

and inside `boot()`, immediately before `window.__aaReady = true;`:

```javascript
  try {
    initMotion({ reduced });
  } catch (err) {
    console.warn("motion unavailable; static layout stands", err);
  }
```

- [ ] **Step 7: Add the lerped scroll scrub**

Native scroll timelines track scroll exactly, with no inertia. The smoothing that
makes scroll-linked motion feel deliberate comes from the loop that is already
running for the canvas. Append to `site/motion.js`:

```javascript
/*
 * Lerped scroll position. Native scroll timelines are one-to-one with the
 * scrollbar; this trails it slightly, which is the inertia GSAP's scrub would
 * have given us. Read by main.js on the frame loop it already runs.
 */
let smoothed = 0;

export function updateScrollProgress() {
  const max = document.documentElement.scrollHeight - innerHeight;
  const raw = max > 0 ? scrollY / max : 0;
  smoothed += (raw - smoothed) * 0.08;
  document.documentElement.style.setProperty("--scroll", smoothed.toFixed(4));
  return smoothed;
}

export function scrollProgress() {
  return smoothed;
}
```

In `site/main.js`, extend the import:

```javascript
import { initMotion, updateScrollProgress } from "./motion.js";
```

and inside the `frame` function, immediately before `driver.tick(...)`:

```javascript
      if (!reduced) updateScrollProgress();
```

- [ ] **Step 8: Write the failing enhancement test**

Create `e2e/tests/site/enhance.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("command blocks gain a copy button", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/site/");

  const copy = page.locator("#surfaces").getByRole("button", { name: /copy/i }).first();
  await expect(copy).toBeVisible();
  await copy.click();

  const clip = await page.evaluate(() => navigator.clipboard.readText());
  expect(clip).toContain("cargo run -p aa");
});

test("the persistent affordance shows a depth glyph and a download", async ({ page }) => {
  await page.goto("/site/");
  const dock = page.getByRole("complementary", { name: /progress/i });

  await expect(dock.getByRole("link", { name: /download/i })).toBeVisible();
  await page.locator("#install").scrollIntoViewIfNeeded();
  await expect(dock.locator("[data-depth]")).toHaveText("@");
});
```

- [ ] **Step 9: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/enhance.spec.ts"`

Expected: FAIL, neither the copy buttons nor the affordance exist.

- [ ] **Step 10: Add the affordance markup**

In `site/index.html`, immediately before `<main id="main">`:

```html
<aside class="dock" aria-label="Progress">
  <span class="dock__depth" data-depth aria-hidden="true">.</span>
  <a class="dock__go" href="https://github.com/Builder106/ascii-arcade/releases">Download</a>
</aside>
```

- [ ] **Step 11: Write the enhancements**

Create `site/enhance.js`:

```javascript
/*
 * Additions to markup that already works. The command blocks are readable and
 * selectable without a copy button; the dock is a shortcut, not the only route
 * to the download. Nothing here is required for the page to make sense.
 */

const DEPTHS = [
  ["stack", "."],
  ["layer", "~"],
  ["scenes", ":"],
  ["palette", "="],
  ["surfaces", "*"],
  ["install", "@"],
];

export function initEnhancements() {
  addCopyButtons();
  trackDepth();
}

function addCopyButtons() {
  if (!navigator.clipboard) return;

  for (const block of document.querySelectorAll("pre.code")) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy";
    btn.textContent = "Copy";

    btn.addEventListener("click", async () => {
      await navigator.clipboard.writeText(block.innerText.trim());
      btn.textContent = "Copied";
      setTimeout(() => {
        btn.textContent = "Copy";
      }, 1600);
    });

    block.after(btn);
  }
}

function trackDepth() {
  const out = document.querySelector("[data-depth]");
  if (!out) return;

  const marks = new Map(DEPTHS);
  const watcher = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting && e.intersectionRatio > 0.5) {
          out.textContent = marks.get(e.target.id) ?? ".";
        }
      }
    },
    { threshold: [0.5] },
  );

  for (const [id] of DEPTHS) {
    const el = document.getElementById(id);
    if (el) watcher.observe(el);
  }
}
```

- [ ] **Step 12: Style the affordance**

Append to `site/styles.css`. No dot, no badge, no status indicator: a glyph and a link.

```css
.dock {
  position: fixed;
  right: 1rem;
  bottom: 1rem;
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  border: 1px solid var(--rule);
  background: var(--bg);
  padding: 0.5rem 0.85rem;
}
.dock__depth { color: var(--fg); }
.dock__go { color: var(--muted); text-decoration: none; }
.dock__go:hover,
.dock__go:focus-visible { color: var(--fg); }

.copy {
  font: inherit;
  font-size: 0.8rem;
  background: none;
  border: 1px solid var(--rule);
  color: var(--muted);
  padding: 0.25rem 0.6rem;
  cursor: pointer;
  justify-self: start;
}
.copy:hover,
.copy:focus-visible { color: var(--fg); border-color: var(--fg); }
```

- [ ] **Step 13: Wire it up**

In `site/main.js`, add the import:

```javascript
import { initEnhancements } from "./enhance.js";
```

and inside `boot()`, immediately after the `initMotion` block:

```javascript
  try {
    initEnhancements();
  } catch (err) {
    console.warn("enhancements unavailable; page still works", err);
  }
```

- [ ] **Step 14: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/"`

Expected: PASS, every spec including the earlier content and accessibility ones. Re-run the axe spec specifically, since the dock adds new foreground-on-background pairs that must clear 4.5:1 on all four palettes.

- [ ] **Step 15: Commit**

```bash
git add site/vendor site/motion.js site/enhance.js site/main.js site/index.html site/styles.css e2e/tests/site/motion.spec.ts e2e/tests/site/enhance.spec.ts
git commit -m "feat: layer choreography, scroll scrub and copy affordances over the page"
```

---

### Task 8: DOOM cold open

**Files:**

- Create: `scripts/record-doom.mjs`
- Create: `site/doom.js`
- Modify: `site/main.js`
- Create: `e2e/tests/site/doom.spec.ts`
- Generated: `site/assets/doom-attract.json`

**Interfaces:**

- Consumes: Task 6's `#doomFrame` and `#playDoom`.
- Produces:
  - `DoomSource` contract: `start()`, `stop()`, `frame(): string | null`
  - `RecordedDoom` implements it from `site/assets/doom-attract.json`
  - `mountDoom(preEl, buttonEl)` wires both and handles the absent-asset case

- [ ] **Step 1: Write the recorder**

Create `scripts/record-doom.mjs`:

```javascript
/*
 * Capture DOOM's attract-mode loop from aa-web as plain character frames.
 *
 * Runs on ampere-dev with doom_ascii already built by scripts/setup.sh. The
 * output is text, so nothing GPL is redistributed: doom_ascii itself never
 * leaves the build machine.
 */
import { writeFileSync } from "node:fs";

const URL_ = process.env.AA_WEB ?? "ws://127.0.0.1:8788/ws";
const SECONDS = Number(process.env.SECONDS ?? 6);
const FPS = 15;

const frames = [];
const ws = new WebSocket(URL_);

ws.addEventListener("message", (ev) => {
  if (frames.length >= SECONDS * FPS) {
    ws.close();
    return;
  }
  frames.push(stripAnsi(String(ev.data)));
});

ws.addEventListener("close", () => {
  writeFileSync(
    "site/assets/doom-attract.json",
    JSON.stringify({ fps: FPS, frames }),
  );
  console.log(`wrote ${frames.length} frames`);
});

function stripAnsi(s) {
  return s.replace(/\x1b\[[0-9;]*[A-Za-z]/g, "");
}
```

- [ ] **Step 2: Record the loop**

Run on the VM, with `aa-web` serving DOOM in one tmux pane:

```bash
verify-on-vm ~/CS/projects/personal/ascii-arcade "mkdir -p site/assets && AA_WEB_ENABLE_DOOM=1 cargo run -p aa-web --features doom & sleep 8 && node scripts/record-doom.mjs"
```

Expected: `site/assets/doom-attract.json` exists and is under 400 kB. If larger, drop `SECONDS` to 4. This file sits outside the 150 kB page budget because it loads only on request.

- [ ] **Step 3: Write the failing test**

Create `e2e/tests/site/doom.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("the cold open shows DOOM frames", async ({ page }) => {
  await page.goto("/site/");
  await expect
    .poll(async () => (await page.locator("#doomFrame").innerText()).length, {
      timeout: 10000,
    })
    .toBeGreaterThan(100);
});

test("a missing recording fails loudly on click, not silently", async ({ page }) => {
  await page.route("**/doom-attract.json", (r) => r.abort());
  await page.goto("/site/");
  await page.getByRole("button", { name: /play it/i }).click();
  await expect(page.getByRole("status")).toContainText(/run it locally/i);
});
```

- [ ] **Step 4: Run it to verify it fails**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/doom.spec.ts"`

Expected: FAIL, `site/doom.js` does not exist.

- [ ] **Step 5: Write the implementation**

Create `site/doom.js`:

```javascript
/*
 * DOOM as a frame source, which is how the app already treats it: one contract,
 * two implementations. RecordedDoom replays captured attract-mode frames today
 * and a WasmDoom can take its place without touching this page.
 */

export class RecordedDoom {
  constructor(data) {
    this.frames = data.frames;
    this.fps = data.fps;
    this.startedAt = 0;
    this.running = false;
  }

  start() {
    this.startedAt = performance.now();
    this.running = true;
  }

  stop() {
    this.running = false;
  }

  frame() {
    if (!this.running || this.frames.length === 0) return null;
    const elapsed = (performance.now() - this.startedAt) / 1000;
    return this.frames[Math.floor(elapsed * this.fps) % this.frames.length];
  }
}

export async function mountDoom(preEl, buttonEl) {
  let source = null;

  try {
    const res = await fetch("assets/doom-attract.json");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    source = new RecordedDoom(await res.json());
    source.start();
  } catch (err) {
    console.warn("DOOM recording unavailable", err);
  }

  if (source) {
    const draw = () => {
      const f = source.frame();
      if (f !== null) preEl.textContent = f;
      requestAnimationFrame(draw);
    };
    requestAnimationFrame(draw);
  }

  // Someone clicked this on purpose, so silence is the wrong answer.
  buttonEl.addEventListener("click", () => {
    let status = document.getElementById("doomStatus");
    if (!status) {
      status = document.createElement("p");
      status.id = "doomStatus";
      status.role = "status";
      buttonEl.after(status);
    }
    status.textContent =
      "The interactive build isn't here yet. Clone the repo and run it locally: ./scripts/setup.sh && swift run AsciiArcade";
  });
}
```

- [ ] **Step 6: Wire it up**

In `site/main.js`, add the import:

```javascript
import { mountDoom } from "./doom.js";
```

and inside `boot()`, before `initMotion`:

```javascript
  const doomFrame = document.getElementById("doomFrame");
  const playDoom = document.getElementById("playDoom");
  if (doomFrame && playDoom) mountDoom(doomFrame, playDoom);
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/"`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add scripts/record-doom.mjs site/doom.js site/main.js site/assets e2e/tests/site/doom.spec.ts
git commit -m "feat: add the recorded DOOM cold open behind a frame-source contract"
```

---

### Task 9: CI and deployment

**Files:**

- Create: `.github/workflows/pages.yml`
- Create: `e2e/tests/site/budget.spec.ts`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**

- Consumes: `scripts/build-wasm.sh`, `scripts/subset-font.sh`, the `e2e/` suite.

- [ ] **Step 1: Write the budget test**

Create `e2e/tests/site/budget.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import { statSync, readdirSync } from "node:fs";
import { join } from "node:path";

const SITE = join(__dirname, "../../../site");
const LIMIT = 150 * 1024;

test("the page stays inside its 150 kB budget", () => {
  const counted = ["index.html", "styles.css", "main.js", "renderer.js",
                   "dissolve.js", "engine.js", "doom.js", "motion.js",
                   "enhance.js"];

  let total = counted.reduce((n, f) => n + statSync(join(SITE, f)).size, 0);
  total += statSync(join(SITE, "vendor/motion.min.js")).size;

  for (const f of readdirSync(join(SITE, "fonts"))) {
    if (f.endsWith(".woff2")) total += statSync(join(SITE, "fonts", f)).size;
  }
  for (const f of readdirSync(join(SITE, "pkg"))) {
    if (f.endsWith(".wasm") || f.endsWith(".js")) {
      total += statSync(join(SITE, "pkg", f)).size;
    }
  }

  // DOOM payloads are deliberately outside this: they load only on request.
  expect(total).toBeLessThan(LIMIT);
});
```

- [ ] **Step 2: Run it**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/budget.spec.ts"`

Expected: PASS. If it fails, the WebAssembly module is the likely cause; apply the `opt-level = "z"` and `lto = true` change noted in Task 1 Step 7 and rebuild. Never raise `LIMIT`.

- [ ] **Step 3: Write the deploy workflow**

Create `.github/workflows/pages.yml`:

```yaml
name: Pages

on:
  push:
    branches: [main]
    paths: ['site/**', 'crates/aa-core/**', 'crates/aa-wasm/**', 'scripts/build-wasm.sh', '.github/workflows/pages.yml']
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: bash scripts/build-wasm.sh
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deploy.outputs.page_url }}
    steps:
      - id: deploy
        uses: actions/deploy-pages@v4
```

- [ ] **Step 4: Add the WebAssembly build to CI**

In `.github/workflows/ci.yml`, add a job that fails the build when the wasm target breaks:

```yaml
  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo build -p aa-wasm --release --target wasm32-unknown-unknown
```

- [ ] **Step 5: Verify the workflow parses**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade "python3 -c \"import yaml,sys; yaml.safe_load(open('.github/workflows/pages.yml')); yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')\""`

Expected: `ok`.

- [ ] **Step 6: Run the whole suite once more**

Run: `verify-on-vm ~/CS/projects/personal/ascii-arcade/e2e "npx playwright test tests/site/"`

Expected: PASS across content, accessibility, renderer, dissolve, engine, motion, doom and budget.

- [ ] **Step 7: Commit**

```bash
git add .github/workflows e2e/tests/site/budget.spec.ts
git commit -m "ci: build the wasm target, gate the page budget, deploy to Pages"
```

---

## Notes for the implementer

**Enabling Pages.** The workflow deploys but the repository setting is manual: Settings, Pages, Source, GitHub Actions. Nothing publishes until that is switched on, and switching it on is the owner's call rather than something to do unasked.

**The `pkg/` directory.** `site/pkg/` is generated by `scripts/build-wasm.sh`. Committing it keeps a plain `python3 -m http.server` working with no build, which suits a repository that has stayed dependency-light. Not committing it means CI is the only way to get a working page. Either is defensible; ask before deciding, and add `site/pkg/` to `.gitignore` if it stays out.

**A known gap, out of scope here.** `crates/aa-render/src/font.rs` covers ASCII `0x20..=0x7E` only and `glyph_bitmap()` returns `None` past that, so Life's `█` and Pipes' six box-drawing characters have no bitmap on the Linux and Windows shells. It does not affect this site, which uses its own font. It is written up at the end of `docs/landing-page-design.md` and wants confirming on a Linux shell before anyone fixes it.
