import AppKit
import AVFoundation
import CoreVideo
import CoreText
import ServiceManagement
import AsciiArcadeCore

struct Theme {
    let name: String
    let textColor: NSColor
    /// Behind-the-glyphs fill for the view's own layer — not the desktop wallpaper.
    /// `nil` leaves the layer transparent so the real desktop shows through.
    let backgroundColor: NSColor?
}

let availableThemes: [Theme] = [
    Theme(name: "Hacker", textColor: .systemGreen, backgroundColor: nil),
    Theme(name: "Amber", textColor: NSColor(calibratedRed: 1.0, green: 0.65, blue: 0.0, alpha: 1.0), backgroundColor: nil),
    Theme(name: "Ice", textColor: .cyan, backgroundColor: nil),
    Theme(name: "Ghost", textColor: NSColor(calibratedRed: 0.11, green: 0.11, blue: 0.118, alpha: 1.0),
          backgroundColor: NSColor(calibratedRed: 0.961, green: 0.961, blue: 0.961, alpha: 1.0)),
]

// MARK: - Scenes

/// Builds a fresh set of cabinets. Each desktop window gets its own instances so
/// per-screen renderers stay independent (and so DOOM runs one PTY per screen it's
/// active on). Order here drives the status-bar menu order.
func makeScenes() -> [any AsciiScene] {
    let cwd = FileManager.default.currentDirectoryPath
    return [
        GeneratorScene(displayName: "Donut") { w, h in DonutFrameGenerator(width: w, height: h) },
        GeneratorScene(displayName: "Helix") { w, h in HelixFrameGenerator(width: w, height: h) },
        MatrixRainScene(),
        GameOfLifeScene(),
        PipesScene(),
        DoomScene(workingDirectory: cwd)
    ]
}

let sceneNames: [String] = makeScenes().map { $0.displayName }
/// Scenes that consume keyboard input (DOOM) are gated behind the "Enable DOOM
/// Scene" opt-in — see `AppDelegate.doomEnabled` — so they don't ambush someone
/// who just wanted an ambient wallpaper.
let sceneIsInteractive: [Bool] = makeScenes().map { $0.isInteractive }

// MARK: - Wallpaper helpers

private func setWallpaper(_ url: URL, for screen: NSScreen) {
    try? NSWorkspace.shared.setDesktopImageURL(url, for: screen, options: [:])
}

// MARK: - Window

final class DesktopSceneWindow: NSWindow {
    init(screen: NSScreen) {
        let frame = screen.frame
        super.init(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
        setFrame(frame, display: true)
        let desktopLevel = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.desktopWindow)))
        level = desktopLevel
        isOpaque = false
        backgroundColor = .clear
        ignoresMouseEvents = true
        collectionBehavior = [.canJoinAllSpaces, .stationary]
    }
}

// MARK: - View

final class SceneView: NSView {
    override var isOpaque: Bool { false }
    override var isFlipped: Bool { true }

    private let scenes: [any AsciiScene]
    private(set) var currentIndex = 0
    var currentScene: any AsciiScene { scenes[currentIndex] }

    private var startTime: CFAbsoluteTime = CFAbsoluteTimeGetCurrent()
    private var displayLink: CVDisplayLink?
    private var themeTextColor: NSColor = availableThemes[0].textColor
    private let font: NSFont
    private let ctFont: CTFont
    private let cellCharWidth: CGFloat
    private let cellLineHeight: CGFloat
    private let cellAscent: CGFloat
    private let scale: CGFloat = 0.92
    private let scanlinesLayer = CAReplicatorLayer()
    private let scanlineStripeLayer = CALayer()
    /// Character → glyph and RGB → CGColor caches so the hot draw path never
    /// re-measures the font or re-creates colours. (RGBColor qualified: AppKit
    /// transitively imports a legacy Quickdraw `RGBColor`.)
    private var glyphCache: [Character: CGGlyph] = [:]
    private var cgColorCache: [AsciiArcadeCore.RGBColor: CGColor] = [:]
    /// Throttle redraws to ~30fps regardless of the display's refresh rate —
    /// ASCII animation doesn't need 60/120Hz and the text fill is the hot path.
    /// The threshold sits just under one 30fps period (not exactly on it) so a
    /// 60Hz panel fires reliably on every 2nd refresh instead of slipping to the
    /// 3rd; a 120Hz panel fires on every 4th. Result: a steady 30fps.
    private let minFrameInterval: CFTimeInterval = 1.0 / 34.0
    private var lastRedrawTime: CFTimeInterval = 0
    /// Set ASCII_FPS=1 to log average draw time + effective FPS once a second.
    private let instrument = ProcessInfo.processInfo.environment["ASCII_FPS"] != nil
    private var instrFrames = 0
    private var instrDrawMs = 0.0
    private var instrLast = CACurrentMediaTime()
    /// Reusable per-colour glyph buckets, collected each frame then drawn in one
    /// `CTFontDrawGlyphs` call apiece.
    private final class GlyphBatch { var glyphs: [CGGlyph] = []; var positions: [CGPoint] = [] }
    /// Persisted across frames so the per-colour bucketing dictionary and its
    /// glyph/position arrays aren't reallocated every draw — just cleared
    /// (keeping capacity) and refilled.
    private var batchPool: [AsciiArcadeCore.RGBColor?: GlyphBatch] = [:]
    /// Last grid size passed to `currentScene.setGrid`, so unchanged layouts
    /// skip the redundant per-frame call.
    private var lastGridSize: (w: Int, h: Int)?

