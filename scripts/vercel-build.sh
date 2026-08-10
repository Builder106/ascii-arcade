#!/usr/bin/env bash
# Vercel's own build entrypoint (see vercel.json's buildCommand — it's
# capped at 256 characters, too short for this directly). Builds both
# WASM targets the site needs: aa-core (Rust, via build-wasm.sh) and
# doom-wasm (C, via build-doom-wasm.sh).
#
# Rust is already present in Vercel's build image; Emscripten is not, so
# this installs a pinned emsdk before delegating to build-doom-wasm.sh.
# build-doom-wasm.sh's own comment says "never locally, per this repo's
# standing VM-only build rule" — that rule is about not installing heavy
# toolchains on the developer's own Mac (see CLAUDE.md), not about CI in
# general; Vercel's build container is an ephemeral cloud sandbox in the
# same category as the GitHub Actions runner pages.yml already builds
# aa-core's own WASM target on, not "the Mac" the rule is about.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EMSDK_VERSION="6.0.6" # matches the version pinned on ampere-dev

rustup target add wasm32-unknown-unknown
bash "$ROOT/scripts/build-wasm.sh"

if ! command -v emcc >/dev/null 2>&1; then
	git clone --depth 1 https://github.com/emscripten-core/emsdk.git /tmp/emsdk
	/tmp/emsdk/emsdk install "$EMSDK_VERSION"
	/tmp/emsdk/emsdk activate "$EMSDK_VERSION"
	# shellcheck source=/dev/null
	source /tmp/emsdk/emsdk_env.sh
fi

bash "$ROOT/scripts/build-doom-wasm.sh"
