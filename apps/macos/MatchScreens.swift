import SwiftUI

// Loading (VS), the match HUD, the pause card and results (DESIGN.md
// "Loading", "Match HUD", "Results").

// MARK: Loading

struct LoadingView: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        GeometryReader { geo in
            let m = MenuMetrics(size: geo.size)
            ZStack {
                stageBackdrop(m)
                HStack(spacing: 0) {
                    fighter(0, m: m, height: geo.size.height * 0.6)
                    vs(m)
                    fighter(1, m: m, height: geo.size.height * 0.6)
                }
                .frame(maxHeight: .infinity)
                progress(m)
                    .frame(maxHeight: .infinity, alignment: .bottom)
                    .padding(.bottom, m(44))
            }
        }
    }

    @ViewBuilder private func stageBackdrop(_ m: MenuMetrics) -> some View {
        if let stage = model.lastStage {
            // The stage, dimmed, filling the window (constrained so the
            // fill-mode image cannot grow the layout).
            Color.clear
                .overlay {
                    if let preview = model.core.stagePreview(stage) {
                        ArtImage(image: preview).aspectRatio(contentMode: .fill).blur(radius: m(6))
                    } else if let info = model.stages.first(where: { $0.id == stage }) {
                        StageGradient(key: info.key)
                    }
                }
                .overlay(Color.black.opacity(0.55))
                .clipped()
                .ignoresSafeArea()
        }
    }

    private func fighter(_ player: Int, m: MenuMetrics, height: CGFloat) -> some View {
        let slot = model.selection.players[player]
        let color = Palette.port(player)
        return VStack(spacing: m(6)) {
            if let id = slot.character {
                ZStack {
                    Circle()
                        .fill(RadialGradient(colors: [color.opacity(0.6), color.opacity(0)], center: .center,
                                             startRadius: 0, endRadius: height * 0.42))
                        .frame(width: height * 0.9, height: height * 0.9)
                    FighterPortrait(model: model, character: id, costume: slot.costume, tint: color)
                        .frame(height: height)
                        .fadedCutEdges()
                        // P2 mirrored: the two face each other.
                        .scaleEffect(x: player == 1 ? -1 : 1, y: 1)
                        .shadow(color: color.opacity(0.5), radius: m(20))
                }
                .staggerIn(player)
                Text(model.character(id)?.name ?? "")
                    .displayStyle(m(34))
                    .shadow(color: .black.opacity(0.7), radius: m(4), y: m(2))
                    .staggerIn(player + 1)
            }
        }
        .frame(maxWidth: .infinity)
    }

    private func vs(_ m: MenuMetrics) -> some View {
        Text("VS")
            .font(.display(m(110)))
            .foregroundStyle(Palette.accent)
            .shadow(color: Palette.accentEnd.opacity(0.6), radius: m(24))
            .shadow(color: .black.opacity(0.6), radius: m(3), y: m(3))
            .staggerIn(1)
            .accessibilityLabel("versus")
    }

    private func progress(_ m: MenuMetrics) -> some View {
        let p = model.progress
        return VStack(spacing: m(10)) {
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    Slanted(radius: 2).fill(.white.opacity(0.12))
                    Slanted(radius: 2).fill(Palette.accent)
                        .frame(width: max(m(8), geo.size.width * p.fraction))
                        .shadow(color: Palette.accentEnd.opacity(0.7), radius: m(8))
                }
            }
            .frame(width: m(440), height: m(6))
            .animation(.easeOut(duration: 0.2), value: p.fraction)
            Text(detail(p))
                .font(.ui(m(13)))
                .monospacedDigit()
                .foregroundStyle(Palette.textDim)
            Button("Cancel", action: model.back)
                .buttonStyle(SlantButtonStyle(kind: .glass, size: m(13)))
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Loading, \(Int(p.fraction * 100)) percent")
    }

    private func detail(_ p: LoadProgress) -> String {
        let mb = { (bytes: UInt64) in String(format: "%.1f MB", Double(bytes) / 1_048_576) }
        if p.filesTotal == 0 { return "Starting the match…" }
        return "\(p.filesDone) of \(p.filesTotal) files  ·  \(mb(p.bytesDone)) of \(mb(p.bytesTotal))"
    }
}

// MARK: HUD

