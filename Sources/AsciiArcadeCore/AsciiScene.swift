import Foundation

/// A selectable ASCII experience that can be rendered into the wallpaper window.
///
/// The donut/helix generators are pull-based (`frame(atTime:)` computes a fresh
/// frame on demand); DOOM is push-based (a PTY streams frames that a screen
/// buffer reconstructs). This protocol hides that difference so the wallpaper
/// host can treat every cabinet the same way.
public protocol AsciiScene: AnyObject {
    var displayName: String { get }

    /// Whether the scene consumes keyboard input. DOOM does; the math scenes don't.
    var isInteractive: Bool { get }

    /// Resize the character grid the scene renders into.
    func setGrid(width: Int, height: Int)

    /// The current frame: `height` newline-joined rows, each `width` columns wide.
    func frame(atTime t: Double) -> String

    /// Forward raw key bytes to the scene. Default: ignored.
    func sendKey(_ bytes: [UInt8])

    /// Begin/stop any backing work (spawning a process, etc.). Default: no-op.
    func start()
    func stop()
}

public extension AsciiScene {
    var isInteractive: Bool { false }
    func sendKey(_ bytes: [UInt8]) {}
    func start() {}
    func stop() {}
}

/// Wraps a pull-based `ShapeFrameGenerator` (donut, helix, …) as a scene.
public final class GeneratorScene: AsciiScene {
    public let displayName: String
    private let factory: (Int, Int) -> any ShapeFrameGenerator
    private var generator: any ShapeFrameGenerator

    public init(displayName: String, factory: @escaping (Int, Int) -> any ShapeFrameGenerator) {
        self.displayName = displayName
        self.factory = factory
        self.generator = factory(10, 10)
    }

    public func setGrid(width: Int, height: Int) {
        guard width > 0, height > 0 else { return }
        if width != generator.width || height != generator.height {
            generator = factory(width, height)
        }
    }

    public func frame(atTime t: Double) -> String {
        generator.frame(atTime: t)
    }
}
