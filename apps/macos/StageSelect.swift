import SwiftUI

// Stage select (DESIGN.md "Stage select"): a hero preview of the focused
// stage in a slanted glass frame with the disc's name plate and emblem, and
// the row of stage tiles plus Random below.

struct StageSelectView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        GeometryReader { geo in
            let m = MenuMetrics(size: geo.size)
            let pad = m(28)
            VStack(spacing: 0) {
                topBar(m).padding(.top, titleBarInset).staggerIn(0)
                StageHero(model: model, index: model.stageCursor, m: m)
                    .frame(height: min(geo.size.height * 0.6, geo.size.height - m(300)))
                    .padding(.top, m(12))
                    .padding(.bottom, m(20))
                    .staggerIn(1)
                Spacer(minLength: m(12))
                StageRow(model: model, m: m, width: geo.size.width - pad * 2).staggerIn(2)
                Spacer(minLength: m(10))
                legend(m).staggerIn(3)
            }
            .padding(.horizontal, pad)
            .padding(.bottom, m(16))
        }
        // Re-read previews as the core renders them.
        .id(model.artGeneration)
    }

    private func topBar(_ m: MenuMetrics) -> some View {
        ZStack {
            ScreenTitle(text: "Choose a stage", size: m(40))
            HStack {
                Button(action: model.back) {
                    HStack(spacing: m(6)) {
                        Image(systemName: "chevron.left").font(.system(size: m(13), weight: .heavy))
                        Text("Fighters")
                    }
                }
                .buttonStyle(SlantButtonStyle(kind: .glass, size: m(15)))
                .accessibilityLabel("Back to character select")
                Spacer()
                Matchup(model: model, m: m)
            }
        }
        .frame(height: m(54))
    }

    private func legend(_ m: MenuMetrics) -> some View {
        HStack(spacing: m(16)) {
            HStack(spacing: m(4)) {
                KeyCap(label: "←", size: m(11)); KeyCap(label: "→", size: m(11))
                Text("Move").font(.ui(m(12))).foregroundStyle(Palette.textDim)
            }
            HStack(spacing: m(4)) {
                KeyCap(label: "⏎", size: m(11))
                Text("Fight").font(.ui(m(12))).foregroundStyle(Palette.textDim)
            }
            HStack(spacing: m(4)) {
                KeyCap(label: "Esc", size: m(11))
                Text("Back").font(.ui(m(12))).foregroundStyle(Palette.textDim)
            }
            Spacer()
        }
    }
}

/// The two picks as stock icons with VS between, top right.
struct Matchup: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics

    var body: some View {
        HStack(spacing: m(10)) {
            ForEach(0..<2, id: \.self) { player in
                let slot = model.selection.players[player]
                if player == 1 { Text("vs").font(.display(m(18))).foregroundStyle(Palette.textDim) }
                HStack(spacing: m(6)) {
                    if let id = slot.character, let icon = model.core.stockIcon(character: id, costume: slot.costume) {
                        ArtImage(image: icon, pixel: true).frame(width: 24, height: 24)
                    }
                    Text(model.character(slot.character)?.name ?? "?")
                        .font(.displayBold(m(16))).textCase(.uppercase).tracking(m(0.5))
                        .foregroundStyle(Palette.text)
                }
            }
            Text("·").foregroundStyle(Palette.textDim)
            Text("\(model.selection.stocks) stock\(model.selection.stocks == 1 ? "" : "s")")
                .font(.ui(m(13), .semibold)).foregroundStyle(Palette.textDim)
        }
        .padding(.horizontal, m(18))
        .frame(height: m(40))
        .glass(Slanted(radius: m(20)))
        .accessibilityElement(children: .combine)
    }
}

struct StageHero: View {
    @ObservedObject var model: AppModel
    let index: Int
    let m: MenuMetrics

