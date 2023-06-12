// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "bindings-swift",
    platforms: [
        .iOS(.v14),
    ],
    products: [
        .library(name: "Coinstr", targets: ["coinstrFFI", "Coinstr"]),
    ],
    dependencies: [
    ],
    targets: [
        .binaryTarget(name: "coinstrFFI", path: "./coinstrFFI.xcframework"),
        .target(name: "Coinstr", dependencies: ["coinstrFFI"]),
        .testTarget(name: "CoinstrTests", dependencies: ["Coinstr"]),
    ]
)
