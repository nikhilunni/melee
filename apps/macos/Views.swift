import SwiftUI
import UniformTypeIdentifiers

// The menus and overlays, in SwiftUI hosted by the AppKit window. They read
// the model and send intent; the core decides what is allowed.

enum Palette {
    static let background = Color(red: 0.06, green: 0.07, blue: 0.10)
    static let panel = Color(red: 0.11, green: 0.12, blue: 0.17)
    static let raised = Color(red: 0.16, green: 0.18, blue: 0.25)
    static let players = [Color(red: 0.93, green: 0.27, blue: 0.27), Color(red: 0.28, green: 0.52, blue: 0.96)]
    static let accent = Color(red: 0.98, green: 0.78, blue: 0.24)
}

struct RootView: View {
    @ObservedObject var model: AppModel
    var openPanel: () -> Void
    var saveReplay: () -> Void

    var body: some View {
        ZStack {
            switch model.screen {
            case .disc: DiscView(model: model, openPanel: openPanel).screenBackground()
            case .characters: CharacterSelectView(model: model).screenBackground()
            case .stages: StageSelectView(model: model).screenBackground()
            case .loading: LoadingView(model: model).screenBackground()
            case .match: MatchOverlay(model: model, saveReplay: saveReplay)
            case .results: ResultsView(model: model, saveReplay: saveReplay)
            }
        }
        .foregroundStyle(.white)
        .preferredColorScheme(.dark)
    }
}

private extension View {
    func screenBackground() -> some View {
        frame(maxWidth: .infinity, maxHeight: .infinity).background(Palette.background)
    }
}

struct Title: View {
    let text: String
    var subtitle: String?
    var body: some View {
        VStack(spacing: 6) {
            Text(text).font(.system(size: 34, weight: .heavy, design: .rounded))
            if let subtitle { Text(subtitle).font(.title3).foregroundStyle(.secondary) }
        }
    }
}

struct BigButton: View {
    let title: String
    var prominent = false
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Text(title).font(.title3.weight(.semibold)).frame(minWidth: 180).padding(.vertical, 6)
        }
        .buttonStyle(.borderedProminent)
        .tint(prominent ? Palette.accent.opacity(0.85) : Palette.raised)
        .controlSize(.large)
    }
}

// MARK: Disc

struct DiscView: View {
    @ObservedObject var model: AppModel
    var openPanel: () -> Void
    @State private var targeted = false

    var body: some View {
        VStack(spacing: 28) {
            Title(text: "Super Smash Bros. Melee",
                  subtitle: "Drop your Melee disc image here to begin")
            VStack(spacing: 14) {
                Image(systemName: "opticaldisc").font(.system(size: 56, weight: .light))
                Text("NTSC-U 1.02 (GALE01) · uncompressed .iso or .gcm").foregroundStyle(.secondary)
                BigButton(title: "Choose Disc Image…", prominent: true, action: openPanel)
            }
            .frame(width: 520, height: 260)
            .background(RoundedRectangle(cornerRadius: 22).fill(targeted ? Palette.raised : Palette.panel))
            .overlay(RoundedRectangle(cornerRadius: 22)
                .strokeBorder(style: StrokeStyle(lineWidth: 2, dash: [10, 8]))
                .foregroundStyle(targeted ? Palette.accent : .secondary.opacity(0.5)))
            .onDrop(of: [.fileURL], isTargeted: $targeted) { providers in
                guard let provider = providers.first else { return false }
                _ = provider.loadObject(ofClass: URL.self) { url, _ in
                    if let url { DispatchQueue.main.async { model.openDisc(url) } }
                }
                return true
            }
            if let disc = model.disc {
                BigButton(title: "Continue with \(disc.gameID)", action: model.resumeDisc)
            }
            if !model.recent.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Recent discs").font(.headline).foregroundStyle(.secondary)
                    ForEach(model.recent) { disc in
                        Button { model.openRecent(disc) } label: {
                            Label(disc.name, systemImage: "clock.arrow.circlepath")
                        }
                        .buttonStyle(.link)
                    }
                }
                .frame(width: 520, alignment: .leading)
            }
        }
        .padding(40)
    }
}

// MARK: Character select

struct CharacterSelectView: View {
    @ObservedObject var model: AppModel

    private var rows: [[CharacterInfo]] {
        Dictionary(grouping: model.characters, by: \.row)
            .sorted { $0.key < $1.key }
            .map { $0.value.sorted { $0.column < $1.column } }
    }

