import Foundation

/// The classic "Doom fire" cellular effect (after Fabien Sanglard's writeup):
/// the bottom row is held at maximum heat and each cell above cools from the
/// cell below it with a little random lateral drift. Heat maps to both a glyph
/// (density ramp) and a black→red→orange→yellow→white palette.
public final class FireScene: SteppedScene {
    private static let maxHeat = 36
    private static let ramp: [Character] = Array(" ..::--==+++***###%%@@")

    private var heat: [Int] = []         // row-major, 0…maxHeat
    private var palette: [RGBColor] = []
    private var rng = SeededGenerator(seed: 0xF15E_0FEE)

    public init() {
        super.init(displayName: "Fire")
        palette = Self.buildPalette()
    }

    public override var settings: [SceneSetting] {
        [
            SceneSetting(id: "intensity", label: "Intensity", options: [
                SceneOption(label: "Calm", value: 0),
                SceneOption(label: "Normal", value: 1),
                SceneOption(label: "Inferno", value: 2),
            ], defaultIndex: 1),
            SceneSetting(id: "wind", label: "Wind", options: [
                SceneOption(label: "Left", value: -1),
                SceneOption(label: "None", value: 0),
                SceneOption(label: "Right", value: 1),
            ], defaultIndex: 1),
        ]
    }

    public override var stepInterval: Double { 1.0 / 30.0 }

    private var intensity: Int { Int(settingValue("intensity", default: 1)) }
    private var wind: Int { Int(settingValue("wind", default: 0)) }

    /// Seed value held on the bottom row + cooling constant, per intensity.
    private var bottomSeed: Int { intensity == 0 ? 28 : Self.maxHeat }
    private var cooling: Int { intensity == 0 ? 2 : (intensity == 1 ? 1 : 0) }

    public override func reset() {
        heat = Array(repeating: 0, count: width * height)
        igniteBottomRow()
    }

    public override func step() {
        igniteBottomRow()
        let size = width * height
        for x in 0..<width {
            for y in 1..<height {
                let src = y * width + x
                let pixel = heat[src]
                if pixel <= 0 {
                    heat[src - width] = 0
                    continue
                }
                let rand = Int.random(in: 0...3, using: &rng)
                var dst = src - rand + 1 + wind - width
                if dst < 0 { dst = src - width }
                if dst >= size { dst = size - 1 }
                let decay = (rand & 1) + cooling
                heat[dst] = max(0, pixel - decay)
            }
        }
    }

    public override func render() -> ColoredFrame {
        let size = width * height
        var chars = Array(repeating: Character(" "), count: size)
        var colors = Array(repeating: RGBColor?.none, count: size)
        for i in 0..<size {
            let h = heat[i]
            guard h > 0 else { continue }
            let rampIdx = min(Self.ramp.count - 1, h * (Self.ramp.count - 1) / Self.maxHeat)
            chars[i] = Self.ramp[rampIdx]
            colors[i] = palette[min(palette.count - 1, h)]
        }
        return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
    }

    private func igniteBottomRow() {
        guard height > 0 else { return }
        let base = (height - 1) * width
        for x in 0..<width { heat[base + x] = bottomSeed }
    }

    /// 37-entry black→red→orange→yellow→white gradient indexed by heat.
    private static func buildPalette() -> [RGBColor] {
        let stops: [(Double, RGBColor)] = [
            (0.00, RGBColor(r: 0, g: 0, b: 0)),
            (0.15, RGBColor(r: 70, g: 0, b: 0)),
            (0.35, RGBColor(r: 180, g: 30, b: 0)),
            (0.55, RGBColor(r: 240, g: 100, b: 0)),
            (0.75, RGBColor(r: 255, g: 180, b: 40)),
            (0.90, RGBColor(r: 255, g: 230, b: 120)),
            (1.00, RGBColor(r: 255, g: 255, b: 255)),
        ]
        return (0...maxHeat).map { h in
            let t = Double(h) / Double(maxHeat)
            var lo = stops[0], hi = stops[stops.count - 1]
            for i in 0..<(stops.count - 1) where t >= stops[i].0 && t <= stops[i + 1].0 {
                lo = stops[i]; hi = stops[i + 1]; break
            }
            let span = hi.0 - lo.0
            let local = span > 0 ? (t - lo.0) / span : 0
            return lo.1.mixed(with: hi.1, t: local)
        }
    }
}
