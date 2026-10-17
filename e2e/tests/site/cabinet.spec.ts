import type { Page } from "@playwright/test";
import { test, expect, open, built, mode } from "./fixtures";

// Real time, not the fake clock: the cabinet renders every frame.
test("the cabinet is the stage, with a view for each station", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  test.setTimeout(60_000);
  const html = page.locator("html");
  await open(page);
  await expect(html).toHaveAttribute("data-view", "cabinet", { timeout: 30_000 });
  await expect(html).toHaveAttribute("data-pose", "title");
  await page.keyboard.press("Shift");
  expect(await mode(page)).toBe("player");
  await page.keyboard.press("ArrowRight");
  await expect(html).toHaveAttribute("data-pose", "demo");
  await expect(page.locator("#status")).toContainText("joystick");
  await page.locator('.strip a[data-station="install"]').click();
  await expect(html).toHaveAttribute("data-pose", "install");
  await page.locator('.strip a[data-station="rules"]').click();
  await expect(html).toHaveAttribute("data-pose", "rules");
  await expect(html).toHaveAttribute("data-view", "cabinet");
  // Idle: the loop resumes and the camera moves on to the next station.
  await expect(html).toHaveAttribute("data-pose", "install", { timeout: 20_000 });
  expect(await mode(page)).toBe("attract");
});

// Synthetic hovers over a grid of points, returning those that give a cursor.
// They run in one evaluate so the camera holds still while they do.
function scanFor(page: Page, cursor: string, stride: number) {
  return page.evaluate(
    ({ cursor, stride }) => {
      const hits: { x: number; y: number }[] = [];
      for (let y = 20; y < innerHeight - 60; y += stride) {
        for (let x = innerWidth / 2; x < innerWidth; x += stride) {
          const target = document.elementFromPoint(x, y) ?? document.body;
          target.dispatchEvent(new PointerEvent("pointermove", { clientX: x, clientY: y, pointerType: "mouse", bubbles: true }));
          if (document.body.style.cursor === cursor) hits.push({ x, y });
        }
      }
      return hits;
    },
    { cursor, stride },
  );
}

// The camera is still flying in from the power-on for a while on a software
// GPU, so wait until two scans in a row find the stick in the same place.
async function findStick(page: Page) {
  let at = (await scanFor(page, "grab", 8))[0] ?? null;
  for (let i = 0; i < 10; i++) {
    await page.waitForTimeout(1000);
    const again = (await scanFor(page, "grab", 8))[0] ?? null;
    if (at && again && Math.hypot(again.x - at.x, again.y - at.y) <= 8) return again;
    at = again;
  }
  return at;
}

test("the stick can be grabbed and pushed to the next station", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  test.setTimeout(90_000);
  const html = page.locator("html");
  await open(page);
  await expect(html).toHaveAttribute("data-view", "cabinet", { timeout: 30_000 });
  await page.keyboard.press("Shift");
  await expect(html).toHaveAttribute("data-pose", "title");
  const at = await findStick(page);
  expect(at, "the stick answers a hover with a grab cursor").not.toBeNull();
  await page.mouse.move(at!.x, at!.y);
  await page.mouse.down();
  await expect.poll(() => page.evaluate(() => document.body.style.cursor)).toBe("grabbing");
  // A small wobble is not a push.
  await page.mouse.move(at!.x + 12, at!.y);
  expect(await html.getAttribute("data-pose")).toBe("title");
  for (let dx = 15; dx <= 60; dx += 5) await page.mouse.move(at!.x + dx, at!.y);
  await expect(html).toHaveAttribute("data-pose", "demo");
  await page.mouse.up();
  // One push is one step, however long it is held.
  await expect(html).toHaveAttribute("data-pose", "demo");
});

test("letting go of the stick over another part doesn't click it", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  test.setTimeout(90_000);
  const html = page.locator("html");
  await open(page);
  await expect(html).toHaveAttribute("data-view", "cabinet", { timeout: 30_000 });
  await page.keyboard.press("Shift");
  await expect(html).toHaveAttribute("data-pose", "title");
  const at = await findStick(page);
  expect(at).not.toBeNull();
  // Just below the ball is the control panel, which leads to Scenes.
  const panel = { x: at!.x, y: at!.y + 40 };
  await page.mouse.move(panel.x, panel.y);
  expect(await page.evaluate(() => document.body.style.cursor)).toBe("pointer");
  // Grab the stick, slide straight down onto the panel (no push), let go.
  await page.mouse.move(at!.x, at!.y);
  await page.mouse.down();
  await page.mouse.move(panel.x, panel.y, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(500);
  expect(await html.getAttribute("data-station")).toBe("title");
  // The cursor describes what's under it now, not the stick it let go of.
  expect(await page.evaluate(() => document.body.style.cursor)).toBe("pointer");
  // A plain click there still goes to Scenes.
  await page.mouse.click(panel.x, panel.y);
  await expect(html).toHaveAttribute("data-station", "scenes");
});

test("playing DOOM goes through the glass, and Esc comes back out", async ({ page }) => {
  test.skip(!built("doom-wasm/doom.js") || !built("pkg/aa_wasm.js"), "needs both WebAssembly builds");
  test.setTimeout(120_000);
  const html = page.locator("html");
  await open(page, "/#demo");
  await expect(html).toHaveAttribute("data-view", "cabinet", { timeout: 30_000 });
  await page.locator('.strip a[data-station="demo"]').click();
  await page.locator("#play-doom").click();
  await expect(html).toHaveAttribute("data-view", "flat", { timeout: 90_000 });
  await page.keyboard.press("Escape");
  await expect(html).toHaveAttribute("data-view", "cabinet", { timeout: 10_000 });
  await expect(html).toHaveAttribute("data-pose", "demo");
});

test.describe("reduced motion", () => {
  test.use({ contextOptions: { reducedMotion: "reduce" } });

  test("never loads the cabinet", async ({ page }) => {
    const requested: string[] = [];
    page.on("request", (r) => requested.push(r.url()));
    await open(page);
    await page.waitForLoadState("networkidle");
    expect(requested.filter((u) => /cabinet\.js|three\.min\.js/.test(u))).toEqual([]);
    await expect(page.locator("#cabinet")).toHaveCount(0);
  });
});
