import { test, expect } from "@playwright/test";
import { gridSize } from "../../../site/renderer.js";

test("gridSize floors to whole cells", () => {
  expect(gridSize(800, 600, { w: 8, h: 16 })).toEqual({ cols: 100, rows: 37 });
});

test("gridSize never returns a zero dimension", () => {
  expect(gridSize(2, 2, { w: 8, h: 16 })).toEqual({ cols: 1, rows: 1 });
});
