import Foundation

/// A platform-neutral 8-bit RGB colour. Lives in `AsciiArcadeCore` so the frame
/// generators can describe colour without importing AppKit; the wallpaper host
/// maps it to an `NSColor`, the browser server could map it to an SGR triple.
public struct RGBColor: Equatable, Hashable, Sendable {
    public let r: UInt8
    public let g: UInt8
    public let b: UInt8

    public init(r: UInt8, g: UInt8, b: UInt8) {
        self.r = r; self.g = g; self.b = b
    }

    /// Scale each channel by `factor` (0…1), clamped. Used to fade trails.
    public func scaled(_ factor: Double) -> RGBColor {
        let f = min(max(factor, 0.0), 1.0)
        return RGBColor(
            r: UInt8((Double(r) * f).rounded()),
            g: UInt8((Double(g) * f).rounded()),
            b: UInt8((Double(b) * f).rounded())
        )
    }

    /// Linear interpolation toward `other` by `t` (0…1).
    public func mixed(with other: RGBColor, t: Double) -> RGBColor {
        let u = min(max(t, 0.0), 1.0)
        func lerp(_ a: UInt8, _ b: UInt8) -> UInt8 {
            UInt8((Double(a) * (1 - u) + Double(b) * u).rounded())
        }
        return RGBColor(r: lerp(r, other.r), g: lerp(g, other.g), b: lerp(b, other.b))
    }

    public static let white = RGBColor(r: 255, g: 255, b: 255)
    public static let black = RGBColor(r: 0, g: 0, b: 0)
}

/// One frame as a `width × height` grid of glyphs plus a parallel grid of
/// optional per-cell colours. A `nil` colour means "use the theme's text
/// colour" so a colour scene can leave some cells themed and tint others.
///
/// `chars` and `colors` are row-major and both exactly `width * height` long.
public struct ColoredFrame {
    public let width: Int
    public let height: Int
    public let chars: [Character]
    public let colors: [RGBColor?]

    public init(width: Int, height: Int, chars: [Character], colors: [RGBColor?]) {
        precondition(chars.count == width * height, "chars must be width*height")
        precondition(colors.count == width * height, "colors must be width*height")
        self.width = width
        self.height = height
        self.chars = chars
        self.colors = colors
    }

    /// The glyphs as `height` newline-joined rows — the monochrome view of the
    /// frame, used for the plain `AsciiScene.frame(atTime:)` fallback and tests.
    public func text() -> String {
        var out = ""
        out.reserveCapacity((width + 1) * height)
        for row in 0..<height {
            let start = row * width
            out.append(contentsOf: chars[start..<(start + width)])
            if row < height - 1 { out.append("\n") }
        }
        return out
    }
}
