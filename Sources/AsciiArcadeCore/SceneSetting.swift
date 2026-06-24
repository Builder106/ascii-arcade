import Foundation

/// One discrete choice within a `SceneSetting` (e.g. "Fast" → 2.0).
public struct SceneOption: Equatable, Sendable {
    public let label: String
    public let value: Double
    public init(label: String, value: Double) {
        self.label = label
        self.value = value
    }
}

/// A scene-tunable knob exposed to the host as a submenu of discrete options.
///
/// We use discrete options rather than continuous sliders because `NSMenu`
/// renders checkmarked items cleanly without custom views, and three or four
/// presets ("Slow / Normal / Fast") cover what a wallpaper needs.
public struct SceneSetting: Sendable {
    public let id: String
    public let label: String
    public let options: [SceneOption]
    public let defaultIndex: Int

    public init(id: String, label: String, options: [SceneOption], defaultIndex: Int = 0) {
        self.id = id
        self.label = label
        self.options = options
        self.defaultIndex = max(0, min(defaultIndex, options.count - 1))
    }
}

/// A small, deterministic `RandomNumberGenerator` (SplitMix64).
///
/// Stateful scenes (Matrix rain, fire, Life, pipes) seed from this so a fixed
/// "Seed" setting reproduces the same animation and so unit tests are stable.
public struct SeededGenerator: RandomNumberGenerator {
    private var state: UInt64

    public init(seed: UInt64) {
        self.state = seed == 0 ? 0x9E37_79B9_7F4A_7C15 : seed
    }

    public mutating func next() -> UInt64 {
        state = state &+ 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}
