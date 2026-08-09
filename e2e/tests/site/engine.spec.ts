import { test, expect } from "@playwright/test";

test("the donut paints a non-empty grid", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, {
    timeout: 15000,
  });

  // Give the loop a few frames to draw something.
  await page.waitForTimeout(500);

  const painted = await page.evaluate(() => {
    const c = document.getElementById("grid") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let lit = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (d[i] > 20 || d[i + 1] > 20 || d[i + 2] > 20) lit++;
    }
    return lit;
  });

  expect(painted).toBeGreaterThan(0);
});

test("the page still renders when WebAssembly fails to load", async ({ page }) => {
  await page.route("**/pkg/**", (r) => r.abort());
  await page.goto("/site/");

  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(
    page.getByRole("link", { name: /download/i }).first(),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Put it on yours" })).toBeAttached();
});

test("interactive scene picker switches active scene", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, {
    timeout: 15000,
  });

  const picker = page.getByRole("group", { name: "Select background scene" });
  await expect(picker).toBeVisible();

  const matrixBtn = picker.getByRole("button", { name: "Matrix" });
  await expect(matrixBtn).toBeVisible();
  await expect(matrixBtn).toHaveAttribute("aria-pressed", "false");

  await matrixBtn.scrollIntoViewIfNeeded();
  await matrixBtn.evaluate((btn) => (btn as HTMLElement).click());
  await expect(matrixBtn).toHaveAttribute("aria-pressed", "true");
});

test("the scene gallery previews are independently live and drive the shared background", async ({
  page,
}) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, {
    timeout: 15000,
  });

  const gallery = page.getByRole("group", { name: /preview and choose a scene/i });
  await expect(gallery).toBeVisible();

  const pipesCard = gallery.getByRole("button", { name: /preview pipes/i });
  await expect(pipesCard).toBeVisible();
  await expect(pipesCard).toHaveAttribute("aria-pressed", "false");

  // Each tile is its own SceneDriver painting to its own canvas, not a crop
  // of the shared background — confirm this one has actually drawn something
  // before trusting the click below to mean anything.
  await pipesCard.scrollIntoViewIfNeeded();
  await page.waitForTimeout(300);
  const painted = await pipesCard.evaluate((card) => {
    const c = card.querySelector("canvas") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let lit = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (d[i] > 20 || d[i + 1] > 20 || d[i + 2] > 20) lit++;
    }
    return lit;
  });
  expect(painted).toBeGreaterThan(0);

  // Clicking a tile sets the real page background (same driver the hero
  // picker controls), not just its own local state.
  await pipesCard.evaluate((btn) => (btn as HTMLElement).click());
  await expect(pipesCard).toHaveAttribute("aria-pressed", "true");
  const heroPipesBtn = page
    .getByRole("group", { name: "Select background scene" })
    .getByRole("button", { name: "Pipes" });
  await expect(heroPipesBtn).toHaveAttribute("aria-pressed", "true");
});