    init(frame: CGRect, scenes: [any AsciiScene]) {
        self.scenes = scenes
        let f = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        self.font = f
        self.ctFont = f as CTFont
        self.cellCharWidth = max(1.0, ("@" as NSString).size(withAttributes: [.font: f]).width)
        self.cellLineHeight = f.ascender - f.descender + f.leading
        self.cellAscent = f.ascender
        super.init(frame: frame)
        // Seed every scene with the starting theme's base colour.
        let rgb = SceneView.rgbColor(from: availableThemes[0].textColor)
        for scene in scenes { scene.applyBaseColor(rgb) }
        wantsLayer = true
        layer?.isOpaque = false
        layer?.backgroundColor = NSColor.clear.cgColor

        scanlinesLayer.addSublayer(scanlineStripeLayer)
        scanlinesLayer.frame = bounds
        scanlinesLayer.autoresizingMask = [.layerWidthSizable, .layerHeightSizable]
        scanlinesLayer.opacity = 1.0
        scanlineStripeLayer.backgroundColor = NSColor.black.withAlphaComponent(0.18).cgColor
        layer?.addSublayer(scanlinesLayer)
        updateScanlines()

        setupDisplayLink()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    /// Switch the active cabinet: stop the old one, reset the clock, start the new one.
    func selectScene(_ index: Int) {
        guard index >= 0, index < scenes.count, index != currentIndex else { return }
        scenes[currentIndex].stop()
        currentIndex = index
        startTime = CFAbsoluteTimeGetCurrent()
        scenes[currentIndex].start()
        // Force the next draw to call setGrid even if the computed size matches
        // the previous scene's — each scene tracks its own grid state.
        lastGridSize = nil
        needsDisplay = true
    }

    func cycleScene() {
        selectScene((currentIndex + 1) % scenes.count)
    }

    /// Forward key bytes if the current cabinet is interactive (DOOM).
    func forwardKey(_ bytes: [UInt8]) {
        guard currentScene.isInteractive else { return }
        currentScene.sendKey(bytes)
    }

    func stopCurrentScene() {
        currentScene.stop()
    }

    /// Pause rendering when the displays sleep — stops the display link so we
    /// stop issuing draws (and stop spinning DOOM/Matrix/fire) while asleep.
    func pause() {
        if let displayLink = displayLink { CVDisplayLinkStop(displayLink) }
    }

    /// Resume after wake: reset the clock so the time delta doesn't jump, then
    /// restart the display link.
    func resume() {
        startTime = CFAbsoluteTimeGetCurrent()
        if let displayLink = displayLink, !CVDisplayLinkIsRunning(displayLink) {
            CVDisplayLinkStart(displayLink)
        }
        needsDisplay = true
    }

    func applyTheme(_ theme: Theme) {
        themeTextColor = theme.textColor
        layer?.backgroundColor = theme.backgroundColor?.cgColor ?? NSColor.clear.cgColor
        let rgb = SceneView.rgbColor(from: theme.textColor)
        for scene in scenes { scene.applyBaseColor(rgb) }
        needsDisplay = true
    }

    /// Apply a setting value to the current scene (called from the menu).
    func applySettingToCurrentScene(id: String, value: Double) {
        currentScene.applySetting(id: id, value: value)
        needsDisplay = true
    }

    /// Replay persisted per-scene setting choices onto the right scene instances
    /// at launch. `selections` maps scene index → (settingId → chosen option index).
    func applyPersistedSettings(_ selections: [Int: [String: Int]]) {
        for (sceneIndex, chosen) in selections where sceneIndex >= 0 && sceneIndex < scenes.count {
            let scene = scenes[sceneIndex]
            for setting in scene.settings {
                if let optIndex = chosen[setting.id], optIndex >= 0, optIndex < setting.options.count {
                    scene.applySetting(id: setting.id, value: setting.options[optIndex].value)
                }
            }
        }
    }

    override func layout() {
        super.layout()
        updateScanlines()
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        let drawStart = instrument ? CACurrentMediaTime() : 0
        let t = CFAbsoluteTimeGetCurrent() - startTime

        let insetX = bounds.width * (1.0 - scale) / 2.0
        let insetY = bounds.height * (1.0 - scale) / 2.0
        let paddedRect = bounds.insetBy(dx: insetX, dy: insetY).insetBy(dx: 6, dy: 6)

        let viewH = bounds.height

        // DOOM (and any fixed-resolution scene) renders as a scaled colour bitmap
        // that fills the padded rect — its framebuffer is far denser than the text
        // grid, so each cell is painted as a rectangle rather than a font glyph.
        if let fixed = currentScene.fixedGrid {
            drawBitmap(currentScene, fixed: fixed, in: paddedRect, ctx: ctx, t: t)
            if instrument {
                recordInstrumentation(scene: currentScene, w: fixed.width, h: fixed.height,
                                      batches: 0, start: drawStart)
            }
            return
        }

        let charW = cellCharWidth
        let lineH = cellLineHeight
        let (w, h) = DonutFrameGenerator.gridDimensions(
            paddedWidth: Double(paddedRect.width),
            paddedHeight: Double(paddedRect.height),
            charWidth: Double(charW),
            lineHeight: Double(lineH)
        )
        if lastGridSize?.w != w || lastGridSize?.h != h {
            currentScene.setGrid(width: w, height: h)
            lastGridSize = (w, h)
        }

        let contentWidth = CGFloat(w) * charW
        let contentHeight = CGFloat(h) * lineH
        let originX = paddedRect.midX - contentWidth / 2.0
        let originY = paddedRect.midY - contentHeight / 2.0
        let ascent = cellAscent

        // Bucket every non-blank cell's glyph by colour. Adjacent cells that share
        // a colour (or a palette entry) collapse into the same bucket, so a
        // full-screen frame becomes a few dozen draw calls instead of thousands.
        // `batchPool` persists across frames — cleared here (keeping the arrays'
        // capacity) instead of rebuilding a fresh dictionary every draw.
        for (_, batch) in batchPool {
            batch.glyphs.removeAll(keepingCapacity: true)
            batch.positions.removeAll(keepingCapacity: true)
        }
        func emit(_ ch: Character, row: Int, col: Int, color: AsciiArcadeCore.RGBColor?) {
            guard ch != " ", let g = glyph(for: ch) else { return }
            let batch: GlyphBatch
            if let existing = batchPool[color] {
                batch = existing
            } else {
                batch = GlyphBatch()
                batchPool[color] = batch
            }
            batch.glyphs.append(g)
            batch.positions.append(CGPoint(
                x: originX + CGFloat(col) * charW,
                y: viewH - (originY + CGFloat(row) * lineH + ascent)
            ))
        }

        if let colored = currentScene.coloredFrame(atTime: t) {
            let chars = colored.chars, colors = colored.colors
            for row in 0..<h {
                let base = row * w
                for col in 0..<w {
                    emit(chars[base + col], row: row, col: col, color: colors[base + col])
                }
            }
        } else {
            var row = 0, col = 0
            for ch in currentScene.frame(atTime: t) {
                if ch == "\n" { row += 1; col = 0; continue }
                emit(ch, row: row, col: col, color: nil)
                col += 1
            }
        }

        // Draw. Flip into a y-up space so Core Text glyphs render upright in this
        // flipped view, then one fill + one CTFontDrawGlyphs per colour bucket.
        ctx.saveGState()
        ctx.translateBy(x: 0, y: viewH)
        ctx.scaleBy(x: 1, y: -1)
        ctx.textMatrix = .identity
        for (color, batch) in batchPool where !batch.glyphs.isEmpty {
            ctx.setFillColor(color.map { cgColor(for: $0) } ?? themeTextColor.cgColor)
            CTFontDrawGlyphs(ctFont, batch.glyphs, batch.positions, batch.glyphs.count, ctx)
        }
        ctx.restoreGState()

        if instrument {
            recordInstrumentation(scene: currentScene, w: w, h: h,
                                  batches: batchPool.count, start: drawStart)
        }
    }

    /// Paint a fixed-resolution scene (DOOM) as a scaled colour bitmap filling
    /// `rect`. Each cell becomes a rectangle; horizontally-adjacent cells of the
    /// same colour merge into one fill, and fills are batched per colour — so a
    /// dense frame is a few hundred `fill` calls, not tens of thousands.
    private func drawBitmap(_ scene: any AsciiScene, fixed: (width: Int, height: Int),
                            in rect: CGRect, ctx: CGContext, t: Double) {
        guard let colored = scene.coloredFrame(atTime: t) else { return }
        let w = fixed.width, h = fixed.height
        guard w > 0, h > 0 else { return }
        let cellW = rect.width / CGFloat(w)
        let cellH = rect.height / CGFloat(h)
        let chars = colored.chars, colors = colored.colors
        guard chars.count >= w * h, colors.count >= w * h else { return }

        var rectsByColor: [AsciiArcadeCore.RGBColor: [CGRect]] = [:]
        // Uncolored non-blank cells (e.g. the "doom_ascii not found" message, or
        // any frame before ANSI colour arrives) fall back to the theme colour —
        // otherwise they'd paint nothing and the screen would read as black.
        var fallbackRects: [CGRect] = []
        for row in 0..<h {
            let base = row * w
            // The view is flipped (isFlipped == true): y grows downward, origin
            // top-left. Row 0 is the top of the framebuffer, so it maps to
            // rect.minY. (Using rect.maxY here rendered the frame upside down.)
            let yTop = rect.minY + CGFloat(row) * cellH
            var col = 0
            while col < w {
                let idx = base + col
                guard chars[idx] != " " else { col += 1; continue }
                let color = colors[idx]
                var end = col + 1
                while end < w, chars[base + end] != " ", colors[base + end] == color { end += 1 }
                let r = CGRect(x: rect.minX + CGFloat(col) * cellW, y: yTop,
                               width: CGFloat(end - col) * cellW, height: cellH)
                if let color { rectsByColor[color, default: []].append(r) }
                else { fallbackRects.append(r) }
                col = end
            }
        }
        for (color, rects) in rectsByColor {
            ctx.setFillColor(cgColor(for: color))
            ctx.fill(rects)
        }
        if !fallbackRects.isEmpty {
            ctx.setFillColor(themeTextColor.cgColor)
            ctx.fill(fallbackRects)
        }
    }

    private func recordInstrumentation(scene: any AsciiScene, w: Int, h: Int,
                                       batches: Int, start: CFTimeInterval) {
        instrFrames += 1
        instrDrawMs += (CACurrentMediaTime() - start) * 1000.0
        let now = CACurrentMediaTime()
        if now - instrLast >= 1.0 {
            let avg = instrDrawMs / Double(max(1, instrFrames))
            NSLog("ASCII_FPS scene=%@ grid=%dx%d fps=%d avgDraw=%.2fms batches=%d",
                  scene.displayName, w, h, instrFrames, avg, batches)
            instrFrames = 0; instrDrawMs = 0; instrLast = now
        }
    }

    /// Look up (and cache) the glyph for a single-cell character. Returns nil for
    /// blanks, missing glyphs, or anything that isn't a single UTF-16 unit.
    private func glyph(for ch: Character) -> CGGlyph? {
        if let cached = glyphCache[ch] { return cached == 0 ? nil : cached }
        let units = Array(ch.utf16)
        guard units.count == 1 else { glyphCache[ch] = 0; return nil }
        var input = units
        var out = [CGGlyph](repeating: 0, count: 1)
        let ok = CTFontGetGlyphsForCharacters(ctFont, &input, &out, 1)
        let g = ok ? out[0] : 0
        glyphCache[ch] = g
        return g == 0 ? nil : g
    }

    private func cgColor(for rgb: AsciiArcadeCore.RGBColor) -> CGColor {
        if let cached = cgColorCache[rgb] { return cached }
        let c = CGColor(srgbRed: CGFloat(rgb.r) / 255.0,
                        green: CGFloat(rgb.g) / 255.0,
                        blue: CGFloat(rgb.b) / 255.0,
                        alpha: 1.0)
        cgColorCache[rgb] = c
        return c
    }

    /// Convert an `NSColor` (theme text colour) to the core's `RGBColor`.
    static func rgbColor(from color: NSColor) -> AsciiArcadeCore.RGBColor {
        let c = color.usingColorSpace(.sRGB) ?? color
        return AsciiArcadeCore.RGBColor(r: UInt8((c.redComponent * 255).rounded()),
                                        g: UInt8((c.greenComponent * 255).rounded()),
                                        b: UInt8((c.blueComponent * 255).rounded()))
    }

    private func updateScanlines() {
        let stripeSpacing: CGFloat = 2
        let stripeHeight: CGFloat = 1
        let stripeCount = max(1, Int(bounds.height / stripeSpacing) + 2)

        scanlinesLayer.frame = bounds
        scanlinesLayer.instanceCount = stripeCount
        scanlinesLayer.instanceTransform = CATransform3DMakeTranslation(0, stripeSpacing, 0)

        scanlineStripeLayer.frame = CGRect(x: 0, y: 0, width: bounds.width, height: stripeHeight)
    }

    private func setupDisplayLink() {
        var link: CVDisplayLink?
        CVDisplayLinkCreateWithActiveCGDisplays(&link)
        guard let displayLink = link else { return }
        self.displayLink = displayLink
        CVDisplayLinkSetOutputCallback(displayLink, { (_, _, _, _, _, userData) -> CVReturn in
            let view = Unmanaged<SceneView>.fromOpaque(userData!).takeUnretainedValue()
            // Throttle to ~30fps. `lastRedrawTime` is only touched on this
            // (serial) display-link thread, so no synchronization is needed.
            let now = CACurrentMediaTime()
            if now - view.lastRedrawTime >= view.minFrameInterval {
                view.lastRedrawTime = now
                DispatchQueue.main.async { view.needsDisplay = true }
            }
            return kCVReturnSuccess
        }, Unmanaged.passUnretained(self).toOpaque())
        CVDisplayLinkStart(displayLink)
    }

    deinit {
        if let displayLink = displayLink { CVDisplayLinkStop(displayLink) }
    }
}

// MARK: - Key mapping (NSEvent → doom_ascii bytes)

/// Maps a key event to the byte sequence `doom_ascii` expects.
/// Controls: arrows move, `,`/`.` strafe, space fires, `e` uses, `]` runs,
/// `1`–`7` select weapons, Return confirms, Esc opens the menu.
func doomBytes(for event: NSEvent) -> [UInt8]? {
    switch event.keyCode {
    case 126: return Array("\u{1b}[A".utf8) // up arrow
    case 125: return Array("\u{1b}[B".utf8) // down arrow
    case 124: return Array("\u{1b}[C".utf8) // right arrow
    case 123: return Array("\u{1b}[D".utf8) // left arrow
    case 36, 76: return [0x0a]              // return / keypad enter
    case 53:  return [0x1b]                 // escape
    case 49:  return [0x20]                 // space (fire)
    default: break
    }
    if let chars = event.charactersIgnoringModifiers,
       let scalar = chars.unicodeScalars.first,
       scalar.value >= 0x20, scalar.value < 0x7f {
        return Array(chars.lowercased().utf8)
    }
    return nil
}

// MARK: - App Delegate

/// Everything we remember across launches, stored as JSON in `UserDefaults`.
struct PersistedState: Codable {
    var sceneIndex: Int
    var themeIndex: Int
    var captureKeysForDoom: Bool
    var idleAutoCycle: Bool
    var doomEnabled: Bool
    /// scene index (as String for JSON) → (settingId → chosen option index).
    var sceneSettings: [String: [String: Int]]

