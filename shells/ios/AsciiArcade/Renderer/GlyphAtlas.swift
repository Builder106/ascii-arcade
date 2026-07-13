import UIKit
import Metal
import CoreText
import CoreGraphics

// Builds a Metal R8Unorm texture atlas for all glyphs used by aa-core scenes.
//
// Coordinate note: CGBitmapContext stores rows top-to-bottom in memory despite
// CG's y-up space. Atlas row k is drawn at CG y = (totalRows-1-k)*cellH so it
// lands at memory row k → Metal UV-y = k*cellH/texH. uvRect uses `row` directly.
final class GlyphAtlas {
    let texture: MTLTexture
    let cellWidthPx: Int
    let cellHeightPx: Int
    let atlasColCount: Int = 16

    private(set) var scalars: [Unicode.Scalar] = []
    private var indexMap: [Unicode.Scalar: Int] = [:]
    private var atlasRowCount: Int = 0

    init?(device: MTLDevice, font: UIFont, scale: CGFloat) {
        // Create CTFont at pixel size so all metric queries return pixel values directly.
        let ctFont = CTFontCreateWithName(font.fontName as CFString, font.pointSize * scale, nil)

        // Cell width: advance of 'M' (every char has the same advance in a monospace font).
        var mChar: UniChar = 77
        var mGlyph: CGGlyph = 0
        CTFontGetGlyphsForCharacters(ctFont, &mChar, &mGlyph, 1)
        let refAdv = mGlyph != 0
            ? CTFontGetAdvancesForGlyphs(ctFont, .horizontal, [mGlyph], nil, 1)
            : Double(font.pointSize * scale * 0.6)
        let cellW = Int(ceil(refAdv))

        // Cell height: ascent + descent + leading (= UIFont.lineHeight at pixel size).
        let ascent  = CTFontGetAscent(ctFont)
        let descent = CTFontGetDescent(ctFont)   // positive magnitude
        let leading = CTFontGetLeading(ctFont)
        let cellH   = Int(ceil(ascent + descent + leading))

        self.cellWidthPx  = cellW
        self.cellHeightPx = cellH

        var set = Set<Unicode.Scalar>()
        for cp in 0x21...0x7E    { set.insert(Unicode.Scalar(cp)!) }  // printable ASCII
        for cp in 0xFF65...0xFF9F { set.insert(Unicode.Scalar(cp)!) }  // half-width katakana (Matrix)
        for cp in 0x2580...0x259F { set.insert(Unicode.Scalar(cp)!) }  // block elements (Fire)
        for cp in 0x2500...0x253C { set.insert(Unicode.Scalar(cp)!) }  // box drawing (Pipes)

        scalars = set.sorted { $0.value < $1.value }
        for (i, s) in scalars.enumerated() { indexMap[s] = i }

        let cols = atlasColCount
        let rows = (scalars.count + cols - 1) / cols
        atlasRowCount = rows
        let texW = cols * cellW
        let texH = rows * cellH

        guard let colorSpace = CGColorSpace(name: CGColorSpace.linearGray),
              let ctx = CGContext(
                data: nil,
                width: texW, height: texH,
                bitsPerComponent: 8, bytesPerRow: texW,
                space: colorSpace,
                bitmapInfo: CGImageAlphaInfo.none.rawValue
              ) else { return nil }

        ctx.setFillColor(gray: 0, alpha: 1)
        ctx.fill(CGRect(x: 0, y: 0, width: texW, height: texH))
        ctx.setFillColor(gray: 1, alpha: 1)
        ctx.textMatrix = CGAffineTransform.identity

        for (idx, scalar) in scalars.enumerated() {
            let col = idx % cols
            let row = idx / cols

            // Flip row: atlas row 0 draws at high CG-y → top of memory → Metal UV-y = 0.
            let cgCellBottom = CGFloat((rows - 1 - row) * cellH)
            // Baseline is `descent` pixels above the cell bottom (descent is positive magnitude).
            let baselineY = cgCellBottom + CGFloat(descent)
            let xOff = CGFloat(col * cellW)

            var ch = scalar.utf16[scalar.utf16.startIndex]
            var glyph: CGGlyph = 0
            CTFontGetGlyphsForCharacters(ctFont, &ch, &glyph, 1)
            if glyph == 0 { continue }

            let adv = CTFontGetAdvancesForGlyphs(ctFont, .horizontal, [glyph], nil, 1)
            let centreOff = (CGFloat(cellW) - CGFloat(adv)) / 2
            CTFontDrawGlyphs(ctFont, [glyph], [CGPoint(x: xOff + centreOff, y: baselineY)], 1, ctx)
        }

        guard let data = ctx.data else { return nil }

        let desc = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: .r8Unorm, width: texW, height: texH, mipmapped: false
        )
        desc.usage = MTLTextureUsage.shaderRead
        guard let tex = device.makeTexture(descriptor: desc) else { return nil }
        tex.replace(
            region: MTLRegionMake2D(0, 0, texW, texH),
            mipmapLevel: 0,
            withBytes: data,
            bytesPerRow: texW
        )
        self.texture = tex
    }

    func index(for scalar: Unicode.Scalar) -> Int? { indexMap[scalar] }

    // Normalised UV rect — atlas row 0 maps to Metal UV-y = 0 (top), no flip needed.
    func uvRect(for index: Int) -> (x: Float, y: Float, w: Float, h: Float) {
        let cols = atlasColCount
        let rows = atlasRowCount
        let col  = index % cols
        let row  = index / cols
        let texW = Float(cols * cellWidthPx)
        let texH = Float(rows * cellHeightPx)
        return (
            x: Float(col * cellWidthPx) / texW,
            y: Float(row * cellHeightPx) / texH,
            w: Float(cellWidthPx) / texW,
            h: Float(cellHeightPx) / texH
        )
    }
}
