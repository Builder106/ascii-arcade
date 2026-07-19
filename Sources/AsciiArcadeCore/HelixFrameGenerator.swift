import Foundation

// An infinite helix, lit and z-buffered, flown down from an aerial view.
//
// The coil itself never moves and has no fixed length — the helix is
// conceptually infinite, so each frame only generates the window of turns
// currently near the camera (visibleTurns worth of u, centered on wherever
// the camera currently is) rather than a fixed-size coil. The shape is
// translation-invariant along its own axis, so a freshly generated window
// looks identical to the last one — nothing to stitch or blend, just keep
// sliding the window as the camera moves.
//
// The only motion is the camera: it rides the coil's own central axis,
// looking straight down it (aerial view), and descends at a constant rate
// forever — one direction, no turning around, no bouncing between ends,
// because an infinite helix has no ends to bounce between.
//
// depth is just how far below the camera a point sits (camZ - pz), since
// the camera only ever looks down. Points at or above the camera
// (depth <= nearClip) are skipped the way a near clip plane would skip
// them, rather than letting 1/depth blow up as a turn's height passes the
// camera's current position. The camera rides the coil's exact central
// axis (radius zero), while the tube's surface never reaches that axis
// (R - r is well clear of zero) — so depth can approach nearClip as the
// camera passes a turn's height, but the point is never actually *at* the
// camera in 3D, only close in the one axial coordinate.
public struct HelixFrameGenerator: ShapeFrameGenerator {
    public let width: Int
    public let height: Int

    private let luminanceChars: [Character] = Array(".,-~:;=!*#$@ ")

    private let R: Double = 1.5
    // Kept small relative to pitch (vertical turn spacing is ~6x the tube
    // diameter) so consecutive turns show real background gaps between them
    // and read as a coil rather than a solid filled blob — a thicker tube
    // visually merges turns together well before they touch in 3D.
    private let r: Double = 0.2
    private let pitch: Double = 0.4

    // How many turns' worth of u-range to generate around the camera each
    // frame — enough to see turns both ahead of and behind it, like a
    // tunnel, without generating the (literally unbounded) rest of the coil.
    private let visibleTurns: Double = 3.0

    // Units/sec the camera descends. Constant and one-directional.
    private let camSpeed: Double = 0.5

    // Reference axial distance used only to calibrate the projection scale
    // (K1) — roughly one turn's vertical spacing, so a turn at a "normal"
    // viewing distance fills a sensible fraction of the screen.
    private let camRefDepth: Double = 2.5
    // Depths shallower than this are treated as behind/at the camera and
    // skipped, rather than letting 1/depth blow up as a turn's height
    // passes the camera's current position.
    private let nearClip: Double = 0.3

    public init(width: Int, height: Int) {
        self.width = width
        self.height = height
    }

    public func frame(atTime t: Double) -> String {
        let screenSize = width * height
        var zBuffer = Array(repeating: 0.0, count: screenSize)
        var output = Array(repeating: Character(" "), count: screenSize)

        let camZ = -camSpeed * t

        let projectionFactor = camRefDepth * 2.0 / (8.0 * (R + r))
        let K1 = Double(min(width, height)) * projectionFactor

        // The window of turns to generate this frame, centered on wherever
        // the camera currently is (pz ~= pitch * u, so uCenter is just the
        // camera's position rescaled into u-space).
        let uCenter = camZ / pitch
        let uHalfRange = visibleTurns * Double.pi
        let uEnd = uCenter + uHalfRange

        var u = uCenter - uHalfRange
        while u < uEnd {
            let cosu = cos(u), sinu = sin(u)

            var v = 0.0
            while v < 2 * Double.pi {
                let cosv = cos(v), sinv = sin(v)

                let px = cosu * (R + r * cosv)
                let py = sinu * (R + r * cosv)
                let pz = pitch * u + r * sinv

                let ny = cosv * sinu
                let nz = sinv

                // The camera looks straight down the coil's axis from its
                // current position, so depth is just the axial gap between
                // camera and point — no rotation needed at all, since
                // neither the coil nor the camera's orientation ever
                // changes, only the camera's position along the axis.
                let depth = camZ - pz
                if depth > nearClip {
                    let ooz = 1.0 / depth

                    // Light from (0, 1, −1)/√2, fixed in world space (the
                    // coil never rotates, so the normal needs no rotating).
                    let L = ny - nz

                    if L > 0 {
                        let xp = Int(Double(width) / 2.0 + K1 * ooz * px)
                        let yp = Int(Double(height) / 2.0 - K1 * ooz * py)
                        if xp >= 0 && yp >= 0 && xp < width && yp < height {
                            let index = xp + yp * width
                            if ooz > zBuffer[index] {
                                zBuffer[index] = ooz
                                let luminanceIndex = Int(L * 5.66)
                                let ch = luminanceChars[max(0, min(luminanceChars.count - 1, luminanceIndex))]
                                output[index] = ch
                            }
                        }
                    }
                }

                v += 0.07
            }
            u += 0.04
        }

        var result = ""
        result.reserveCapacity(screenSize + height)
        for row in 0..<height {
            let start = row * width
            result.append(String(output[start..<(start + width)]))
            if row < height - 1 { result.append("\n") }
        }
        return result
    }
}
