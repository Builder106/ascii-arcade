import { test, expect, open, built, mode, station } from "./fixtures";

// Edge cases in getting around: the address, the keyboard, focus, and DOOM's
// download. They run on the flat page, which shares every rule with the
// cabinet; cabinet.spec.ts covers the stick.
test.use({ allowConsole: [/Failed to load resource/] });
test.beforeEach(async ({ page }) => {
  await page.route("**/cabinet.js", (r) => r.abort());
});

test("DOOM finishing its download doesn't pull back a visitor who moved on", async ({ page }) => {
  test.skip(!built("doom-wasm/doom.js"), "needs scripts/build-doom-wasm.sh");
  test.setTimeout(120_000);
  let release = () => {};
  const gate = new Promise<void>((r) => (release = r));
  await page.route("**/doom-wasm/doom.js", async (r) => {
    await gate;
    await r.continue();
  });
  await open(page, "/#demo");
  await page.locator("#play-doom").click();
  await page.locator('.strip a[data-station="install"]').click();
  release();
  await expect(page.locator("#doom-status")).toContainText("ready", { timeout: 90_000 });
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe("install");
});

test.describe("on a phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("the arrow keys scroll a focused command instead of changing station", async ({ page }) => {
    await open(page, "/#install");
    const cmd = page.locator("#cmd-xattr");
    await cmd.focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    expect(await station(page)).toBe("install");
    await expect(cmd).toBeFocused();
    await expect.poll(() => cmd.evaluate((el) => el.scrollLeft)).toBeGreaterThan(0);
  });
});

test("the skip link keeps the current station", async ({ page }) => {
  await open(page, "/#install");
  await page.locator(".skip").focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#stations")).toBeFocused();
  expect(await station(page)).toBe("install");
  await page.goBack();
  expect(await station(page)).toBe("install");
});

test("holding an arrow key steps once", async ({ page }) => {
  await open(page, "/#title");
  const before = await page.evaluate(() => history.length);
  for (let i = 0; i < 8; i++) await page.keyboard.down("ArrowRight");
  await page.keyboard.up("ArrowRight");
  expect(await station(page)).toBe("demo");
  expect(await page.evaluate(() => history.length)).toBe(before + 1);
});

test("Back returns to the station where the visitor took over from the loop", async ({ page }) => {
  await page.clock.install();
  await open(page);
  await page.clock.runFor(9_500);
  expect(await station(page)).toBe("demo");
  await page.keyboard.press("Enter");
  expect(await mode(page)).toBe("player");
  await page.keyboard.press("ArrowRight");
  expect(await station(page)).toBe("scenes");
  await page.goBack();
  await expect.poll(() => station(page)).toBe("demo");
  // The address stays plain, so a reload still powers on.
  expect(new URL(page.url()).hash).toBe("");
});

test("an arrow step takes keyboard focus to the new station", async ({ page }) => {
  await open(page, "/#title");
  await page.locator("#title .cta").focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#demo")).toBeFocused();
});
