import Foundation

public struct HelixFrameGenerator: ShapeFrameGenerator {
    public let width: Int
    public let height: Int

    private let luminanceChars: [Character] = Array(".,-~:;=!*#$@ ")

    private let R: Double = 1.5
    private let r: Double = 0.4
    private let numTurns: Double = 2.5
    private let pitch: Double = 0.4

    public init(width: Int, height: Int) {
        self.width = width
        self.height = height
    }

    public func frame(atTime t: Double) -> String {
        let screenSize = width * height
        var zBuffer = Array(repeating: 0.0, count: screenSize)
        var output = Array(repeating: Character(" "), count: screenSize)

        let A = t * 1.0
        let B = t * 0.5
        let C = sin(t * 0.4) * 0.6  // precessing wobble around z-axis
        let cosA = cos(A), sinA = sin(A)
        let cosB = cos(B), sinB = sin(B)
        let cosC = cos(C), sinC = sin(C)

        let K2 = 5.0
        // Reduced from 3.0 → 2.0 to shrink the helix to roughly donut scale
        let projectionFactor = K2 * 2.0 / (8.0 * (R + r))
        let K1 = Double(min(width, height)) * projectionFactor

        let halfHeight = pitch * numTurns * Double.pi

        var u = 0.0
        while u < numTurns * 2 * Double.pi {
            let cosu = cos(u), sinu = sin(u)

            var v = 0.0
            while v < 2 * Double.pi {
                let cosv = cos(v), sinv = sin(v)

                let px = cosu * (R + r * cosv)
                let py = sinu * (R + r * cosv)
                let pz = pitch * u - halfHeight + r * sinv

                let nx = cosv * cosu
                let ny = cosv * sinu
                let nz = sinv

                // Rz(C) — precession
                let px_c = px * cosC - py * sinC
                let py_c = px * sinC + py * cosC
                let nx_c = nx * cosC - ny * sinC
                let ny_c = nx * sinC + ny * cosC

                // Rx(A)
                let py1 = py_c * cosA - pz * sinA
                let pz1 = py_c * sinA + pz * cosA
                let ny1 = ny_c * cosA - nz * sinA
                let nz1 = ny_c * sinA + nz * cosA

                // Ry(B)
                let x = px_c * cosB + pz1 * sinB
                let y = py1
                let z = K2 - px_c * sinB + pz1 * cosB
                let ooz = 1.0 / z

                let ny_rot = ny1
                let nz_rot = -nx_c * sinB + nz1 * cosB

                // Light from (0, 1, −1)/√2
                let L = ny_rot - nz_rot

                if L > 0 {
                    let xp = Int(Double(width) / 2.0 + K1 * ooz * x)
                    let yp = Int(Double(height) / 2.0 - K1 * ooz * y)
                    let index = xp + yp * width
                    if index >= 0 && index < screenSize && ooz > zBuffer[index] {
                        zBuffer[index] = ooz
                        let luminanceIndex = Int(L * 5.66)
                        let ch = luminanceChars[max(0, min(luminanceChars.count - 1, luminanceIndex))]
                        output[index] = ch
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
