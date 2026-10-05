import AppKit
import QuartzCore
import SwiftUI

// The living backdrop behind every menu screen (DESIGN.md "Backdrop"):
// radial indigo and violet light over deep blue, two drifting star layers,
// our emblem very large and slowly turning, a light sweep along the slant
// and a faint grain. All of it is Core Animation: the layers are built once
// and the render server animates them, so the app does no per-frame work.

/// Our own circle-and-cross emblem (original vector art): a disc cut by one
/// vertical and one horizontal gap, both off centre.
enum Emblem {
    static func path(in rect: CGRect) -> CGPath {
        let side = min(rect.width, rect.height)
        let r = side / 2
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let disc = CGPath(ellipseIn: CGRect(x: center.x - r, y: center.y - r, width: side, height: side), transform: nil)
        let gap = r * 0.11
        let cuts = CGMutablePath()
        // In a flipped-y (top-down) space the horizontal gap sits below centre.
        cuts.addRect(CGRect(x: center.x + r * 0.16 - gap / 2, y: center.y - r - 1, width: gap, height: side + 2))
        cuts.addRect(CGRect(x: center.x - r - 1, y: center.y + r * 0.22 - gap / 2, width: side + 2, height: gap))
        return disc.subtracting(cuts)
    }
}

struct EmblemShape: Shape {
    func path(in rect: CGRect) -> Path { Path(Emblem.path(in: rect)) }
}

final class BackdropView: NSView {
    private let base = CALayer()
    private let indigo = CAGradientLayer()
    private let violet = CAGradientLayer()
    private let glow = CAGradientLayer()
    private let farStars = CAReplicatorLayer()
    private let nearStars = CAReplicatorLayer()
    private let emblem = CAShapeLayer()
    private let sweep = CAGradientLayer()
    private let grain = CAReplicatorLayer()
    private let vignette = CAGradientLayer()
    private var reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion

    private static let starTile: CGFloat = 320
    private static let grainTile: CGFloat = 160

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        let root = CALayer()
        layer = root
        root.masksToBounds = true
        base.backgroundColor = NSColor(srgbRed: 5 / 255, green: 6 / 255, blue: 15 / 255, alpha: 1).cgColor

        func radial(_ layer: CAGradientLayer, _ color: NSColor, center: CGPoint, radius: CGSize) {
            layer.type = .radial
            layer.colors = [color.cgColor, color.withAlphaComponent(color.alphaComponent * 0.35).cgColor,
                            color.withAlphaComponent(0).cgColor]
            layer.locations = [0, 0.45, 1]
            layer.startPoint = center
            layer.endPoint = CGPoint(x: center.x + radius.width, y: center.y + radius.height)
        }
        // Layer space is bottom-up: y 1 is the top of the window.
        radial(indigo, NSColor(srgbRed: 16 / 255, green: 20 / 255, blue: 90 / 255, alpha: 1),
               center: CGPoint(x: 0.28, y: 0.45), radius: CGSize(width: 0.75, height: 0.95))
        radial(violet, NSColor(srgbRed: 58 / 255, green: 26 / 255, blue: 122 / 255, alpha: 0.9),
               center: CGPoint(x: 0.92, y: 1.0), radius: CGSize(width: 0.6, height: 0.85))
        radial(glow, NSColor(srgbRed: 70 / 255, green: 110 / 255, blue: 255 / 255, alpha: 0.10),
               center: CGPoint(x: 0.5, y: 0.0), radius: CGSize(width: 0.6, height: 0.5))

        let scale = NSScreen.main?.backingScaleFactor ?? 2
        for (layer, seed, count, size) in [(farStars, 7, 70, 1.0), (nearStars, 19, 26, 1.7)] {
            let tile = CALayer()
            tile.contents = Self.starImage(seed: UInt64(seed), count: count, dot: CGFloat(size), scale: scale)
            tile.contentsScale = scale
            tile.frame = CGRect(x: 0, y: 0, width: Self.starTile, height: Self.starTile)
            let row = CAReplicatorLayer()
            row.addSublayer(tile)
            row.instanceTransform = CATransform3DMakeTranslation(Self.starTile, 0, 0)
            layer.addSublayer(row)
            layer.instanceTransform = CATransform3DMakeTranslation(0, Self.starTile, 0)
        }
        farStars.opacity = 0.55
        nearStars.opacity = 0.8

