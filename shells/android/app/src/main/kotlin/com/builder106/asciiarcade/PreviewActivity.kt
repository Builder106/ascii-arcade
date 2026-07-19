package com.builder106.asciiarcade

import android.app.Activity
import android.app.WallpaperManager
import android.content.ComponentName
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.Button
import com.builder106.asciiarcade.engine.AaEngine
import com.builder106.asciiarcade.render.CanvasFrameRenderer
import com.builder106.asciiarcade.render.DecodedFrame
import com.builder106.asciiarcade.render.FrameDecoder
import com.builder106.asciiarcade.wallpaper.AaWallpaperService
import kotlin.math.max
import kotlin.math.roundToInt

private const val FRAME_INTERVAL_MS = 33L

/**
 * A plain Activity hosting the same rendering code as AaWallpaperService,
 * so iteration doesn't require going through Android's live-wallpaper
 * picker every time. The "Set as Live Wallpaper" button jumps into the real
 * system chooser for final, end-to-end verification.
 */
class PreviewActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var surfaceView: SurfaceView
    private var nativeEngine: AaEngine? = null
    private val handler = Handler(Looper.getMainLooper())
    private val renderer = CanvasFrameRenderer()
    private val decoded = DecodedFrame(1, 1)
    private var cols = 1
    private var rows = 1
    private var running = false
    private val startNanos = System.nanoTime()

    private val drawRunnable = object : Runnable {
        override fun run() {
            if (!running) return
            val holder = surfaceView.holder
            if (holder.surface.isValid) {
                val t = (System.nanoTime() - startNanos) / 1_000_000_000.0
                nativeEngine?.nextFrame(t)?.let { buffer ->
                    FrameDecoder.decode(buffer, cols, rows, decoded)
                    val canvas = holder.lockCanvas()
                    if (canvas != null) {
                        try {
                            renderer.drawFrame(canvas, decoded, Themes.HACKER)
                        } finally {
                            holder.unlockCanvasAndPost(canvas)
                        }
                    }
                }
            }
            handler.postDelayed(this, FRAME_INTERVAL_MS)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_preview)
        surfaceView = findViewById(R.id.surfaceView)
        surfaceView.holder.addCallback(this)
        findViewById<Button>(R.id.btnSetWallpaper).setOnClickListener { openLiveWallpaperChooser() }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        nativeEngine = AaEngine.create("donut")
        nativeEngine?.setTheme(Themes.HACKER.label)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        cols = max(1, (width / renderer.cellWidthPx).roundToInt())
        rows = max(1, (height / renderer.cellHeightPx).roundToInt())
        nativeEngine?.setGrid(cols, rows)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        stopLoop()
    }

    override fun onResume() {
        super.onResume()
        running = true
        handler.post(drawRunnable)
    }

    override fun onPause() {
        super.onPause()
        stopLoop()
    }

    override fun onDestroy() {
        nativeEngine?.close()
        nativeEngine = null
        super.onDestroy()
    }

    private fun stopLoop() {
        running = false
        handler.removeCallbacksAndMessages(null)
    }

    private fun openLiveWallpaperChooser() {
        startActivity(
            Intent(WallpaperManager.ACTION_CHANGE_LIVE_WALLPAPER).apply {
                putExtra(
                    WallpaperManager.EXTRA_LIVE_WALLPAPER_COMPONENT,
                    ComponentName(this@PreviewActivity, AaWallpaperService::class.java),
                )
            },
        )
    }
}
