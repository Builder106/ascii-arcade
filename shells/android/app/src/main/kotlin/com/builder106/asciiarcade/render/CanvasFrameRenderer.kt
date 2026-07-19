package com.builder106.asciiarcade.render

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Typeface
import com.builder106.asciiarcade.AaTheme

/**
 * Draws a decoded frame directly via android.graphics.Canvas — the Canvas 2D
 * renderer chosen over a GLSurfaceView/glyph-atlas port, matching the
 * Windows shell's existing simple immediate-mode approach. Simplest-correct
 * version first: allocates a small String per glyph per frame via
 * Character.toChars. If profiling on a real device later shows GC pressure,
 * switch to Canvas.drawText(CharArray, ...) with a reused scratch buffer —
 * not worth the complexity before that's measured.
 */
class CanvasFrameRenderer(textSizePx: Float = 28f) {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        typeface = Typeface.MONOSPACE
        textSize = textSizePx
    }

    val cellWidthPx: Float = paint.measureText("0")
    val cellHeightPx: Float = paint.fontMetrics.let { it.descent - it.ascent }

    fun drawFrame(canvas: Canvas, frame: DecodedFrame, theme: AaTheme) {
        canvas.drawColor(theme.background)
        val ascent = -paint.fontMetrics.ascent
        for (row in 0 until frame.height) {
            for (col in 0 until frame.width) {
                val i = row * frame.width + col
                val codepoint = frame.codepoints[i]
                if (codepoint == 0 || codepoint == 0x20) continue
                paint.color = if (frame.hasColor[i]) frame.colorArgb[i] else theme.text
                canvas.drawText(
                    String(Character.toChars(codepoint)),
                    col * cellWidthPx,
                    row * cellHeightPx + ascent,
                    paint,
                )
            }
        }
    }
}
