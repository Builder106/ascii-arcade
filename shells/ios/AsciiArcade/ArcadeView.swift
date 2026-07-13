import SwiftUI
import MetalKit

// UIViewRepresentable that hosts the MTKView and owns the ArcadeRenderer.
// The Coordinator IS the renderer; makeUIView wires the MTKView to it.
struct ArcadeView: UIViewRepresentable {
    let sceneId: String
    let theme: Theme
    let topInsetPt: CGFloat
    let bottomInsetPt: CGFloat

    func makeCoordinator() -> ArcadeRenderer {
        ArcadeRenderer()
    }

    func makeUIView(context: Context) -> MTKView {
        let view = MTKView()
        view.device = MTLCreateSystemDefaultDevice()
        view.isPaused = false
        view.enableSetNeedsDisplay = false
        view.preferredFramesPerSecond = 60
        view.colorPixelFormat = .bgra8Unorm
        view.framebufferOnly = false
        context.coordinator.attach(to: view)
        context.coordinator.engine = AaEngine(sceneId: sceneId)
        context.coordinator.theme = theme
        return view
    }

    func updateUIView(_ view: MTKView, context: Context) {
        let r = context.coordinator
        if r.currentSceneId != sceneId {
            r.engine = AaEngine(sceneId: sceneId)
            r.currentSceneId = sceneId
        }
        r.theme = theme
        r.updateLayout(
            drawableSize: view.drawableSize,
            contentScale: view.contentScaleFactor,
            topInsetPt: topInsetPt,
            bottomInsetPt: bottomInsetPt,
            tabBarHeightPt: 49
        )
    }
}
