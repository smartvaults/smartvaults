// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "bindings-swift",
    platforms: [
        .iOS(.v14),
    ],
    products: [
        .library(name: "CoinstrSDK", targets: ["coinstr_sdkFFI", "CoinstrSDK"]),
    ],
    dependencies: [
    ],
    targets: [
        .binaryTarget(name: "coinstr_sdkFFI", path: "./coinstr_sdkFFI.xcframework"),
        .target(name: "CoinstrSDK", dependencies: ["coinstr_sdkFFI"]),
        .testTarget(name: "CoinstrSDKTests", dependencies: ["CoinstrSDK"]),
    ]
)
