import Foundation

/// Base class for the stateful, time-stepped scenes (Matrix rain, fire, Life,
/// pipes). It turns the host's "give me the frame at time `t`" pull into a
/// fixed-timestep simulation: it accumulates wall-clock `dt` and runs `step()`
/// at `stepInterval` regardless of the display refresh rate.
///
/// Everything here runs on the main thread — `setGrid`/`coloredFrame` are called
/// from `NSView.draw`, and the menu callbacks (`applyBaseColor`/`applySetting`)
/// are also main-thread — so no locking is needed (unlike `DoomScene`, whose PTY
/// feeds it off-thread).
public class SteppedScene: AsciiScene {
    public let displayName: String
    public private(set) var width: Int
    public private(set) var height: Int

    /// Theme text colour the host pushed in; colour scenes key their palette off it.
    public private(set) var baseColor: RGBColor = RGBColor(r: 0, g: 255, b: 65)

    private var settingValues: [String: Double] = [:]
    private var lastTime: Double = 0
    private var accumulator: Double = 0
    private var started = false

    public init(displayName: String, initialWidth: Int = 10, initialHeight: Int = 10) {
        self.displayName = displayName
        self.width = max(1, initialWidth)
        self.height = max(1, initialHeight)
        for s in settings { settingValues[s.id] = s.options[s.defaultIndex].value }
    }

    // MARK: - Overridable hooks

    /// Simulation seconds between `step()` calls. Override per scene.
    public var stepInterval: Double { 1.0 / 30.0 }

    /// User-tunable knobs. Declared here (not just on the protocol) so concrete
    /// scenes can `override` it. Override per scene.
    public var settings: [SceneSetting] { [] }

    /// (Re)seed all state for the current `width`/`height`. Called on resize and start.
    public func reset() {}

    /// Advance the simulation by one tick.
    public func step() {}

    /// Build the current frame from state.
    public func render() -> ColoredFrame {
        ColoredFrame(width: width, height: height,
                     chars: Array(repeating: " ", count: width * height),
                     colors: Array(repeating: nil, count: width * height))
    }

    /// Called after a setting value changes, in case a scene must rebuild state.
    public func settingsChanged() {}

    // MARK: - Setting access for subclasses

    public func settingValue(_ id: String, default def: Double) -> Double {
        settingValues[id] ?? def
    }

    // MARK: - AsciiScene

    public func setGrid(width: Int, height: Int) {
        guard width > 0, height > 0 else { return }
        guard width != self.width || height != self.height else { return }
        self.width = width
        self.height = height
        reset()
    }

    public func frame(atTime t: Double) -> String {
        coloredFrame(atTime: t)?.text() ?? blankText()
    }

    public func coloredFrame(atTime t: Double) -> ColoredFrame? {
        advance(to: t)
        return render()
    }

    public func applyBaseColor(_ color: RGBColor) {
        baseColor = color
    }

    public func applySetting(id: String, value: Double) {
        settingValues[id] = value
        settingsChanged()
    }

    public func start() {
        started = false
        accumulator = 0
        reset()
    }

    public func stop() {}

    // MARK: - Internals

    private func advance(to t: Double) {
        if !started {
            started = true
            lastTime = t
            accumulator = 0
            return
        }
        var dt = t - lastTime
        lastTime = t
        if dt < 0 { dt = 0 }          // clock reset on scene switch
        if dt > 0.25 { dt = 0.25 }    // clamp after a stall so we don't fast-forward
        accumulator += dt
        let interval = max(stepInterval, 0.0001)
        var budget = 12               // cap catch-up work per frame
        while accumulator >= interval && budget > 0 {
            step()
            accumulator -= interval
            budget -= 1
        }
    }

    private func blankText() -> String {
        var out = ""
        for row in 0..<height {
            out.append(String(repeating: " ", count: width))
            if row < height - 1 { out.append("\n") }
        }
        return out
    }
}