    var body: some View {
        let stage = index < model.stages.count ? model.stages[index] : nil
        let shape = Slanted(radius: m(16), maxShift: m(60))
        GeometryReader { geo in
            // A 16:9 window onto the 1920x1080 preview, widened by the slant
            // so the image covers the parallelogram.
            let frameHeight = geo.size.height - m(34)
            let frameWidth = min(geo.size.width * 0.9,
                                 frameHeight * 16 / 9 + shape.shift(height: frameHeight))
            let frameX = (geo.size.width - frameWidth) / 2
            ZStack {
                ZStack {
                    picture(stage, size: geo.size)
                        .id("picture-\(index)")
                        .transition(.opacity)
                    // Depth: darken toward the bottom so the plate reads.
                    LinearGradient(colors: [.clear, .black.opacity(0.55)], startPoint: .center, endPoint: .bottom)
                    // The series emblem, scaled up and faint: a corner
                    // watermark over the preview, large and centred while
                    // the preview is pending.
                    if let stage, let emblem = model.core.stageEmblem(stage.id) {
                        let pending = model.core.stagePreview(stage.id) == nil
                        let side = frameHeight * (pending ? 0.8 : 0.62)
                        ArtImage(image: emblem, tint: .white)
                            .opacity(pending ? 0.3 : 0.22)
                            .blendMode(.plusLighter)
                            .frame(width: side, height: side)
                            .frame(maxWidth: .infinity, maxHeight: .infinity,
                                   alignment: pending ? .trailing : .bottomTrailing)
                            .padding(.trailing, pending ? frameWidth * 0.12 : m(40))
                            .padding(.bottom, pending ? m(30) : -frameHeight * 0.06)
                            .id("emblem-\(index)")
                            .transition(.opacity)
                    }
                }
                .frame(width: frameWidth, height: frameHeight)
                .clipShape(shape)
                .overlay { shape.strokeBorder(.white.opacity(0.22), lineWidth: 1) }
                .overlay {
                    shape.strokeBorder(LinearGradient(colors: [.white.opacity(0.5), .clear], startPoint: .top,
                                                      endPoint: UnitPoint(x: 0.5, y: 0.3)), lineWidth: 1.5)
                }
                .shadow(color: Color(hex: 0x6b7bff).opacity(0.35), radius: m(30))
                .shadow(color: .black.opacity(0.5), radius: m(12), y: m(8))
                .frame(maxHeight: .infinity, alignment: .top)
                namePlate(stage)
                    .frame(maxHeight: .infinity, alignment: .bottom)
                    .offset(y: -m(22))
                    .padding(.leading, frameX + m(10))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .id("plate-\(index)")
                    .transition(.opacity)
            }
            .animation(.easeInOut(duration: 0.15), value: index)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(stage.map { "\($0.name) preview" } ?? "Random stage")
    }

    @ViewBuilder private func picture(_ stage: StageInfo?, size: CGSize) -> some View {
        if let stage, let preview = model.core.stagePreview(stage.id) {
            Color.clear.overlay { ArtImage(image: preview).aspectRatio(contentMode: .fill) }.clipped()
        } else if let stage {
            // Until the preview is rendered: our own art (the disc's icons
            // are too small to stretch or blur).
            ZStack {
                StageGradient(key: stage.key)
                Shimmer()
            }
        } else {
            RandomArt(m: m)
        }
    }

    @ViewBuilder private func namePlate(_ stage: StageInfo?) -> some View {
        if let stage, let plate = model.core.stageName(stage.id) {
            // The disc's plate is an intensity mask: tint it white over a shadow.
            // At most 2x: past that the disc's plate smears.
            let scale = min(m.scale * 1.4, 2)
            ArtImage(image: plate, tint: .white)
                .frame(width: (224 * scale).rounded(), height: (56 * scale).rounded())
                .shadow(color: .black.opacity(0.9), radius: m(3), y: m(2))
                .shadow(color: .black.opacity(0.6), radius: m(14))
        } else {
            Text(stage?.name ?? "Random")
                .displayStyle(m(48))
                .shadow(color: .black.opacity(0.8), radius: m(6), y: m(2))
                .padding(.bottom, m(8))
        }
    }
}

/// Our own art for the Random tile and hero: the emblem over shifting colour.
struct RandomArt: View {
    let m: MenuMetrics
    var body: some View {
        GeometryReader { geo in
            ZStack {
                LinearGradient(colors: [Color(hex: 0x2b1a6e), Color(hex: 0x10145a), Color(hex: 0x05060f)],
                               startPoint: .topLeading, endPoint: .bottomTrailing)
                Text("?")
                    .font(.display(geo.size.height * 0.7))
                    .foregroundStyle(.white.opacity(0.85))
                    .shadow(color: Color(hex: 0x8f7bff), radius: geo.size.height * 0.1)
            }
        }
    }
}

struct StageRow: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics
    let width: CGFloat

    var body: some View {
        let count = model.stageTileCount
        let spacing = m(14)
        let tileWidth = min(((width - spacing * CGFloat(count - 1)) / CGFloat(count)).rounded(.down), m(170))
        HStack(alignment: .top, spacing: spacing) {
            ForEach(0..<count, id: \.self) { index in
                StageTile(model: model, index: index, m: m, width: tileWidth)
            }
        }
    }
}

