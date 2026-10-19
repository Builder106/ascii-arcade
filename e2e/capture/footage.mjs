// Records the site footage that the HyperFrames videos in trailer/ and
// ui-demo/ are cut from, and that the README GIFs are made from.
//
// A screen recording of the page drops frames: the cabinet renders on a
// software GPU here, and Playwright's own recorder is soft. So instead the
// page runs on Playwright's fake clock, which owns requestAnimationFrame and
// performance.now, and every frame is stepped and screenshotted. CSS
// animations run on the compositor's clock instead, so they are paused on
// sight and seeked to the same fake time. The result is a clean 30 fps take
// however slow the machine is.
//
//   node capture/footage.mjs [take ...] [--out dir] [--base url]
//
// With no takes named it records all of them. With no --base it serves
// ../site itself, which needs site/pkg and site/doom-wasm built first.

import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const FPS = 30;
const HERE = dirname(fileURLToPath(import.meta.url));

const DEVICES = {
  desktop: { viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 },
  // 360 by 640 at 3x is exactly 1080 by 1920, the social cut's frame.
  phone: { viewport: { width: 360, height: 640 }, deviceScaleFactor: 3, isMobile: true, hasTouch: true },
};

// Hides the page's copy and turns off WebGL 2, so the wall is the whole
// window: each scene as it looks as a wallpaper.
function wallOnly() {
  const css = document.createElement("style");
  css.textContent = ".skip, .top, main, .strip { display: none !important; }";
  // Init scripts run before <html> exists.
  new MutationObserver((_, watch) => {
    if (!document.documentElement) return;
    watch.disconnect();
    document.documentElement.appendChild(css);
  }).observe(document, { childList: true });
  const getContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (kind, ...rest) {
    return kind === "webgl2" ? null : getContext.call(this, kind, ...rest);
  };
}

// The playable DOOM build waits for its next tic by spinning on the clock
// (DG_SleepMs is a no-op), and a fake clock never moves during a spin, so the
// page would hang. Run before it loads, this hands out animation frames at
// DOOM's own 35 Hz, so each step of its main loop finds a tic waiting, and
// nudges the clock on only if something reads it 2000 times without it
// changing. The site paints at 30 Hz, so it doesn't mind the throttle.
function doomClock() {
  const raf = window.requestAnimationFrame.bind(window);
  let queue = [];
  let last = -Infinity;
  let waiting = false;
  const pump = (t) => {
    waiting = false;
    if (t - last >= 1000 / 35 - 0.5) {
      last = t;
      const due = queue;
      queue = [];
      for (const cb of due) cb(t);
    }
    if (queue.length && !waiting) {
      waiting = true;
      raf(pump);
    }
  };
  window.requestAnimationFrame = (cb) => {
    queue.push(cb);
    if (!waiting) {
      waiting = true;
      raf(pump);
    }
    return queue.length;
  };
  const now = performance.now.bind(performance);
  let drift = 0;
  let seen = -1;
  let repeats = 0;
  performance.now = () => {
    const t = now() + drift;
    if (t !== seen) {
      seen = t;
      repeats = 0;
    } else if (++repeats > 2000) {
      drift += 1;
      repeats = 0;
    }
    return t;
  };
}

// Page functions can't close over values, so each passes its own as the
// argument to page.evaluate.
const pick = (scene) => ["run", (s) => document.querySelector(`button[data-scene="${s}"]`).click(), scene];
const theme = (name) => ["run", (n) => document.querySelector(`[data-theme-name="${n}"]`).click(), name];
const station = (id) => ["run", (i) => document.querySelector(`.strip a[href="#${i}"]`).click(), id];

