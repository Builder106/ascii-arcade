import ExpoModulesCore

// AaEngine.xcframework is linked by the Xcode project (built by scripts/build-mobile-ios.sh).
// The Swift compiler sees the C declarations from aa_engine.h via the bridging header
// or the framework's module map, which the build script generates.

public class AaEngineModule: Module {
  private var engine: OpaquePointer? = nil

  deinit {
    if let e = engine {
      aa_engine_destroy(e)
    }
  }

  public func definition() -> ModuleDefinition {
    Name("AaEngine")

    Function("create") { (sceneId: String) -> Bool in
      if let e = self.engine {
        aa_engine_destroy(e)
      }
      self.engine = aa_engine_create(sceneId)
      return self.engine != nil
    }

    Function("destroy") {
      if let e = self.engine {
        aa_engine_destroy(e)
        self.engine = nil
      }
    }

    Function("setGrid") { (width: Int, height: Int) in
      guard let e = self.engine else { return }
      aa_engine_set_grid(e, UInt32(width), UInt32(height))
    }

    Function("setTheme") { (themeName: String) in
      guard let e = self.engine else { return }
      aa_engine_set_theme(e, themeName)
    }

    Function("applySetting") { (id: String, value: Double) in
      guard let e = self.engine else { return }
      aa_engine_apply_setting(e, id, value)
    }

    Function("nextFrame") { (t: Double) -> Data? in
      guard let e = self.engine else { return nil }
      var w: UInt32 = 0
      var h: UInt32 = 0
      guard let ptr = aa_engine_next_frame(e, t, &w, &h) else { return nil }
      let byteCount = Int(w) * Int(h) * 8
      // Copy into a Swift Data; the Rust buffer is owned by the engine and may
      // be invalidated on the next nextFrame() call.
      return Data(bytes: ptr, count: byteCount)
    }

    Function("sceneNames") { () -> [String] in
      var count: UInt32 = 0
      guard let names = aa_scene_names(&count) else { return [] }
      defer { aa_scene_names_free(names, count) }
      return (0..<Int(count)).compactMap { i in
        guard let ptr = names[i] else { return nil }
        return String(validatingUTF8: ptr)
      }
    }
  }
}
