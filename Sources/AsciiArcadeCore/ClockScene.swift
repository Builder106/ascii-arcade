import Foundation

/// A large block-digit clock. Renders the current time with a 3×5 pixel font
/// scaled up to fill the wallpaper, centred. Monochrome by design — every glyph
/// is emitted with a `nil` colour so the host paints it in the active theme
/// colour (green/amber/ice/ghost).
public final class ClockScene: SteppedScene {
    /// Injectable for tests; defaults to the wall clock.
    public var now: () -> Date = { Date() }

    private static let glyphHeight = 5

    /// 3×5 pixel patterns for the digits; the colon is 1×5 (see `colon`).
    private static let digits: [[String]] = [
        ["###", "# #", "# #", "# #", "###"], // 0
        [" # ", "## ", " # ", " # ", "###"], // 1
        ["###", "  #", "###", "#  ", "###"], // 2
        ["###", "  #", "###", "  #", "###"], // 3
        ["# #", "# #", "###", "  #", "  #"], // 4
        ["###", "#  ", "###", "  #", "###"], // 5
        ["###", "#  ", "###", "# #", "###"], // 6
        ["###", "  #", "  #", "  #", "  #"], // 7
        ["###", "# #", "###", "# #", "###"], // 8
        ["###", "# #", "###", "  #", "###"], // 9
    ]
    private static let colon: [String] = [" ", "#", " ", "#", " "]

    public init() {
        super.init(displayName: "Clock")
    }

    public override var settings: [SceneSetting] {
        [
            SceneSetting(id: "size", label: "Size", options: [
                SceneOption(label: "Small", value: 0.45),
                SceneOption(label: "Medium", value: 0.70),
                SceneOption(label: "Large", value: 0.95),
            ], defaultIndex: 1),
            SceneSetting(id: "seconds", label: "Seconds", options: [
                SceneOption(label: "On", value: 1),
                SceneOption(label: "Off", value: 0),
            ], defaultIndex: 0),
        ]
    }

    private var sizeFactor: Double { settingValue("size", default: 0.70) }
    private var showSeconds: Bool { settingValue("seconds", default: 1) > 0.5 }

    public override func render() -> ColoredFrame {
        let size = width * height
        var chars = Array(repeating: Character(" "), count: size)
        let colors = Array(repeating: RGBColor?.none, count: size)

        // Build the small source bitmap for the time string.
        var cal = Calendar.current
        cal.timeZone = TimeZone.current
        let comps = cal.dateComponents([.hour, .minute, .second], from: now())
        let h = comps.hour ?? 0, m = comps.minute ?? 0, s = comps.second ?? 0
        let timeString = showSeconds
            ? String(format: "%02d:%02d:%02d", h, m, s)
            : String(format: "%02d:%02d", h, m)
        let bitmap = buildBitmap(timeString)          // rows of "#/ " strings
        let bmpH = Self.glyphHeight
        let bmpW = bitmap.first?.count ?? 0
        guard bmpW > 0, height > 0, width > 0 else {
            return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
        }

        // Pick an integer scale that fits, biased by the Size setting.
        let maxScaleW = Double(width) / Double(bmpW)
        let maxScaleH = Double(height) / Double(bmpH)
        let fit = min(maxScaleW, maxScaleH)
        let scale = max(1, Int((fit * sizeFactor).rounded(.down)))

        let drawW = bmpW * scale
        let drawH = bmpH * scale
        let offX = (width - drawW) / 2
        let offY = (height - drawH) / 2

        let bmpRows = bitmap.map { Array($0) }
        for gy in 0..<drawH {
            let by = gy / scale
            let ty = offY + gy
            guard ty >= 0, ty < height, by < bmpH else { continue }
            let rowChars = bmpRows[by]
            for gx in 0..<drawW {
                let bx = gx / scale
                guard bx < rowChars.count, rowChars[bx] == "#" else { continue }
                let tx = offX + gx
                guard tx >= 0, tx < width else { continue }
                chars[ty * width + tx] = "█"
            }
        }
        return ColoredFrame(width: width, height: height, chars: chars, colors: colors)
    }

    /// Assemble the 5-row source bitmap for the whole time string, with a
    /// 1-pixel gap between glyphs.
    private func buildBitmap(_ text: String) -> [String] {
        var rows = Array(repeating: "", count: Self.glyphHeight)
        var first = true
        for ch in text {
            let glyph: [String]
            if ch == ":" {
                glyph = Self.colon
            } else if let d = ch.wholeNumberValue, d >= 0, d <= 9 {
                glyph = Self.digits[d]
            } else {
                glyph = ["   ", "   ", "   ", "   ", "   "]
            }
            for r in 0..<Self.glyphHeight {
                if !first { rows[r] += " " }
                rows[r] += glyph[r]
            }
            first = false
        }
        return rows
    }
}
