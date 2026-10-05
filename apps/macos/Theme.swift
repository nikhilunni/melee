import AppKit
import CoreText
import SwiftUI

// The menu design system (docs/DESIGN.md): colour, type, the slanted shape,
// glass, glows, buttons and motion. Every screen draws with these.

extension Color {
    init(hex: UInt32, opacity: Double = 1) {
        self.init(.sRGB, red: Double(hex >> 16 & 0xff) / 255, green: Double(hex >> 8 & 0xff) / 255,
                  blue: Double(hex & 0xff) / 255, opacity: opacity)
    }
}

enum Palette {
    static let deep = Color(hex: 0x05060f)
    static let indigo = Color(hex: 0x10145a)
    static let violet = Color(hex: 0x3a1a7a)
    static let glass = Color(.sRGB, red: 18 / 255, green: 22 / 255, blue: 48 / 255, opacity: 0.55)
    static let text = Color(hex: 0xf4f6ff)
    static let textDim = Color(.sRGB, red: 226 / 255, green: 232 / 255, blue: 1, opacity: 0.62)
    static let players = [Color(hex: 0xff3b3b), Color(hex: 0x3b7bff), Color(hex: 0xffc93b), Color(hex: 0x2fd36a)]
    static let cpu = Color(hex: 0x9aa3b5)
    static let accentStart = Color(hex: 0xffb21e)
    static let accentEnd = Color(hex: 0xff5e1e)
    static let accent = LinearGradient(colors: [accentStart, accentEnd], startPoint: .leading, endPoint: .trailing)
    static let danger = Color(hex: 0xff4d6d)

    static func port(_ index: Int) -> Color { players[min(max(index, 0), players.count - 1)] }
}

// MARK: Type

enum Fonts {
    /// Register the bundled OFL fonts (Contents/Resources/Fonts) for this process.
    static func register() {
        guard let folder = Bundle.main.resourceURL?.appendingPathComponent("Fonts"),
              let files = try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)
        else { return }
        for url in files where url.pathExtension == "ttf" {
            CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
        }
    }
}

extension Font {
    /// Barlow Condensed Black Italic: titles and big names.
    static func display(_ size: CGFloat) -> Font { .custom("BarlowCondensed-BlackItalic", fixedSize: size) }
    /// Barlow Condensed ExtraBold Italic: smaller display text (buttons, tiles).
    static func displayBold(_ size: CGFloat) -> Font { .custom("BarlowCondensed-ExtraBoldItalic", fixedSize: size) }
    static func displaySemi(_ size: CGFloat) -> Font { .custom("BarlowCondensed-SemiBoldItalic", fixedSize: size) }
    /// Barlow UI text.
    static func ui(_ size: CGFloat, _ weight: Font.Weight = .medium) -> Font {
        let name = switch weight {
        case .bold, .heavy, .black: "Barlow-Bold"
        case .semibold: "Barlow-SemiBold"
        default: "Barlow-Medium"
        }
        return .custom(name, fixedSize: size)
    }
}

extension Text {
    /// Display type: all caps, +2% tracking.
    func displayStyle(_ size: CGFloat, bold: Bool = false) -> some View {
        self.font(bold ? .displayBold(size) : .display(size)).tracking(size * 0.02).textCase(.uppercase)
    }
}

// MARK: Shape

enum Slant {
    /// tan(12 degrees): the horizontal lean per point of height.
    static let lean: CGFloat = 0.2126
}

/// A parallelogram leaning like italic type (DESIGN.md: skewed -12 degrees),
/// with rounded corners. `lean` scales the slant (1 is the full 12 degrees);
/// the leaning shape stays inside the rect.
struct Slanted: Shape, InsettableShape {
    var radius: CGFloat = 10
    var lean: CGFloat = 1
    var maxShift: CGFloat = .infinity
    var inset: CGFloat = 0

    func shift(height: CGFloat) -> CGFloat { min(height * Slant.lean * lean, maxShift) }

