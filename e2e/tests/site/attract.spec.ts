import { test, expect, open, built, waitForScene, mode, station } from "./fixtures";

// The page's own clock runs on requestAnimationFrame and performance.now, so
// Playwright's fake clock drives the whole loop without real waiting. These
// specs run the flat page: stepping the fake clock through hundreds of frames
// of the 3D cabinet on a software GPU is too slow, and the flat page is the
// cabinet's own fallback anyway. cabinet.spec.ts covers the cabinet.
test.use({ allowConsole: [/Failed to load resource/] });

test.beforeEach(async ({ page }) => {
  await page.route("**/cabinet.js", (r) => r.abort());
  await page.clock.install();
});

test("the loop runs through the stations on its own", async ({ page }) => {
  await open(page);
  expect(await mode(page)).toBe("attract");
  expect(await station(page)).toBe("title");
  await expect(page.locator("#title")).toBeVisible();
  await expect(page.locator("#demo")).toBeHidden();

  await page.clock.runFor(9_500);
  expect(await station(page)).toBe("demo");
  await page.clock.runFor(7_500);
  expect(await station(page)).toBe("scenes");
  await expect(page.locator('.strip a[data-station="scenes"]')).toHaveAttribute("aria-current", "location");
});

test("the scenes station steps through every scene", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  await open(page, "/");
  await waitForScene(page, "donut");
  await page.clock.runFor(16_600);
  expect(await station(page)).toBe("scenes");
  await page.clock.runFor(4_000);
  await expect(page.locator("html")).toHaveAttribute("data-scene", "helix");
  await page.clock.runFor(4_000);
  await expect(page.locator("html")).toHaveAttribute("data-scene", "matrix");
});

test("a key takes over, idle brings the loop back, and the mouse snaps it back", async ({ page }) => {
  await open(page);
  await page.clock.runFor(2_000);
  await page.keyboard.press("Shift");
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe("title");

  await page.locator('.strip a[data-station="install"]').click();
  expect(await station(page)).toBe("install");
  await expect(page).toHaveURL(/#install$/);

  // Idle: the app moves on as soon as idle runs out, and so does the page.
  await page.clock.runFor(10_500);
  expect(await mode(page)).toBe("attract");
  expect(await station(page)).toBe("title");
  await expect(page.locator("#status")).toContainText("loop came back");

  await page.mouse.move(200, 200);
  await page.mouse.move(220, 210);
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe("install");
});

test("the arrow keys work like a joystick through the stations", async ({ page }) => {
  await open(page);
  await page.keyboard.press("ArrowRight");
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe("demo");
  await page.keyboard.press("ArrowRight");
  expect(await station(page)).toBe("scenes");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  await page.keyboard.press("ArrowLeft");
  expect(await station(page)).toBe("install");
  await expect(page).toHaveURL(/#install$/);
});

test("a passing pointer doesn't end the first demo", async ({ page }) => {
  await open(page);
  await page.mouse.move(100, 100);
  await page.mouse.move(300, 300);
  expect(await mode(page)).toBe("attract");
});

test("a deep link lands on its station, ready to read", async ({ page }) => {
  await open(page, "/#rules");
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe("rules");
  await expect(page.locator("#rules")).toBeVisible();
});

test("the back button returns to the previous station", async ({ page }) => {
  await open(page);
  await page.locator('.strip a[data-station="scenes"]').click();
  await page.locator('.strip a[data-station="rules"]').click();
  await page.goBack();
  expect(await station(page)).toBe("scenes");
  await expect(page.locator("#scenes")).toBeVisible();
});

test("switching off auto-cycle stops the loop for good", async ({ page }) => {
  await open(page);
  await page.locator("#cycle").click();
  await expect(page.locator("#cycle")).toHaveAttribute("aria-pressed", "false");
  const at = await station(page);
  await page.clock.runFor(60_000);
  expect(await mode(page)).toBe("player");
  expect(await station(page)).toBe(at);
});

test("a scene and a theme chosen by hand survive the snap-back", async ({ page }) => {
  test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");
  await open(page, "/#scenes");
  await waitForScene(page);
  await page.locator('[data-scene="pipes"]').click();
  await page.locator('[data-theme-name="Amber"]').click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "Amber");
  await page.clock.runFor(10_500);
  expect(await mode(page)).toBe("attract");
  await page.keyboard.press("Shift");
  expect(await station(page)).toBe("scenes");
  await expect(page.locator("html")).toHaveAttribute("data-scene", "pipes");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "Amber");
});

test.describe("reduced motion", () => {
  test.use({ contextOptions: { reducedMotion: "reduce" } });

  test("the loop starts off and the page holds still", async ({ page }) => {
    await open(page);
    expect(await mode(page)).toBe("player");
    await expect(page.locator("#cycle")).toHaveAttribute("aria-pressed", "false");
    await page.clock.runFor(60_000);
    expect(await station(page)).toBe("title");
    await expect(page.locator("#status")).toContainText("Reduced motion");
  });
});
