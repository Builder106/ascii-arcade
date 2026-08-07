import { test, expect } from "@playwright/test";
import { makeThresholds, blend } from "../../../site/dissolve.js";

test("thresholds are stable for a seed", () => {
  expect(Array.from(makeThresholds(64, 7))).toEqual(
    Array.from(makeThresholds(64, 7)),
  );
});

test("different seeds scatter differently", () => {
  expect(Array.from(makeThresholds(64, 7))).not.toEqual(
    Array.from(makeThresholds(64, 8)),
  );
});

test("progress 0 is the from frame and 1 is the to frame", () => {
  const from = "aaaa".split("");
  const to = "bbbb".split("");
  const fc = new Uint32Array([0, 0, 0, 0]);
  const tc = new Uint32Array([1, 1, 1, 1]);
  const th = makeThresholds(4, 1);

  expect(blend(from, to, fc, tc, th, 0).glyphs.join("")).toBe("aaaa");
  expect(blend(from, to, fc, tc, th, 1).glyphs.join("")).toBe("bbbb");
});

test("colour follows the glyph it belongs to", () => {
  const th = makeThresholds(4, 1);
  const out = blend(
    "aaaa".split(""),
    "bbbb".split(""),
    new Uint32Array([7, 7, 7, 7]),
    new Uint32Array([9, 9, 9, 9]),
    th,
    1,
  );
  expect(Array.from(out.colors)).toEqual([9, 9, 9, 9]);
});

test("a partial dissolve is a mix of both frames", () => {
  const n = 400;
  const from = Array(n).fill("a");
  const to = Array(n).fill("b");
  const fc = new Uint32Array(n);
  const tc = new Uint32Array(n).fill(1);
  const out = blend(from, to, fc, tc, makeThresholds(n, 3), 0.5).glyphs.join("");

  expect(out).toContain("a");
  expect(out).toContain("b");
});
