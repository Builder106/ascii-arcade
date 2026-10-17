import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { test, expect, SITE_DIR } from "./fixtures";
import { checkCabinetModel } from "../../../scripts/check-cabinet-model.mjs";

// The same check scripts/optimize-cabinet.sh runs, so a model that lost a
// node, a UV set or a material can't reach the page by skipping the script.
test("the cabinet model has every node, UV set and material the page uses", () => {
  expect(checkCabinetModel(readFileSync(resolve(SITE_DIR, "models/cabinet.glb")))).toEqual([]);
});
