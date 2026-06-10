import Foundation

public struct DonutFrameGenerator: ShapeFrameGenerator {
    public let width: Int
    public let height: Int

    private let luminanceChars: [Character] = Array(".,-~:;=!*#$@ ")

    public init(width: Int, height: Int) {
        self.width = width
        self.height = height
    }

    public static func gridDimensions(
        paddedWidth: Double,
        paddedHeight: Double,
        charWidth: Double,
        lineHeight: Double,
        minimumWidth: Int = 10,
        minimumHeight: Int = 10
    ) -> (width: Int, height: Int) {
        let cw = max(1.0, charWidth)
        let lh = lineHeight > 0 ? lineHeight : 1.0
        let w = max(minimumWidth, Int(paddedWidth / cw))
        let h = max(minimumHeight, Int(paddedHeight / lh))
        return (w, h)
    }

    public func frame(atTime t: Double) -> String {
        let screenSize = width * height
        var zBuffer = Array(repeating: 0.0, count: screenSize)
        var output = Array(repeating: Character(" "), count: screenSize)

        // Rotation parameters based on time
        let A = t * 1.0
        let B = t * 0.5

        let cosA = cos(A), sinA = sin(A)
        let cosB = cos(B), sinB = sin(B)

        let R1 = 1.0
        let R2 = 2.0
        let K2 = 5.0
        let projectionFactor = K2 * 3.0 / (8.0 * (R1 + R2))
        let K1 = Double(min(width, height)) * projectionFactor

        var theta = 0.0
        while theta < 2 * Double.pi {
            let costheta = cos(theta)
            let sintheta = sin(theta)

            var phi = 0.0
            while phi < 2 * Double.pi {
                let cosphi = cos(phi)
                let sinphi = sin(phi)

                let circlex = R2 + R1 * costheta
                let circley = R1 * sintheta

                // 3D coordinates after rotation
                let x = circlex * (cosB * cosphi + sinA * sinB * sinphi) - circley * cosA * sinB
                let y = circlex * (sinB * cosphi - sinA * cosB * sinphi) + circley * cosA * cosB
                let z = K2 + cosA * circlex * sinphi + circley * sinA
                let ooz = 1.0 / z

                let xp = Int(Double(width) / 2.0 + K1 * ooz * x)
                let yp = Int(Double(height) / 2.0 - K1 * ooz * y)

                let L = cosphi * costheta * sinB - cosA * costheta * sinphi - sinA * sintheta + cosB * (cosA * sintheta - costheta * sinA * sinphi)

                if L > 0 {
                    let index = xp + yp * width
                    if index >= 0 && index < screenSize && ooz > zBuffer[index] {
                        zBuffer[index] = ooz
                        let luminanceIndex = Int(L * 8.0)
                        let ch = luminanceChars[max(0, min(luminanceChars.count - 1, luminanceIndex))]
                        output[index] = ch
                    }
                }

                phi += 0.02
            }
            theta += 0.07
        }

        var result = ""
        result.reserveCapacity(screenSize + height)
        for y in 0..<height {
            let start = y * width
            result.append(String(output[start..<(start + width)]))
            if y < height - 1 { result.append("\n") }
        }
        return result
    }
}