/// Damage colour: white at 0%, orange at 100%, deep red from 200%.
func percentColor(_ percent: Float) -> Color {
    let stops: [(Float, (Double, Double, Double))] = [(0, (1, 1, 1)), (100, (1, 0.62, 0.22)), (200, (0.78, 0.06, 0.12))]
    let p = min(max(percent, 0), 200)
    for index in 1..<stops.count where p <= stops[index].0 {
        let (p0, a) = stops[index - 1], (p1, b) = stops[index]
        let t = Double((p - p0) / (p1 - p0))
        return Color(.sRGB, red: a.0 + (b.0 - a.0) * t, green: a.1 + (b.1 - a.1) * t, blue: a.2 + (b.2 - a.2) * t)
    }
    return Color(.sRGB, red: 0.78, green: 0.06, blue: 0.12)
}

/// Display text with a dark outline and drop shadow, like Melee's damage digits.
struct OutlinedText: View {
    let text: String
    let font: Font
    let color: Color
    var outline: CGFloat = 2

    var body: some View {
        ZStack {
            ForEach(0..<8, id: \.self) { index in
                let angle = Double(index) * .pi / 4
                Text(text).font(font).foregroundStyle(Color.black.opacity(0.9))
                    .offset(x: cos(angle) * outline, y: sin(angle) * outline)
            }
            Text(text).font(font).foregroundStyle(color)
        }
        .shadow(color: .black.opacity(0.55), radius: 4, y: 2)
    }
}

struct HudBar: View {
    @ObservedObject var model: AppModel
    var body: some View {
        if let hud = model.hud {
            HStack(spacing: 64) {
                ForEach(0..<hud.players.count, id: \.self) { index in
                    var player = hud.players[index]
                    let _ = model.previewPercents.map { player.percent = $0[min(index, $0.count - 1)] }
                    PlayerPlate(model: model, player: player)
                }
            }
            .padding(.bottom, 16)
        }
    }
}

/// One player's plate: stock icons, the face on a port-coloured slanted
/// plate, and the outlined percent.
struct PlayerPlate: View {
    @ObservedObject var model: AppModel
    let player: PlayerHud
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let color = Palette.port(player.port)
        let percent = Int(player.percent)
        HStack(alignment: .bottom, spacing: -6) {
            face(color)
            VStack(alignment: .trailing, spacing: 0) {
                stocks
                percentText(percent)
            }
            .padding(.trailing, 4)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Player \(player.port + 1), \(percent) percent, \(player.stocks) stocks")
    }

    /// The outlined percent; a quick shake when it jumps.
    private func percentText(_ percent: Int) -> some View {
        let color = percentColor(player.percent)
        let amplitude: CGFloat = reduceMotion ? 0 : 1
        return HStack(alignment: .firstTextBaseline, spacing: 1) {
            OutlinedText(text: "\(percent)", font: .display(44), color: color)
            OutlinedText(text: "%", font: .display(24), color: color, outline: 1.5)
        }
        .keyframeAnimator(initialValue: CGFloat(0), trigger: percent) { content, shake in
            content.offset(x: shake * amplitude)
        } keyframes: { _ in
            KeyframeTrack {
                CubicKeyframe(5, duration: 0.04)
                CubicKeyframe(-4, duration: 0.05)
                CubicKeyframe(2, duration: 0.05)
                CubicKeyframe(0, duration: 0.06)
            }
        }
    }

