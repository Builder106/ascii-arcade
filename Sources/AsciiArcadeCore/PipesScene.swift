import Foundation

/// The old "pipes" screensaver in ASCII: several pipes wander the grid drawing
/// box-drawing segments and corners in their own colour, turning at random and
/// at the edges. When the board fills past a threshold it clears and respawns —
/// an endless, ever-changing weave.
public final class PipesScene: SteppedScene {
    // Directions: 0 = up, 1 = right, 2 = down, 3 = left.
    private static let dx = [0, 1, 0, -1]
    private static let dy = [-1, 0, 1, 0]

    private struct Pipe {
        var x: Int
        var y: Int
        var dir: Int
        var color: RGBColor
    }

    private var grid: [Character] = []
    private var colors: [RGBColor?] = []
    private var pipes: [Pipe] = []
    private var filled = 0
    private var hueCursor = 0
    private var rng = SeededGenerator(seed: 0xC0FFEE) // re-seeded per grid in reset()

    public init() {
        super.init(displayName: "Pipes")
    }

    public override var settings: [SceneSetting] {
        [
            SceneSetting(id: "speed", label: "Speed", options: [
                SceneOption(label: "Slow", value: 12),
                SceneOption(label: "Normal", value: 22),
                SceneOption(label: "Fast", value: 40),
            ], defaultIndex: 1),
            SceneSetting(id: "pipes", label: "Pipes", options: [
                SceneOption(label: "Few", value: 2),
                SceneOption(label: "Some", value: 4),
                SceneOption(label: "Many", value: 8),
            ], defaultIndex: 1),
        ]
    }

    public override var stepInterval: Double {
        1.0 / max(1.0, settingValue("speed", default: 22))
    }

    private var pipeCount: Int { Int(settingValue("pipes", default: 4)) }

    public override func reset() {
        let size = width * height
        rng = SeededGenerator(seed: 0xC0FFEE ^ UInt64(size &* 2654435761))
        grid = Array(repeating: Character(" "), count: size)
        colors = Array(repeating: RGBColor?.none, count: size)
        filled = 0
        pipes = (0..<pipeCount).map { _ in spawnPipe() }
    }

    public override func settingsChanged() {
        reset()
    }

    public override func step() {
        let size = width * height
        guard size > 0, grid.count == size else { return }
        for i in pipes.indices {
            advance(&pipes[i])
        }
        if filled > (size * 55) / 100 {
            grid = Array(repeating: Character(" "), count: size)
            colors = Array(repeating: RGBColor?.none, count: size)
            filled = 0
            pipes = (0..<pipeCount).map { _ in spawnPipe() }
        }
    }

    public override func render() -> ColoredFrame {
        ColoredFrame(width: width, height: height, chars: grid, colors: colors)
    }

    // MARK: - Pipe movement

    private func advance(_ pipe: inout Pipe) {
        // Pick the next heading: mostly continue, sometimes turn; always choose a
        // direction that keeps the pipe on the grid.
        let turning = Double.random(in: 0...1, using: &rng) < 0.18
        let preferred = turning
            ? (Bool.random(using: &rng) ? (pipe.dir + 1) % 4 : (pipe.dir + 3) % 4)
            : pipe.dir
        let candidates = [preferred, (pipe.dir + 1) % 4, (pipe.dir + 3) % 4, pipe.dir, (pipe.dir + 2) % 4]
        var newDir = pipe.dir
        for c in candidates where inBounds(pipe.x + Self.dx[c], pipe.y + Self.dy[c]) {
            newDir = c
            break
        }

        // Draw the connector at the current cell joining where we came from
        // (opposite of current heading) to where we're going (newDir).
        let incoming = (pipe.dir + 2) % 4
        let glyph = connector(incoming, newDir)
        let idx = pipe.y * width + pipe.x
        if grid[idx] == " " { filled += 1 }
        grid[idx] = glyph
        colors[idx] = pipe.color

        let nx = pipe.x + Self.dx[newDir]
        let ny = pipe.y + Self.dy[newDir]
        if inBounds(nx, ny) {
            pipe.x = nx; pipe.y = ny
        } else {
            // Boxed in — wrap to a fresh spot rather than stall.
            pipe = spawnPipe(color: pipe.color)
        }
        pipe.dir = newDir
    }

    /// Box-drawing glyph joining two cell edges, given as direction indices
    /// (0 = up, 1 = right, 2 = down, 3 = left).
    private func connector(_ a: Int, _ b: Int) -> Character {
        if a == b { return a % 2 == 0 ? "│" : "─" }   // up/down vertical, left/right horizontal
        switch (min(a, b), max(a, b)) {
        case (0, 2): return "│"   // up + down
        case (1, 3): return "─"   // right + left
        case (1, 2): return "┌"   // right + down
        case (2, 3): return "┐"   // down + left
        case (0, 1): return "└"   // up + right
        case (0, 3): return "┘"   // up + left
        default:     return "+"
        }
    }

    private func inBounds(_ x: Int, _ y: Int) -> Bool {
        x >= 0 && x < width && y >= 0 && y < height
    }

    private func spawnPipe(color: RGBColor? = nil) -> Pipe {
        let c = color ?? nextHue()
        return Pipe(
            x: Int.random(in: 0..<max(1, width), using: &rng),
            y: Int.random(in: 0..<max(1, height), using: &rng),
            dir: Int.random(in: 0..<4, using: &rng),
            color: c
        )
    }

    private func nextHue() -> RGBColor {
        let hue = Double(hueCursor) * 47.0
        hueCursor += 1
        return Self.hsv(h: hue.truncatingRemainder(dividingBy: 360), s: 0.65, v: 1.0)
    }

    /// HSV → RGB so each pipe gets a distinct, saturated hue.
    private static func hsv(h: Double, s: Double, v: Double) -> RGBColor {
        let c = v * s
        let hp = h / 60.0
        let x = c * (1 - abs(hp.truncatingRemainder(dividingBy: 2) - 1))
        var r = 0.0, g = 0.0, b = 0.0
        switch hp {
        case 0..<1: (r, g, b) = (c, x, 0)
        case 1..<2: (r, g, b) = (x, c, 0)
        case 2..<3: (r, g, b) = (0, c, x)
        case 3..<4: (r, g, b) = (0, x, c)
        case 4..<5: (r, g, b) = (x, 0, c)
        default:    (r, g, b) = (c, 0, x)
        }
        let m = v - c
        return RGBColor(
            r: UInt8(((r + m) * 255).rounded()),
            g: UInt8(((g + m) * 255).rounded()),
            b: UInt8(((b + m) * 255).rounded())
        )
    }
}
