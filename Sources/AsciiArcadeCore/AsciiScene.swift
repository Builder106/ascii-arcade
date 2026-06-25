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

    /// A scene with a *fixed* pixel resolution (DOOM's framebuffer) returns its
    /// grid here; the host then paints it as a scaled colour bitmap that fills
    /// the screen, instead of as font glyphs bound to the text grid. Return
    /// `nil` (the default) for a normal text scene driven by `setGrid`.
    var fixedGrid: (width: Int, height: Int)? { get }

    /// The current frame: `height` newline-joined rows, each `width` columns wide.
    func frame(atTime t: Double) -> String

    /// The current frame with per-cell colour. Return `nil` (the default) for a
    /// monochrome scene — the host then paints every glyph in the theme colour.
    /// Colour scenes (Matrix, fire, pipes, DOOM) override this.
    func coloredFrame(atTime t: Double) -> ColoredFrame?

    /// Tell the scene the theme's text colour, so colour scenes can key their
    /// palette off it (e.g. Matrix rain turns amber under the Amber theme).
    /// Default: ignored.
    func applyBaseColor(_ color: RGBColor)

    /// User-tunable knobs for this scene (speed, density, …), surfaced by the
    /// host as a menu. Default: none.
    var settings: [SceneSetting] { get }

    /// Apply a value chosen for one of `settings`. Default: ignored.
    func applySetting(id: String, value: Double)

    /// Forward raw key bytes to the scene. Default: ignored.
    func sendKey(_ bytes: [UInt8])

    /// Begin/stop any backing work (spawning a process, etc.). Default: no-op.
    func start()
    func stop()
}

public extension AsciiScene {
    var isInteractive: Bool { false }
    var fixedGrid: (width: Int, height: Int)? { nil }
    func coloredFrame(atTime t: Double) -> ColoredFrame? { nil }
    func applyBaseColor(_ color: RGBColor) {}
    var settings: [SceneSetting] { [] }
    func applySetting(id: String, value: Double) {}
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
