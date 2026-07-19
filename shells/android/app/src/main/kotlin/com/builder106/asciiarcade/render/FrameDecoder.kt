package com.builder106.asciiarcade.render

import android.graphics.Color

/**
 * Reusable scratch buffers for one decoded frame — grown, never reallocated
 * per frame (mirrors the discipline of the instance buffer in
 * shells/ios/AsciiArcade/Renderer/ArcadeRenderer.swift).
 */
class DecodedFrame(w: Int, h: Int) {
    var width = w
    var height = h
    var codepoints = IntArray(w * h)
    var colorArgb = IntArray(w * h)
    var hasColor = BooleanArray(w * h)

    fun ensureCapacity(w: Int, h: Int) {
        if (w != width || h != height || codepoints.size < w * h) {
            width = w
            height = h
            val n = w * h
            codepoints = IntArray(n)
            colorArgb = IntArray(n)
            hasColor = BooleanArray(n)
        }
    }
}

/**
 * Decodes the aa-ffi frame buffer — mirrors
 * shells/ios/AsciiArcade/Renderer/FrameEncoding.swift's buildGlyphInstances,
 * but decodes into plain arrays for a Canvas draw call instead of GPU
 * instances.
 *
 * Buffer layout, 8 bytes/cell: [0-3] Unicode codepoint as u32 LE,
 * [4] R, [5] G, [6] B, [7] has_color (1 = use RGB, 0 = use active theme).
 * JNI byte[] is signed — every byte must be masked `and 0xFF` before use.
 */
object FrameDecoder {
    fun decode(buffer: ByteArray, width: Int, height: Int, out: DecodedFrame) {
        out.ensureCapacity(width, height)
        for (i in 0 until width * height) {
            val off = i * 8
            out.codepoints[i] = (buffer[off].toInt() and 0xFF) or
                ((buffer[off + 1].toInt() and 0xFF) shl 8) or
                ((buffer[off + 2].toInt() and 0xFF) shl 16) or
                ((buffer[off + 3].toInt() and 0xFF) shl 24)

            val hasColor = buffer[off + 7] == 1.toByte()
            out.hasColor[i] = hasColor
            if (hasColor) {
                out.colorArgb[i] = Color.rgb(
                    buffer[off + 4].toInt() and 0xFF,
                    buffer[off + 5].toInt() and 0xFF,
                    buffer[off + 6].toInt() and 0xFF,
                )
            }
        }
    }
}
