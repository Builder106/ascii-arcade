// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AsciiArcade",
    platforms: [
        .iOS(.v17),
    ],
    products: [
        .library(
            name: "AsciiArcade",
            targets: ["AsciiArcade"]
        ),
    ],
    targets: [
        .target(
            name: "AsciiArcade",
            dependencies: ["AaEngine"],
            path: "AsciiArcade",
            exclude: ["Assets.xcassets", "Info.plist"],
            resources: [
                .copy("Renderer/Shaders.metal"),
            ]
        ),
        .binaryTarget(
            name: "AaEngine",
            path: "Frameworks/AaEngine.xcframework"
        ),
    ]
)
