import SwiftUI
import UniformTypeIdentifiers

// The disc screen (DESIGN.md "Disc"): emblem, title, a glass drop zone with
// Open Disc and the recent disc, the footnote, and validation errors as a
// danger card under the zone.

struct DiscView: View {
    @ObservedObject var model: AppModel
    @State private var targeted = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        GeometryReader { geo in
            let m = MenuMetrics(size: geo.size)
            VStack(spacing: 0) {
                Spacer(minLength: titleBarInset)
                hero(m).staggerIn(0)
                Spacer().frame(height: m(34))
                dropZone(m).staggerIn(1)
                if let notice = model.notice {
                    NoticeCard(notice: notice) { model.notice = nil }
                        .frame(width: m(560))
                        .padding(.top, m(16))
                        .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
                }
                Spacer()
                Text("NTSC-U 1.02 (GALE01, revision 2)  ·  nothing leaves your machine")
                    .font(.ui(m(12)))
                    .tracking(0.4)
                    .foregroundStyle(Palette.textDim)
                    .padding(.bottom, m(22))
                    .staggerIn(2)
            }
            .frame(maxWidth: .infinity)
        }
        .onDrop(of: [.fileURL], isTargeted: $targeted) { providers in
            guard let provider = providers.first else { return false }
            _ = provider.loadObject(ofClass: URL.self) { url, _ in
                if let url { DispatchQueue.main.async { model.openDisc(url) } }
            }
            return true
        }
    }

    private func hero(_ m: MenuMetrics) -> some View {
        VStack(spacing: m(14)) {
            ZStack {
                EmblemShape()
                    .fill(Color(hex: 0x7f8cff).opacity(0.55))
                    .blur(radius: m(28))
                EmblemShape()
                    .fill(LinearGradient(colors: [.white, Color(hex: 0xd4dbff)], startPoint: .top, endPoint: .bottom))
                    .shadow(color: .white.opacity(0.55), radius: m(10))
            }
            .frame(width: m(118), height: m(118))
            .accessibilityHidden(true)
            VStack(spacing: m(2)) {
                Text("Super Smash Bros.")
                    .displayStyle(m(26), bold: true)
                    .foregroundStyle(Palette.text.opacity(0.85))
                ScreenTitle(text: "Melee", size: m(92))
                    .padding(.top, -m(14))
                Text("PORT")
                    .font(.ui(m(12), .bold))
                    .tracking(m(12) * 0.6)
                    .foregroundStyle(Palette.accent)
                    .padding(.top, -m(6))
            }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Super Smash Bros. Melee port")
        }
    }

    private func dropZone(_ m: MenuMetrics) -> some View {
        let shape = Slanted(radius: 14, maxShift: m(26))
        let actions = model.discActions
        return VStack(spacing: m(16)) {
            Image(systemName: targeted ? "arrow.down.circle.fill" : "opticaldisc")
                .font(.system(size: m(30), weight: .light))
                .foregroundStyle(targeted ? Palette.accentStart : Palette.text.opacity(0.75))
                .symbolEffect(.bounce, value: targeted)
            VStack(spacing: m(4)) {
                Text("Drop your Melee disc image")
                    .displayStyle(m(30))
                Text("An uncompressed .iso or .gcm of the NTSC-U 1.02 disc")
                    .font(.ui(m(13)))
                    .foregroundStyle(Palette.textDim)
            }
            HStack(spacing: m(12)) {
                ForEach(Array(actions.enumerated()), id: \.element) { index, action in
                    Button { model.perform(action) } label: { label(action) }
                        .buttonStyle(SlantButtonStyle(kind: index == 0 && model.disc == nil || isResume(action)
                                                        ? .accent : .glass,
                                                      focused: model.focus == index, size: m(17)))
                        .onHover { if $0 { model.focus = index } }
                }
            }
            .padding(.top, m(4))
        }
        .padding(.vertical, m(30))
        .padding(.horizontal, m(48))
        .frame(minWidth: m(620))
        .glass(shape, tint: targeted ? Palette.accentStart : nil, tintAmount: 0.18)
        .overlay {
            shape.inset(by: m(8))
                .stroke(style: StrokeStyle(lineWidth: targeted ? 2 : 1.25, dash: [m(9), m(7)]))
                .foregroundStyle(targeted ? Palette.accentStart.opacity(0.9) : Color.white.opacity(0.22))
        }
        .shadow(color: targeted ? Palette.accentEnd.opacity(0.35) : .black.opacity(0.35), radius: m(30), y: m(10))
        .scaleEffect(targeted && !reduceMotion ? 1.02 : 1)
        .animation(.spring(response: 0.35, dampingFraction: 0.8), value: targeted)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Disc drop zone")
    }

    private func isResume(_ action: DiscAction) -> Bool {
        if case .resume = action { return true }
        return false
    }

    @ViewBuilder private func label(_ action: DiscAction) -> some View {
        switch action {
        case .open: Text("Open Disc…")
        case .resume(let id): Text("Continue with \(id)")
        case .recent(let disc): Label("Open \(disc.name)", systemImage: "clock.arrow.circlepath").labelStyle(.titleOnly)
        }
    }
}
