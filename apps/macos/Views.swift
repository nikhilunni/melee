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
                Spacer()
                Button(copied ? "Copied" : "Copy Details") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString("\(notice.title)\n\(notice.message)", forType: .string)
                    copied = true
                }
                .buttonStyle(SlantButtonStyle(kind: .glass, size: 14))
                Button("OK", action: dismiss)
                    .buttonStyle(SlantButtonStyle(kind: .danger, focused: true, size: 14, minWidth: 80))
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

/// Our own fighter silhouette (original art): head and shoulders.
struct Silhouette: View {
    var color: Color = .white
    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width, h = geo.size.height
            ZStack {
                Ellipse()
                    .frame(width: w * 0.36, height: h * 0.30)
                    .position(x: w * 0.5, y: h * 0.27)
                UnevenRoundedRectangle(topLeadingRadius: w * 0.3, bottomLeadingRadius: 0, bottomTrailingRadius: 0,
                                       topTrailingRadius: w * 0.3, style: .continuous)
                    .frame(width: w * 0.86, height: h * 0.42)
                    .position(x: w * 0.5, y: h * 0.79)
            }
            .foregroundStyle(color)
        }
        .aspectRatio(136 / 188, contentMode: .fit)
    }
}

/// A fighter's select portrait (136x188) from the disc. Where the disc has
/// none (Sheik) or the art is not read yet: our silhouette, with the stock
/// icon over it when there is one.
struct FighterPortrait: View {
    @ObservedObject var model: AppModel
    let character: Int32
    let costume: Int
    var tint: Color = .white

    var body: some View {
        if let image = model.core.portrait(character: character, costume: costume) {
            ArtImage(image: image).aspectRatio(136 / 188, contentMode: .fit)
        } else {
            ZStack {
                Silhouette(color: tint.opacity(0.22))
                if let stock = model.core.stockIcon(character: character, costume: costume) {
                    GeometryReader { geo in
                        // Integer multiples of 24 keep the icon's pixels crisp.
                        let side = max(24, (geo.size.width * 0.42 / 24).rounded(.down) * 24)
                        ArtImage(image: stock, pixel: true)
                            .frame(width: side, height: side)
                            .shadow(color: .black.opacity(0.5), radius: 4, y: 2)
                            .position(x: geo.size.width / 2, y: geo.size.height * 0.27)
                    }
                }
            }
            .aspectRatio(136 / 188, contentMode: .fit)
        }
    }
}

/// Where each fighter's face sits in their 136x188 select portrait, as
/// fractions of its width and height (our own measurements; the grid and
/// HUD crop around it).
enum FaceFocus {
    private static let table: [String: CGPoint] = [
        "DrMario": CGPoint(x: 0.62, y: 0.25), "Mario": CGPoint(x: 0.58, y: 0.24),
        "Luigi": CGPoint(x: 0.6, y: 0.21), "Bowser": CGPoint(x: 0.42, y: 0.3),
        "Peach": CGPoint(x: 0.55, y: 0.21), "Yoshi": CGPoint(x: 0.42, y: 0.18),
        "DonkeyKong": CGPoint(x: 0.72, y: 0.36), "CaptainFalcon": CGPoint(x: 0.66, y: 0.17),
        "Ganondorf": CGPoint(x: 0.62, y: 0.17), "Falco": CGPoint(x: 0.6, y: 0.27),
        "Fox": CGPoint(x: 0.58, y: 0.25), "Ness": CGPoint(x: 0.5, y: 0.22),
        "IceClimbers": CGPoint(x: 0.5, y: 0.27), "Samus": CGPoint(x: 0.62, y: 0.17),
        "Zelda": CGPoint(x: 0.58, y: 0.2), "Link": CGPoint(x: 0.62, y: 0.17),
        "YoungLink": CGPoint(x: 0.56, y: 0.2), "Pichu": CGPoint(x: 0.42, y: 0.26),
        "Pikachu": CGPoint(x: 0.5, y: 0.24), "Jigglypuff": CGPoint(x: 0.5, y: 0.32),
        "Mewtwo": CGPoint(x: 0.56, y: 0.22), "GameAndWatch": CGPoint(x: 0.42, y: 0.22),
        "Marth": CGPoint(x: 0.56, y: 0.2), "Roy": CGPoint(x: 0.52, y: 0.18),
    ]
    static func of(_ key: String) -> CGPoint { table[key] ?? CGPoint(x: 0.55, y: 0.22) }
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
            let focus = FaceFocus.of(model.character(character)?.key ?? "")
            // Centre the face, then keep the image covering the frame.
            let x = min(max(size.width / 2 - focus.x * image.width, size.width - image.width), 0)
            let y = min(max(size.height * 0.46 - focus.y * image.height, size.height - image.height), 0)
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
            Silhouette(color: .white.opacity(0.15)).padding(size.height * 0.1)
                .frame(width: size.width, height: size.height)
        }
    }
}

extension View {
    /// Soften the hard edges where retail's portrait texture crops a fighter.
    func featheredEdges(horizontal: CGFloat = 0.1, top: CGFloat = 0.04, bottom: CGFloat = 0.14) -> some View {
        mask {
            LinearGradient(stops: [.init(color: .clear, location: 0), .init(color: .white, location: horizontal),
                                   .init(color: .white, location: 1 - horizontal), .init(color: .clear, location: 1)],
                           startPoint: .leading, endPoint: .trailing)
                .mask {
                    LinearGradient(stops: [.init(color: .clear, location: 0), .init(color: .white, location: top),
                                           .init(color: .white, location: 1 - bottom),
                                           .init(color: .clear, location: 1)],
                                   startPoint: .top, endPoint: .bottom)
                }
        }
    }
}
