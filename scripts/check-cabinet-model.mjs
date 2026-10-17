// Checks that a cabinet GLB still has everything site/cabinet.js relies on:
// the named nodes it finds, UVs on the meshes it paints, and each painted mesh
// on its own material. Optimisers drop or merge these without a word, so this
// makes that loud. Used by scripts/optimize-cabinet.sh and the e2e suite.
//
// Usage: node scripts/check-cabinet-model.mjs site/models/cabinet.glb

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const NODES = [
  "SCREEN", "SCREEN_CENTER", "SCREEN_UP", "SCREEN_OUT", "STICK", "BUTTON_0", "BUTTON_1",
  "COIN_SLOT_L", "COIN_SLOT_R", "PART_title", "PART_demo", "PART_scenes", "PART_rules", "PART_install",
];

// Node name -> the material its mesh must use. All of these get UVs painted.
export const PAINTED = {
  SCREEN: "MAT_Screen",
  MARQUEE: "MAT_Marquee",
  PANEL_ART: "MAT_PanelArt",
  PRICE_L: "MAT_Price",
  PRICE_R: "MAT_Price",
  COIN_LABEL: "MAT_CoinLabel",
  SIDE_L: "MAT_SideArt",
  SIDE_R: "MAT_SideArt",
};

/** Returns the GLB's JSON chunk. */
export function glbJson(buffer) {
  if (buffer.readUInt32LE(0) !== 0x46546c67) throw new Error("not a GLB file");
  const length = buffer.readUInt32LE(12);
  return JSON.parse(buffer.subarray(20, 20 + length).toString("utf8"));
}

/** Returns a list of problems; empty means the page can use the model. */
export function checkCabinetModel(buffer) {
  const gltf = glbJson(buffer);
  const problems = [];
  const nodes = new Map((gltf.nodes ?? []).map((n) => [n.name, n]));
  const materials = (gltf.materials ?? []).map((m) => m.name);
  for (const name of NODES) if (!nodes.has(name)) problems.push(`missing node ${name}`);
  for (const [name, material] of Object.entries(PAINTED)) {
    const node = nodes.get(name);
    if (!node || node.mesh === undefined) {
      problems.push(`missing mesh node ${name}`);
      continue;
    }
    const primitives = gltf.meshes[node.mesh].primitives;
    const own = primitives.filter((p) => materials[p.material] === material);
    if (own.length === 0) problems.push(`${name} does not use ${material}`);
    if (own.some((p) => p.attributes.TEXCOORD_0 === undefined)) problems.push(`${name} has no UVs for ${material}`);
  }
  return problems;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const file = process.argv[2];
  if (!file) {
    console.error("usage: check-cabinet-model.mjs path/to/cabinet.glb");
    process.exit(2);
  }
  const problems = checkCabinetModel(readFileSync(file));
  if (problems.length) {
    console.error(`${file} is missing what site/cabinet.js needs:\n  ${problems.join("\n  ")}`);
    process.exit(1);
  }
  console.log(`${file}: all ${NODES.length} nodes and ${Object.keys(PAINTED).length} painted meshes present`);
}