    func path(in rect: CGRect) -> Path {
        let rect = rect.insetBy(dx: inset, dy: inset)
        let shift = shift(height: rect.height)
        let tan = rect.height > 0 ? shift / rect.height : 0
        let base = CGRect(x: rect.minX + shift / 2, y: rect.minY, width: max(rect.width - shift, 0), height: rect.height)
        let r = min(radius, base.width / 2, base.height / 2)
        let path = Path(roundedRect: base, cornerRadius: r, style: .continuous)
        // x' = x - tan * (y - midY): the top leans right.
        let shear = CGAffineTransform(a: 1, b: 0, c: -tan, d: 1, tx: tan * rect.midY, ty: 0)
        return path.applying(shear)
    }

    func inset(by amount: CGFloat) -> Slanted {
        var copy = self
        copy.inset += amount
        return copy
    }
}

// MARK: Accessibility

struct MotionPolicy {
    var reduce: Bool
    /// The design spring, or a short cross-fade with Reduce Motion.
    var spring: Animation { reduce ? .easeInOut(duration: 0.15) : .spring(response: 0.35, dampingFraction: 0.8) }
    var fade: Animation { .easeInOut(duration: 0.15) }
}

extension View {
    /// Glass panel: blurred material tinted with `glass`, a 1 px inner edge
    /// and a brighter top highlight. Increase Contrast strengthens the edge.
    func glass<S: InsettableShape>(_ shape: S, tint: Color? = nil, tintAmount: Double = 0.16,
                                   edge: Double = 1) -> some View {
        modifier(GlassModifier(shape: shape, tint: tint, tintAmount: tintAmount, edge: edge))
    }

    /// The soft port-coloured glow: the same hue at 45%, 24-40 px blur.
    func portGlow(_ color: Color, radius: CGFloat = 28, on: Bool = true) -> some View {
        shadow(color: on ? color.opacity(0.45) : .clear, radius: radius / 2)
    }
}

struct GlassModifier<S: InsettableShape>: ViewModifier {
    let shape: S
    let tint: Color?
    let tintAmount: Double
    let edge: Double
    @Environment(\.colorSchemeContrast) private var contrast

    func body(content: Content) -> some View {
        let strong = contrast == .increased
        content
            .background {
                ZStack {
                    shape.fill(.ultraThinMaterial)
                    shape.fill(Palette.glass)
                    if let tint {
                        shape.fill(LinearGradient(colors: [tint.opacity(tintAmount), tint.opacity(tintAmount * 0.25)],
                                                  startPoint: .top, endPoint: .bottom))
                    }
                }
            }
            .overlay {
                shape.strokeBorder(Color.white.opacity((strong ? 0.38 : 0.10) * edge), lineWidth: strong ? 1.5 : 1)
            }
            .overlay {
                // The top highlight: brightest along the upper edge.
                shape.strokeBorder(LinearGradient(colors: [.white.opacity((strong ? 0.5 : 0.22) * edge), .clear],
                                                  startPoint: .top, endPoint: UnitPoint(x: 0.5, y: 0.35)),
                                   lineWidth: 1)
            }
    }
}

// MARK: Buttons