    private func face(_ color: Color) -> some View {
        let shape = Slanted(radius: 7)
        return ZStack {
            shape.fill(LinearGradient(colors: [color, color.opacity(0.55)], startPoint: .top, endPoint: .bottom))
            shape.fill(LinearGradient(colors: [.white.opacity(0.3), .clear], startPoint: .top, endPoint: .center))
            FaceCrop(model: model, character: player.character, costume: player.costume,
                     size: CGSize(width: 70, height: 60), span: 0.66)
                .clipShape(shape)
            Text("P\(player.port + 1)")
                .font(.display(13))
                .foregroundStyle(.white)
                .shadow(color: .black, radius: 1)
                .padding(.leading, 8).padding(.top, 2)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(width: 70, height: 60)
        .overlay(shape.strokeBorder(.white.opacity(0.45), lineWidth: 1))
        .shadow(color: color.opacity(0.45), radius: 10)
    }

    @ViewBuilder private var stocks: some View {
        HStack(spacing: 2) {
            if player.stocks > 5 {
                stockIcon
                Text("×\(player.stocks)").font(.display(16)).shadow(color: .black, radius: 2)
            } else {
                ForEach(0..<player.stocks, id: \.self) { _ in stockIcon }
            }
        }
        .frame(height: 24)
    }

    @ViewBuilder private var stockIcon: some View {
        if let icon = model.core.stockIcon(character: player.character, costume: player.costume) {
            ArtImage(image: icon, pixel: true).frame(width: 24, height: 24)
                .shadow(color: .black.opacity(0.6), radius: 1, y: 1)
        } else {
            Circle().fill(Palette.port(player.port)).frame(width: 12, height: 12).frame(width: 18, height: 24)
        }
    }
}

// MARK: Pause

struct MatchOverlay: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            VStack { Spacer(); HudBar(model: model) }.allowsHitTesting(false)
            if model.paused {
                // The game view blurs itself while paused (GameView.blurred);
                // this dims it.
                LinearGradient(colors: [Palette.deep.opacity(0.35), Palette.deep.opacity(0.6)],
                               startPoint: .top, endPoint: .bottom)
                    .ignoresSafeArea()
                    .transition(.opacity)
                PauseCard(model: model)
                    .transition(reduceMotion ? .opacity : .scale(scale: 0.94).combined(with: .opacity))
            }
        }
        .animation(reduceMotion ? .easeInOut(duration: 0.15) : .spring(response: 0.3, dampingFraction: 0.8),
                   value: model.paused)
    }
}

struct PauseCard: View {
    @ObservedObject var model: AppModel

    var body: some View {
        VStack(spacing: 12) {
            ScreenTitle(text: "Paused", size: 52).padding(.bottom, 8)
            ForEach(Array(PauseAction.allCases.enumerated()), id: \.offset) { index, action in
                Button(title(action)) { model.perform(action) }
                    .buttonStyle(SlantButtonStyle(kind: index == 0 ? .accent : .glass,
                                                  focused: model.focus == index, size: 18, minWidth: 300))
                    .onHover { if $0 { model.focus = index } }
            }
            HStack(spacing: 6) {
                KeyCap(label: "Esc")
                Text("Resume").font(.ui(12)).foregroundStyle(Palette.textDim)
            }
            .padding(.top, 6)
        }
        .padding(.horizontal, 44)
        .padding(.vertical, 30)
        .glass(Slanted(radius: 16, maxShift: 40))
        .shadow(color: .black.opacity(0.5), radius: 30, y: 10)
    }

    private func title(_ action: PauseAction) -> String {
        switch action {
        case .resume: "Resume"
        case .restart: "Restart"
        case .saveReplay: "Save Replay…"
        case .quit: "Quit to Character Select"
        }
    }
}

// MARK: Results

