package com.builder106.asciiarcade.engine

// Raw JNI declarations — the Kotlin-side analogue of aa_engine.h.
//
// This MUST be a top-level `object`, never a `companion object`: a
// `@JvmStatic external fun` inside a companion mangles to a
// `$Companion`-suffixed JNI symbol (e.g. Java_..._AaEngine_00024Companion_...),
// because there's no method body for Kotlin to generate a static-delegate
// bridge into — that bridging trick only works for functions with real
// bodies. A plain top-level `object` *is* already the outer class, so
// `@JvmStatic external fun` compiles to a true `public static native` method
// directly on it, matching crates/aa-ffi's `Java_com_builder106_asciiarcade_
// engine_AaEngineNative_native*` symbols with no name-mangling ambiguity.
internal object AaEngineNative {
    init {
        System.loadLibrary("aa_ffi")
    }

    @JvmStatic external fun nativeCreate(sceneId: String): Long
    @JvmStatic external fun nativeDestroy(handle: Long)
    @JvmStatic external fun nativeSetGrid(handle: Long, width: Int, height: Int)
    @JvmStatic external fun nativeSetTheme(handle: Long, themeName: String)
    @JvmStatic external fun nativeApplySetting(handle: Long, settingId: String, value: Double)
    @JvmStatic external fun nativeNextFrame(handle: Long, t: Double): ByteArray?
    @JvmStatic external fun nativeSceneNames(): Array<String>
}
