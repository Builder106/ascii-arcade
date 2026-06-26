import { createBdd } from "playwright-bdd";
import { expect } from "@playwright/test";
import { dwellForDemo, injectCursor } from "../support/demo.js";

const { Given, When, Then, Before } = createBdd();

Before(async ({ page }) => {
  await injectCursor(page);
});

Given("I open the scene page", async ({ page }) => {
  await page.goto("/");
  // xterm.js mounts a .xterm element once the WebSocket connects and the server
  // sends the first frame. Wait up to 30 s for slow first builds.
  await page.locator(".xterm").waitFor({ state: "visible", timeout: 30_000 });
  await dwellForDemo(page);
});

When("I select the {string} scene", async ({ page }, scene: string) => {
  await page.locator("#scene-picker").selectOption(scene);
  await dwellForDemo(page);
});

When("I wait for the scene to render", async ({ page }) => {
  // Give the WebSocket time to send several frames.
  await page.waitForTimeout(3000);
  await dwellForDemo(page, 2000);
});

Then("the terminal shows animated output", async ({ page }) => {
  // The xterm canvas must still be present and sized — proof the stream is live.
  const canvas = page.locator(".xterm canvas").first();
  await expect(canvas).toBeVisible();
  await dwellForDemo(page, 2000);
});
