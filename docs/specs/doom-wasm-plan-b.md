# Interactive DOOM — Plan B: Input, Session Lifecycle, Licensing, Accessibility

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Plan A's walking skeleton (DOOM's boot screen rendering through the canvas, no input) into an actually playable session: keyboard and touch input, a real start/stop lifecycle with focus and scroll management, the GPL-2.0 licensing artifacts the feature has required since `docs/doom-wasm-design.md`'s first line, and an accessibility bar the site's own a11y suite actually checks instead of silently excluding.

**Architecture:** No new C code and no rebuild — Plan A's `wasm_push_key(int pressed, unsigned char key)` (`patches/doom-wasm/doomgeneric_wasm.c`) is already compiled in and exported. Plan B is entirely JS: a keyboard/touch-to-doomkey translation layer in `site/doom-play.js`that calls the existing export, a session object replacing the current fire-and-forget`loadDoomSkeleton`, new touch-control markup and CSS, a vendored license file, and test coverage for all of it.

**Tech Stack:** Vanilla ES modules (no new dependency), the existing `site/renderer.js` `Renderer`, doomgeneric's own key constants (verified against the pinned commit's `src/doomkeys.h`, not recalled from memory).

## Global Constraints

- `doom-ascii`stays pinned to`b5188d7c9c4da6c81264a7803e8725ac3df2cfea` — same commit Plan A pinned, unchanged by this plan.
- No new C changes and no rebuild of `doom-wasm/`: `wasm_push_key`already exists and is already in`EXPORTED_FUNCTIONS`(verified:`scripts/build-doom-wasm.sh`'s `emcc`invocation already lists`_wasm_push_key`). If any task below discovers this assumption is wrong, stop and say so rather than silently patching the build script — that would be new, unplanned scope.
- All builds/tests run on `ampere-dev`via`verify-on-vm`, never locally, per this environment's standing rule.
- No new JS dependencies — every file this plan touches stays a vanilla ES module.
- `budget.spec.ts`'s 150 kB budget does not grow to include `doom-wasm/` (already true, unchanged) — this plan's touch-control CSS/markup is small enough it must not push the *page* budget (currently 147,104 / 153,600 bytes, ~6.5 kB headroom per the last measurement) over the limit; check it after Task 2 and Task 5, the two tasks adding the most CSS/HTML.
- Every mapped keyboard code is verified against the pinned commit's actual `src/doomkeys.h` (reproduced in Task 1) — no guessed key codes.

---

## Task 1: Keyboard input

**Files:**

- Modify: `site/doom-play.js` — add the key-translation table and keyboard wiring.
- Test: `e2e/tests/site/doom-play.spec.ts` (new file — this task creates it; later tasks add to it).

**Interfaces:**

- Consumes: `mod.cwrap("wasm_push_key", null, ["number", "number"])` — the existing Plan A export (`patches/doom-wasm/doomgeneric_wasm.c:33-44`), signature `void wasm_push_key(int pressed, unsigned char key)`.
- Produces: `pushDoomKey(mod, pressed, code)`— a small helper Task 2 (touch) reuses so both input sources funnel through one function.`KEY_MAP`(a plain object,`KeyboardEvent.code`string → doomgeneric key byte) — Task 2 does not consume this directly (touch buttons hardcode their own key bytes), but Task 5's accessibility work references it when writing the keyboard-instructions`aria-label`.

- [ ] **Step 1: Reproduce and verify the real key constants**

Already pulled from the pinned commit's `src/doomkeys.h` during planning (not re-derive from memory):

```c

# define KEY_RIGHTARROW    0xae

# define KEY_LEFTARROW    0xac

# define KEY_UPARROW        0xad

# define KEY_DOWNARROW    0xaf

# define KEY_USE            0xa2

# define KEY_FIRE        0xa3

# define KEY_ESCAPE        27

# define KEY_ENTER        13

```

- [ ] **Step 2: Add the translation table and push helper to `site/doom-play.js`**

Add near the top of `site/doom-play.js`, after the existing imports:

```javascript
// doomgeneric's own key constants, verified against the pinned commit's
// src/doomkeys.h directly (not recalled from memory or another source
// port's conventions — vanilla Doom's byte values are not ASCII for the
// non-printable ones). Multiple JS codes map to the same doomgeneric
// constant (WASD alongside arrows): doomgeneric only understands its own
// fixed key space, not which physical key produced an event.
const KEY_UPARROW = 0xad;
const KEY_DOWNARROW = 0xaf;
const KEY_LEFTARROW = 0xac;
const KEY_RIGHTARROW = 0xae;
const KEY_USE = 0xa2;
const KEY_FIRE = 0xa3;
const KEY_ESCAPE = 27;
const KEY_ENTER = 13;

const KEY_MAP = {
  ArrowUp: KEY_UPARROW,
  ArrowDown: KEY_DOWNARROW,
  ArrowLeft: KEY_LEFTARROW,
  ArrowRight: KEY_RIGHTARROW,
  KeyW: KEY_UPARROW,
  KeyS: KEY_DOWNARROW,
  KeyA: KEY_LEFTARROW,
  KeyD: KEY_RIGHTARROW,
  ControlLeft: KEY_FIRE,
  ControlRight: KEY_FIRE,
  Space: KEY_USE,
  Enter: KEY_ENTER,
  Escape: KEY_ESCAPE,
};

// Touch controls (Task 2) call this directly with a hardcoded key byte;
// keyboard listeners (below) go through KEY_MAP first. Either way, this is
// the single funnel into wasm_push_key — doomgeneric never knows which
// input source produced an event.
function pushDoomKey(mod, pressed, key) {
  mod.cwrap("wasm_push_key", null, ["number", "number"])(pressed ? 1 : 0, key);
}
```

`mod.cwrap`re-wraps the same export on every call above — cheap (it's a closure over an existing WASM function pointer, not a new lookup) but wasteful at input-event frequency. Cache it once per session in Step 3 instead of calling`pushDoomKey` as written above verbatim; the cache is threaded through in that step.

- [ ] **Step 3: Wire keydown/keyup listeners into the session, gated to when a session is active**

`loadDoomSkeleton` currently starts painting immediately and has no concept of "listening for input." Modify its body (`site/doom-play.js`) to cache the push function once and attach listeners for the session's lifetime:

```javascript
export async function loadDoomSkeleton(canvas) {
  const mod = await (await import("./doom-wasm/doom.js")).default({
    arguments: ["-iwad", "/freedoom1.wad"],
    locateFile: (path) => new URL(`./doom-wasm/${path}`, import.meta.url).pathname,
  });

  const push = mod.cwrap("wasm_push_key", null, ["number", "number"]);

  const getBuffer = mod.cwrap("wasm_get_screen_buffer", "number", []);
  const getWidth = mod.cwrap("wasm_get_screen_width", "number", []);
  const getHeight = mod.cwrap("wasm_get_screen_height", "number", []);

  const width = getWidth();
  const height = getHeight();
  const renderer = new Renderer(canvas);
  const rect = canvas.getBoundingClientRect();

  const estimatedCell = cellAspectAt100();
  let gridCols = gridColsForBox(rect, height, estimatedCell.h / estimatedCell.w);
  let fontPx = fitDoomFont(rect, gridCols, height);
  renderer.resize(rect.width, rect.height, fontPx);

  gridCols = gridColsForBox(rect, height, renderer.cell.h / renderer.cell.w);
  fontPx = fitDoomFont(rect, gridCols, height);
  renderer.resize(rect.width, rect.height, fontPx);

  renderer.cols = gridCols;
  renderer.rows = height;

  const onKeyDown = (e) => {
    const key = KEY_MAP[e.code];
    if (key === undefined) return;
    e.preventDefault();
    push(1, key);
  };
  const onKeyUp = (e) => {
    const key = KEY_MAP[e.code];
    if (key === undefined) return;
    e.preventDefault();
    push(0, key);
  };
  addEventListener("keydown", onKeyDown);
  addEventListener("keyup", onKeyUp);

  let running = true;
  const themeColor = { r: 200, g: 200, b: 200 };

  const draw = () => {
    if (!running) return;
    const ptr = getBuffer();
    if (ptr !== 0) {
      const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
      const { glyphs, colors } = pixelsToGlyphs(pixels, width, height, gridCols);
      renderer.paint(glyphs, colors, themeColor);
    }
    requestAnimationFrame(draw);
  };
  requestAnimationFrame(draw);

  return {
    push,
    stop() {
      running = false;
      removeEventListener("keydown", onKeyDown);
      removeEventListener("keyup", onKeyUp);
    },
  };
}
```

Notes on this diff against Plan A's version: `push`is cached once and returned on the handle (Task 2's touch controls and Task 3's session wrapper both need to call it without re-deriving`mod`). `onKeyDown`/`onKeyUp`only`preventDefault()`for codes actually in`KEY_MAP`— an unmapped key (say, a browser devtools shortcut) passes through untouched. Listeners are attached on`window`, not `canvas`, deliberately: a canvas needs explicit focus management to receive key events reliably across browsers (Task 3's job), and until that lands, window-level listeners are what makes input work at all during this task's own test.

- [ ] **Step 4: Write the first test in the new spec file**

Create `e2e/tests/site/doom-play.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("keyboard input reaches doom-wasm's key queue", async ({ page }) => {
  await page.goto("/site/");

  const result = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    document.body.appendChild(canvas);

    const handle = await loadDoomSkeleton(canvas);
    await new Promise((resolve) => setTimeout(resolve, 1500));

    // wasm_push_key has no observable return value and DG_ScreenBuffer's
    // content depends on game state, not directly on key presses in a way
    // a test can assert against cheaply. What's actually testable without
    // reaching into the module's internals: calling handle.push() doesn't
    // throw, and dispatching a mapped keydown while a session is active
    // calls preventDefault (proving the listener is wired and the code is
    // recognized), while an unmapped key does not.
    let threw = false;
    try {
      handle.push(1, 0xad); // KEY_UPARROW
      handle.push(0, 0xad);
    } catch {
      threw = true;
    }

    const mappedEvent = new KeyboardEvent("keydown", { code: "ArrowUp", cancelable: true });
    dispatchEvent(mappedEvent);
    const mappedPrevented = mappedEvent.defaultPrevented;

    const unmappedEvent = new KeyboardEvent("keydown", { code: "KeyQ", cancelable: true });
    dispatchEvent(unmappedEvent);
    const unmappedPrevented = unmappedEvent.defaultPrevented;

    handle.stop();
    canvas.remove();
    return { threw, mappedPrevented, unmappedPrevented };
  });

  expect(result.threw).toBe(false);
  expect(result.mappedPrevented).toBe(true);
  expect(result.unmappedPrevented).toBe(false);
});
```

- [ ] **Step 5: Run it**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/doom-play.spec.ts"
```

Expected: PASS. Also re-run `doom-play-skeleton.spec.ts`(Plan A's own test) to confirm this task's changes to`loadDoomSkeleton` didn't regress it:

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/doom-play-skeleton.spec.ts tests/site/doom.spec.ts"
```

Expected: all still PASS — `doom-play-skeleton.spec.ts`never calls`handle.push`, `handle.stop()` still exists with the same signature.

- [ ] **Step 6: Commit**

```bash
git add site/doom-play.js e2e/tests/site/doom-play.spec.ts
git commit -m "feat: map keyboard input into doom-wasm's key queue"
```

---

## Task 2: Touch controls

**Files:**

- Modify: `site/doom-play.js` — build and wire the on-screen control overlay.
- Modify: `site/styles.css`—`.doom-controls` and children.
- Test: `e2e/tests/site/doom-play.spec.ts` (append).

**Interfaces:**

- Consumes: `pushDoomKey`-equivalent (the cached `push`function from Task 1's handle) and the same`KEY_UPARROW`/`KEY_DOWNARROW`/`KEY_LEFTARROW`/`KEY_RIGHTARROW`/`KEY_FIRE`/`KEY_USE`/`KEY_ENTER`/`KEY_ESCAPE` constants Task 1 defined.
- Produces: `buildTouchControls(mount, push)`— called by`loadDoomSkeleton`, returns an object with `{ el, destroy() }` so Task 3's session teardown can remove it.

- [ ] **Step 1: Add the touch-control builder to `site/doom-play.js`**

```javascript
// Shown only during an active session, and only on a touch-capable device
// — a mouse/keyboard visitor never sees this. Feature-detected rather than
// UA-sniffed: matchMedia("(pointer: coarse)") covers touchscreen laptops
// too, not just phones, matching the design's "never mutually exclusive
// with keyboard" intent (a touchscreen laptop with a keyboard gets both).
function touchCapable() {
  return matchMedia("(pointer: coarse)").matches || "ontouchstart" in window;
}

const TOUCH_BUTTONS = [
  { label: "↑", key: KEY_UPARROW, className: "doom-controls__up" },
  { label: "↓", key: KEY_DOWNARROW, className: "doom-controls__down" },
  { label: "←", key: KEY_LEFTARROW, className: "doom-controls__left" },
  { label: "→", key: KEY_RIGHTARROW, className: "doom-controls__right" },
  { label: "Fire", key: KEY_FIRE, className: "doom-controls__fire" },
  { label: "Use", key: KEY_USE, className: "doom-controls__use" },
  { label: "Enter", key: KEY_ENTER, className: "doom-controls__enter" },
  { label: "Esc", key: KEY_ESCAPE, className: "doom-controls__esc" },
];

function buildTouchControls(mount, push) {
  const el = document.createElement("div");
  el.className = "doom-controls";
  el.setAttribute("role", "group");
  el.setAttribute("aria-label", "Touch controls");

  const cleanups = [];
  for (const spec of TOUCH_BUTTONS) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `doom-controls__btn ${spec.className}`;
    btn.textContent = spec.label;

    // touchstart/touchend, not click: click fires after a delay on touch
    // devices and doesn't give a press/release pair at all, which is what
    // a game button needs (holding "fire" must keep firing, not act like
    // a single click). preventDefault on touchstart stops the synthetic
    // mouse events + scroll gesture Safari/Chrome would otherwise generate
    // from the touch.
    const onStart = (e) => {
      e.preventDefault();
      push(1, spec.key);
    };
    const onEnd = (e) => {
      e.preventDefault();
      push(0, spec.key);
    };
    btn.addEventListener("touchstart", onStart, { passive: false });
    btn.addEventListener("touchend", onEnd, { passive: false });
    btn.addEventListener("touchcancel", onEnd, { passive: false });
    cleanups.push(() => {
      btn.removeEventListener("touchstart", onStart);
      btn.removeEventListener("touchend", onEnd);
      btn.removeEventListener("touchcancel", onEnd);
    });

    el.append(btn);
  }

  mount.append(el);

  return {
    el,
    destroy() {
      for (const fn of cleanups) fn();
      el.remove();
    },
  };
}
```

- [ ] **Step 2: Wire it into `loadDoomSkeleton`, gated to touch-capable devices**

In `site/doom-play.js`, after the `addEventListener("keydown", onKeyDown)`/`addEventListener("keyup", onKeyUp)` lines added in Task 1, add:

```javascript
  const touchControls = touchCapable() ? buildTouchControls(canvas.parentElement, push) : null;
```

And in the returned handle's `stop()`:

```javascript
    stop() {
      running = false;
      removeEventListener("keydown", onKeyDown);
      removeEventListener("keyup", onKeyUp);
      touchControls?.destroy();
    },
```

`canvas.parentElement`is`#open`(the hero section) at the point this runs, per`site/doom.js`'s existing `preEl.replaceWith(canvas)` — confirm this still holds when Task 3 restructures the DOM around the canvas; if Task 3 wraps the canvas in a new container, update this to mount into that wrapper instead so the controls overlay positions relative to the right element.

- [ ] **Step 3: Add the CSS**

Add to `site/styles.css`, near `.gallery` (same "bordered box, no dot indicators, no left rail" idiom already established in this file):

```css
.doom-controls {
  position: absolute;
  inset: auto 0 0 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.4rem;
  padding: 0.6rem;
  background: color-mix(in oklab, var(--bg) 80%, transparent);
}
.doom-controls__btn {
  font: inherit;
  font-size: 0.9rem;
  padding: 0.6rem 0;
  border: 1px solid var(--rule);
  background: var(--bg);
  color: var(--fg);
  cursor: pointer;
  /*Touch targets, not mouse targets: 44px is the standard minimum.*/
  min-height: 44px;
}
.doom-controls__btn:active {
  border-color: var(--fg);
  background: color-mix(in oklab, var(--fg) 12%, transparent);
}
```

`.open__doom`/`#doomPlayCanvas`needs`position: relative`for the controls'`position: absolute`to anchor to the canvas's own box rather than the page. Check the current rule in`site/styles.css` (`.open__doom`) — it has no `position`declared today (relies on being a normal grid child of`.open`), so add `position: relative`to the canvas element itself at creation time in`site/doom.js` (`canvas.style.position = "relative"`) rather than editing `.open__doom`globally, since the attract-mode`<pre>` sharing that class has no reason to become a positioning context.

- [ ] **Step 4: Extend the test**

Append to `e2e/tests/site/doom-play.spec.ts`:

```typescript
test("touch controls appear only on touch-capable devices and feed the same key queue", async ({
  browser,
}) => {
  const context = await browser.newContext({ hasTouch: true, isMobile: true });
  const page = await context.newPage();
  await page.goto("/site/");

  const found = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    const wrap = document.createElement("div");
    wrap.appendChild(canvas);
    document.body.appendChild(wrap);

    const handle = await loadDoomSkeleton(canvas);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const hasControls = !!document.querySelector(".doom-controls");
    handle.stop();
    const removedAfterStop = !document.querySelector(".doom-controls");
    wrap.remove();
    return { hasControls, removedAfterStop };
  });

  expect(found.hasControls).toBe(true);
  expect(found.removedAfterStop).toBe(true);
  await context.close();
});

test("touch controls do not appear on a non-touch device", async ({ page }) => {
  await page.goto("/site/");
  const hasControls = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    document.body.appendChild(canvas);
    const handle = await loadDoomSkeleton(canvas);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const result = !!document.querySelector(".doom-controls");
    handle.stop();
    canvas.remove();
    return result;
  });
  expect(hasControls).toBe(false);
});
```

- [ ] **Step 5: Run it**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/doom-play.spec.ts"
```

Expected: all 3 tests (Task 1's + this task's 2) PASS.

- [ ] **Step 6: Check the page budget**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/budget.spec.ts"
```

Expected: PASS. If it fails, the CSS added in Step 3 pushed the page over 150 kB — trim before continuing rather than deferring the fix to a later task.

- [ ] **Step 7: Commit**

```bash
git add site/doom-play.js site/styles.css e2e/tests/site/doom-play.spec.ts
git commit -m "feat: add touch controls for doom-wasm, shown only on touch-capable devices"
```

---

## Task 3: Session lifecycle — focus, scroll lock, Stop button, ambient background pause

**Files:**

- Modify: `site/doom-play.js` — scroll lock, focus management, richer session handle.
- Modify: `site/doom.js`— Stop button, wiring the ambient`SceneDriver` pause/resume.
- Modify: `site/main.js`— expose the ambient driver's pause/resume to`doom.js`(currently`driver`is a local variable inside`boot()`, not reachable from `mountDoom`).
- Modify: `site/index.html` — Stop button markup.
- Modify: `site/styles.css`— Stop button styling (reuse`.btn`, no new class needed beyond that).
- Test: `e2e/tests/site/doom-play.spec.ts` (append).

**Interfaces:**

- Consumes: `SceneDriver`from`site/engine.js`— no signature change, just calling it from a new location.`driver.tick(t)`already exists; this task adds nothing to`SceneDriver`itself, only stops *calling*`tick` for the ambient ones during a session and resumes after.
- Produces: `loadDoomSkeleton(canvas, { onSessionEnd })`— extends Task 1/2's signature with an options object;`onSessionEnd`fires once, whether the session ended via the Stop button or (pending Step 2's verification) DOOM's own Quit menu path, so`doom.js` has one place to restore attract mode regardless of which path triggered it.

- [ ] **Step 1: Add scroll lock and focus management to `loadDoomSkeleton`**

In `site/doom-play.js`, before the `addEventListener("keydown", ...)` line:

```javascript
  const previousOverflow = document.documentElement.style.overflow;
  document.documentElement.style.overflow = "hidden";
  canvas.tabIndex = 0;
  canvas.focus();
```

And in the returned handle's `stop()`, restore both:

```javascript
    stop() {
      running = false;
      removeEventListener("keydown", onKeyDown);
      removeEventListener("keyup", onKeyUp);
      touchControls?.destroy();
      document.documentElement.style.overflow = previousOverflow;
    },
```

Restoring `previousOverflow`(its actual prior value, likely`""`) rather than unconditionally setting `""`matters if some other feature ever sets`overflow`on`<html>` for an unrelated reason — this doesn't clobber that.

- [ ] **Step 2: Empirically verify what happens when DOOM's own menu "Quit" is selected**

This is the one genuinely unverified claim carried over from `docs/doom-wasm-design.md` ("Escape doubles as the exit path... whose 'quit' already terminates the session" — asserted there, not yet confirmed against this actual WASM build). Check it now rather than assuming:

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && node -e '
import(\"../site/doom-wasm/doom.js\").then(async (m) => {
  const mod = await m.default({
    arguments: [\"-iwad\", \"/freedoom1.wad\"],
    onExit: (code) => console.log(\"ON_EXIT\", code),
    print: (t) => console.log(\"[out]\", t),
    printErr: (t) => console.log(\"[err]\", t),
  });
  const push = mod.cwrap(\"wasm_push_key\", null, [\"number\", \"number\"]);
  await new Promise((r) => setTimeout(r, 1500));
  push(1, 27); push(0, 27);
  await new Promise((r) => setTimeout(r, 500));
  push(1, 0xaf); push(0, 0xaf);
  push(1, 0xaf); push(0, 0xaf);
  push(1, 13); push(0, 13);
  await new Promise((r) => setTimeout(r, 1500));
  console.log(\"DONE, still alive\");
}).catch((e) => console.log(\"THREW\", e && e.message));
' --experimental-vm-modules"
```

This opens the menu (Escape), navigates down twice (toward "Quit", though the exact menu position of "Quit" varies by menu depth — adjust the down-count if `DONE, still alive`prints without an`ON_EXIT` line, meaning the navigation didn't actually land on Quit), then confirms with Enter.

Two possible outcomes, and what each means for this task:

- **`ON_EXIT`prints, or the process throws with an`ExitStatus`-shaped error**: Emscripten's `exit()`path is reachable and observable. Pass`onExit`in the module options inside`loadDoomSkeleton`and call the same teardown`stop()` already does from inside it — a real, working "Escape-to-quit" path, exactly as the design doc describes. Implement this.
- **`DONE, still alive`prints with no`ON_EXIT`**: this build's Emscripten runtime doesn't tear down or surface the exit in an observable way (plausible — `EXIT_RUNTIME`isn't set in`build-doom-wasm.sh`'s `emcc`flags, and without it`exit()`is close to a no-op). In this case, **do not** implement an unreliable quit-detection path. Keep the visible Stop button as the only authoritative way to end a session, and correct`docs/doom-wasm-design.md`'s "Escape doubles as the exit path" claim — Escape still opens DOOM's own menu (real, harmless), it just doesn't auto-teardown the *page's* session state on Quit. Note this correction in this task's commit message so it's not silently contradicted later.

Whichever outcome occurs, write it down here before moving to Step 3 — the rest of this task's shape depends on which branch is real, not on which one `docs/doom-wasm-design.md` guessed.

- [ ] **Step 3: Add the Stop button to the hero markup**

In `site/index.html`, next to the existing `#playDoom` button:

```html
<button class="btn btn--go" id="playDoom" type="button">Play it</button>
<button class="btn" id="stopDoom" type="button" hidden>Stop</button>
<a class="btn" href="https://github.com/Builder106/ascii-arcade/releases">Download</a>
```

`hidden`by default —`site/doom.js` un-hides it only once a session actually starts, matching "always visible" *during play*, not before.

- [ ] **Step 4: Wire the Stop button and ambient-driver pause/resume through `site/main.js`and`site/doom.js`**

`site/main.js`'s `boot()`currently keeps`driver`(the ambient`SceneDriver`) as a local variable with no way for `mountDoom` to reach it. Thread a pause/resume pair through instead of exposing the whole driver:

In `site/main.js`, change the `mountDoom` call:

```javascript
  const doomFrame = document.getElementById("doomFrame");
  const playDoom = document.getElementById("playDoom");
  const stopDoom = document.getElementById("stopDoom");
  let ambientPaused = false;
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
```

And gate the ambient tick in the existing `frame()` loop:

```javascript
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
    requestAnimationFrame(draw);
  };
```

(The gallery tick is paused along with the main ambient scene — both are background decoration competing for the same frame budget a live DOOM session needs; pausing one and not the other would be an arbitrary distinction.)

Note the typo risk here: the existing `frame`function's`requestAnimationFrame(draw)`call at the end — confirm the actual identifier in the current file is`frame` recursing into itself (`requestAnimationFrame(frame)`), not `draw` (`draw`is`doom-play.js`'s own separate render loop, a different function entirely). Copy the real trailing line from the current `site/main.js`, don't retype it from this plan's memory of it.

- [ ] **Step 5: Update `mountDoom`'s signature and click handlers in `site/doom.js`**

```javascript
export async function mountDoom(preEl, buttonEl, stopButtonEl, { pauseAmbient, resumeAmbient } = {}) {
  let source = null;
  let attractRunning = true;
  let activeSession = null;

  const startAttract = () => {
    // ... unchanged from the current file ...
  };

  // ... unchanged fetch/RecordedDoom setup and status() helper ...

  function endSession() {
    activeSession?.stop();
    activeSession = null;
    stopButtonEl.hidden = true;
    resumeAmbient?.();
    buttonEl.disabled = false;
    buttonEl.textContent = "Play it";
    buttonEl.focus();
  }

  stopButtonEl.addEventListener("click", endSession);

  buttonEl.addEventListener(
    "click",
    async () => {
      attractRunning = false;
      buttonEl.disabled = true;
      buttonEl.textContent = "Loading…";
      status("Loading DOOM (about 27MB — this only happens once)…");

      const canvas = document.createElement("canvas");
      canvas.id = "doomPlayCanvas";
      canvas.className = "open__doom";
      canvas.style.display = "block";
      canvas.style.position = "relative";
      canvas.setAttribute("role", "application");
      canvas.setAttribute(
        "aria-label",
        "DOOM, playable. Arrow keys or WASD to move, Control to fire, Space to use, Enter to confirm menu selections, Escape for the in-game menu.",
      );
      preEl.replaceWith(canvas);

      try {
        pauseAmbient?.();
        activeSession = await loadDoomSkeleton(canvas, { onSessionEnd: endSession });
        stopButtonEl.hidden = false;
        buttonEl.textContent = "Live";
        status(
          "That's real DOOM, compiled to WebAssembly and running live in your browser right now — not a recording. Arrow keys or WASD to move, Control to fire, Space to use.",
        );
      } catch (err) {
        console.warn("doom-wasm unavailable", err);
        resumeAmbient?.();
        buttonEl.disabled = false;
        buttonEl.textContent = "Play it";
        canvas.replaceWith(preEl);
        startAttract();
        status(
          "Couldn't load DOOM in this browser. Clone the repo and run ./scripts/setup.sh && swift run AsciiArcade instead.",
        );
      }
    },
    { once: true },
  );
}
```

`role="application"`replaces the Plan A placeholder`role="img"`— this canvas is now a real interactive game surface, not a static image;`role="application"`is the correct ARIA role for a canvas-driven game where the site can't offer a non-visual equivalent (matches`docs/doom-wasm-design.md`'s "Accessibility" section's own framing: "a deliberately minimal bar... consistent with how most canvas-rendered browser games ship").

`onSessionEnd`in`loadDoomSkeleton`'s options — wire it only if Step 2 found a working exit-detection path; otherwise, this callback is unused (accept it as a parameter for API stability but only call it from `stop()`itself, which`endSession`already calls directly via`activeSession?.stop()`— meaning in the "Stop-button-only" branch,`onSessionEnd`and`stop()`collapse to the same call site and`onSessionEnd` never needs to fire on its own; still worth keeping the parameter so this doesn't need another signature change if a later, more capable doom-wasm build ever does support it).

- [ ] **Step 6: Add the CSS for the Stop button's `hidden`interaction with`.open__acts`'s flex layout**

`.open__acts { display: flex; gap: 0.75rem; flex-wrap: wrap; }`(existing rule) already handles a`hidden`button correctly —`hidden`maps to`display: none`by the UA stylesheet regardless of the parent's`display: flex`, so no new CSS is needed here. Confirm this by inspection rather than adding a redundant `.btn[hidden] { display: none; }` override.

- [ ] **Step 7: Extend the test**

Append to `e2e/tests/site/doom-play.spec.ts`:

```typescript
test("Stop button ends the session, restores scroll, and returns focus", async ({ page }) => {
  await page.goto("/site/");
  await page.getByRole("button", { name: /play it/i }).click();
  await expect(page.getByRole("button", { name: "Stop" })).toBeVisible({ timeout: 15000 });

  const overflowDuring = await page.evaluate(
    () => document.documentElement.style.overflow,
  );
  expect(overflowDuring).toBe("hidden");

  await page.getByRole("button", { name: "Stop" }).click();
  await expect(page.getByRole("button", { name: "Stop" })).toBeHidden();
  await expect(page.getByRole("button", { name: /play it/i })).toBeFocused();

  const overflowAfter = await page.evaluate(
    () => document.documentElement.style.overflow,
  );
  expect(overflowAfter).not.toBe("hidden");
});
```

- [ ] **Step 8: Run it**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/doom-play.spec.ts tests/site/doom.spec.ts tests/site/motion.spec.ts"
```

Expected: all PASS. `doom.spec.ts`and`motion.spec.ts`are re-run because this task changed`mountDoom`'s signature and the ambient scroll/scene-switch loop in `main.js` — both have existing coverage that would catch a wiring regression.

- [ ] **Step 9: Commit**

```bash
git add site/doom-play.js site/doom.js site/main.js site/index.html e2e/tests/site/doom-play.spec.ts
git commit -m "feat: add DOOM session lifecycle — Stop button, scroll lock, focus, ambient-scene pause"
```

Commit body: note Step 2's actual finding (which branch was real) so it's on record, not just in this plan.

---

## Task 4: Preload courtesy

**Files:**

- Modify: `site/main.js` — schedule the background preload.

**Interfaces:**

- Consumes: none new — reuses the existing dynamic `import("./doom-wasm/doom.js")`already inside`loadDoomSkeleton` (`site/doom-play.js`), just triggering it earlier under idle conditions instead of only on click.
- Produces: nothing new exported — this is a scheduling change only.

**Reconciling this task with `docs/doom-wasm-design.md`'s "WAD hosting and preload" section:** that section assumed `freedoom1.wad`would be a separate fetch from`doom.wasm`, hosted as its own GitHub Release asset. Plan A instead bundled the WAD into `doom.data`via Emscripten's`--preload-file` (`scripts/build-doom-wasm.sh`), so `doom.js`/`doom.wasm`/`doom.data`are one unit, fetched together the moment`import("./doom-wasm/doom.js")`runs and the module's own runtime pulls its`.data` file. There is no separate WAD-fetch step left to preload independently — the whole bundle is the thing to preload, or not. This task adapts the design's intent (background-fetch during idle time, skip on a metered connection) to the bundle that actually exists.

- [ ] **Step 1: Add the idle-scheduled preload to `boot()`**

In `site/main.js`, after the existing `mountDoom`wiring (end of`boot()`, before `window.__aaReady = true;`):

```javascript
  // Preload courtesy: fetch the ~27MB doom-wasm bundle in the background
  // once the page has settled, so a later click on "Play it" is a near-
  // instant start rather than the visitor's first-ever wait on it. Skipped
  // on a metered connection — costing every visitor this weight regardless
  // of whether they ever press play would be the wrong tradeoff there.
  const saveData = navigator.connection?.saveData || matchMedia("(prefers-reduced-data: reduce)").matches;
  if (!saveData && "requestIdleCallback" in window) {
    requestIdleCallback(() => {
      import("./doom-wasm/doom.js").catch((err) => {
        console.warn("doom-wasm preload failed; will retry on Play click", err);
      });
    });
  }
```

This only preloads the *module* (`doom.js`/`.wasm`/`.data`fetch + parse) — it does not call the default-exported factory function (that still only happens inside`loadDoomSkeleton`, on click), so no WASM instance is actually instantiated or running until the visitor presses Play. A failed preload is silently swallowed (logged, not surfaced) because Task 3's existing click-handler `catch`block already has a real, visible failure path for when`loadDoomSkeleton` itself fails — this preload failing just means the click-time fetch does the work instead, at the cost of a slightly slower first click, not a broken feature.

`requestIdleCallback`has no Safari support as of this writing; the`"requestIdleCallback" in window` guard means Safari visitors simply don't get the preload (click-time fetch only) rather than crashing on a missing global — an acceptable degradation, not a bug to work around with a polyfill (no new dependency, per this plan's constraints).

- [ ] **Step 2: Verify manually that a preload actually happens and doesn't instantiate the module**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && node -e '
const { chromium } = require(\"playwright\");
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const doomRequests = [];
  page.on(\"request\", (r) => { if (r.url().includes(\"doom-wasm\")) doomRequests.push(r.url()); });
  await page.goto(\"http://127.0.0.1:8899/site/\");
  await page.waitForTimeout(4000);
  console.log(\"doom-wasm requests before any click:\", doomRequests.length, doomRequests);
  await browser.close();
})();
'"
```

Expected: at least one `doom-wasm/doom.js`(and likely`.wasm`/`.data`) request fires within the 4s window, with no click ever performed — proving the idle-scheduled preload actually ran. (Run this against the live `dev-on-vm`server, same as this session's earlier manual verification passes — not the`verify-on-vm` Playwright webServer, since this is a one-off manual check, not part of the suite.)

- [ ] **Step 3: Commit**

```bash
git add site/main.js
git commit -m "feat: preload doom-wasm in the background, skipped on metered connections"
```

---

## Task 5: GPL-2.0 licensing artifacts

**Files:**

- Create: `LICENSES/doom-ascii.GPL-2.0`— vendored from the pinned commit's own`LICENSE` file.
- Modify: `site/index.html` — footer link.

**Interfaces:** none — static artifacts and a link, no code interface.

- [ ] **Step 1: Vendor the license text from the pinned commit itself**

```bash
ssh ampere-dev "rm -rf /tmp/doom-license-vendor && git clone -q https://github.com/wojciech-graj/doom-ascii.git /tmp/doom-license-vendor && cd /tmp/doom-license-vendor && git checkout -q b5188d7c9c4da6c81264a7803e8725ac3df2cfea"
mkdir -p LICENSES
scp ampere-dev:/tmp/doom-license-vendor/LICENSE LICENSES/doom-ascii.GPL-2.0
ssh ampere-dev "rm -rf /tmp/doom-license-vendor"
```

Verify it's the real GPLv2 text, not an empty or truncated file:

```bash
wc -l LICENSES/doom-ascii.GPL-2.0
head -2 LICENSES/doom-ascii.GPL-2.0
```

Expected: 339 lines, first line `GNU GENERAL PUBLIC LICENSE` (confirmed against the pinned commit directly during this plan's own research — matches).

- [ ] **Step 2: Add the visible, specific source-offer link to the footer**

In `site/index.html`, inside `<footer class="foot">`, after the existing colophon paragraph:

```html
<p class="fine">
  DOOM runs via a WebAssembly build of
  <a href="https://github.com/wojciech-graj/doom-ascii/tree/b5188d7c9c4da6c81264a7803e8725ac3df2cfea">doom-ascii</a>,
  GPL-2.0 licensed. The exact patches applied to that pinned commit are in
  <a href="https://github.com/Builder106/ascii-arcade/tree/main/patches/doom-wasm">patches/doom-wasm</a>
  in this repo — together with the pin, that is the corresponding source.
</p>
```

`.fine`is the existing "terse fact line" class already used for`#install`'s "macOS 13 or newer. GPL-2.0. No account, no telemetry." — reused here rather than inventing a new style for what's the same kind of statement.

- [ ] **Step 3: Confirm the license file doesn't trip the budget test**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/budget.spec.ts"
```

Expected: PASS — `LICENSES/`isn't in`budget.spec.ts`'s `counted`list (verify by reading the test file if unsure; it enumerates specific`site/` files and directories only), so this shouldn't affect it. The footer HTML addition might, though it's small — this step exists specifically to catch that rather than assume it's fine.

- [ ] **Step 4: Commit**

```bash
git add LICENSES/doom-ascii.GPL-2.0 site/index.html
git commit -m "docs: vendor doom-ascii's GPL-2.0 license and link the pinned source"
```

---

## Task 6: Accessibility scoping and final a11y test

**Files:**

- Modify: `e2e/tests/site/a11y.spec.ts` — add a scoped check for the live canvas.

**Interfaces:** none — test-only task.

**What changed already, from earlier tasks:** Task 3 already replaced the placeholder `role="img"`with`role="application"`and a real, control-accurate`aria-label`(Step 5). Task 2's touch controls already carry`role="group"`/`aria-label="Touch controls"`on their container and are real`<button>`elements (Tab-reachable,`Enter`/`Space`-activatable by default — no custom keyboard handling needed for the buttons themselves, only for the canvas's own game input, which Task 1 already handles). Task 3's Stop button is a real `<button>`, no ARIA needed beyond its visible text. This task is about *verifying* that combination passes axe, not building new markup.

- [ ] **Step 1: Add the scoped, session-active a11y test**

Add to `e2e/tests/site/a11y.spec.ts`:

```typescript
test("the live DOOM canvas and its controls pass axe once a session is active", async ({ page }) => {
  await page.goto("/site/");
  await page.getByRole("button", { name: /play it/i }).click();
  await expect(page.locator("#doomPlayCanvas")).toBeVisible({ timeout: 15000 });

  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa"])
    .include("#doomPlayCanvas")
    .include(".doom-controls")
    .include("#stopDoom")
    .analyze();

  expect(results.violations).toEqual([]);
});
```

`.include(".doom-controls")`only matches on a run where touch controls actually rendered — under Playwright's default (non-touch) browser context,`touchCapable()`returns`false`and`.doom-controls`never exists in the DOM.`AxeBuilder.include()`on a selector matching zero elements is not an error (axe-core simply scans nothing for that selector); confirm this assumption holds by checking the test doesn't fail purely because`.doom-controls`is absent — if it does, drop that`.include()`line and cover touch-controls a11y in a separate,`hasTouch: true` context test instead (following the same pattern Task 2 already established for the two touch-controls-visibility tests).

- [ ] **Step 2: Run it**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/a11y.spec.ts"
```

Expected: all PASS, including the 4 pre-existing per-theme tests and the new one. If the new test fails on a real violation (not the `.include()`empty-selector question from Step 1), fix the underlying markup — don't loosen the test to`.exclude()`the failing element, which is exactly the "blanket exclusion" pattern`docs/doom-wasm-design.md` called out as needing to be narrowed, not repeated.

- [ ] **Step 3: Run the complete site suite once, end to end**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/"
```

Expected: every test in `tests/site/` passes — this is the first point in Plan B where all six tasks' changes run together against the full existing suite (a11y, content, dissolve, doom, doom-play, doom-play-skeleton, engine, enhance, motion, renderer, budget).

- [ ] **Step 4: Commit**

```bash
git add e2e/tests/site/a11y.spec.ts
git commit -m "test: scope a11y coverage to the live DOOM canvas instead of a blanket exclusion"
```

---

## Self-Review Notes

**Spec coverage against `docs/doom-wasm-design.md`:** "Input" (keyboard: Task 1; touch: Task 2; focus and scroll: Task 3) — covered. "Rendering"'s remaining Plan-B-flagged pieces (ambient `SceneDriver`pause during a session) — Task 3, Step 4. "WAD hosting and preload" — Task 4, explicitly reconciled against Plan A's actual`--preload-file`bundling rather than the design doc's separate-fetch assumption. "Licensing and GPL compliance" — all three artifacts:`patches/doom-wasm/*.patch`already exists from Plan A,`LICENSES/doom-ascii.GPL-2.0`and the footer link are Task 5. "Failure behavior" — already handled by Plan A's existing try/catch in`doom.js`; Task 4's preload failure explicitly falls back to that same existing path rather than adding a second one. "Accessibility" — Task 3 (role/label) and Task 6 (test coverage) together. "Testing and CI tier" — `doom-play.spec.ts`built incrementally across Tasks 1-3 exactly as the design doc named it;`budget.spec.ts`checked twice (Tasks 2 and 5) rather than assumed safe;`a11y.spec.ts` narrowed in Task 6, not left as a wider exclusion.

**Design-doc claim this plan verifies rather than inherits on faith:** "Escape doubles as the exit path... whose 'quit' already terminates the session" (Task 3, Step 2) — the plan does not assume this is true; it's checked against the real build, and the plan branches on the actual result rather than presenting both branches as equally likely in the final code.

**Out of scope, unchanged from the design doc:** audio, save games, multiplayer, IWAD selection, older-browser fallback beyond the existing static message, offline caching. Nothing in Plan B reopens any of these.

**Type/interface consistency check:**`loadDoomSkeleton(canvas)`(Plan A) →`loadDoomSkeleton(canvas, { onSessionEnd })`(Task 3) — the options parameter is additive (defaults via destructuring with`= {}`if not already present from Task 3's own code, confirm this default exists in the actual diff since Task 1/2's snippets don't show it) so Plan A's`doom-play-skeleton.spec.ts`calling`loadDoomSkeleton(canvas)`with one argument keeps working unchanged.`mountDoom(preEl, buttonEl)`(current) →`mountDoom(preEl, buttonEl, stopButtonEl, { pauseAmbient, resumeAmbient })`(Task 3) — this one is**not** backward compatible;`site/main.js`'s call site is updated in the same task that changes the signature (Task 3, Steps 3-4), so no other caller is left stale. `push(pressed, key)` — same parameter order (`pressed`first,`key` second) in the C function (`wasm_push_key(int pressed, unsigned char key)`), the cached JS `push` from Task 1, and every call site in Tasks 1-3 — verified consistent across all of them while writing this plan, not just assumed.