    init(sceneIndex: Int, themeIndex: Int, captureKeysForDoom: Bool, idleAutoCycle: Bool,
         doomEnabled: Bool, sceneSettings: [String: [String: Int]]) {
        self.sceneIndex = sceneIndex
        self.themeIndex = themeIndex
        self.captureKeysForDoom = captureKeysForDoom
        self.idleAutoCycle = idleAutoCycle
        self.doomEnabled = doomEnabled
        self.sceneSettings = sceneSettings
    }

    /// Custom decode so state saved before `doomEnabled` existed still loads —
    /// missing key defaults to `false` (opt-in stays off) instead of discarding
    /// the whole saved state (scene/theme/settings) on every returning user.
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        sceneIndex = try c.decode(Int.self, forKey: .sceneIndex)
        themeIndex = try c.decode(Int.self, forKey: .themeIndex)
        captureKeysForDoom = try c.decode(Bool.self, forKey: .captureKeysForDoom)
        idleAutoCycle = try c.decode(Bool.self, forKey: .idleAutoCycle)
        doomEnabled = try c.decodeIfPresent(Bool.self, forKey: .doomEnabled) ?? false
        sceneSettings = try c.decode([String: [String: Int]].self, forKey: .sceneSettings)
    }
}

// MARK: - Screen recording

