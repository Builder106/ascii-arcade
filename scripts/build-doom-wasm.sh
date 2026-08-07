#!/usr/bin/env bash
# Builds doom-ascii to WebAssembly: pinned upstream clone, this repo's
# patches applied, compiled with Emscripten. Run on ampere-dev — never
# locally, per this repo's standing VM-only build rule.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
PATCH_DIR="$ROOT/patches/doom-wasm"
WAD="$ROOT/wad/freedoom1.wad"
OUT_DIR="$ROOT/site/doom-wasm"

PINNED_COMMIT="b5188d7c9c4da6c81264a7803e8725ac3df2cfea"

if [ ! -f "$WAD" ]; then
	echo "missing $WAD" >&2
	exit 1
fi

if ! command -v emcc >/dev/null 2>&1; then
	# emsdk_env.sh isn't sourced by non-interactive shells by default.
	if [ -f "$HOME/emsdk/emsdk_env.sh" ]; then
		# shellcheck source=/dev/null
		source "$HOME/emsdk/emsdk_env.sh"
	fi
fi
if ! command -v emcc >/dev/null 2>&1; then
	echo "emcc not found; run: cd ~/emsdk && ./emsdk install latest && ./emsdk activate latest" >&2
	exit 1
fi

BUILD_DIR="$(mktemp -d /tmp/doom-wasm-build-XXXXXX)"
CLEANUP() { rm -rf "$BUILD_DIR" || true; }
trap CLEANUP EXIT

cd "$BUILD_DIR"
git clone https://github.com/wojciech-graj/doom-ascii.git
cd doom-ascii
git checkout "$PINNED_COMMIT"

cp "$PATCH_DIR/doomgeneric_wasm.c" src/doomgeneric_wasm.c
git apply "$PATCH_DIR/main-loop.patch"

# Same 72-file list the native Makefile's SRC variable builds, with
# doomgeneric_ascii.c swapped for the new WASM backend.
SRC_FILES="i_main.c dummy.c am_map.c doomdef.c doomstat.c dstrings.c d_event.c d_items.c d_iwad.c \
	d_loop.c d_main.c d_mode.c d_net.c f_finale.c f_wipe.c g_game.c hu_lib.c hu_stuff.c info.c \
	i_cdmus.c i_endoom.c i_joystick.c i_scale.c i_sound.c i_system.c i_timer.c memio.c m_argv.c \
	m_bbox.c m_cheat.c m_config.c m_controls.c m_fixed.c m_menu.c m_misc.c m_random.c \
	p_ceilng.c p_doors.c p_enemy.c p_floor.c p_inter.c p_lights.c p_map.c p_maputl.c p_mobj.c \
	p_plats.c p_pspr.c p_saveg.c p_setup.c p_sight.c p_spec.c p_switch.c p_telept.c p_tick.c \
	p_user.c r_bsp.c r_data.c r_draw.c r_main.c r_plane.c r_segs.c r_sky.c r_things.c sha1.c \
	sounds.c statdump.c st_lib.c st_stuff.c s_sound.c tables.c v_video.c wi_stuff.c \
	w_checksum.c w_file.c w_main.c w_wad.c z_zone.c w_file_stdc.c i_input.c i_video.c \
	doomgeneric.c doomgeneric_wasm.c"

mkdir -p "$OUT_DIR"

cd src
# _DEFAULT_SOURCE: the native Makefile builds with the same c99 mode and
# works fine, since host glibc exposes POSIX functions like strdup() under
# c99 anyway. Emscripten's libc is stricter about feature-test macros and
# needs this said explicitly, or d_iwad.c fails with "call to undeclared
# function 'strdup'".
#
# gnu99, not c99: doomgeneric_wasm.c's DG_SetWindowTitle uses EM_ASM, which
# needs GNU statement-expression support Emscripten refuses to compile
# under strict -std=c*. gnu99 is a strict superset of c99, so this doesn't
# change how the other 71 files (all c99-conformant already, verified by
# this exact build succeeding on them under gnu99) get compiled.
# shellcheck disable=SC2086
emcc $SRC_FILES \
	-O2 -DNORMALUNIX -DLINUX -std=gnu99 -D_DEFAULT_SOURCE \
	-s WASM=1 \
	-s MODULARIZE=1 \
	-s EXPORT_ES6=1 \
	-s EXPORT_NAME=DoomModule \
	-s 'EXPORTED_FUNCTIONS=["_main","_wasm_push_key","_wasm_get_screen_buffer","_wasm_get_screen_width","_wasm_get_screen_height"]' \
	-s 'EXPORTED_RUNTIME_METHODS=["cwrap"]' \
	-s ALLOW_MEMORY_GROWTH=1 \
	--preload-file "$WAD@/freedoom1.wad" \
	-o "$OUT_DIR/doom.js"

echo "built $OUT_DIR/doom.js"
ls -la "$OUT_DIR"