// Each take is a page load and a list of [seconds, action, ...args], timed
// from the load on the fake clock.
const TAKES = {
  // A first visit left alone: the power-on, then the whole loop and back
  // round to the title.
  attract: { device: "desktop", path: "/", seconds: 70, actions: [] },

  // A visitor takes over, works the joystick, tries scenes and themes, then
  // walks away. CONTINUE? counts down, the loop comes back, and a nudge of
  // the mouse snaps it back.
  player: {
    device: "desktop",
    path: "/",
    seconds: 38,
    actions: [
      [0.2, "key", "Shift"],
      [3.2, "key", "Space"],
      [5.0, "key", "ArrowRight"],
      [7.2, "key", "ArrowRight"],
      [9.6, "click", 'button[data-scene="helix"]'],
      [11.2, "click", 'button[data-scene="matrix"]'],
      [12.6, "click", '[data-theme-name="Amber"]'],
      [14.0, "click", 'button[data-scene="pipes"]'],
      [15.4, "click", '[data-theme-name="Ice"]'],
      [16.8, "click", 'button[data-scene="life"]'],
      [18.2, "click", '[data-theme-name="Ghost"]'],
      [19.6, "click", '[data-theme-name="Hacker"]'],
      [20.4, "click", 'button[data-scene="donut"]'],
      [22.0, "key", "ArrowRight"],
      [24.2, "key", "ArrowRight"],
      [36.0, "move", 900, 500],
      [36.2, "move", 960, 520],
    ],
  },

  // Play DOOM here: through the glass, Enter through the menus to a new
  // game, a few seconds of play, then Esc pulls back out with DOOM still
  // running on the cabinet's screen.
  doom: {
    device: "desktop",
    path: "/#demo",
    seconds: 24,
    actions: [
      [2.5, "run", doomClock],
      [3.0, "click", "#play-doom"],
      [4.6, "key", "Enter"],
      [5.3, "key", "Enter"],
      [6.0, "key", "Enter"],
      [6.7, "key", "Enter"],
      [8.0, "down", "ArrowUp"],
      [9.6, "up", "ArrowUp"],
      [9.7, "down", "ArrowRight"],
      [10.15, "up", "ArrowRight"],
      [10.3, "down", "ArrowUp"],
      [11.7, "up", "ArrowUp"],
      [11.8, "key", "ControlLeft"],
      [12.3, "key", "ControlLeft"],
      [12.8, "key", "ControlLeft"],
      [13.2, "down", "ArrowLeft"],
      [13.8, "up", "ArrowLeft"],
      [13.9, "down", "ArrowUp"],
      [15.9, "up", "ArrowUp"],
      [16.0, "key", "Space"],
      [16.4, "down", "ArrowUp"],
      [18.0, "up", "ArrowUp"],
      [18.2, "key", "ControlLeft"],
      [19.5, "key", "Escape"],
    ],
  },

  // Every scene full-window, as a wallpaper, through the themes. Changes go
  // through the app's own 0.9 s dissolve.
  wall: {
    device: "desktop",
    path: "/#scenes",
    setup: wallOnly,
    seconds: 52,
    actions: [
      [0.0, "run", () => document.getElementById("cycle").click()],
      [0.0, ...pick("donut")],
      [4.0, ...pick("helix")],
      [8.0, ...pick("matrix")],
      [12.0, ...pick("life")],
      [16.0, ...pick("pipes")],
      [20.0, ...station("demo")],
      [26.0, ...station("scenes")],
      [26.0, ...theme("Amber")],
      [26.0, ...pick("matrix")],
      [30.0, ...theme("Ice")],
      [30.0, ...pick("donut")],
      [34.0, ...theme("Ghost")],
      [34.0, ...pick("helix")],
      [38.0, ...theme("Amber")],
      [38.0, ...pick("life")],
      [42.0, ...theme("Ice")],
      [42.0, ...pick("pipes")],
      [46.0, ...theme("Hacker")],
      [46.0, ...pick("matrix")],
    ],
  },

  // The phone, left alone: power-on and the start of the loop.
  "phone-attract": { device: "phone", path: "/", seconds: 34, actions: [] },

  // The phone, played: a tap takes over, swipes work the stations, and the
  // scene and theme buttons repaint the cabinet.
  "phone-player": {
    device: "phone",
    path: "/",
    seconds: 28,
    actions: [
      [0.2, "tap", "#title h1"],
      [3.4, "tap", "#title h1"],
      [5.2, "swipe", -1],
      [7.4, "swipe", -1],
      [9.6, "tap", 'button[data-scene="matrix"]'],
      [11.0, "tap", '[data-theme-name="Amber"]'],
      [12.4, "tap", 'button[data-scene="pipes"]'],
      [13.8, "tap", '[data-theme-name="Ice"]'],
      [15.2, "tap", 'button[data-scene="helix"]'],
      [16.6, "tap", '[data-theme-name="Hacker"]'],
      [18.0, "swipe", -1],
      [20.4, "swipe", -1],
      [23.0, "swipe", 1],
    ],
  },
};

