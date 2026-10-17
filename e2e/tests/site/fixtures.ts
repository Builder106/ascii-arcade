import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test as base, expect, type Page } from "@playwright/test";

export { expect };

const SITE = resolve(dirname(fileURLToPath(import.meta.url)), "../../../site");

/** Whether a build output the page loads exists, e.g. "pkg/aa_wasm.js". */
export function built(path: string): boolean {
  return existsSync(resolve(SITE, path));
}

export const SITE_DIR = SITE;

export const STATIONS = ["title", "demo", "scenes", "rules", "install"] as const;
export const THEMES = ["Hacker", "Amber", "Ice", "Ghost"] as const;

/**
 * Every test fails on a console error, a console warning or an uncaught
 * exception, unless it names the message it expects with `allowConsole`.
 * The page reports its own survivable failures with console.info, so
 * anything louder is a real problem.
 */
export const test = base.extend<{ allowConsole: RegExp[]; consoleProblems: string[] }>({
  allowConsole: [[], { option: true }],
  consoleProblems: [
    async ({ page, allowConsole }, use) => {
      const problems: string[] = [];
      // Headless Chromium draws WebGL on a software GPU, which logs driver
      // performance notes ("GPU stall due to ReadPixels"). They are about the
      // test machine, not the page.
      const note = (text: string) => {
        if (/GL Driver Message/.test(text)) return;
        if (!allowConsole.some((re) => re.test(text))) problems.push(text);
      };
      page.on("console", (m) => {
        if (m.type() === "error" || m.type() === "warning") note(`${m.type()}: ${m.text()}`);
      });
      page.on("pageerror", (e) => note(`pageerror: ${e.message}`));
      await use(problems);
      expect(problems).toEqual([]);
    },
    { auto: true },
  ],
});

/**
 * Loads a path and waits for the page's first render. The power-on plays on
 * every load; specs skip it the way a visitor can, with a key press, unless
 * they ask to watch it with { boot: true }.
 */
export async function open(page: Page, path = "/", { boot = false } = {}): Promise<void> {
  await page.goto(path);
  const html = page.locator("html");
  await expect(html).toHaveAttribute("data-mode", /.+/);
  if (!boot && (await html.evaluate((el) => el.classList.contains("booting")))) {
    await page.keyboard.press("Shift");
    await expect(html).not.toHaveClass(/booting/);
  }
}

/** Waits until the wall shows a scene, which needs the aa-core build. */
export async function waitForScene(page: Page, scene?: string): Promise<void> {
  await expect(page.locator("html")).toHaveAttribute("data-scene", scene ?? /.+/, { timeout: 30_000 });
}

export async function mode(page: Page): Promise<string | null> {
  return page.locator("html").getAttribute("data-mode");
}

export async function station(page: Page): Promise<string | null> {
  return page.locator("html").getAttribute("data-station");
}

/** Number of sampled canvas pixels that differ from the page's ground colour. */
export async function litPixels(page: Page): Promise<number> {
  return page.evaluate(() => {
    const wall = document.getElementById("wall") as HTMLCanvasElement;
    const probe = document.createElement("canvas");
    probe.width = 160;
    probe.height = 100;
    const ctx = probe.getContext("2d", { willReadFrequently: true });
    if (!ctx) return 0;
    ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue("--ground");
    ctx.fillRect(0, 0, 1, 1);
    const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
    ctx.drawImage(wall, 0, 0, probe.width, probe.height);
    const d = ctx.getImageData(0, 0, probe.width, probe.height).data;
    let lit = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (Math.abs(d[i] - r) + Math.abs(d[i + 1] - g) + Math.abs(d[i + 2] - b) > 30) lit++;
    }
    return lit;
  });
}
