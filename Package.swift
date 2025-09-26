// swift-tools-version: 6.2

import PackageDescription
import CompilerPluginSupport

let package = Package(
    name: "dropbear",
    products: [
        .library(
            name: "dropbear",
            type: .dynamic,
            targets: ["dropbear"],
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/swiftlang/swift-syntax", from: "602.0.0")
    ],
    targets: [
        .macro(
            name: "dropbear_macro",
            dependencies: [
                .product(name: "SwiftSyntaxMacros", package: "swift-syntax"),
                .product(name: "SwiftCompilerPlugin", package: "swift-syntax")
            ]
        ),
        .target(
            name: "dropbear",
            dependencies: ["dropbear_macro"]
        ),
        .testTarget(
            name: "dropbearTests",
            dependencies: ["dropbear"]
        ),
    ]
)
