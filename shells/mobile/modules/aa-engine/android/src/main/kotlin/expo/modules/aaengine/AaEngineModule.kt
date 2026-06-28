package expo.modules.aaengine

import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

class AaEngineModule : Module() {

  companion object {
    init {
      // libaa_ffi.so is placed in jniLibs by scripts/build-mobile-android.sh.
      System.loadLibrary("aa_ffi")
    }
  }

  // ── JNI declarations ─────────────────────────────────────────────────────
  // Implemented in crates/aa-ffi/src/lib.rs (android mod).
  // The handle is an AaEngine* cast to Long (stable across GC moves because
  // it lives in Rust-managed heap, not the Java heap).

  private external fun nativeCreate(sceneName: String): Long
  private external fun nativeDestroy(handle: Long)
  private external fun nativeSetGrid(handle: Long, width: Int, height: Int)
  private external fun nativeSetTheme(handle: Long, themeName: String)
  private external fun nativeApplySetting(handle: Long, id: String, value: Double)
  private external fun nativeNextFrame(handle: Long, t: Double): ByteArray?
  private external fun nativeSceneNames(): Array<String>

  // ── State ────────────────────────────────────────────────────────────────

  private var engineHandle: Long = 0L

  // ── Expo module definition ───────────────────────────────────────────────

  override fun definition() = ModuleDefinition {
    Name("AaEngine")

    Function("create") { sceneId: String ->
      if (engineHandle != 0L) nativeDestroy(engineHandle)
      engineHandle = nativeCreate(sceneId)
      engineHandle != 0L
    }

    Function("destroy") {
      if (engineHandle != 0L) {
        nativeDestroy(engineHandle)
        engineHandle = 0L
      }
    }

    Function("setGrid") { width: Int, height: Int ->
      if (engineHandle != 0L) nativeSetGrid(engineHandle, width, height)
    }

    Function("setTheme") { themeName: String ->
      if (engineHandle != 0L) nativeSetTheme(engineHandle, themeName)
    }

    Function("applySetting") { id: String, value: Double ->
      if (engineHandle != 0L) nativeApplySetting(engineHandle, id, value)
    }

    Function("nextFrame") { t: Double ->
      if (engineHandle == 0L) return@Function null
      // Kotlin ByteArray → Expo converts to JS ArrayBuffer automatically.
      nativeNextFrame(engineHandle, t)
    }

    Function("sceneNames") {
      nativeSceneNames().toList()
    }

    // Release native resources when the React context tears down.
    OnDestroy {
      if (engineHandle != 0L) {
        nativeDestroy(engineHandle)
        engineHandle = 0L
      }
    }
  }
}
