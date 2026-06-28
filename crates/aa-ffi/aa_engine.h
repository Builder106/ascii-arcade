#ifndef AA_ENGINE_H
#define AA_ENGINE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AaEngine AaEngine;

/**
 * Construct an engine running the named scene.
 * Valid scene ids: "donut", "helix", "matrix", "fire", "pipes", "life", "clock".
 * Returns NULL for unknown ids.
 */
AaEngine* aa_engine_create(const char* scene_id);

/** Destroy the engine and release all resources. */
void aa_engine_destroy(AaEngine* engine);

/** Resize the character grid the scene renders into. */
void aa_engine_set_grid(AaEngine* engine, uint32_t width, uint32_t height);

/**
 * Set the colour theme by name ("Hacker", "Amber", "Ice", "Ghost").
 * Unknown names are silently ignored.
 */
void aa_engine_set_theme(AaEngine* engine, const char* theme_name);

/** Forward a scene-specific setting (id + numeric value). */
void aa_engine_apply_setting(AaEngine* engine, const char* id, double value);

/**
 * Render the next frame at animation time `t` (seconds).
 *
 * Returns a pointer to a flat byte buffer owned by the engine — valid until
 * the next call to `aa_engine_next_frame` or `aa_engine_destroy`.
 * Returns NULL on error.
 *
 * Buffer layout: width * height cells, 8 bytes each:
 *   [0–3]  char as uint32_t little-endian (Unicode scalar value)
 *   [4]    red   (0 when has_color == 0)
 *   [5]    green
 *   [6]    blue
 *   [7]    has_color: 1 = use rgb above, 0 = use the active theme colour
 */
const uint8_t* aa_engine_next_frame(
    AaEngine*  engine,
    double     t,
    uint32_t*  out_width,
    uint32_t*  out_height
);

/**
 * Return a null-terminated array of built-in scene id strings.
 * `*out_count` is set to the number of ids (excluding the null terminator).
 * Free the result with `aa_scene_names_free(names, count)`.
 */
char** aa_scene_names(uint32_t* out_count);

/** Free a names array returned by `aa_scene_names`. */
void aa_scene_names_free(char** names, uint32_t count);

#ifdef __cplusplus
}
#endif

#endif /* AA_ENGINE_H */
