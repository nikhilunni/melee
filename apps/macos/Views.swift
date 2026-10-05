import SwiftUI

// The menus and overlays, in SwiftUI hosted by the AppKit window. They read
// the model and send intent; the core decides what is allowed. The design is
// docs/DESIGN.md; tokens and components are in Theme.swift, the backdrop in
// Backdrop.swift (an AppKit layer under this view).

struct RootView: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let motion = MotionPolicy(reduce: reduceMotion)
        ZStack {
            Group {
                if model.showsResults {
                    ResultsView(model: model)
                } else {
                    switch model.screen {
                    case .disc: DiscView(model: model)
                    case .characters: CharacterSelectView(model: model)
                    case .stages: StageSelectView(model: model)
                    case .loading: LoadingView(model: model)
                    case .match: MatchOverlay(model: model)
                    case .results: ResultsView(model: model)
                    }
                }
            }
            .id(model.showsResults ? 100 : Int(model.screen.rawValue))
            .transition(screenTransition(reduce: reduceMotion))

            if let notice = model.notice, model.screen != .disc {
                NoticeCard(notice: notice) { model.notice = nil }
                    .frame(maxWidth: 520)
                    .padding(.top, 52)
                    .frame(maxHeight: .infinity, alignment: .top)
                    .transition(reduceMotion ? .opacity : .move(edge: .top).combined(with: .opacity))
                    .zIndex(10)
            }
        }
        .animation(motion.spring, value: model.screen)
        .animation(motion.spring, value: model.showsResults)
        .animation(motion.spring, value: model.notice)
        .foregroundStyle(Palette.text)
        .preferredColorScheme(.dark)
    }

    /// Outgoing content slides 40 pt along the slant and fades; incoming
    /// comes from the other side. Reduce Motion: cross-fade only.
    private func screenTransition(reduce: Bool) -> AnyTransition {
        if reduce { return .opacity }
        let lean = 40 * Slant.lean
        return .asymmetric(insertion: .offset(x: 40, y: -lean).combined(with: .opacity),
                           removal: .offset(x: -40, y: lean).combined(with: .opacity))
    }
}

// MARK: Layout

/// Scale for a menu laid out at 1280x800, so the composition holds from
/// 1024x640 to a large full-screen window.
struct MenuMetrics {
    let size: CGSize
    var scale: CGFloat { min(max(min(size.width / 1280, size.height / 800), 0.8), 1.7) }
    func callAsFunction(_ value: CGFloat) -> CGFloat { (value * scale).rounded() }
}

/// Space for the transparent title bar's window buttons.
let titleBarInset: CGFloat = 28

/// Elements of a screen arrive in groups, 60 ms apart, sliding along the slant.
struct StaggerIn: ViewModifier {
    let index: Int
    @State private var shown = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .opacity(shown ? 1 : 0)
            .offset(x: shown || reduceMotion ? 0 : 24, y: shown || reduceMotion ? 0 : -24 * Slant.lean)
            .onAppear {
                let animation: Animation = reduceMotion ? .easeInOut(duration: 0.15)
                    : .spring(response: 0.35, dampingFraction: 0.8).delay(0.06 * Double(index))
                withAnimation(animation) { shown = true }
            }
    }
}

extension View {
    func staggerIn(_ index: Int) -> some View { modifier(StaggerIn(index: index)) }
}

// MARK: Notices

/// An error or notice: a danger-tinted glass card, the message, a copy
/// button for the details and one obvious action.
struct NoticeCard: View {
    let notice: Notice
    var dismiss: () -> Void
    @State private var copied = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 10) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundStyle(Palette.danger)
                Text(notice.title).font(.ui(16, .bold))
                Spacer(minLength: 0)
            }
            Text(notice.message)
                .font(.ui(14))
                .foregroundStyle(Palette.text.opacity(0.85))
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
                .lineLimit(8)
            HStack(spacing: 10) {
                // Copy is a small link; OK is the one obvious action.
                Button(copied ? "Copied" : "Copy details") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString("\(notice.title)\n\(notice.message)", forType: .string)
                    copied = true
                }
                .buttonStyle(.plain)
                .font(.ui(12, .semibold))
                .foregroundStyle(Palette.textDim)
                .underline()
                .onHover { inside in if inside { NSCursor.pointingHand.push() } else { NSCursor.pop() } }
                Spacer()
                Button("OK", action: dismiss)
                    .buttonStyle(SlantButtonStyle(kind: .accent, focused: true, size: 15, minWidth: 90))
            }
        }
        .padding(18)
        .glass(RoundedRectangle(cornerRadius: 14, style: .continuous), tint: Palette.danger, tintAmount: 0.22)
        .shadow(color: Palette.danger.opacity(0.25), radius: 24)
        .shadow(color: .black.opacity(0.4), radius: 12, y: 6)
        .accessibilityElement(children: .contain)
    }
}

// MARK: Shared art

/// A fighter's select portrait (136x188) from the disc. Where the disc has
/// none (Sheik) or the art is not read yet: the stock icon at an integer
/// scale, pixel-sharp, over our emblem.
struct FighterPortrait: View {
    @ObservedObject var model: AppModel
    let character: Int32
    let costume: Int
    var tint: Color = .white