/// Captures the app's window backing store frame by frame and writes an MP4.
/// All mutable state lives on `queue`; completions are dispatched to main.
final class ScreenRecorder {
    private let queue = DispatchQueue(label: "com.builder106.ascii-arcade.recorder", qos: .userInitiated)
    private var writer: AVAssetWriter?
    private var writerInput: AVAssetWriterInput?
    private var adaptor: AVAssetWriterInputPixelBufferAdaptor?
    private var captureTimer: DispatchSourceTimer?
    private var frameCount = 0
    private var captureWindowID: CGWindowID = 0
    private var pendingCompletion: ((Result<URL, Error>) -> Void)?
    private var pendingURL: URL?
    private(set) var isRecording = false

    private static let fps = 15
    private static let durationSec = 3

    func start(windowID: CGWindowID, completion: @escaping (Result<URL, Error>) -> Void) {
        queue.async { [self] in
            guard !isRecording else { return }
            isRecording = true
            captureWindowID = windowID
            pendingCompletion = completion
            frameCount = 0

            let ts = Int(Date().timeIntervalSince1970)
            let url = FileManager.default.homeDirectoryForCurrentUser
                .appendingPathComponent("Desktop/ASCII-Arcade-\(ts).mp4")
            pendingURL = url

            guard let seed = snapshot() else {
                isRecording = false
                DispatchQueue.main.async { completion(.failure(RecErr.capture)) }
                return
            }

            do {
                try? FileManager.default.removeItem(at: url)
                let aw = try AVAssetWriter(outputURL: url, fileType: .mp4)
                let vs: [String: Any] = [
                    AVVideoCodecKey: AVVideoCodecType.h264,
                    AVVideoWidthKey: seed.width,
                    AVVideoHeightKey: seed.height
                ]
                let inp = AVAssetWriterInput(mediaType: .video, outputSettings: vs)
                inp.expectsMediaDataInRealTime = true
                let adp = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: inp,
                                                               sourcePixelBufferAttributes: nil)
                aw.add(inp)
                writer = aw; writerInput = inp; adaptor = adp
                aw.startWriting()
                aw.startSession(atSourceTime: .zero)
                append(image: seed)

                let t = DispatchSource.makeTimerSource(queue: queue)
                let iv = 1.0 / Double(ScreenRecorder.fps)
                t.schedule(deadline: .now() + iv, repeating: iv, leeway: .milliseconds(10))
                t.setEventHandler { [weak self] in self?.tick() }
                captureTimer = t
                t.resume()
            } catch {
                isRecording = false
                DispatchQueue.main.async { completion(.failure(error)) }
            }
        }
    }

    func stop() {
        queue.async { [self] in
            guard isRecording else { return }
            finalize()
        }
    }

    private func tick() {
        guard isRecording else { return }
        if let img = snapshot() { append(image: img) }
        if frameCount >= ScreenRecorder.fps * ScreenRecorder.durationSec { finalize() }
    }

    private func finalize() {
        captureTimer?.cancel(); captureTimer = nil
        isRecording = false
        guard let inp = writerInput, let aw = writer, let url = pendingURL else { return }
        let handler = pendingCompletion; pendingCompletion = nil
        inp.markAsFinished()
        aw.finishWriting {
            DispatchQueue.main.async {
                handler?(aw.status == .completed ? .success(url) : .failure(aw.error ?? RecErr.unknown))
            }
        }
        writer = nil; writerInput = nil; adaptor = nil
    }

    private func snapshot() -> CGImage? {
        CGWindowListCreateImage(.null, .optionIncludingWindow, captureWindowID, .bestResolution)
    }

    private func append(image: CGImage) {
        guard let adp = adaptor, let inp = writerInput, inp.isReadyForMoreMediaData else { return }
        let t = CMTime(value: CMTimeValue(frameCount), timescale: CMTimeScale(ScreenRecorder.fps))
        if let pb = makePixelBuffer(from: image) { adp.append(pb, withPresentationTime: t) }
        frameCount += 1
    }

    private func makePixelBuffer(from image: CGImage) -> CVPixelBuffer? {
        var pb: CVPixelBuffer?
        CVPixelBufferCreate(kCFAllocatorDefault, image.width, image.height,
                            kCVPixelFormatType_32BGRA,
                            [kCVPixelBufferCGImageCompatibilityKey: true,
                             kCVPixelBufferCGBitmapContextCompatibilityKey: true] as CFDictionary,
                            &pb)
        guard let pixBuf = pb else { return nil }
        CVPixelBufferLockBaseAddress(pixBuf, [])
        defer { CVPixelBufferUnlockBaseAddress(pixBuf, []) }
        guard let ctx = CGContext(
            data: CVPixelBufferGetBaseAddress(pixBuf),
            width: image.width, height: image.height,
            bitsPerComponent: 8,
            bytesPerRow: CVPixelBufferGetBytesPerRow(pixBuf),
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        else { return nil }
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return pixBuf
    }

    enum RecErr: Error { case capture, unknown }
}