        emblem.fillColor = NSColor.white.cgColor
        emblem.opacity = 0.04

        sweep.colors = [NSColor.white.withAlphaComponent(0).cgColor, NSColor.white.withAlphaComponent(0.05).cgColor,
                        NSColor.white.withAlphaComponent(0).cgColor]
        sweep.startPoint = CGPoint(x: 0, y: 0.5)
        sweep.endPoint = CGPoint(x: 1, y: 0.5)
        sweep.opacity = 0

        let grainTile = CALayer()
        grainTile.contents = Self.grainImage(size: Int(Self.grainTile * scale))
        grainTile.contentsScale = scale
        grainTile.frame = CGRect(x: 0, y: 0, width: Self.grainTile, height: Self.grainTile)
        let grainRow = CAReplicatorLayer()
        grainRow.addSublayer(grainTile)
        grainRow.instanceTransform = CATransform3DMakeTranslation(Self.grainTile, 0, 0)
        grain.addSublayer(grainRow)
        grain.instanceTransform = CATransform3DMakeTranslation(0, Self.grainTile, 0)
        grain.opacity = 0.025

        vignette.type = .radial
        vignette.colors = [NSColor.black.withAlphaComponent(0).cgColor, NSColor.black.withAlphaComponent(0).cgColor,
                           NSColor.black.withAlphaComponent(0.55).cgColor]
        vignette.locations = [0, 0.55, 1]
        vignette.startPoint = CGPoint(x: 0.5, y: 0.5)
        vignette.endPoint = CGPoint(x: 1.15, y: 1.15)