struct ResultsView: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let results = model.previewResults ?? model.results
        GeometryReader { geo in
            let m = MenuMetrics(size: geo.size)
            ZStack {
                ZStack {
                    Rectangle().fill(.ultraThinMaterial)
                    LinearGradient(colors: [Palette.deep.opacity(0.75), Palette.indigo.opacity(0.6)],
                                   startPoint: .bottom, endPoint: .top)
                }
                .ignoresSafeArea()
                if let results {
                    content(results, m: m, size: geo.size)
                }
            }
        }
    }

    private func content(_ results: MatchResults, m: MenuMetrics, size: CGSize) -> some View {
        let winner = results.winner.map { results.hud.players[$0] }
        let color = winner.map { Palette.port($0.port) } ?? Palette.cpu
        return HStack(spacing: m(24)) {
            hero(winner, color: color, m: m, height: size.height * 0.78)
                .frame(width: size.width * 0.44)
            VStack(alignment: .leading, spacing: m(14)) {
                Text(winner == nil ? "No contest" : "Winner")
                    .font(.displayBold(m(22))).tracking(m(3)).textCase(.uppercase)
                    .foregroundStyle(color)
                    .staggerIn(1)
                ScreenTitle(text: winner.flatMap { model.character($0.character)?.name } ?? "Draw", size: m(96))
                    .staggerIn(1)
                if let winner {
                    Text("Player \(winner.port + 1) wins")
                        .font(.ui(m(15), .semibold)).foregroundStyle(Palette.textDim)
                        .padding(.top, -m(10))
                        .staggerIn(1)
                }
                table(results, m: m).padding(.top, m(8)).staggerIn(2)
                buttons(m).padding(.top, m(10)).staggerIn(3)
            }
            .frame(maxWidth: m(520), alignment: .leading)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, m(40))
        .padding(.top, titleBarInset)
    }

    private func hero(_ winner: PlayerHud?, color: Color, m: MenuMetrics, height: CGFloat) -> some View {
        ZStack {
            // A light burst: soft rays in the winner's colour.
            AngularGradient(gradient: Gradient(colors: (0..<32).map { $0 % 2 == 0 ? color.opacity(0.5) : .clear }),
                            center: .center)
                .mask(RadialGradient(colors: [.white, .white.opacity(0.4), .clear], center: .center, startRadius: 0,
                                     endRadius: height * 0.75))
                .frame(width: height * 1.5, height: height * 1.5)
                .rotationEffect(.degrees(reduceMotion ? 0 : 6))
            Circle()
                .fill(RadialGradient(colors: [color.opacity(0.7), color.opacity(0)], center: .center, startRadius: 0,
                                     endRadius: height * 0.38))
                .frame(width: height * 0.8, height: height * 0.8)
            if let winner, let emblem = model.core.emblem(character: winner.character) {
                ArtImage(image: emblem, tint: .white)
                    .opacity(0.12)
                    .frame(width: height * 0.9, height: height * 0.9 * 64 / 80)
            }
            if let winner {
                FighterPortrait(model: model, character: winner.character, costume: winner.costume, tint: color)
                    .frame(height: height * 0.92)
                    .fadedCutEdges()
                    .shadow(color: color.opacity(0.55), radius: m(26))
                    .staggerIn(0)
            } else {
                EmblemShape().fill(.white.opacity(0.18)).frame(width: height * 0.4, height: height * 0.4)
            }
        }
        .frame(height: height)
        .accessibilityHidden(true)
    }

    private func table(_ results: MatchResults, m: MenuMetrics) -> some View {
        Grid(alignment: .leading, horizontalSpacing: m(18), verticalSpacing: m(10)) {
            GridRow {
                Text("")
                Text("Fighter")
                Text("Stocks").gridColumnAlignment(.trailing)
                Text("Damage").gridColumnAlignment(.trailing)
            }
            .font(.ui(m(11), .bold)).tracking(m(1)).textCase(.uppercase).foregroundStyle(Palette.textDim)
            ForEach(results.hud.players.indices, id: \.self) { index in
                let player = results.hud.players[index]
                GridRow {
                    PortCoin(port: player.port, size: m(24))
                    HStack(spacing: m(8)) {
                        if let icon = model.core.stockIcon(character: player.character, costume: player.costume) {
                            ArtImage(image: icon, pixel: true).frame(width: 24, height: 24)
                        }
                        Text(model.character(player.character)?.name ?? "")
                            .font(.displayBold(m(20))).textCase(.uppercase)
                    }
                    Text("\(player.stocks)").font(.display(m(24))).monospacedDigit()
                    Text("\(Int(player.percent))%").font(.display(m(24))).monospacedDigit()
                        .foregroundStyle(percentColor(player.percent))
                }
            }
        }
        .padding(m(18))
        .glass(RoundedRectangle(cornerRadius: m(14), style: .continuous))
    }

    private func buttons(_ m: MenuMetrics) -> some View {
        HStack(spacing: m(10)) {
            ForEach(Array(ResultsAction.allCases.enumerated()), id: \.offset) { index, action in
                Button(title(action)) { model.perform(action) }
                    .buttonStyle(SlantButtonStyle(kind: index == 0 ? .accent : .glass,
                                                  focused: model.focus == index, size: m(index == 0 ? 18 : 15)))
                    .onHover { if $0 { model.focus = index } }
            }
        }
    }

    private func title(_ action: ResultsAction) -> String {
        switch action {
        case .rematch: "Rematch"
        case .characters: "Characters"
        case .stages: "Stages"
        case .saveReplay: "Save Replay…"
        }
    }
}