/// Carried on each scene-setting menu item so the action knows what to apply.
final class SettingChoice {
    let settingId: String
    let value: Double
    let optionIndex: Int
    init(settingId: String, value: Double, optionIndex: Int) {
        self.settingId = settingId
        self.value = value
        self.optionIndex = optionIndex
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    var windows: [NSWindow] = []
    var views: [SceneView] = []
    var globalMonitor: Any?
    var statusItem: NSStatusItem?
    var currentThemeIndex = 0
    var currentSceneIndex = 0
    var captureKeysForDoom = true
    /// DOOM is opt-in: hidden from the Scene menu and skipped by cycling/idle
    /// auto-cycle until the user explicitly turns it on. Defaults off so it
    /// can't ambush someone who just wanted an ambient wallpaper.
    var doomEnabled = false
    var originalWallpapers: [NSScreen: URL] = [:]

    // Per-scene chosen setting options: sceneIndex → (settingId → optionIndex).
    var sceneSettingSelections: [Int: [String: Int]] = [:]
    var settingsMenuItem: NSMenuItem?

    // Screenshot / recording
    private let recorder = ScreenRecorder()
    private var recordingActive = false
    private var recordingMenuItem: NSMenuItem?
    private var blinkTimer: Timer?

    // Idle auto-cycle: when the Mac sits untouched, rotate through the scenes
    // like a slideshow, then snap back to the chosen scene on the next input.
    var idleAutoCycle = false
    let idleThreshold: TimeInterval = 90
    let idleCycleInterval: TimeInterval = 20
    var idleTimer: Timer?
    var wasIdle = false
    var preIdleSceneIndex = 0
    var lastAutoCycle: CFTimeInterval = 0
    var isAsleep = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Save original wallpapers before we touch anything
        for screen in NSScreen.screens {
            if let url = NSWorkspace.shared.desktopImageURL(for: screen) {
                originalWallpapers[screen] = url
            }
        }

        // Restore remembered scene / theme / settings from a previous run.
        let restored = loadState()
        if let s = restored {
            currentSceneIndex = min(max(0, s.sceneIndex), sceneNames.count - 1)
            currentThemeIndex = min(max(0, s.themeIndex), availableThemes.count - 1)
            captureKeysForDoom = s.captureKeysForDoom
            idleAutoCycle = s.idleAutoCycle
            doomEnabled = s.doomEnabled
            sceneSettingSelections = Dictionary(uniqueKeysWithValues:
                s.sceneSettings.compactMap { key, val in Int(key).map { ($0, val) } })
        }
        // A scene lineup change (e.g. removing scenes) can shift indices out from
        // under a restored `sceneIndex` and land it on DOOM by coincidence — never
        // let that bypass the opt-in.
        if sceneIsInteractive[currentSceneIndex], !doomEnabled {
            currentSceneIndex = 0
        }

        for screen in NSScreen.screens {
            let window = DesktopSceneWindow(screen: screen)
            let view = SceneView(frame: window.contentView!.bounds, scenes: makeScenes())
            view.autoresizingMask = [.width, .height]
            window.contentView = view
            window.makeKeyAndOrderFront(nil)
            window.orderBack(nil)
            windows.append(window)
            views.append(view)
            // Apply remembered per-scene settings before the scene starts.
            view.applyPersistedSettings(sceneSettingSelections)
            view.applyTheme(availableThemes[currentThemeIndex])
            view.selectScene(currentSceneIndex)
        }

        setupStatusItem()

        let axOptions = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        AXIsProcessTrustedWithOptions(axOptions)

        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self = self else { return }
            // ⌘⌥C cycles scenes.
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            if flags == [.command, .option],
               event.charactersIgnoringModifiers?.lowercased() == "c" {
                DispatchQueue.main.async { self.cycleScenes() }
                return
            }
            if flags == [.command, .option],
               event.charactersIgnoringModifiers?.lowercased() == "s" {
                DispatchQueue.main.async { self.saveScreenshot() }
                return
            }
            if flags == [.command, .option],
               event.charactersIgnoringModifiers?.lowercased() == "r" {
                DispatchQueue.main.async { self.toggleRecording() }
                return
            }
            // Otherwise, when DOOM is the active wallpaper, play it.
            if self.captureKeysForDoom,
               self.views.first?.currentScene.isInteractive == true,
               !flags.contains(.command), !flags.contains(.control),
               let bytes = doomBytes(for: event) {
                DispatchQueue.main.async {
                    for view in self.views { view.forwardKey(bytes) }
                }
            }
        }

