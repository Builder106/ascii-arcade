import SwiftUI

struct SceneView: View {
    @AppStorage("sceneId")  private var sceneId: String  = "matrix"
    @AppStorage("themeName") private var themeName: String = "Hacker"
    @Environment(\.displayScale) private var displayScale
    @StateObject private var exporter = LivePhotoExporter()

    private var theme: Theme { Theme.named(themeName) }
    private let scenes = AaEngine.sceneNames()

    var body: some View {
        GeometryReader { geo in
            let insets = geo.safeAreaInsets
            ZStack(alignment: .bottomLeading) {
                ArcadeView(
                    sceneId: sceneId,
                    theme: theme,
                    topInsetPt: insets.top,
                    bottomInsetPt: insets.bottom
                )
                .ignoresSafeArea(.all)

                // HUD: scene label + theme badge
                HUD(
                    sceneId: $sceneId,
                    themeName: $themeName,
                    scenes: scenes,
                    theme: theme,
                    isExporting: exporter.isExporting,
                    onExport: {
                        let pixelSize = CGSize(width: geo.size.width * displayScale, height: geo.size.height * displayScale)
                        exporter.export(sceneId: sceneId, theme: theme, pixelSize: pixelSize, displayScale: displayScale)
                    }
                )
                .padding(.horizontal, 12)
                .padding(.bottom, insets.bottom + 49 + 8)
            }
        }
        .background(Color(theme.background))
        .onAppear {
            // A scene removed from the engine since this id was last persisted
            // (e.g. a retired scene) would otherwise leave `AaEngine(sceneId:)`
            // returning nil and the canvas permanently blank.
            if !scenes.contains(sceneId), let fallback = scenes.first {
                sceneId = fallback
            }
        }
        .alert("Saved to Photos", isPresented: $exporter.didSucceed) {
            Button("OK") {}
        } message: {
            Text("Open Photos, find the Live Photo, then long-press it and choose \"Use as Wallpaper.\" It animates on long-press, not continuously — that's an iOS limit, not a bug.")
        }
        .alert("Couldn't Export", isPresented: exporter.errorMessageBinding) {
            Button("OK") {}
        } message: {
            Text(exporter.errorMessage ?? "Unknown error.")
        }
    }
}

private struct HUD: View {
    @Binding var sceneId: String
    @Binding var themeName: String
    let scenes: [String]
    let theme: Theme
    let isExporting: Bool
    let onExport: () -> Void

    private var themes: [String] { Theme.all.map(\.name) }
    private let color: Color

    init(sceneId: Binding<String>, themeName: Binding<String>, scenes: [String], theme: Theme, isExporting: Bool, onExport: @escaping () -> Void) {
        _sceneId = sceneId
        _themeName = themeName
        self.scenes = scenes
        self.theme = theme
        self.isExporting = isExporting
        self.onExport = onExport
        self.color = Color(theme.text)
    }

    var body: some View {
        HStack {
            Button {
                guard let idx = scenes.firstIndex(of: sceneId) else { return }
                sceneId = scenes[(idx + 1) % scenes.count]
            } label: {
                Text(sceneId.uppercased())
                    .font(.system(size: 11, design: .monospaced))
                    .kerning(2)
                    .foregroundStyle(color.opacity(0.7))
            }
            .buttonStyle(.plain)

            Spacer()

            HStack(spacing: 10) {
                Button {
                    guard let idx = themes.firstIndex(of: themeName) else { return }
                    themeName = themes[(idx + 1) % themes.count]
                } label: {
                    Text(themeName.uppercased())
                        .font(.system(size: 9, design: .monospaced))
                        .kerning(1.5)
                        .foregroundStyle(color.opacity(0.6))
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .overlay(RoundedRectangle(cornerRadius: 2).stroke(color.opacity(0.6), lineWidth: 1))
                }
                .buttonStyle(.plain)

                Button {
                    onExport()
                } label: {
                    if isExporting {
                        ProgressView()
                            .tint(color.opacity(0.6))
                            .scaleEffect(0.7)
                            .frame(width: 16, height: 16)
                    } else {
                        Image(systemName: "square.and.arrow.up")
                            .font(.system(size: 15))
                            .foregroundStyle(color.opacity(0.6))
                    }
                }
                .disabled(isExporting)

                NavigationLink {
                    SettingsView()
                } label: {
                    Text("⚙")
                        .font(.system(size: 16))
                        .foregroundStyle(color.opacity(0.6))
                }
            }
        }
    }
}
