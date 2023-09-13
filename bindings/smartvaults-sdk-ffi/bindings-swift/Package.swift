// swift-tools-version:5.5
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "smartvaults-sdk-swift",
    platforms: [
        .iOS(.v14),
    ],
    products: [
        .library(name: "SmartVaultsSDK", targets: ["smartvaults_sdkFFI", "SmartVaultsSDK"]),
    ],
    dependencies: [],
    targets: [
        .binaryTarget(name: "smartvaults_sdkFFI", path: "./smartvaults_sdkFFI.xcframework"),
        .target(name: "SmartVaultsSDK", dependencies: ["smartvaults_sdkFFI"]),
        .testTarget(name: "SmartVaultsSDKTests", dependencies: ["SmartVaultsSDK"]),
    ]
)