        // Pause/resume rendering when the displays sleep, to save power.
        let nc = NSWorkspace.shared.notificationCenter
        nc.addObserver(self, selector: #selector(screensDidSleep),
                       name: NSWorkspace.screensDidSleepNotification, object: nil)
        nc.addObserver(self, selector: #selector(screensDidWake),
                       name: NSWorkspace.screensDidWakeNotification, object: nil)

        // Poll system idle time for the auto-cycle slideshow.
        idleTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { [weak self] _ in
            self?.checkIdle()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let monitor = globalMonitor { NSEvent.removeMonitor(monitor) }
        idleTimer?.invalidate()
        blinkTimer?.invalidate()
        NSWorkspace.shared.notificationCenter.removeObserver(self)
        if recordingActive { recorder.stop() }
        // Tear down any running DOOM PTY.
        for view in views { view.stopCurrentScene() }
        // Restore original wallpapers
        for (screen, url) in originalWallpapers {
            setWallpaper(url, for: screen)
        }
    }

    // MARK: - Sleep / wake

    @objc func screensDidSleep() {
        isAsleep = true
        for view in views { view.pause() }
    }

    @objc func screensDidWake() {
        isAsleep = false
        for view in views { view.resume() }
    }

    // MARK: - Idle auto-cycle