async function centre(page, selector) {
  const box = await page.locator(selector).boundingBox();
  if (!box) throw new Error(`${selector} is not on screen`);
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

async function act(page, cdp, [, kind, ...args]) {
  if (kind === "key") return page.keyboard.press(args[0]);
  if (kind === "down") return page.keyboard.down(args[0]);
  if (kind === "up") return page.keyboard.up(args[0]);
  if (kind === "move") return page.mouse.move(args[0], args[1]);
  if (kind === "run") return page.evaluate(args[0], args[1]);
  if (kind === "click") {
    const { x, y } = await centre(page, args[0]);
    return page.mouse.click(x, y);
  }
  if (kind === "tap") {
    const { x, y } = await centre(page, args[0]);
    return page.touchscreen.tap(x, y);
  }
  if (kind === "swipe") {
    // A horizontal drag across the middle of the screen; -1 swipes left,
    // which steps forward.
    const { width, height } = page.viewportSize();
    const y = Math.round(height * 0.45);
    const [from, to] = args[0] < 0 ? [width * 0.8, width * 0.2] : [width * 0.2, width * 0.8];
    const point = (x) => [{ x: Math.round(x), y }];
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: point(from) });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: point((from + to) / 2) });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: point(to) });
    return cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  }
  throw new Error(`unknown action ${kind}`);
}

async function record(browser, base, name, take, outDir) {
  const context = await browser.newContext(DEVICES[take.device]);
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  page.on("pageerror", (e) => console.error(`${name}: pageerror ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error" || m.type() === "warning") console.error(`${name}: ${m.type()} ${m.text()}`);
  });
  await page.addInitScript(() => {
    const css = document.createElement("style");
    css.textContent = "*, *::before, *::after { animation-play-state: paused !important; }";
    new MutationObserver((_, watch) => {
      if (!document.documentElement) return;
      watch.disconnect();
      document.documentElement.appendChild(css);
    }).observe(document, { childList: true });
    const seen = new WeakMap();
    window.__seekCss = (ms) => {
      for (const a of document.getAnimations()) {
        let at = seen.get(a);
        if (!at) {
          a.pause();
          at = { since: ms, from: Number(a.currentTime) || 0 };
          seen.set(a, at);
        }
        a.currentTime = at.from + (ms - at.since);
      }
    };
  });
  if (take.setup) await page.addInitScript(take.setup);
  await page.clock.install({ time: 0 });
  await page.clock.pauseAt(1000);
  await page.goto(base + take.path, { waitUntil: "load" });

  const out = resolve(outDir, `${name}.mp4`);
  const ffmpeg = spawn(
    "ffmpeg",
    ["-y", "-loglevel", "error", "-f", "image2pipe", "-framerate", String(FPS), "-c:v", "png", "-i", "-",
      "-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out],
    { stdio: ["pipe", "inherit", "inherit"] },
  );
  const done = new Promise((ok, fail) => ffmpeg.on("close", (code) => (code === 0 ? ok() : fail(new Error(`ffmpeg exited ${code}`)))));

  const queue = [...take.actions].sort((a, b) => a[0] - b[0]);
  const frames = Math.round(take.seconds * FPS);
  let elapsed = 0;
  for (let i = 0; i < frames; i++) {
    const t = i / FPS;
    while (queue.length && queue[0][0] <= t) await act(page, cdp, queue.shift());
    const target = Math.round(((i + 1) * 1000) / FPS);
    await page.clock.runFor(target - elapsed);
    elapsed = target;
    await page.evaluate((ms) => window.__seekCss(ms), 1000 + elapsed);
    const png = await page.screenshot({ type: "png", animations: "allow", caret: "initial" });
    if (!ffmpeg.stdin.write(png)) await new Promise((r) => ffmpeg.stdin.once("drain", r));
    if (i % (FPS * 5) === 0) console.log(`${name}: ${t.toFixed(0)} s of ${take.seconds}`);
  }
  ffmpeg.stdin.end();
  await done;
  await context.close();
  console.log(`${name}: wrote ${out}`);
}

const argv = process.argv.slice(2);
const flag = (name) => {
  const i = argv.indexOf(name);
  return i < 0 ? null : argv.splice(i, 2)[1];
};
const outDir = resolve(flag("--out") ?? resolve(HERE, "../recordings/footage"));
let base = flag("--base");
const names = argv.length ? argv : Object.keys(TAKES);
for (const n of names) if (!TAKES[n]) throw new Error(`no take called ${n}; takes: ${Object.keys(TAKES).join(", ")}`);
mkdirSync(outDir, { recursive: true });

// Loopback only, as in playwright.config.ts.
let server = null;
if (!base) {
  server = spawn("python3", ["-m", "http.server", "8898", "--bind", "127.0.0.1", "--directory", resolve(HERE, "../../site")], { stdio: "ignore" });
  base = "http://127.0.0.1:8898";
  await new Promise((r) => setTimeout(r, 800));
}

// Software WebGL, for a machine without a GPU.
const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
try {
  for (const n of names) await record(browser, base, n, TAKES[n], outDir);
} finally {
  await browser.close();
  server?.kill();
}
