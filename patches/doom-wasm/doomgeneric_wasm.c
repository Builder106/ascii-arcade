/*
 * WASM platform backend for doom-ascii, implementing the DG_* interface
 * doomgeneric.h declares. Nothing about the engine, menu, or renderer
 * changes — this is the same seam doomgeneric_ascii.c implements for a
 * terminal, implemented here for a browser instead.
 *
 * DG_ScreenBuffer is a pixel buffer, not a character grid: DOOMGENERIC_RESX
 * * DOOMGENERIC_RESY uint32_t values, one per pixel, already resolved
 * through DOOM's palette. Converting that to glyphs happens in JS
 * (site/doom-play.js), not here — this file's only job is to make the
 * buffer and its dimensions reachable from JS.
 */
#include <emscripten.h>
#include "doomgeneric.h"

#define KEY_QUEUE_LEN 64

typedef struct
{
    unsigned char key;
    int pressed;
} wasm_key_event_t;

static wasm_key_event_t key_queue[KEY_QUEUE_LEN];
static int key_queue_head = 0;
static int key_queue_tail = 0;

/* Called from JS (Plan B) on every keydown/keyup and touch-control press.
 * Silently drops the event if the queue is full rather than blocking —
 * DOOM polls this once per tic (roughly 35Hz), so a full 64-slot queue
 * means input is arriving faster than the game can consume it, and
 * dropping the newest event is the right failure mode for a game loop. */
EMSCRIPTEN_KEEPALIVE
void wasm_push_key(int pressed, unsigned char key)
{
    int next = (key_queue_tail + 1) % KEY_QUEUE_LEN;
    if (next == key_queue_head)
    {
        return;
    }
    key_queue[key_queue_tail].pressed = pressed;
    key_queue[key_queue_tail].key = key;
    key_queue_tail = next;
}

void DG_Init(void)
{
    /* DG_ScreenBuffer is already allocated by dg_Create() (doomgeneric.c)
     * before DG_Init runs. No terminal, no termios, nothing to set up. */
}

void DG_DrawFrame(void)
{
    /* JS reads DG_ScreenBuffer directly via wasm_get_screen_buffer(); there
     * is nothing to push from the C side. This function exists only
     * because the DG_* interface requires it to be defined. */
}

void DG_SleepMs(uint32_t ms)
{
    /* A real sleep would block the browser's main thread under
     * emscripten_set_main_loop, freezing the tab. TryRunTics's
     * network-sync fallback path can reach this; as a no-op it just
     * spins faster, which is harmless for a local single-player game. */
    (void)ms;
}

uint32_t DG_GetTicksMs(void)
{
    return (uint32_t)emscripten_get_now();
}

int DG_GetKey(int *pressed, unsigned char *key)
{
    if (key_queue_head == key_queue_tail)
    {
        return 0;
    }
    *pressed = key_queue[key_queue_head].pressed;
    *key = key_queue[key_queue_head].key;
    key_queue_head = (key_queue_head + 1) % KEY_QUEUE_LEN;
    return 1;
}

void DG_ReadInput(void)
{
    /* The terminal backend polls raw stdin bytes here. There is nothing to
     * poll: wasm_push_key() already fills key_queue directly from JS event
     * listeners as they fire. */
}

void DG_SetWindowTitle(const char *title)
{
    EM_ASM({ document.title = UTF8ToString($0); }, title);
}

EMSCRIPTEN_KEEPALIVE
uint32_t *wasm_get_screen_buffer(void)
{
    return DG_ScreenBuffer;
}

EMSCRIPTEN_KEEPALIVE
unsigned wasm_get_screen_width(void)
{
    return DOOMGENERIC_RESX;
}

EMSCRIPTEN_KEEPALIVE
unsigned wasm_get_screen_height(void)
{
    return DOOMGENERIC_RESY;
}
