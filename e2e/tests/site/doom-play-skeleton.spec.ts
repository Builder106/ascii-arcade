import { test, expect } from "@playwright/test";

test("doom-wasm skeleton paints non-empty pixels to its own canvas", async ({ page }) => {
  await page.goto("/site/");

  const painted = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    document.body.appendChild(canvas);

    const handle = await loadDoomSkeleton(canvas);
    // D_DoomLoop syncs to requestAnimationFrame; give the engine real
    // frames to tick and reach a non-blank screen (boot logo / menu).
    await new Promise((resolve) => setTimeout(resolve, 3000));

    const ctx = canvas.getContext("2d")!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let lit = 0;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] > 20 || data[i + 1] > 20 || data[i + 2] > 20) lit++;
    }

    handle.stop();
    canvas.remove();
    return lit;
  });

  expect(painted).toBeGreaterThan(0);
});
