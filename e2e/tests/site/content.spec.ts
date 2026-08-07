import { test, expect } from "@playwright/test";

const HEADINGS = [
  "macOS can't screenshot this",
  "It lives under everything",
  "DOOM was the sideshow",
  "Every glyph keys off one colour",
  "It doesn't need a desktop",
  "Put it on yours",
];

test("every section heading is present", async ({ page }) => {
  await page.goto("/site/");
  for (const h of HEADINGS) {
    await expect(page.getByRole("heading", { name: h })).toBeVisible();
  }
});

test("no numbered section labels", async ({ page }) => {
  await page.goto("/site/");
  const body = await page.locator("body").innerText();
  expect(body).not.toMatch(/^\s*0[1-6]\s*[—-]/m);
});

test("Gatekeeper is explained rather than buried", async ({ page }) => {
  await page.goto("/site/");
  await expect(page.getByText(/gatekeeper/i)).toBeVisible();
  await expect(page.getByText("xattr -dr com.apple.quarantine")).toBeVisible();
});

test("the depth rail exposes accessible names, not bare glyphs", async ({ page }) => {
  await page.goto("/site/");
  const rail = page.getByRole("navigation", { name: /sections/i });
  for (const h of HEADINGS) {
    await expect(rail.getByRole("link", { name: h })).toBeAttached();
  }
});

test("palette buttons are built from the engine's own themes", async ({ page }) => {
  await page.goto("/site/");
  await page.waitForFunction(() => window.__aaReady === true, null, { timeout: 15000 });

  const group = page.getByRole("group", { name: /choose a palette/i });
  for (const name of ["Hacker", "Amber", "Ice", "Ghost"]) {
    await expect(group.getByRole("button", { name })).toBeVisible();
  }

  await group.getByRole("button", { name: "Amber" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "amber");
});
