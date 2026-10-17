#!/usr/bin/env bash
# Bundle the parts of three.js that site/cabinet.js uses (including the glTF
# loader and meshopt decoder for models/cabinet.glb) into
# site/vendor/three.min.js. The site has no package manager, so the result is
# committed, the same way the design doc vendors its other browser code.
#
# Runs on the build host, never on the Mac. three.js is MIT licensed, which
# sits fine beside this repository's GPL-2.0; its licence header is kept in
# the output. Rerun after adding a three.js class to cabinet.js, and add the
# class to the export list below.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
THREE_VERSION="0.186.1"
ESBUILD_VERSION="0.28.2"
OUT="$ROOT/site/vendor/three.min.js"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
npm pack "three@$THREE_VERSION" --silent >/dev/null
tar xzf "three-$THREE_VERSION.tgz"

cat > entry.js <<'EOF'
export {
  CanvasTexture, Color, CylinderGeometry, Fog, HemisphereLight, LinearFilter,
  MathUtils, Matrix4, Mesh, MeshBasicMaterial, MeshStandardMaterial,
  NeutralToneMapping, Object3D, PerspectiveCamera, PlaneGeometry,
  PMREMGenerator, PointLight, Quaternion, Raycaster, RepeatWrapping, Scene,
  SpotLight, SRGBColorSpace, Vector2, Vector3, WebGLRenderer,
} from "./package/build/three.module.js";
export { GLTFLoader } from "./package/examples/jsm/loaders/GLTFLoader.js";
export { MeshoptDecoder } from "./package/examples/jsm/libs/meshopt_decoder.module.js";
export { RoomEnvironment } from "./package/examples/jsm/environments/RoomEnvironment.js";
EOF

mkdir -p "$(dirname "$OUT")"
npx --yes "esbuild@$ESBUILD_VERSION" entry.js --bundle --minify --format=esm \
  --alias:three=./package/build/three.module.js \
  --legal-comments=inline --outfile="$OUT"

printf 'built: %s (%s bytes, %s gzipped)\n' "$OUT" "$(wc -c < "$OUT")" "$(gzip -c "$OUT" | wc -c)"
