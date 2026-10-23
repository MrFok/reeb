import AppKit
import Foundation

// Package the supplied artwork with the standard macOS icon inset. Do not change
// the source artwork or apply this padding to the in-app or Windows assets.
let arguments = CommandLine.arguments
precondition(arguments.count == 3, "Usage: macos-icon.swift source.png destination.iconset")
guard let source = NSImage(contentsOfFile: arguments[1]) else {
    fatalError("Cannot load the supplied reeb artwork")
}
let directory = URL(fileURLWithPath: arguments[2], isDirectory: true)
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels,
            pixelsHigh: pixels, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
            isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
            let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
            fatalError("Cannot create the icon canvas")
        }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = context
        context.cgContext.clear(CGRect(x: 0, y: 0, width: pixels, height: pixels))
        context.imageInterpolation = .high
        let side = CGFloat(pixels) * 824.0 / 1024.0
        let inset = (CGFloat(pixels) - side) / 2
        source.draw(in: NSRect(x: inset, y: inset, width: side, height: side),
            from: .zero, operation: .copy, fraction: 1)
        NSGraphicsContext.restoreGraphicsState()
        guard let data = bitmap.representation(using: .png, properties: [:]) else {
            fatalError("Cannot encode the icon canvas")
        }
        let suffix = scale == 2 ? "@2x" : ""
        try data.write(to: directory.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
        // Verify the exported canvas has transparent margins on all four sides.
        let corner = bitmap.colorAt(x: 0, y: 0)!.alphaComponent
        precondition(corner == 0, "Icon must preserve transparent outer margins")
    }
}
