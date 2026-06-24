import AppKit
import CoreVideo
import CoreText
import AsciiArcadeCore

struct Theme {
    let name: String
    let textColor: NSColor
    let backgroundColor: NSColor
}

let availableThemes: [Theme] = [
    Theme(name: "Hacker",
          textColor: .systemGreen,
          backgroundColor: .black),
    Theme(name: "Amber",
          textColor: NSColor(calibratedRed: 1.0,  green: 0.65, blue: 0.0,   alpha: 1.0),
          backgroundColor: NSColor(calibratedRed: 0.102, green: 0.031, blue: 0.0, alpha: 1.0)),
    Theme(name: "Ice",
          textColor: .cyan,
          backgroundColor: NSColor(calibratedRed: 0.0, green: 0.051, blue: 0.102, alpha: 1.0)),
    Theme(name: "Ghost",
          textColor: NSColor(calibratedRed: 0.11, green: 0.11, blue: 0.118, alpha: 1.0),
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
        FireScene(),
        GameOfLifeScene(),
        PipesScene(),
        ClockScene(),
        DoomScene(workingDirectory: cwd)
    ]
}

let sceneNames: [String] = makeScenes().map { $0.displayName }

// MARK: - Wallpaper helpers

private func solidColorWallpaperURL(_ color: NSColor) -> URL? {
    let size = NSSize(width: 2, height: 2)
    let image = NSImage(size: size)
    image.lockFocus()
    color.setFill()
    NSRect(origin: .zero, size: size).fill()
    image.unlockFocus()
    guard let tiff = image.tiffRepresentation,
          let rep  = NSBitmapImageRep(data: tiff),
          let png  = rep.representation(using: .png, properties: [:]) else { return nil }
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("ascii_arcade_wallpaper_\(abs(color.hashValue)).png")
    try? png.write(to: url)
    return url
}

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

        layer?.shadowColor = availableThemes[0].textColor.cgColor
        layer?.shadowRadius = 10
        layer?.shadowOpacity = 0.45
        layer?.shadowOffset = .zero
        layer?.shadowPath = CGPath(rect: bounds, transform: nil)

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
        layer?.shadowColor = theme.textColor.cgColor
        let rgb = SceneView.rgbColor(from: theme.textColor)
        for scene in scenes { scene.applyBaseColor(rgb) }
        needsDisplay = true
    }

    /// Apply a setting value to the current scene (called from the menu).
    func applySettingToCurrentScene(id: String, value: Double) {
        currentScene.applySetting(id: id, value: value)
        needsDisplay = true
    }

    override func layout() {
        super.layout()
        updateScanlines()
        layer?.shadowPath = CGPath(rect: bounds, transform: nil)
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        let drawStart = instrument ? CACurrentMediaTime() : 0
        let t = CFAbsoluteTimeGetCurrent() - startTime

        let insetX = bounds.width * (1.0 - scale) / 2.0
        let insetY = bounds.height * (1.0 - scale) / 2.0
        let paddedRect = bounds.insetBy(dx: insetX, dy: insetY).insetBy(dx: 6, dy: 6)

        let charW = cellCharWidth
        let lineH = cellLineHeight
        let (w, h) = DonutFrameGenerator.gridDimensions(
            paddedWidth: Double(paddedRect.width),
            paddedHeight: Double(paddedRect.height),
            charWidth: Double(charW),
            lineHeight: Double(lineH)
        )
        currentScene.setGrid(width: w, height: h)

        let contentWidth = CGFloat(w) * charW
        let contentHeight = CGFloat(h) * lineH
        let originX = paddedRect.midX - contentWidth / 2.0
        let originY = paddedRect.midY - contentHeight / 2.0
        let viewH = bounds.height
        let ascent = cellAscent

        // Bucket every non-blank cell's glyph by colour. Adjacent cells that share
        // a colour (or a palette entry) collapse into the same bucket, so a
        // full-screen frame becomes a few dozen draw calls instead of thousands.
        var batches: [AsciiArcadeCore.RGBColor?: GlyphBatch] = [:]
        func emit(_ ch: Character, row: Int, col: Int, color: AsciiArcadeCore.RGBColor?) {
            guard ch != " ", let g = glyph(for: ch) else { return }
            let batch: GlyphBatch
            if let existing = batches[color] {
                batch = existing
            } else {
                batch = GlyphBatch()
                batches[color] = batch
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
        for (color, batch) in batches where !batch.glyphs.isEmpty {
            ctx.setFillColor(color.map { cgColor(for: $0) } ?? themeTextColor.cgColor)
            CTFontDrawGlyphs(ctFont, batch.glyphs, batch.positions, batch.glyphs.count, ctx)
        }
        ctx.restoreGState()

        if instrument {
            instrFrames += 1
            instrDrawMs += (CACurrentMediaTime() - drawStart) * 1000.0
            let now = CACurrentMediaTime()
            if now - instrLast >= 1.0 {
                let avg = instrDrawMs / Double(max(1, instrFrames))
                NSLog("ASCII_FPS scene=%@ grid=%dx%d fps=%d avgDraw=%.2fms batches=%d",
                      currentScene.displayName, w, h, instrFrames, avg, batches.count)
                instrFrames = 0; instrDrawMs = 0; instrLast = now
            }
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
    var originalWallpapers: [NSScreen: URL] = [:]

    // Per-scene chosen setting options: sceneIndex → (settingId → optionIndex).
    var sceneSettingSelections: [Int: [String: Int]] = [:]
    var settingsMenuItem: NSMenuItem?

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

        for screen in NSScreen.screens {
            let window = DesktopSceneWindow(screen: screen)
            let view = SceneView(frame: window.contentView!.bounds, scenes: makeScenes())
            view.autoresizingMask = [.width, .height]
            window.contentView = view
            window.makeKeyAndOrderFront(nil)
            window.orderBack(nil)
            windows.append(window)
            views.append(view)
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
        NSWorkspace.shared.notificationCenter.removeObserver(self)
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
                selectScene((currentSceneIndex + 1) % sceneNames.count)
            }
        } else if wasIdle {
            // User came back — restore the scene they had chosen.
            wasIdle = false
            selectScene(preIdleSceneIndex)
        }
    }

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        statusItem?.button?.title = "◎"

        let menu = NSMenu()

        let header = NSMenuItem(title: "Scene", action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        for (i, name) in sceneNames.enumerated() {
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

        let captureItem = NSMenuItem(title: "Capture keys for DOOM", action: #selector(toggleCapture(_:)), keyEquivalent: "")
        captureItem.target = self
        captureItem.state = captureKeysForDoom ? .on : .off
        menu.addItem(captureItem)

        let settingsItem = NSMenuItem(title: "Scene Settings", action: nil, keyEquivalent: "")
        settingsItem.submenu = NSMenu(title: "Scene Settings")
        menu.addItem(settingsItem)
        settingsMenuItem = settingsItem

        let idleItem = NSMenuItem(title: "Auto-cycle when idle", action: #selector(toggleIdleAutoCycle(_:)), keyEquivalent: "")
        idleItem.target = self
        idleItem.state = idleAutoCycle ? .on : .off
        menu.addItem(idleItem)

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
    }

    @objc func toggleIdleAutoCycle(_ sender: NSMenuItem) {
        idleAutoCycle.toggle()
        sender.state = idleAutoCycle ? .on : .off
        if !idleAutoCycle, wasIdle {
            wasIdle = false
            selectScene(preIdleSceneIndex)
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
    }

    @objc func cycleScenes() {
        selectScene((currentSceneIndex + 1) % sceneNames.count)
    }

    @objc func toggleCapture(_ sender: NSMenuItem) {
        captureKeysForDoom.toggle()
        sender.state = captureKeysForDoom ? .on : .off
    }

    @objc func selectTheme(_ sender: NSMenuItem) {
        currentThemeIndex = sender.tag
        let theme = availableThemes[currentThemeIndex]
        for view in views { view.applyTheme(theme) }
        if let url = solidColorWallpaperURL(theme.backgroundColor) {
            for screen in NSScreen.screens { setWallpaper(url, for: screen) }
        }
        updateMenuThemeCheckmarks()
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
