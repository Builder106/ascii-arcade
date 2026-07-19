package com.builder106.asciiarcade.engine

/**
 * Kotlin wrapper around the aa-ffi C API, 1:1 in shape with
 * shells/ios/AsciiArcade/Engine/AaEngine.swift.
 *
 * Unlike the C ABI's aa_engine_next_frame (which writes width/height via
 * out-params), nativeNextFrame returns only the raw byte[] — callers must
 * track the (width, height) they last passed to setGrid themselves. This is
 * safe here because every scene reachable from this shell (donut, and every
 * other built-in scene) unconditionally honors setGrid.
 */
class AaEngine private constructor(private var handle: Long) : AutoCloseable {

    companion object {
        fun create(sceneId: String): AaEngine? {
            val handle = AaEngineNative.nativeCreate(sceneId)
            return if (handle == 0L) null else AaEngine(handle)
        }

        fun sceneNames(): List<String> = AaEngineNative.nativeSceneNames().toList()
    }

    fun setGrid(width: Int, height: Int) {
        if (handle != 0L) AaEngineNative.nativeSetGrid(handle, width, height)
    }

    fun setTheme(name: String) {
        if (handle != 0L) AaEngineNative.nativeSetTheme(handle, name)
    }

    fun applySetting(id: String, value: Double) {
        if (handle != 0L) AaEngineNative.nativeApplySetting(handle, id, value)
    }

    fun nextFrame(t: Double): ByteArray? =
        if (handle == 0L) null else AaEngineNative.nativeNextFrame(handle, t)

    override fun close() {
        if (handle != 0L) {
            AaEngineNative.nativeDestroy(handle)
            handle = 0L
        }
    }

    /** Backstop only — close() called explicitly from onDestroy() is the real contract. */
    protected fun finalize() {
        close()
    }
}
