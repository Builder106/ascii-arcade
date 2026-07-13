import AVFoundation
import CoreMedia
import ImageIO

// A Live Photo is a JPEG + a paired ~3s .mov joined by a shared identifier:
// the JPEG carries it in an Apple maker-note key, the video carries it as a
// QuickTime content-identifier metadata item plus a dedicated "still image
// time" metadata track marking which frame is the photo moment. This is
// undocumented-but-stable surface — every third-party Live-Photo-from-video
// tool (and Apple's own Camera app) uses this exact recipe.
enum LivePhotoMetadata {

    // Embeds the asset identifier into JPEG data via the Apple maker-note "17" key.
    static func taggedJPEGData(from imageData: Data, assetIdentifier: String) -> Data? {
        guard let source = CGImageSourceCreateWithData(imageData as CFData, nil),
              let type = CGImageSourceGetType(source) else { return nil }

        let output = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(output, type, 1, nil) else { return nil }

        let makerNote: [String: Any] = ["17": assetIdentifier]
        let metadata: [String: Any] = [kCGImagePropertyMakerAppleDictionary as String: makerNote]
        CGImageDestinationAddImageFromSource(destination, source, 0, metadata as CFDictionary)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return output as Data
    }

    // Tags the whole video asset with the shared content identifier.
    // Attach via `assetWriter.metadata = [contentIdentifierItem(...)]`.
    static func contentIdentifierItem(assetIdentifier: String) -> AVMetadataItem {
        let item = AVMutableMetadataItem()
        item.keySpace = .quickTimeMetadata
        item.key = AVMetadataKey.quickTimeMetadataKeyContentIdentifier as NSString
        item.value = assetIdentifier as NSString
        item.dataType = "com.apple.metadata.datatype.UTF-8"
        return item
    }

    // A dedicated metadata-track input marking which frame is the "key photo"
    // moment. Must be added to the AVAssetWriter alongside the video track.
    static func makeStillImageTimeAdaptor() -> AVAssetWriterInputMetadataAdaptor {
        let spec: [NSString: Any] = [
            kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier as NSString:
                "mdta/com.apple.quicktime.still-image-time",
            kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType as NSString:
                kCMMetadataBaseDataType_SInt8
        ]
        var formatDescription: CMFormatDescription?
        CMMetadataFormatDescriptionCreateWithMetadataSpecifications(
            allocator: kCFAllocatorDefault,
            metadataType: kCMMetadataFormatType_Boxed,
            metadataSpecifications: [spec] as CFArray,
            formatDescriptionOut: &formatDescription
        )
        let input = AVAssetWriterInput(mediaType: .metadata, outputSettings: nil, sourceFormatHint: formatDescription)
        input.expectsMediaDataInRealTime = false
        return AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
    }

    // Appends the single "still image time" marker at the given presentation time.
    static func markStillImageTime(_ adaptor: AVAssetWriterInputMetadataAdaptor, at time: CMTime, frameDuration: CMTime) {
        let item = AVMutableMetadataItem()
        item.key = "com.apple.quicktime.still-image-time" as NSString
        item.keySpace = AVMetadataKeySpace(rawValue: "mdta")
        item.value = 0 as NSNumber
        item.dataType = kCMMetadataBaseDataType_SInt8 as String

        let group = AVTimedMetadataGroup(items: [item], timeRange: CMTimeRange(start: time, duration: frameDuration))
        adaptor.append(group)
    }
}
