import AppKit
import CoreVideo
import DonutCore

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
        .appendingPathComponent("donut_wallpaper_\(abs(color.hashValue)).png")
    try? png.write(to: url)
    return url
}

private func setWallpaper(_ url: URL, for screen: NSScreen) {
    try? NSWorkspace.shared.setDesktopImageURL(url, for: screen, options: [:])
}

// MARK: - Window

final class DesktopDonutWindow: NSWindow {
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

final class DonutView: NSView {
    override var isOpaque: Bool { false }
    override var isFlipped: Bool { true }

    private var generator: any ShapeFrameGenerator
    private var shapeFactories: [(Int, Int) -> any ShapeFrameGenerator] = [
        { w, h in DonutFrameGenerator(width: w, height: h) },
        { w, h in HelixFrameGenerator(width: w, height: h) }
    ]
    private var currentShapeIndex = 0
    private var startTime: CFAbsoluteTime = CFAbsoluteTimeGetCurrent()
    private var displayLink: CVDisplayLink?
    private var textAttributesBase: [NSAttributedString.Key: Any]
    private let font: NSFont
    private let scale: CGFloat = 0.92
    private let scanlinesLayer = CAReplicatorLayer()
    private let scanlineStripeLayer = CALayer()

    override init(frame: CGRect) {
        self.generator = DonutFrameGenerator(width: 10, height: 10)
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

    func cycleToNextShape() {
        currentShapeIndex = (currentShapeIndex + 1) % shapeFactories.count
        generator = shapeFactories[currentShapeIndex](generator.width, generator.height)
        startTime = CFAbsoluteTimeGetCurrent()
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
        if w != generator.width || h != generator.height {
            generator = shapeFactories[currentShapeIndex](w, h)
        }
        let frameText = generator.frame(atTime: t)

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
            let view = Unmanaged<DonutView>.fromOpaque(userData!).takeUnretainedValue()
            DispatchQueue.main.async { view.needsDisplay = true }
            return kCVReturnSuccess
        }, Unmanaged.passUnretained(self).toOpaque())
        CVDisplayLinkStart(displayLink)
    }

    deinit {
        if let displayLink = displayLink { CVDisplayLinkStop(displayLink) }
    }
}

// MARK: - App Delegate

final class AppDelegate: NSObject, NSApplicationDelegate {
    var windows: [NSWindow] = []
    var globalMonitor: Any?
    var statusItem: NSStatusItem?
    var currentThemeIndex = 0
    var originalWallpapers: [NSScreen: URL] = [:]

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Save original wallpapers before we touch anything
        for screen in NSScreen.screens {
            if let url = NSWorkspace.shared.desktopImageURL(for: screen) {
                originalWallpapers[screen] = url
            }
        }

        for screen in NSScreen.screens {
            let window = DesktopDonutWindow(screen: screen)
            let view = DonutView(frame: window.contentView!.bounds)
            view.autoresizingMask = [.width, .height]
            window.contentView = view
            window.makeKeyAndOrderFront(nil)
            window.orderBack(nil)
            windows.append(window)
        }

        setupStatusItem()

        let axOptions = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        AXIsProcessTrustedWithOptions(axOptions)

        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            if flags == [.command, .option],
               event.charactersIgnoringModifiers?.lowercased() == "c" {
                DispatchQueue.main.async { self?.cycleShapes() }
            }
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let monitor = globalMonitor { NSEvent.removeMonitor(monitor) }
        // Restore original wallpapers
        for (screen, url) in originalWallpapers {
            setWallpaper(url, for: screen)
        }
    }

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        statusItem?.button?.title = "◎"

        let menu = NSMenu()

        let shapeItem = NSMenuItem(title: "Next Shape  (⌘⌥C)", action: #selector(cycleShapes), keyEquivalent: "")
        shapeItem.target = self
        menu.addItem(shapeItem)
        menu.addItem(.separator())

        for (i, theme) in availableThemes.enumerated() {
            let item = NSMenuItem(title: theme.name, action: #selector(selectTheme(_:)), keyEquivalent: "")
            item.target = self
            item.tag = i
            item.state = i == 0 ? .on : .off
            menu.addItem(item)
        }

        menu.addItem(.separator())
        menu.addItem(withTitle: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")

        statusItem?.menu = menu
    }

    @objc func cycleShapes() {
        for window in windows {
            (window.contentView as? DonutView)?.cycleToNextShape()
        }
    }

    @objc func selectTheme(_ sender: NSMenuItem) {
        currentThemeIndex = sender.tag
        let theme = availableThemes[currentThemeIndex]
        for window in windows {
            (window.contentView as? DonutView)?.applyTheme(theme)
        }
        if let url = solidColorWallpaperURL(theme.backgroundColor) {
            for screen in NSScreen.screens { setWallpaper(url, for: screen) }
        }
        updateMenuThemeCheckmarks()
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
