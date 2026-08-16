package com.builder106.asciiarcade.wallpaper

import android.os.Handler
import android.os.Looper
import android.service.wallpaper.WallpaperService
import android.view.SurfaceHolder
import com.builder106.asciiarcade.Themes
import com.builder106.asciiarcade.engine.AaEngine
import com.builder106.asciiarcade.render.CanvasFrameRenderer
import com.builder106.asciiarcade.render.DecodedFrame
import com.builder106.asciiarcade.render.FrameDecoder
import kotlin.math.max
import kotlin.math.roundToInt

// ~30fps, matching this codebase's own aa-web shell precedent. A wallpaper
// sits behind app icons — matching a phone's native (often 90-120Hz) refresh
// rate via Choreographer would burn meaningfully more battery for no visible
// benefit, and VSYNC delivery is less predictable for a frequently-obscured,
// unfocused surface than a plain Looper timer.
private const val FRAME_INTERVAL_MS = 33L

class AaWallpaperService : WallpaperService() {
    override fun onCreateEngine(): Engine = AaWallpaperEngine()

    private inner class AaWallpaperEngine : Engine() {
        private var nativeEngine: AaEngine? = null
        private val handler = Handler(Looper.getMainLooper())
        private val renderer = CanvasFrameRenderer()
        private val decoded = DecodedFrame(1, 1)
        private var cols = 1
        private var rows = 1
        private var visible = false
        private var inAmbientMode = false
        private val startNanos = System.nanoTime()

        private val drawRunnable = object : Runnable {
            override fun run() {
                drawFrame()
                if (visible) handler.postDelayed(this, FRAME_INTERVAL_MS)
            }
        }

        override fun onCreate(holder: SurfaceHolder) {
            super.onCreate(holder)
            nativeEngine = AaEngine.create("donut")
            nativeEngine?.setTheme(Themes.HACKER.label)
        }

        override fun onSurfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
            super.onSurfaceChanged(holder, format, width, height)
            cols = max(1, (width / renderer.cellWidthPx).roundToInt())
            rows = max(1, (height / renderer.cellHeightPx).roundToInt())
            nativeEngine?.setGrid(cols, rows)
        }

        // The battery-drain correctness point: a wallpaper is "invisible" far
        // more often than "destroyed" — home screen covered by an app, screen
        // off, lock screen swapped out — and the frame loop must stop every
        // one of those times, not just at actual teardown.
        override fun onVisibilityChanged(visible: Boolean) {
            this.visible = visible
            handler.removeCallbacks(drawRunnable)
            if (visible && !inAmbientMode) handler.post(drawRunnable)
        }

        override fun onAmbientModeChanged(inAmbientMode: Boolean, animated: Boolean) {
            super.onAmbientModeChanged(inAmbientMode, animated)
            this.inAmbientMode = inAmbientMode
            handler.removeCallbacks(drawRunnable)
            if (visible && !inAmbientMode) {
                handler.post(drawRunnable)
            } else if (visible && inAmbientMode) {
                drawFrame() // Draw one static frame for AOD
            }
        }

        override fun onSurfaceDestroyed(holder: SurfaceHolder) {
            visible = false
            handler.removeCallbacks(drawRunnable)
            super.onSurfaceDestroyed(holder)
        }

        override fun onDestroy() {
            handler.removeCallbacks(drawRunnable)
            // Release the native handle deterministically — no Kotlin
            // deinit, don't rely on GC finalization to free the Rust-side
            // engine and its frame buffer.
            nativeEngine?.close()
            nativeEngine = null
            super.onDestroy()
        }

        private fun drawFrame() {
            if (!surfaceHolder.surface.isValid) return
            val t = (System.nanoTime() - startNanos) / 1_000_000_000.0
            val buffer = nativeEngine?.nextFrame(t) ?: return
            FrameDecoder.decode(buffer, cols, rows, decoded)
            val canvas = surfaceHolder.lockCanvas() ?: return
            try {
                renderer.drawFrame(canvas, decoded, Themes.HACKER)
            } finally {
                surfaceHolder.unlockCanvasAndPost(canvas)
            }
        }
    }
}