/// Slanted button. `.accent` is the primary action on the accent gradient;
/// `.glass` is secondary. `focused` draws the keyboard focus ring.
struct SlantButtonStyle: ButtonStyle {
    enum Kind { case accent, glass, danger }
    var kind: Kind = .glass
    var focused = false
    var size: CGFloat = 20
    var minWidth: CGFloat = 0
    @Environment(\.isEnabled) private var enabled
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        SlantButton(configuration: configuration, kind: kind, focused: focused, size: size, minWidth: minWidth,
             enabled: enabled, reduceMotion: reduceMotion)
    }

    struct SlantButton: View {
        let configuration: Configuration
        let kind: Kind
        let focused: Bool
        let size: CGFloat
        let minWidth: CGFloat
        let enabled: Bool
        let reduceMotion: Bool
        @State private var hover = false

        var body: some View {
            let shape = Slanted(radius: 8)
            let lit = hover || focused
            configuration.label
                .font(.displayBold(size))
                .tracking(size * 0.04)
                .textCase(.uppercase)
                .foregroundStyle(kind == .accent ? Color(hex: 0x1a0c00) : Palette.text)
                .padding(.horizontal, size * 1.3)
                .frame(minWidth: minWidth, minHeight: size * 2.1)
                .background {
                    switch kind {
                    case .accent:
                        ZStack {
                            shape.fill(Palette.accent)
                            shape.fill(LinearGradient(colors: [.white.opacity(0.35), .clear],
                                                      startPoint: .top, endPoint: .center))
                        }
                        .shadow(color: Palette.accentEnd.opacity(lit ? 0.6 : 0.35), radius: lit ? 18 : 10)
                    case .glass:
                        shape.fill(Color.white.opacity(lit ? 0.14 : 0.07))
                            .background(shape.fill(.ultraThinMaterial))
                    case .danger:
                        shape.fill(Palette.danger.opacity(lit ? 0.4 : 0.25))
                    }
                }
                .overlay {
                    shape.strokeBorder(Color.white.opacity(kind == .accent ? 0.35 : (lit ? 0.35 : 0.14)), lineWidth: 1)
                }
                .overlay {
                    if focused {
                        shape.inset(by: -4).strokeBorder(Color.white.opacity(0.9), lineWidth: 2)
                    }
                }
                .scaleEffect(configuration.isPressed ? 0.97 : (lit && !reduceMotion ? 1.03 : 1))
                .opacity(enabled ? 1 : 0.4)
                .animation(reduceMotion ? nil : .spring(response: 0.25, dampingFraction: 0.7), value: lit)
                .animation(reduceMotion ? nil : .spring(response: 0.2, dampingFraction: 0.7),
                           value: configuration.isPressed)
                .onHover { hover = $0 }
                .contentShape(shape)
        }
    }
}

/// A small key cap for control legends.
struct KeyCap: View {
    let label: String
    var size: CGFloat = 11
    var body: some View {
        Text(label)
            .font(.ui(size, .semibold))
            .foregroundStyle(Palette.text)
            .padding(.horizontal, size * 0.45)
            .frame(minWidth: size * 1.65, minHeight: size * 1.65)
            .background(RoundedRectangle(cornerRadius: 4, style: .continuous).fill(Color.white.opacity(0.10)))
            .overlay(RoundedRectangle(cornerRadius: 4, style: .continuous)
                .strokeBorder(Color.white.opacity(0.18), lineWidth: 1))
    }
}

/// "P1" coin token: a port-coloured circle in display type.
struct PortCoin: View {
    let port: Int
    var size: CGFloat = 22
    var body: some View {
        Text("P\(port + 1)")
            .font(.display(size * 0.55))
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background(Circle().fill(
                RadialGradient(colors: [Palette.port(port).opacity(1), Palette.port(port).opacity(0.75)],
                               center: UnitPoint(x: 0.35, y: 0.3), startRadius: 0, endRadius: size)))
            .overlay(Circle().strokeBorder(Color.white.opacity(0.85), lineWidth: max(1, size / 14)))
            .shadow(color: .black.opacity(0.5), radius: 2, y: 1)
    }
}

/// A disc image (from Core) drawn at any size. Scale-ups over 2x use high
/// quality interpolation; `pixel` keeps hard pixels for icons at integer scales.
struct ArtImage: View {
    let image: CGImage
    var pixel = false
    /// Intensity images (emblems, name plates): their alpha is a mask in this colour.
    var tint: Color?
    var body: some View {
        let base = Image(decorative: image, scale: 1)
            .resizable()
            .interpolation(pixel ? .none : .high)
            .antialiased(!pixel)
        if let tint {
            base.renderingMode(.template).foregroundStyle(tint)
        } else {
            base
        }
    }
}

/// The screen title: display type with a dim accent underline along the slant.
struct ScreenTitle: View {
    let text: String
    var size: CGFloat = 44
    var body: some View {
        Text(text)
            .displayStyle(size)
            .foregroundStyle(LinearGradient(colors: [.white, Color(hex: 0xc9d2ff)], startPoint: .top,
                                            endPoint: .bottom))
            .shadow(color: Color(hex: 0x6b7bff).opacity(0.45), radius: 16)
            .shadow(color: .black.opacity(0.5), radius: 2, y: 2)
            .lineLimit(1)
            .minimumScaleFactor(0.6)
            .accessibilityAddTraits(.isHeader)
    }
}