    var body: some View {
        if let image = model.core.portrait(character: character, costume: costume) {
            ArtImage(image: image).aspectRatio(136 / 188, contentMode: .fit)
        } else {
            GeometryReader { geo in
                ZStack {
                    EmblemShape()
                        .fill(tint.opacity(0.14))
                        .frame(width: geo.size.width * 0.85, height: geo.size.width * 0.85)
                    if let stock = model.core.stockIcon(character: character, costume: costume) {
                        let side = max(24, (geo.size.width * 0.5 / 24).rounded(.down) * 24)
                        ArtImage(image: stock, pixel: true)
                            .frame(width: side, height: side)
                            .shadow(color: .black.opacity(0.5), radius: 4, y: 2)
                    }
                }
                .position(x: geo.size.width / 2, y: geo.size.height * 0.42)
            }
            .aspectRatio(136 / 188, contentMode: .fit)
        }
    }
}

/// Where a portrait's face is, computed from its alpha (DESIGN.md
/// refinement 10): the top of the figure, and the centre of the mass of
/// its top third. Retail's soft drop shadow (alpha under ~0.6) is ignored.
enum FaceFocus {
    struct Focus { var x: CGFloat; var top: CGFloat }
    private static var cache: [String: Focus] = [:]

    static func of(_ image: CGImage, key: String) -> Focus {
        if let known = cache[key] { return known }
        let focus = measure(image)
        cache[key] = focus
        return focus
    }

    private static func measure(_ image: CGImage) -> Focus {
        let width = image.width, height = image.height
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        let drawn = pixels.withUnsafeMutableBytes { buffer -> Bool in
            guard let context = CGContext(data: buffer.baseAddress, width: width, height: height,
                                          bitsPerComponent: 8, bytesPerRow: width * 4,
                                          space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
            else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard drawn else { return Focus(x: 0.5, top: 0) }
        // Rows from the top (the bitmap's first row is the image's top).
        func solid(_ x: Int, _ y: Int) -> Bool { pixels[(y * width + x) * 4 + 3] > 160 }
        let threshold = max(2, width / 40)
        var top = 0
        while top < height, (0..<width).filter({ solid($0, top) }).count < threshold { top += 1 }
        if top >= height { return Focus(x: 0.5, top: 0) }
        let third = top + (height - top) / 3
        var sum = 0, count = 0
        for y in top..<third { for x in 0..<width where solid(x, y) { sum += x; count += 1 } }
        let x = count > 0 ? CGFloat(sum) / CGFloat(count) + 0.5 : CGFloat(width) / 2
        return Focus(x: x / CGFloat(width), top: CGFloat(top) / CGFloat(height))
    }
}

/// A fighter's portrait cropped around the face to fill `size`; `span` is
/// how much of the portrait's width shows.
struct FaceCrop: View {
    @ObservedObject var model: AppModel
    let character: Int32
    var costume = 0
    let size: CGSize
    var span: CGFloat = 0.78

    var body: some View {
        if let portrait = model.core.portrait(character: character, costume: costume) {
            let scale = size.width / (136 * span)
            let image = CGSize(width: 136 * scale, height: 188 * scale)
            let focus = FaceFocus.of(portrait, key: "\(character)-\(costume)")
            // The figure's top just inside the frame, its upper mass centred;
            // the image always covers the frame.
            let x = min(max(size.width / 2 - focus.x * image.width, size.width - image.width), 0)
            let y = min(max(size.height * 0.05 - focus.top * image.height, size.height - image.height), 0)
            ArtImage(image: portrait)
                .frame(width: image.width, height: image.height)
                .offset(x: x + (image.width - size.width) / 2, y: y + (image.height - size.height) / 2)
                .frame(width: size.width, height: size.height)
                .clipped()
        } else if let stock = model.core.stockIcon(character: character, costume: costume) {
            let side = max(24, (min(size.width, size.height) * 0.9 / 24).rounded(.down) * 24)
            ArtImage(image: stock, pixel: true)
                .frame(width: side, height: side)
                .frame(width: size.width, height: size.height)
        } else {
            EmblemShape().fill(.white.opacity(0.12))
                .frame(width: size.height * 0.6, height: size.height * 0.6)
                .frame(width: size.width, height: size.height)
        }
    }
}

extension View {
    /// Fade the flat edges where retail's portrait texture crops a fighter
    /// (its right and bottom; DESIGN.md refinement 5).
    func fadedCutEdges(right: CGFloat = 0.09, bottom: CGFloat = 0.12) -> some View {
        mask {
            LinearGradient(stops: [.init(color: .white, location: 0), .init(color: .white, location: 1 - right),
                                   .init(color: .clear, location: 1)],
                           startPoint: .leading, endPoint: .trailing)
                .mask {
                    LinearGradient(stops: [.init(color: .white, location: 0),
                                           .init(color: .white, location: 1 - bottom),
                                           .init(color: .clear, location: 1)],
                                   startPoint: .top, endPoint: .bottom)
                }
        }
    }
}