    var body: some View {
        VStack(spacing: 18) {
            HStack {
                Button("Back", systemImage: "chevron.left", action: model.back)
                    .keyboardShortcut(.cancelAction)
                Spacer()
                Title(text: "Choose Your Fighters")
                Spacer()
                Stepper("Stocks: \(model.selection.stocks)",
                        value: Binding(get: { model.selection.stocks }, set: { model.setStocks($0) }),
                        in: 1...99)
                    .font(.title3)
                    .frame(width: 170)
            }
            VStack(spacing: 8) {
                ForEach(rows, id: \.first?.id) { row in
                    HStack(spacing: 8) {
                        ForEach(row) { character in
                            CharacterCell(character: character, selection: model.selection) {
                                model.choose(character)
                            }
                        }
                    }
                }
            }
            HStack(alignment: .top, spacing: 18) {
                ForEach(0..<2, id: \.self) { player in PlayerPanel(model: model, player: player) }
            }
            HStack(spacing: 24) {
                ControlsLegend()
                Spacer()
                BigButton(title: "Choose Stage", prominent: true, action: model.confirmCharacters)
                    .disabled(!model.selection.ready)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(28)
    }
}

struct CharacterCell: View {
    let character: CharacterInfo
    let selection: Selection
    let action: () -> Void

    var body: some View {
        let pickedBy = (0..<2).filter { selection.players[$0].character == character.id }
        Button(action: action) {
            ZStack(alignment: .topTrailing) {
                Text(character.name)
                    .font(.system(size: 13, weight: .bold, design: .rounded))
                    .multilineTextAlignment(.center)
                    .lineLimit(2)
                    .minimumScaleFactor(0.7)
                    .frame(width: 92, height: 62)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Palette.panel))
                HStack(spacing: 2) {
                    ForEach(pickedBy, id: \.self) { player in
                        Text("P\(player + 1)").font(.caption2.weight(.heavy))
                            .padding(.horizontal, 4).padding(.vertical, 1)
                            .background(Capsule().fill(Palette.players[player]))
                    }
                }
                .padding(4)
            }
            .overlay(RoundedRectangle(cornerRadius: 10)
                .strokeBorder(pickedBy.isEmpty ? Color.white.opacity(0.08) : Palette.players[pickedBy[0]],
                              lineWidth: pickedBy.isEmpty ? 1 : 3))
        }
        .buttonStyle(.plain)
        .help(character.name)
    }
}

struct PlayerPanel: View {
    @ObservedObject var model: AppModel
    let player: Int

    var body: some View {
        let slot = model.selection.players[player]
        let character = model.character(slot.character)
        let active = model.picking == player
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("P\(player + 1)").font(.title2.weight(.heavy)).foregroundStyle(Palette.players[player])
                Text(character?.name ?? "Choose a character").font(.title2.weight(.bold))
                    .foregroundStyle(character == nil ? .secondary : .primary)
                Spacer()
                if character != nil {
                    Button("Clear", systemImage: "xmark.circle.fill") { model.clear(player: player) }
                        .buttonStyle(.borderless).labelStyle(.iconOnly)
                }
            }
            if let character {
                HStack(spacing: 6) {
                    Text("Costume").foregroundStyle(.secondary)
                    ForEach(0..<character.costumeCount, id: \.self) { costume in
                        let taken = model.selection.players[1 - player].character == character.id
                            && model.selection.players[1 - player].costume == costume
                        Button { model.setCostume(player: player, costume: costume) } label: {
                            Text("\(costume + 1)").font(.callout.weight(.bold)).frame(width: 26, height: 26)
                                .background(Circle().fill(slot.costume == costume
                                    ? Palette.players[player] : Palette.raised))
                        }
                        .buttonStyle(.plain)
                        .disabled(taken)
                        .opacity(taken ? 0.3 : 1)
                    }
                }
            } else {
                Text(active ? "Click a character above" : "Waiting…").foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Palette.panel))
        .overlay(RoundedRectangle(cornerRadius: 14)
            .strokeBorder(active ? Palette.players[player] : .clear, lineWidth: 2))
        .contentShape(Rectangle())
        .onTapGesture { model.picking = player }
    }
}

struct ControlsLegend: View {
    var body: some View {
        Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 3) {
            GridRow {
                Text("")
                ForEach(["Move", "Attack", "Special", "Jump", "Shield", "Grab"], id: \.self) {
                    Text($0).foregroundStyle(.secondary)
                }
            }
            GridRow {
                Text("P1").foregroundStyle(Palette.players[0]).fontWeight(.heavy)
                ForEach(["W A S D", "K", "J", "Space", "L", "I"], id: \.self) { Text($0) }
            }
            GridRow {
                Text("P2").foregroundStyle(Palette.players[1]).fontWeight(.heavy)
                ForEach(["Arrows", "N", "M", ",", ".", "/"], id: \.self) { Text($0) }
            }
        }
        .font(.system(.callout, design: .monospaced))
    }
}

// MARK: Stage select

struct StageSelectView: View {
    @ObservedObject var model: AppModel
    private let columns = [GridItem(.adaptive(minimum: 230), spacing: 14)]

    var body: some View {
        VStack(spacing: 24) {
            HStack {
                Button("Back", systemImage: "chevron.left", action: model.back).keyboardShortcut(.cancelAction)
                Spacer()
                Title(text: "Choose a Stage", subtitle: matchup)
                Spacer()
                Color.clear.frame(width: 60, height: 1)
            }
            LazyVGrid(columns: columns, spacing: 14) {
                ForEach(model.stages) { stage in
                    Button { model.chooseStage(stage) } label: {
                        Text(stage.name).font(.title3.weight(.bold))
                            .frame(maxWidth: .infinity, minHeight: 110)
                            .background(RoundedRectangle(cornerRadius: 14).fill(Palette.panel))
                            .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(.white.opacity(0.1)))
                    }
                    .buttonStyle(.plain)
                }
            }
            .frame(maxWidth: 820)
            Spacer()
        }
        .padding(28)
    }

    private var matchup: String {
        let names = model.selection.players.map { model.character($0.character)?.name ?? "?" }
        return "\(names[0]) vs \(names[1]) · \(model.selection.stocks) stocks"
    }
}