    /// System idle seconds (time since the last user input event).
    private func systemIdleSeconds() -> TimeInterval {
        let anyInput = CGEventType(rawValue: ~UInt32(0)) ?? .null
        return CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: anyInput)
    }

    private func checkIdle() {
        guard idleAutoCycle, !isAsleep else { return }
        let idle = systemIdleSeconds()
        let now = CACurrentMediaTime()
        if idle >= idleThreshold {
            if !wasIdle {
                wasIdle = true
                preIdleSceneIndex = currentSceneIndex
                lastAutoCycle = now
            }
            if now - lastAutoCycle >= idleCycleInterval {
                lastAutoCycle = now
                selectScene(nextCyclableSceneIndex(from: currentSceneIndex))
            }
        } else if wasIdle {
            // User came back — restore the scene they had chosen.
            wasIdle = false
            selectScene(preIdleSceneIndex)
        }
    }

    // MARK: - Capture

    private func captureWindowImage() -> CGImage? {
        guard let win = windows.first else { return nil }
        return CGWindowListCreateImage(.null, .optionIncludingWindow,
                                       CGWindowID(win.windowNumber), .bestResolution)
    }

    /// Flash a brief message in the status-bar button, then restore "◎".
    private func flashStatus(_ text: String, duration: Double = 2.0) {
        guard !recordingActive else { return }
        statusItem?.button?.title = text
        DispatchQueue.main.asyncAfter(deadline: .now() + duration) { [weak self] in
            guard self?.recordingActive != true else { return }
            self?.statusItem?.button?.title = "◎"
        }
    }

    private func startBlinking() {
        statusItem?.button?.title = "◉"
        var on = true
        blinkTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.statusItem?.button?.title = on ? "◎" : "◉"
            on = !on
        }
    }

    private func stopBlinking() {
        blinkTimer?.invalidate(); blinkTimer = nil
        statusItem?.button?.title = "◎"
    }

    /// Save a PNG of the current scene to ~/Desktop and copy it to the clipboard.
    @objc func saveScreenshot() {
        guard let img = captureWindowImage() else { flashStatus("✗"); return }
        let ts = Int(Date().timeIntervalSince1970)
        let url = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Desktop/ASCII-Arcade-\(ts).png")
        let rep = NSBitmapImageRep(cgImage: img)
        if let data = rep.representation(using: .png, properties: [:]),
           (try? data.write(to: url)) != nil {
            let pb = NSPasteboard.general
            pb.clearContents()
            pb.writeObjects([NSImage(cgImage: img, size: .zero)])
            flashStatus("✓")
        } else {
            flashStatus("✗")
        }
    }

    /// Start (or stop early) a 3-second MP4 clip of the current scene, saved to ~/Desktop.
    @objc func toggleRecording() {
        if recordingActive { recorder.stop(); return }
        guard let win = windows.first else { return }
        let windowID = CGWindowID(win.windowNumber)
        recordingActive = true
        recordingMenuItem?.title = "Stop Recording  (⌘⌥R)"
        startBlinking()
        recorder.start(windowID: windowID) { [weak self] result in
            guard let self else { return }
            self.recordingActive = false
            self.stopBlinking()
            self.recordingMenuItem?.title = "Record 3-Sec Clip  (⌘⌥R)"
            switch result {
            case .success(let url):
                self.flashStatus("✓ Clip saved", duration: 3.0)
                NSWorkspace.shared.activateFileViewerSelecting([url])
            case .failure:
                self.flashStatus("✗ Clip failed")
            }
        }
    }

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        statusItem?.button?.title = "◎"
        rebuildMenu()
    }

    /// (Re)build the status-bar menu from scratch. Called at startup and again
    /// whenever something that changes its shape — the DOOM opt-in toggle — is
    /// flipped, so the Scene list and the DOOM-only items stay in sync.
    private func rebuildMenu() {
        let menu = NSMenu()

        let header = NSMenuItem(title: "Scene", action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        for (i, name) in sceneNames.enumerated() where !sceneIsInteractive[i] || doomEnabled {
            let item = NSMenuItem(title: name, action: #selector(selectSceneMenu(_:)), keyEquivalent: "")
            item.target = self
            item.tag = i
            item.state = i == currentSceneIndex ? .on : .off
            menu.addItem(item)
        }

        menu.addItem(.separator())
        let cycleItem = NSMenuItem(title: "Next Scene  (⌘⌥C)", action: #selector(cycleScenes), keyEquivalent: "")
        cycleItem.target = self
        menu.addItem(cycleItem)

        let doomItem = NSMenuItem(title: "Enable DOOM Scene", action: #selector(toggleDoomEnabled(_:)), keyEquivalent: "")
        doomItem.target = self
        doomItem.state = doomEnabled ? .on : .off
        menu.addItem(doomItem)

        // Only meaningful once DOOM is opted into — hidden otherwise so it
        // doesn't clutter the default menu with a control for a scene that
        // isn't reachable.
        if doomEnabled {
            let captureItem = NSMenuItem(title: "Capture keys for DOOM", action: #selector(toggleCapture(_:)), keyEquivalent: "")
            captureItem.target = self
            captureItem.state = captureKeysForDoom ? .on : .off
            menu.addItem(captureItem)
        }

        let settingsItem = NSMenuItem(title: "Scene Settings", action: nil, keyEquivalent: "")
        settingsItem.submenu = NSMenu(title: "Scene Settings")
        menu.addItem(settingsItem)
        settingsMenuItem = settingsItem

        let idleItem = NSMenuItem(title: "Auto-cycle when idle", action: #selector(toggleIdleAutoCycle(_:)), keyEquivalent: "")
        idleItem.target = self
        idleItem.state = idleAutoCycle ? .on : .off
        menu.addItem(idleItem)

        let loginItem = NSMenuItem(title: "Launch at Login", action: #selector(toggleLaunchAtLogin(_:)), keyEquivalent: "")
        loginItem.target = self
        loginItem.state = SMAppService.mainApp.status == .enabled ? .on : .off
        menu.addItem(loginItem)

        menu.addItem(.separator())
        let captureHeader = NSMenuItem(title: "Capture", action: nil, keyEquivalent: "")
        captureHeader.isEnabled = false
        menu.addItem(captureHeader)
        let screenshotItem = NSMenuItem(title: "Save Screenshot  (⌘⌥S)", action: #selector(saveScreenshot), keyEquivalent: "")
        screenshotItem.target = self
        menu.addItem(screenshotItem)
        let recordItem = NSMenuItem(title: "Record 3-Sec Clip  (⌘⌥R)", action: #selector(toggleRecording), keyEquivalent: "")
        recordItem.target = self
        recordingMenuItem = recordItem
        menu.addItem(recordItem)

        menu.addItem(.separator())
        let themeHeader = NSMenuItem(title: "Theme", action: nil, keyEquivalent: "")
        themeHeader.isEnabled = false
        menu.addItem(themeHeader)
        for (i, theme) in availableThemes.enumerated() {
            let item = NSMenuItem(title: theme.name, action: #selector(selectTheme(_:)), keyEquivalent: "")
            item.target = self
            item.tag = i
            item.state = i == currentThemeIndex ? .on : .off
            menu.addItem(item)
        }

        menu.addItem(.separator())
        menu.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")

        statusItem?.menu = menu
        rebuildSettingsMenu()
    }

    /// Repopulate the "Scene Settings" submenu to reflect the current scene's
    /// `settings`. Called at startup and whenever the active scene changes.
    private func rebuildSettingsMenu() {
        guard let submenu = settingsMenuItem?.submenu else { return }
        submenu.removeAllItems()
        let settings = views.first?.currentScene.settings ?? []
        settingsMenuItem?.isEnabled = !settings.isEmpty
        if settings.isEmpty {
            let none = NSMenuItem(title: "No settings for this scene", action: nil, keyEquivalent: "")
            none.isEnabled = false
            submenu.addItem(none)
            return
        }
        var selections = sceneSettingSelections[currentSceneIndex] ?? [:]
        for (settingIndex, setting) in settings.enumerated() {
            let header = NSMenuItem(title: setting.label, action: nil, keyEquivalent: "")
            header.isEnabled = false
            submenu.addItem(header)
            let chosen = selections[setting.id] ?? setting.defaultIndex
            selections[setting.id] = chosen
            for (i, option) in setting.options.enumerated() {
                let item = NSMenuItem(title: "  " + option.label, action: #selector(selectSetting(_:)), keyEquivalent: "")
                item.target = self
                item.representedObject = SettingChoice(settingId: setting.id, value: option.value, optionIndex: i)
                item.state = (i == chosen) ? .on : .off
                submenu.addItem(item)
            }
            if settingIndex < settings.count - 1 { submenu.addItem(.separator()) }
        }
        sceneSettingSelections[currentSceneIndex] = selections
    }

    @objc func selectSetting(_ sender: NSMenuItem) {
        guard let choice = sender.representedObject as? SettingChoice else { return }
        var selections = sceneSettingSelections[currentSceneIndex] ?? [:]
        selections[choice.settingId] = choice.optionIndex
        sceneSettingSelections[currentSceneIndex] = selections
        for view in views { view.applySettingToCurrentScene(id: choice.settingId, value: choice.value) }
        rebuildSettingsMenu()
        saveState()
    }

    @objc func toggleIdleAutoCycle(_ sender: NSMenuItem) {
        idleAutoCycle.toggle()
        sender.state = idleAutoCycle ? .on : .off
        if !idleAutoCycle, wasIdle {
            wasIdle = false
            selectScene(preIdleSceneIndex)
        }
        saveState()
    }

    @objc func toggleLaunchAtLogin(_ sender: NSMenuItem) {
        do {
            if SMAppService.mainApp.status == .enabled {
                try SMAppService.mainApp.unregister()
            } else {
                try SMAppService.mainApp.register()
            }
        } catch {
            NSLog("ASCII Arcade: launch-at-login toggle failed: \(error.localizedDescription)")
        }
        sender.state = SMAppService.mainApp.status == .enabled ? .on : .off
    }

    // MARK: - Persistence

    private let stateKey = "AsciiArcadeState"

    private func loadState() -> PersistedState? {
        guard let data = UserDefaults.standard.data(forKey: stateKey) else { return nil }
        return try? JSONDecoder().decode(PersistedState.self, from: data)
    }

    private func saveState() {
        let sceneSettings = Dictionary(uniqueKeysWithValues:
            sceneSettingSelections.map { (String($0.key), $0.value) })
        let state = PersistedState(
            sceneIndex: currentSceneIndex,
            themeIndex: currentThemeIndex,
            captureKeysForDoom: captureKeysForDoom,
            idleAutoCycle: idleAutoCycle,
            doomEnabled: doomEnabled,
            sceneSettings: sceneSettings)
        if let data = try? JSONEncoder().encode(state) {
            UserDefaults.standard.set(data, forKey: stateKey)
        }
    }

    @objc func selectSceneMenu(_ sender: NSMenuItem) {
        selectScene(sender.tag)
    }

    func selectScene(_ index: Int) {
        guard index >= 0, index < sceneNames.count else { return }
        currentSceneIndex = index
        for view in views { view.selectScene(index) }
        updateMenuSceneCheckmarks()
        rebuildSettingsMenu()
        saveState()
    }

    @objc func cycleScenes() {
        selectScene(nextCyclableSceneIndex(from: currentSceneIndex))
    }

    /// Next scene index in rotation order, skipping DOOM (or any interactive
    /// scene) while it isn't opted into. Used by ⌘⌥C and idle auto-cycle so
    /// neither can land on DOOM behind the user's back.
    private func nextCyclableSceneIndex(from index: Int) -> Int {
        var next = (index + 1) % sceneNames.count
        while sceneIsInteractive[next], !doomEnabled, next != index {
            next = (next + 1) % sceneNames.count
        }
        return next
    }

    @objc func toggleCapture(_ sender: NSMenuItem) {
        captureKeysForDoom.toggle()
        sender.state = captureKeysForDoom ? .on : .off
        saveState()
    }

    /// Toggle the DOOM opt-in. Turning it off while DOOM is the active scene
    /// switches away from it first, same as disabling idle auto-cycle mid-idle
    /// restores the prior scene.
    @objc func toggleDoomEnabled(_ sender: NSMenuItem) {
        doomEnabled.toggle()
        if !doomEnabled, sceneIsInteractive[currentSceneIndex] {
            selectScene(0)
        } else {
            saveState()
        }
        rebuildMenu()
    }

    @objc func selectTheme(_ sender: NSMenuItem) {
        currentThemeIndex = sender.tag
        let theme = availableThemes[currentThemeIndex]
        for view in views { view.applyTheme(theme) }
        updateMenuThemeCheckmarks()
        saveState()
    }

    private func updateMenuSceneCheckmarks() {
        guard let menu = statusItem?.menu else { return }
        for item in menu.items where item.action == #selector(selectSceneMenu(_:)) {
            item.state = item.tag == currentSceneIndex ? .on : .off
        }
    }

    private func updateMenuThemeCheckmarks() {
        guard let menu = statusItem?.menu else { return }
        for item in menu.items where item.action == #selector(selectTheme(_:)) {
            item.state = item.tag == currentThemeIndex ? .on : .off
        }
    }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.setActivationPolicy(.accessory)
app.delegate = delegate
app.run()
