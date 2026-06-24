import Foundation

/// Falling-glyph "digital rain". Each column is an independent stream with a
/// bright head and a trail that fades from the theme colour to black. Keying the
/// palette off `baseColor` means the rain is green under the Hacker theme, amber
/// under Amber, and so on.
public final class MatrixRainScene: SteppedScene {
    /// ASCII-only so every glyph is exactly one monospaced cell wide.
    private static let glyphs: [Character] =
        Array("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@#$%&*+=-<>?/\\|{}[]()")

    private struct Column {
        var head: Double      // row index of the leading glyph (may be < 0)
        var speed: Double     // rows per second
        var trail: Int        // trail length in rows
        var active: Bool
        var glyphs: [Character]
    }

    private var columns: [Column] = []
    private var rng = SeededGenerator(seed: 0x5EED_1234)

    public init() {
        super.init(displayName: "Matrix")
    }

    public override var settings: [SceneSetting] {
        [
            SceneSetting(id: "speed", label: "Speed", options: [
                SceneOption(label: "Slow", value: 9),
                SceneOption(label: "Normal", value: 16),
                SceneOption(label: "Fast", value: 26),
            ], defaultIndex: 1),
            SceneSetting(id: "density", label: "Density", options: [
                SceneOption(label: "Sparse", value: 0.45),
                SceneOption(label: "Normal", value: 0.72),
                SceneOption(label: "Dense", value: 0.95),
            ], defaultIndex: 1),
        ]
    }

    public override var stepInterval: Double { 1.0 / 60.0 }

    private var baseSpeed: Double { settingValue("speed", default: 16) }
    private var density: Double { settingValue("density", default: 0.72) }

    public override func reset() {
        rng = SeededGenerator(seed: 0x5EED_1234 ^ UInt64(width &* 2654435761 &+ height))
        columns = (0..<width).map { _ in makeColumn(spawnAbove: true) }
        // Stagger initial activation so the screen fills in rather than all at once.
        for i in columns.indices {
            columns[i].active = Double.random(in: 0...1, using: &rng) < density
            if columns[i].active {
                columns[i].head = Double.random(in: 0...Double(max(1, height)), using: &rng)
            }
        }
    }

    public override func step() {
        let dt = stepInterval
        for i in columns.indices {
            guard columns[i].active else {
                // Re-activate idle columns to drift toward the density target.
                if Double.random(in: 0...1, using: &rng) < density * 0.02 {
                    columns[i] = makeColumn(spawnAbove: true)
                    columns[i].active = true
                }
                continue
            }
            columns[i].head += columns[i].speed * dt
            // Shimmer: occasionally mutate a glyph in the visible trail.
            if Double.random(in: 0...1, using: &rng) < 0.10 {
                let r = Int.random(in: 0..<height, using: &rng)
                columns[i].glyphs[r] = Self.glyphs.randomElement(using: &rng)!
            }
            if columns[i].head - Double(columns[i].trail) > Double(height) {
                columns[i] = makeColumn(spawnAbove: true)
                columns[i].active = Double.random(in: 0...1, using: &rng) < density
            }
        }
    }

    public override func render() -> ColoredFrame {
        let size = width * height
        var chars = Array(repeating: Character(" "), count: size)
        var colors = Array(repeating: RGBColor?.none, count: size)
        let headColor = RGBColor.white.mixed(with: baseColor, t: 0.30)

        for (x, col) in columns.enumerated() where col.active {
            let headRow = Int(col.head.rounded(.down))
            for d in 0..<col.trail {
                let row = headRow - d
                guard row >= 0, row < height else { continue }
                let idx = row * width + x
                chars[idx] = col.glyphs[row % col.glyphs.count]
                if d == 0 {
                    colors[idx] = headColor
                } else {
                    let brightness = max(0.06, 1.0 - Double(d) / Double(col.trail))
                    colors[idx] = baseColor.scaled(brightness)
                }
            }
        }
        return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
    }

    private func makeColumn(spawnAbove: Bool) -> Column {
        let trail = Int.random(in: max(4, height / 6)...max(6, (height * 2) / 3), using: &rng)
        let speed = baseSpeed * Double.random(in: 0.6...1.3, using: &rng)
        let start = spawnAbove ? -Double.random(in: 0...Double(max(1, height)), using: &rng) : 0
        let glyphs = (0..<max(1, height)).map { _ in Self.glyphs.randomElement(using: &rng)! }
        return Column(head: start, speed: speed, trail: trail, active: false, glyphs: glyphs)
    }
}