struct StageTile: View {
    @ObservedObject var model: AppModel
    let index: Int
    let m: MenuMetrics
    let width: CGFloat
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let stage = index < model.stages.count ? model.stages[index] : nil
        let focused = model.stageCursor == index
        let shuffled = model.shuffling == index
        let shape = Slanted(radius: m(10))
        let height = (width * 0.62).rounded()
        Button { model.activateStageTile(index) } label: {
            VStack(spacing: m(8)) {
                art(stage)
                    .frame(width: width, height: height)
                    .clipShape(shape)
                    .overlay { shape.strokeBorder(.white.opacity(focused ? 0.95 : 0.15), lineWidth: focused ? 2 : 1) }
                    .shadow(color: focused ? Color(hex: 0x8fa0ff).opacity(0.7) : .black.opacity(0.4),
                            radius: focused ? m(16) : m(4), y: focused ? 0 : m(3))
                    .overlay {
                        if shuffled { shape.fill(.white.opacity(0.25)) }
                    }
                Text(stage?.name ?? "Random")
                    .font(.ui(m(12), .bold))
                    .tracking(m(0.8))
                    .textCase(.uppercase)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                    .foregroundStyle(focused ? Palette.text : Palette.textDim)
            }
            .offset(y: focused && !reduceMotion ? -m(8) : 0)
            .scaleEffect(focused && !reduceMotion ? 1.04 : 1)
        }
        .buttonStyle(.plain)
        .onHover { if $0 && model.shuffling == nil { model.stageCursor = index } }
        .animation(reduceMotion ? .easeInOut(duration: 0.15) : .spring(response: 0.3, dampingFraction: 0.7),
                   value: focused)
        .accessibilityLabel(stage?.name ?? "Random stage")
    }

    @ViewBuilder private func art(_ stage: StageInfo?) -> some View {
        if let stage, let preview = model.core.stagePreview(stage.id) {
            Color.clear.overlay { ArtImage(image: preview).aspectRatio(contentMode: .fill) }.clipped()
        } else if let stage, let icon = model.core.stageIcon(stage.id) {
            // The icon at 1x on the stage's gradient until the preview is
            // rendered (DESIGN.md refinement 8).
            ZStack {
                StageGradient(key: stage.key)
                ArtImage(image: icon, pixel: true)
                    .frame(width: CGFloat(icon.width), height: CGFloat(icon.height))
                    .clipShape(RoundedRectangle(cornerRadius: 3))
                    .shadow(color: .black.opacity(0.45), radius: 6, y: 3)
            }
        } else {
            RandomArt(m: MenuMetrics(size: CGSize(width: 400, height: 250)))
        }
    }
}

/// Our own per-stage colours (original art): the hero and tiles show them
/// while the rendered previews are pending, and loading falls back to them.
struct StageGradient: View {
    let key: String

    private var colors: [Color] {
        switch key {
        case "Battlefield": [Color(hex: 0x2a1c6e), Color(hex: 0x1b3f7a), Color(hex: 0x0b0d2a)]
        case "FinalDestination": [Color(hex: 0x4a1078), Color(hex: 0x1a0a4a), Color(hex: 0x05030f)]
        case "DreamLand": [Color(hex: 0x4f9be0), Color(hex: 0x2c6a9e), Color(hex: 0x173a2a)]
        case "FountainOfDreams": [Color(hex: 0x6a3aa8), Color(hex: 0x2b2470), Color(hex: 0x0d0b26)]
        case "PokemonStadium": [Color(hex: 0x3c7fc4), Color(hex: 0x24506e), Color(hex: 0x16301c)]
        case "YoshisStory": [Color(hex: 0xf2b84b), Color(hex: 0x7cc06a), Color(hex: 0x2c5a2e)]
        default: [Color(hex: 0x2b1a6e), Color(hex: 0x10145a), Color(hex: 0x05060f)]
        }
    }

    var body: some View {
        ZStack {
            LinearGradient(colors: colors, startPoint: .topLeading, endPoint: .bottomTrailing)
            RadialGradient(colors: [.white.opacity(0.16), .clear], center: UnitPoint(x: 0.3, y: 0.2),
                           startRadius: 0, endRadius: 500)
        }
    }
}

/// A soft light sweeping along the slant a few times, then resting (a
/// pending preview should take well under a second).
struct Shimmer: View {
    @State private var phase: CGFloat = -0.4
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        GeometryReader { geo in
            LinearGradient(colors: [.clear, .white.opacity(0.12), .clear], startPoint: .leading,
                           endPoint: .trailing)
                .frame(width: geo.size.width * 0.35)
                .rotationEffect(.degrees(12))
                .offset(x: geo.size.width * phase)
                .frame(maxHeight: .infinity)
        }
        .allowsHitTesting(false)
        .onAppear {
            guard !reduceMotion else { return }
            withAnimation(.easeInOut(duration: 1.4).repeatCount(3, autoreverses: false)) { phase = 1.2 }
        }
    }
}