        for layer in [base, indigo, violet, glow, farStars, nearStars, emblem, sweep, grain, vignette] as [CALayer] {
            root.addSublayer(layer)
        }
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(accessibilityChanged),
            name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil)
    }
    required init?(coder: NSCoder) { fatalError("not used") }

    override var isFlipped: Bool { false }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let bounds = self.bounds
        for layer in [base, indigo, violet, glow, sweep, vignette] as [CALayer] { layer.frame = bounds }
        let tile = Self.starTile
        for layer in [farStars, nearStars] {
            layer.frame = CGRect(x: -tile, y: -tile, width: bounds.width + 2 * tile, height: bounds.height + 2 * tile)
            layer.instanceCount = Int((bounds.height / tile).rounded(.up)) + 3
            (layer.sublayers?.first as? CAReplicatorLayer)?.instanceCount = Int((bounds.width / tile).rounded(.up)) + 3
        }
        grain.frame = bounds
        grain.instanceCount = Int((bounds.height / Self.grainTile).rounded(.up)) + 1
        (grain.sublayers?.first as? CAReplicatorLayer)?.instanceCount =
            Int((bounds.width / Self.grainTile).rounded(.up)) + 1
        // The emblem: 120% of the window height, centred right of middle.
        let side = bounds.height * 1.2
        emblem.bounds = CGRect(x: 0, y: 0, width: side, height: side)
        emblem.position = CGPoint(x: bounds.width * 0.62, y: bounds.height * 0.5)
        var flip = CGAffineTransform(scaleX: 1, y: -1).translatedBy(x: 0, y: -side)
        emblem.path = Emblem.path(in: emblem.bounds).copy(using: &flip)
        sweep.bounds = CGRect(x: 0, y: 0, width: max(bounds.width * 0.35, 280), height: bounds.height * 1.6)
        sweep.position = CGPoint(x: bounds.midX, y: bounds.midY)
        sweep.setAffineTransform(CGAffineTransform(rotationAngle: -12 * .pi / 180))
        CATransaction.commit()
        if animationsNeedRefresh { startAnimations() }
    }

    private var animationsNeedRefresh = true
    private var animatedWidth: CGFloat = 0

    override func viewDidHide() { super.viewDidHide(); stopAnimations() }
    override func viewDidUnhide() { super.viewDidUnhide(); animationsNeedRefresh = true; needsLayout = true }
    override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); animationsNeedRefresh = true; needsLayout = true }

    @objc private func accessibilityChanged() {
        reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        animationsNeedRefresh = true
        needsLayout = true
    }

    private func stopAnimations() {
        for layer in [farStars, nearStars, emblem, sweep] as [CALayer] { layer.removeAllAnimations() }
        animationsNeedRefresh = true
    }

    private func startAnimations() {
        guard window != nil, !isHiddenOrHasHiddenAncestor, bounds.width > 0 else { return }
        animationsNeedRefresh = false
        for layer in [farStars, nearStars, emblem, sweep] as [CALayer] { layer.removeAllAnimations() }
        if reduceMotion { return }
        let slow = CAFrameRateRange(minimum: 15, maximum: 30, preferred: 30)
        // Stars drift diagonally (down-left, along the slant) one tile per cycle.
        let tile = Self.starTile
        for (layer, seconds) in [(farStars, 90.0), (nearStars, 60.0)] {
            let drift = CABasicAnimation(keyPath: "transform.translation")
            drift.fromValue = NSValue(size: .zero)
            drift.toValue = NSValue(size: CGSize(width: -tile, height: -tile))
            drift.duration = seconds
            drift.repeatCount = .infinity
            drift.preferredFrameRateRange = slow
            layer.add(drift, forKey: "drift")
        }
        let spin = CABasicAnimation(keyPath: "transform.rotation.z")
        spin.fromValue = 0
        spin.toValue = -2 * Double.pi
        spin.duration = 720 // 0.5 degrees per second
        spin.repeatCount = .infinity
        spin.preferredFrameRateRange = CAFrameRateRange(minimum: 8, maximum: 20, preferred: 15)
        emblem.add(spin, forKey: "spin")
        // The light sweep: 1.6 s across the window, then rest, every 8 s.
        let width = bounds.width + sweep.bounds.width * 2
        let move = CABasicAnimation(keyPath: "position.x")
        move.fromValue = -sweep.bounds.width
        move.toValue = width
        move.duration = 1.6
        move.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        let fade = CAKeyframeAnimation(keyPath: "opacity")
        fade.values = [0, 1, 1, 0]
        fade.keyTimes = [0, 0.2, 0.8, 1]
        fade.duration = 1.6
        let group = CAAnimationGroup()
        group.animations = [move, fade]
        group.duration = 8
        group.repeatCount = .infinity
        group.beginTime = CACurrentMediaTime() + 1.5
        group.preferredFrameRateRange = slow
        sweep.add(group, forKey: "sweep")
        animatedWidth = bounds.width
    }

    // MARK: Textures (generated, original)

    private static func starImage(seed: UInt64, count: Int, dot: CGFloat, scale: CGFloat) -> CGImage? {
        let px = Int(starTile * scale)
        guard let context = CGContext(data: nil, width: px, height: px, bitsPerComponent: 8, bytesPerRow: 0,
                                      space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        context.scaleBy(x: scale, y: scale)
        var state = seed &* 0x9E37_79B9_7F4A_7C15
        func next() -> CGFloat {
            state = state &* 6364136223846793005 &+ 1442695040888963407
            return CGFloat(state >> 33) / CGFloat(1 << 31)
        }
        for _ in 0..<count {
            let x = next() * starTile, y = next() * starTile
            let size = dot * (0.5 + next())
            let alpha = 0.25 + next() * 0.75
            let tint = next()
            let color = tint < 0.2 ? NSColor(srgbRed: 1, green: 0.85, blue: 0.7, alpha: alpha)
                : tint < 0.5 ? NSColor(srgbRed: 0.7, green: 0.8, blue: 1, alpha: alpha)
                : NSColor(white: 1, alpha: alpha)
            context.setFillColor(color.cgColor)
            // Draw wrapped copies so the tile repeats seamlessly.
            for dx in [-starTile, 0, starTile] {
                for dy in [-starTile, 0, starTile] {
                    context.fillEllipse(in: CGRect(x: x + dx - size / 2, y: y + dy - size / 2, width: size, height: size))
                }
            }
        }
        return context.makeImage()
    }

    private static func grainImage(size: Int) -> CGImage? {
        var pixels = [UInt8](repeating: 0, count: size * size * 4)
        var state: UInt32 = 0x1234_5678
        for index in 0..<(size * size) {
            state ^= state << 13; state ^= state >> 17; state ^= state << 5
            let value = UInt8(truncatingIfNeeded: state)
            pixels[index * 4] = value
            pixels[index * 4 + 1] = value
            pixels[index * 4 + 2] = value
            pixels[index * 4 + 3] = 255
        }
        guard let provider = CGDataProvider(data: Data(pixels) as CFData) else { return nil }
        return CGImage(width: size, height: size, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: size * 4,
                       space: CGColorSpace(name: CGColorSpace.sRGB)!,
                       bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    }
}
