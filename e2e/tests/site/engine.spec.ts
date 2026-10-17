import { test, expect, open, built, waitForScene, litPixels, THEMES } from "./fixtures";

test.skip(!built("pkg/aa_wasm.js"), "needs scripts/build-wasm.sh");

test.describe("on the flat page", () => {
  test.use({ allowConsole: [/Failed to load resource/] });

  // The wall canvas is the flat page's. Once the cabinet loads, the grid
  // paints the cabinet's screen instead, so this blocks the cabinet.
  test("the wall paints aa-core frames", async ({ page }) => {
    await page.route("**/cabinet.js", (r) => r.abort());
    await open(page);
    await waitForScene(page, "donut");
    await expect.poll(() => litPixels(page)).toBeGreaterThan(50);
  });
});

test("the page's palettes match the engine's themes", async ({ page }) => {
  await open(page);
  const pairs = await page.evaluate(async (names) => {
    const m = await import("/pkg/aa_wasm.js");
    await m.default();
    const themes: { name: string; text: number[] }[] = JSON.parse(m.themes_json());
    const root = document.documentElement;
    return names.map((name) => {
      root.dataset.theme = name;
      const ink = getComputedStyle(root).getPropertyValue("--ink").trim();
      const t = themes.find((x) => x.name === name);
      const hex = t ? `#${t.text.map((c) => c.toString(16).padStart(2, "0")).join("")}` : "missing";
      return [ink, hex];
    });
  }, [...THEMES]);
  for (const [css, engine] of pairs) expect(css).toBe(engine);
});

test("every scene the page offers exists in the engine", async ({ page }) => {
  await open(page);
  const ids = await page.evaluate(async () => {
    const m = await import("/pkg/aa_wasm.js");
    await m.default();
    return m.scene_ids() as string[];
  });
  const offered = await page.locator("button[data-scene]").evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.scene));
  for (const id of offered) expect(ids).toContain(id);
});
