import { test, expect } from "@playwright/test";

// #doomFrame is a canvas (Renderer-painted, not innerHTML), so "has content"
// and "content changed" are read from its actual pixels, not markup.
const countLitPixels = () => {
  const c = document.getElementById("doomFrame") as HTMLCanvasElement;
  const ctx = c.getContext("2d")!;
  const d = ctx.getImageData(0, 0, c.width, c.height).data;
  let lit = 0;
  for (let i = 0; i < d.length; i += 4) {
    if (d[i] > 20 || d[i + 1] > 20 || d[i + 2] > 20) lit++;
  }
  return lit;
};

test("the cold open plays real DOOM frames", async ({ page }) => {
  await page.goto("/site/");
  await expect
    .poll(async () => page.evaluate(countLitPixels), { timeout: 10000 })
    .toBeGreaterThan(0);
});

test("the loop advances rather than holding one still", async ({ page }) => {
  await page.goto("/site/");
  await expect
    .poll(async () => page.evaluate(countLitPixels), { timeout: 10000 })
    .toBeGreaterThan(0);

  const snapshot = () =>
    page.evaluate(
      (id) => (document.getElementById(id) as HTMLCanvasElement).toDataURL(),
      "doomFrame",
    );
  const first = await snapshot();
  await expect.poll(snapshot, { timeout: 4000 }).not.toBe(first);
});

test("a missing recording fails loudly on click, not silently", async ({ page }) => {
  // Play it now depends on doom-wasm/, not the attract recording — the two
  // are independent resources (mountDoom() falls back to a blank hero if
  // the recording is missing, but Play it's own success/failure is
  // governed entirely by whether doom-wasm/ loads). Abort the resource
  // Play it actually needs to test its own failure path.
  await page.route("**/doom-wasm/**", (r) => r.abort());
  await page.goto("/site/");
  await page.getByRole("button", { name: /play it/i }).click();
  await expect(page.getByRole("status")).toContainText(/run \.\/scripts\/setup\.sh|clone the repo/i, {
    timeout: 10000,
  });
});