// MARK: Loading

struct LoadingView: View {
    @ObservedObject var model: AppModel
    var body: some View {
        VStack(spacing: 20) {
            Title(text: "Loading")
            ProgressView(value: model.progress.fraction).frame(width: 420).tint(Palette.accent)
            Text(detail).foregroundStyle(.secondary).monospacedDigit()
            Button("Cancel", action: model.back).keyboardShortcut(.cancelAction)
        }
    }
    private var detail: String {
        let p = model.progress
        let mb = { (bytes: UInt64) in String(format: "%.1f MB", Double(bytes) / 1_048_576) }
        if p.filesTotal == 0 { return "Starting the match…" }
        return "\(p.filesDone) of \(p.filesTotal) files · \(mb(p.bytesDone)) of \(mb(p.bytesTotal))"
    }
}

// MARK: Match

struct HudBar: View {
    @ObservedObject var model: AppModel
    var body: some View {
        if let hud = model.hud {
            HStack(spacing: 40) {
                ForEach(0..<hud.players.count, id: \.self) { index in
                    PlayerCard(player: hud.players[index], name: model.character(hud.players[index].character)?.name ?? "")
                }
            }
            .padding(.bottom, 18)
        }
    }
}

struct PlayerCard: View {
    let player: PlayerHud
    let name: String
    var body: some View {
        VStack(spacing: 2) {
            Text("\(Int(player.percent))%")
                .font(.system(size: 40, weight: .heavy, design: .rounded))
                .monospacedDigit()
                .shadow(color: .black, radius: 3)
            Text(name).font(.headline).shadow(color: .black, radius: 2)
            HStack(spacing: 4) {
                ForEach(0..<min(player.stocks, 12), id: \.self) { _ in
                    Circle().fill(Palette.players[min(player.port, 1)]).frame(width: 10, height: 10)
                }
                if player.stocks > 12 { Text("×\(player.stocks)").font(.caption.bold()) }
            }
        }
        .padding(.horizontal, 18).padding(.vertical, 8)
        .background(RoundedRectangle(cornerRadius: 12).fill(.black.opacity(0.45)))
        .overlay(alignment: .topLeading) {
            Text("P\(player.port + 1)").font(.caption.weight(.heavy))
                .foregroundStyle(Palette.players[min(player.port, 1)]).padding(6)
        }
    }
}

struct MatchOverlay: View {
    @ObservedObject var model: AppModel
    var saveReplay: () -> Void
    var body: some View {
        ZStack {
            VStack { Spacer(); HudBar(model: model) }.allowsHitTesting(false)
            if model.paused {
                Color.black.opacity(0.55)
                Dialog(title: "Paused") {
                    BigButton(title: "Resume", prominent: true) { model.setPaused(false) }
                        .keyboardShortcut(.cancelAction)
                    BigButton(title: "Restart", action: model.restart)
                    BigButton(title: "Save Replay…", action: saveReplay)
                    BigButton(title: "Quit to Character Select", action: model.quitToMenu)
                }
            }
        }
    }
}

struct Dialog<Content: View>: View {
    let title: String
    var subtitle: String?
    @ViewBuilder let content: Content
    var body: some View {
        VStack(spacing: 14) {
            Title(text: title, subtitle: subtitle).padding(.bottom, 6)
            content
        }
        .padding(32)
        .background(RoundedRectangle(cornerRadius: 20).fill(Palette.panel.opacity(0.96)))
    }
}

struct ResultsView: View {
    @ObservedObject var model: AppModel
    var saveReplay: () -> Void
    var body: some View {
        ZStack {
            Color.black.opacity(0.55)
            if let results = model.results {
                Dialog(title: headline(results)) {
                    HStack(spacing: 24) {
                        ForEach(0..<results.hud.players.count, id: \.self) { index in
                            PlayerCard(player: results.hud.players[index],
                                       name: model.character(results.hud.players[index].character)?.name ?? "")
                        }
                    }
                    .padding(.bottom, 8)
                    BigButton(title: "Rematch", prominent: true, action: model.rematch)
                        .keyboardShortcut(.defaultAction)
                    BigButton(title: "Character Select", action: model.quitToMenu)
                        .keyboardShortcut(.cancelAction)
                    BigButton(title: "Save Replay…", action: saveReplay)
                }
            }
        }
    }
    private func headline(_ results: MatchResults) -> String {
        guard let winner = results.winner else { return "Draw" }
        let player = results.hud.players[winner]
        return "P\(player.port + 1) \(model.character(player.character)?.name ?? "") Wins!"
    }
}
