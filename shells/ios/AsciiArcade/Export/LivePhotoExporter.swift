import SwiftUI
import Photos
import simd

// Drives a Live Photo export end to end: requests add-only Photos access,
// renders+encodes the clip on a background queue (LivePhotoRenderJob), then
// saves the photo/video pair as one asset. Published state drives SceneView's
// progress UI.
@MainActor
final class LivePhotoExporter: ObservableObject {
    @Published private(set) var isExporting = false
    @Published var errorMessage: String?
    @Published var didSucceed = false

    var errorMessageBinding: Binding<Bool> {
        Binding(get: { self.errorMessage != nil }, set: { if !$0 { self.errorMessage = nil } })
    }

    func export(sceneId: String, theme: Theme, pixelSize: CGSize, displayScale: CGFloat) {
        guard !isExporting, pixelSize.width > 0, pixelSize.height > 0 else { return }
        isExporting = true
        errorMessage = nil
        didSucceed = false

        let bgColor = SIMD4<Float>(0, 0, 0, 1)
        let fgColor = rgba(theme.text)

        Task {
            do {
                guard await Self.requestAddOnlyAuthorization() else {
                    throw LivePhotoExportError.photoLibraryDenied
                }
                let (photoURL, videoURL) = try await Self.renderAndEncode(
                    sceneId: sceneId, pixelSize: pixelSize, displayScale: displayScale,
                    bgColor: bgColor, fgColor: fgColor
                )
                defer {
                    try? FileManager.default.removeItem(at: photoURL)
                    try? FileManager.default.removeItem(at: videoURL)
                }
                try await Self.saveLivePhoto(photoURL: photoURL, videoURL: videoURL)
                isExporting = false
                didSucceed = true
            } catch {
                isExporting = false
                errorMessage = (error as? LocalizedError)?.errorDescription ?? "\(error)"
            }
        }
    }

    // nonisolated: PHPhotoLibrary always runs performChanges' change-block (and,
    // in practice, these completion handlers) on its own private queue, never
    // the main actor. Leaving these MainActor-isolated makes Swift's closure
    // literals inherit @MainActor, and the runtime traps (SIGTRAP, dispatch
    // queue assertion) the moment Photos actually invokes them off-main.
    private nonisolated static func requestAddOnlyAuthorization() async -> Bool {
        let status = await withCheckedContinuation { continuation in
            PHPhotoLibrary.requestAuthorization(for: .addOnly) { status in
                continuation.resume(returning: status)
            }
        }
        return status == .authorized || status == .limited
    }

    private nonisolated static func renderAndEncode(
        sceneId: String, pixelSize: CGSize, displayScale: CGFloat,
        bgColor: SIMD4<Float>, fgColor: SIMD4<Float>
    ) async throws -> (photoURL: URL, videoURL: URL) {
        try await withCheckedThrowingContinuation { continuation in
            DispatchQueue(label: "com.builder106.asciiarcade.livephoto-export", qos: .userInitiated).async {
                do {
                    let result = try LivePhotoRenderJob.run(
                        sceneId: sceneId, pixelSize: pixelSize, displayScale: displayScale,
                        bgColor: bgColor, fgColor: fgColor
                    )
                    continuation.resume(returning: result)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    private nonisolated static func saveLivePhoto(photoURL: URL, videoURL: URL) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            PHPhotoLibrary.shared().performChanges({
                let creationRequest = PHAssetCreationRequest.forAsset()
                creationRequest.addResource(with: .photo, fileURL: photoURL, options: nil)
                let videoOptions = PHAssetResourceCreationOptions()
                creationRequest.addResource(with: .pairedVideo, fileURL: videoURL, options: videoOptions)
            }, completionHandler: { success, error in
                if success {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? LivePhotoExportError.saveFailed(NSError(domain: "LivePhotoExporter", code: -1)))
                }
            })
        }
    }
}
