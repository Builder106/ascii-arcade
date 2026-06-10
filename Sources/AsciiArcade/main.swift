import AppKit
import CoreVideo
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
    private var textAttributesBase: [NSAttributedString.Key: Any]
    private let font: NSFont
    private let scale: CGFloat = 0.92
    private let scanlinesLayer = CAReplicatorLayer()
    private let scanlineStripeLayer = CALayer()

    init(frame: CGRect, scenes: [any AsciiScene]) {
        self.scenes = scenes
        self.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        self.textAttributesBase = [
            .font: font,
            .foregroundColor: availableThemes[0].textColor,
            .backgroundColor: NSColor.clear
        ]
        super.init(frame: frame)
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

    func applyTheme(_ theme: Theme) {
        textAttributesBase[.foregroundColor] = theme.textColor
        layer?.shadowColor = theme.textColor.cgColor
        needsDisplay = true
    }

    override func layout() {
        super.layout()
        updateScanlines()
        layer?.shadowPath = CGPath(rect: bounds, transform: nil)
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        let t = CFAbsoluteTimeGetCurrent() - startTime

        let insetX = bounds.width * (1.0 - scale) / 2.0
        let insetY = bounds.height * (1.0 - scale) / 2.0
        let drawRect = bounds.insetBy(dx: insetX, dy: insetY)

        let padding: CGFloat = 6
        let paddedRect = drawRect.insetBy(dx: padding, dy: padding)

        let charWidth = max(1.0, ("@" as NSString).size(withAttributes: [.font: font]).width)
        let lineHeight = CGFloat(font.ascender - font.descender + font.leading)

        let (w, h) = DonutFrameGenerator.gridDimensions(
            paddedWidth: Double(paddedRect.width),
            paddedHeight: Double(paddedRect.height),
            charWidth: Double(charWidth),
            lineHeight: Double(lineHeight)
        )
        currentScene.setGrid(width: w, height: h)
        let frameText = currentScene.frame(atTime: t)

        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.alignment = .left
        paragraphStyle.minimumLineHeight = lineHeight
        paragraphStyle.maximumLineHeight = lineHeight
        paragraphStyle.lineBreakMode = .byClipping

        let attrs: [NSAttributedString.Key: Any] = textAttributesBase.merging([.paragraphStyle: paragraphStyle]) { $1 }
        let attributed = NSAttributedString(string: frameText, attributes: attrs)

        let contentWidth = CGFloat(w) * charWidth
        let contentHeight = CGFloat(h) * lineHeight
        let textRect = CGRect(
            x: paddedRect.midX - contentWidth / 2.0,
            y: paddedRect.midY - contentHeight / 2.0,
            width: contentWidth,
            height: contentHeight
        )
        attributed.draw(in: textRect)
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
            DispatchQueue.main.async { view.needsDisplay = true }
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

final class AppDelegate: NSObject, NSApplicationDelegate {
    var windows: [NSWindow] = []
    var views: [SceneView] = []
    var globalMonitor: Any?
    var statusItem: NSStatusItem?
    var currentThemeIndex = 0
    var currentSceneIndex = 0
    var captureKeysForDoom = true
    var originalWallpapers: [NSScreen: URL] = [:]

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
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let monitor = globalMonitor { NSEvent.removeMonitor(monitor) }
        // Tear down any running DOOM PTY.
        for view in views { view.stopCurrentScene() }
        // Restore original wallpapers
        for (screen, url) in originalWallpapers {
            setWallpaper(url, for: screen)
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
    }

    @objc func selectSceneMenu(_ sender: NSMenuItem) {
        selectScene(sender.tag)
    }

    func selectScene(_ index: Int) {
        guard index >= 0, index < sceneNames.count else { return }
        currentSceneIndex = index
        for view in views { view.selectScene(index) }
        updateMenuSceneCheckmarks()
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
