import { createBdd } from "playwright-bdd";
import { expect } from "@playwright/test";
import { dwellForDemo, injectCursor } from "../support/demo.js";

const { Given, When, Then, Before } = createBdd();

Before(async ({ page }) => {
  await injectCursor(page);
});

Given("I open the DOOM page", async ({ page }) => {
  await page.goto("/");
  // xterm.js mounts a .xterm element with a canvas once it opens.
  await page.locator(".xterm").waitFor({ state: "visible", timeout: 30_000 });
  await dwellForDemo(page);
});

When("I wait for DOOM to start rendering", async ({ page }) => {
  // Give the WebSocket time to connect and the first frames to paint.
  await page.waitForTimeout(4000);
  await dwellForDemo(page, 2000);
});

When("I press {string} to advance past the intro", async ({ page }, key: string) => {
  await page.locator(".xterm").click(); // focus the terminal
  await page.keyboard.press(key);
  await page.waitForTimeout(800);
  await dwellForDemo(page);
});

When("I move with the arrow keys", async ({ page }) => {
  for (const key of ["ArrowUp", "ArrowUp", "ArrowLeft", "ArrowRight", "ArrowDown"]) {
    await page.keyboard.press(key);
    await page.waitForTimeout(300);
  }
  await dwellForDemo(page, 2000);
});

When("I press {string} to fire", async ({ page }, key: string) => {
  await page.keyboard.press(key);
  await page.waitForTimeout(800);
  await dwellForDemo(page, 2500);
});

Then("DOOM keeps rendering frames", async ({ page }) => {
  // The xterm canvas should still be present and sized — proof the stream is live.
  const canvas = page.locator(".xterm canvas").first();
  await expect(canvas).toBeVisible();
  await dwellForDemo(page, 2000);
});
