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

  await matrixBtn.click();
  await expect(matrixBtn).toHaveAttribute("aria-pressed", "true");
});

