import AaFfi

// Swift wrapper around the aa-ffi C API.
// @unchecked Sendable: single-threaded use from the Metal render thread.
final class AaEngine: @unchecked Sendable {
    private var handle: OpaquePointer?

    init?(sceneId: String) {
        handle = aa_engine_create(sceneId)
        guard handle != nil else { return nil }
    }

    deinit {
        if let h = handle { aa_engine_destroy(h) }
    }

    func setGrid(width: Int, height: Int) {
        guard let h = handle else { return }
        aa_engine_set_grid(h, UInt32(width), UInt32(height))
    }

    func setTheme(_ name: String) {
        guard let h = handle else { return }
        aa_engine_set_theme(h, name)
    }

    func applySetting(_ id: String, value: Double) {
        guard let h = handle else { return }
        aa_engine_apply_setting(h, id, value)
    }

    // Returns a pointer valid until the next call to nextFrame or deinit.
    func nextFrame(t: Double) -> (buffer: UnsafePointer<UInt8>, width: Int, height: Int)? {
        guard let h = handle else { return nil }
        var w: UInt32 = 0, ht: UInt32 = 0
        guard let ptr = aa_engine_next_frame(h, t, &w, &ht) else { return nil }
        return (ptr, Int(w), Int(ht))
    }

    static func sceneNames() -> [String] {
        var count: UInt32 = 0
        guard let names = aa_scene_names(&count) else { return [] }
        defer { aa_scene_names_free(names, count) }
        return (0..<Int(count)).compactMap { i in
            guard let ptr = names[i] else { return nil }
            return String(validatingCString: ptr)
        }
    }
}
