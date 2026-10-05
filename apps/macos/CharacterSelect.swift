import SwiftUI

// Character select (DESIGN.md "Character select"): top bar, the retail
// 9/9/7 grid of slanted face tiles, a tall glass panel per player with the
// selected costume's portrait, and the READY TO FIGHT banner.

struct CharacterSelectView: View {
    @ObservedObject var model: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var rows: [[CharacterInfo]] {
        Dictionary(grouping: model.characters, by: \.row)
            .sorted { $0.key < $1.key }
            .map { $0.value.sorted { $0.column < $1.column } }
    }

    var body: some View {
        GeometryReader { geo in
            let m = MenuMetrics(size: geo.size)
            let pad = m(28)
            // The grid takes what the panels leave; tiles keep a 5:4 shape.
            let gridWidth = min(geo.size.width - pad * 2, m(1000))
            let tileWidth = ((gridWidth - m(8) * 8) / 9).rounded(.down)
            let tileHeight = (tileWidth * 0.78).rounded()
            VStack(spacing: 0) {
                topBar(m).padding(.top, titleBarInset).staggerIn(0)
                grid(m, tile: CGSize(width: tileWidth, height: tileHeight))
                    .padding(.top, m(14))
                    .staggerIn(1)
                // The READY TO FIGHT row between the grid and the panels.
                ZStack { readyBanner(m) }
                    .frame(maxWidth: geo.size.height * 1.75)
                    .frame(height: m(60))
                    .padding(.vertical, m(10))
                panels(m)
                    .frame(maxWidth: geo.size.height * 1.75)
                    .staggerIn(2)
                legend(m).padding(.top, m(12)).staggerIn(3)
            }
            .padding(.horizontal, pad)
            .padding(.bottom, m(16))
        }
    }

    // MARK: Top bar

    private func topBar(_ m: MenuMetrics) -> some View {
        ZStack {
            ScreenTitle(text: "Choose your character", size: m(40))
            HStack {
                Button(action: model.back) {
                    HStack(spacing: m(6)) {
                        Image(systemName: "chevron.left").font(.system(size: m(13), weight: .heavy))
                        Text("Back")
                    }
                }
                .buttonStyle(SlantButtonStyle(kind: .glass, size: m(15)))
                .accessibilityLabel("Back to the disc screen")
                Spacer()
                StockStepper(model: model, m: m)
            }
        }
        .frame(height: m(54))
    }

    // MARK: Grid

    private func grid(_ m: MenuMetrics, tile: CGSize) -> some View {
        VStack(spacing: m(8)) {
            ForEach(rows, id: \.first?.id) { row in
                HStack(spacing: m(8)) {
                    ForEach(row) { character in
                        CharacterTile(model: model, character: character, size: tile, m: m)
                    }
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Fighters")
    }

    // MARK: Panels

    private func panels(_ m: MenuMetrics) -> some View {
        HStack(spacing: m(36)) {
            PlayerPanel(model: model, player: 0, m: m)
            PlayerPanel(model: model, player: 1, m: m)
        }
        .frame(minHeight: m(200), maxHeight: m(420))
    }

    @ViewBuilder private func readyBanner(_ m: MenuMetrics) -> some View {
        if model.selection.ready {
            ReadyBanner(m: m, action: model.confirmCharacters)
                .transition(reduceMotion ? .opacity
                    : .asymmetric(insertion: .offset(x: -m(120), y: 0).combined(with: .opacity),
                                  removal: .opacity))
        } else {
            // Who picks next, in their colour.
            HStack(spacing: m(10)) {
                PortCoin(port: model.picking, size: m(26))
                Text("Choose your fighter")
                    .displayStyle(m(24), bold: true)
                    .foregroundStyle(Palette.text.opacity(0.75))
            }
            .id(model.picking)
            .transition(.opacity)
            .animation(.easeInOut(duration: 0.15), value: model.picking)
        }
    }

    // MARK: Legend

    private func legend(_ m: MenuMetrics) -> some View {
        HStack(spacing: m(16)) {
            legendItem(["←", "→", "↑", "↓"], "Move", m)
            legendItem(["⏎"], model.selection.ready ? "Stage select" : "Pick", m)
            legendItem(["Tab"], "Switch player", m)
            legendItem(["Q", "E"], "Costume", m)
            legendItem(["⌫"], "Clear", m)
                        legendItem(["Esc"], "Back", m)
            Spacer(minLength: m(16))
            ControlsLegend(m: m)
        }
        .animation(nil, value: model.selection.ready)
    }

    private func legendItem(_ keys: [String], _ label: String, _ m: MenuMetrics) -> some View {
        HStack(spacing: m(4)) {
            ForEach(keys, id: \.self) { KeyCap(label: $0, size: m(11)) }
            Text(label).font(.ui(m(12))).foregroundStyle(Palette.textDim).padding(.leading, m(2))
        }
    }
}

// MARK: Stock stepper

/// "STOCK ◀ 4 ▶" as a slanted pill.
struct StockStepper: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics

    var body: some View {
        let stocks = model.selection.stocks
        HStack(spacing: m(10)) {
            Text("Stock").font(.displayBold(m(15))).tracking(m(1)).textCase(.uppercase)
                .foregroundStyle(Palette.textDim)
            arrow("arrowtriangle.left.fill", enabled: stocks > 1) { model.setStocks(stocks - 1) }
                .accessibilityLabel("Fewer stocks")
            Text("\(stocks)")
                .font(.display(m(26)))
                .monospacedDigit()
                .frame(minWidth: m(30))
                .contentTransition(.numericText(value: Double(stocks)))
                .animation(.spring(response: 0.3, dampingFraction: 0.8), value: stocks)
            arrow("arrowtriangle.right.fill", enabled: stocks < 99) { model.setStocks(stocks + 1) }
                .accessibilityLabel("More stocks")
        }
        .padding(.horizontal, m(20))
        .frame(height: m(40))
        .glass(Slanted(radius: m(20)))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Stocks \(stocks)")
    }

    private func arrow(_ symbol: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol).font(.system(size: m(11)))
                .frame(width: m(22), height: m(22))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .foregroundStyle(enabled ? Palette.accentStart : Palette.textDim.opacity(0.4))
        .disabled(!enabled)
    }
}

// MARK: Tile

struct CharacterTile: View {
    @ObservedObject var model: AppModel
    let character: CharacterInfo
    let size: CGSize
    let m: MenuMetrics
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let pickedBy = (0..<2).filter { model.selection.players[$0].character == character.id }
        let focused = model.cursor == character.id
        let active = Palette.port(model.picking)
        let shape = Slanted(radius: m(9))
        Button { model.choose(character) } label: {
            ZStack(alignment: .bottom) {
                face
                // The name along the bottom edge on a dark gradient.
                LinearGradient(colors: [.clear, .black.opacity(0.85)], startPoint: .center, endPoint: .bottom)
                Text(shortName)
                    .font(.displayBold(m(13)))
                    .tracking(m(0.4))
                    .textCase(.uppercase)
                    .lineLimit(1)
                    .minimumScaleFactor(0.6)
                    .foregroundStyle(.white)
                    .shadow(color: .black, radius: 1, y: 1)
                    .padding(.horizontal, m(10))
                    .padding(.bottom, m(3))
            }
            .frame(width: size.width, height: size.height)
            .background(LinearGradient(colors: [Color(hex: 0x2a3070), Color(hex: 0x141838)],
                                       startPoint: .top, endPoint: .bottom))
            .clipShape(shape)
            .overlay {
                // Picked: the player's coloured ring (both when both picked).
                ForEach(Array(pickedBy.enumerated()), id: \.element) { index, player in
                    shape.inset(by: CGFloat(index) * m(3)).strokeBorder(Palette.port(player), lineWidth: m(2.5))
                }
            }
            .overlay {
                if focused {
                    shape.strokeBorder(Color.white, lineWidth: m(2))
                    shape.inset(by: -m(3)).strokeBorder(active, lineWidth: m(2.5))
                } else {
                    shape.strokeBorder(Color.white.opacity(0.12), lineWidth: 1)
                }
            }
            .overlay(alignment: .topTrailing) {
                HStack(spacing: -m(5)) {
                    ForEach(pickedBy, id: \.self) { PortCoin(port: $0, size: m(20)) }
                }
                .offset(x: m(4), y: -m(6))
            }
            .shadow(color: focused ? active.opacity(0.6) : .black.opacity(0.35), radius: focused ? m(14) : m(3),
                    y: focused ? 0 : m(2))
            .scaleEffect(focused && !reduceMotion ? 1.06 : 1)
            .zIndex(focused ? 1 : 0)
        }
        .buttonStyle(.plain)
        .onHover { if $0 { model.cursor = character.id } }
        .animation(reduceMotion ? .easeInOut(duration: 0.15) : .spring(response: 0.3, dampingFraction: 0.7),
                   value: focused)
        .zIndex(focused ? 1 : 0)
        .help(character.name)
        .accessibilityLabel(character.name)
        .accessibilityValue(pickedBy.map { "picked by P\($0 + 1)" }.joined(separator: ", "))
    }

    /// Names that fit a tile.
    private var shortName: String {
        switch character.key {
        case "CaptainFalcon": "C. Falcon"
        case "GameAndWatch": "G&W"
        case "DonkeyKong": "DK"
        case "IceClimbers": "Ice Climbers"
        default: character.name
        }
    }

    /// The portrait cropped to the face: crisper than the 64x56 grid face
    /// at these sizes, and it has no name baked in.
    private var face: some View {
        FaceCrop(model: model, character: character.id, size: size, span: 0.92)
    }
}

// MARK: Player panel

/// A player's tall glass card: the big port mark, the selected costume's
/// portrait with a port-coloured light, the name and costume chips. P2's
/// card mirrors P1's layout (mark and name on the right).
struct PlayerPanel: View {
    @ObservedObject var model: AppModel
    let player: Int
    let m: MenuMetrics
    @State private var parallax: CGFloat = 0
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var mirrored: Bool { player == 1 }

    var body: some View {
        let slot = model.selection.players[player]
        let character = model.character(slot.character)
        let active = model.picking == player
        let color = Palette.port(player)
        let shape = Slanted(radius: m(14), maxShift: m(34))
        GeometryReader { geo in
            let lean = shape.shift(height: geo.size.height)
            ZStack {
                portMark(height: geo.size.height, lean: lean, active: active, color: color)
                if let character {
                    portrait(character, costume: slot.costume, color: color, size: geo.size)
                    namePlate(character, color: color, lean: lean)
                } else {
                    empty(active: active, size: geo.size)
                }
            }
            .onContinuousHover { phase in
                guard !reduceMotion else { return }
                switch phase {
                case .active(let point): parallax = (point.x / max(1, geo.size.width) - 0.5) * 8
                case .ended: parallax = 0
                }
            }
        }
        .clipShape(shape)
        .glass(shape, tint: color, tintAmount: active ? (character == nil ? 0.16 : 0.26) : 0.12)
        .overlay { shape.strokeBorder(color.opacity(active ? 0.9 : 0.0), lineWidth: 2) }
        .shadow(color: active ? color.opacity(0.45) : .black.opacity(0.3), radius: active ? m(22) : m(10))
        .contentShape(shape)
        .onTapGesture { model.activate(player: player) }
        .animation(.spring(response: 0.35, dampingFraction: 0.8), value: active)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Player \(player + 1)\(active ? ", choosing" : "")")
        .accessibilityValue(character?.name ?? "no fighter")
        .accessibilityAddTraits(.isButton)
    }

    /// "P1" / "P2" in display type, about a quarter of the panel's height.
    private func portMark(height: CGFloat, lean: CGFloat, active: Bool, color: Color) -> some View {
        Text("P\(player + 1)")
            .font(.display(height * 0.26))
            .foregroundStyle(LinearGradient(colors: [color.opacity(active ? 1 : 0.6), color.opacity(active ? 0.55 : 0.25)],
                                            startPoint: .top, endPoint: .bottom))
            .shadow(color: color.opacity(active ? 0.5 : 0), radius: m(12))
            .padding(mirrored ? .trailing : .leading, mirrored ? m(22) : lean + m(14))
            .padding(.top, m(6))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: mirrored ? .topTrailing : .topLeading)
            .accessibilityHidden(true)
    }

    private func portrait(_ character: CharacterInfo, costume: Int, color: Color, size: CGSize) -> some View {
        let height = size.height * 0.98
        let key = "\(character.id)-\(costume)"
        return ZStack {
            // Port-coloured light behind the fighter.
            Ellipse()
                .fill(RadialGradient(colors: [color.opacity(0.55), color.opacity(0)], center: .center,
                                     startRadius: 0, endRadius: height * 0.45))
                .frame(width: height * 0.95, height: height * 0.8)
            FighterPortrait(model: model, character: character.id, costume: costume, tint: color)
                .frame(height: height)
                .fadedCutEdges()
                .shadow(color: color.opacity(0.45), radius: m(16))
                .offset(x: parallax)
                .id(key)
                .transition(reduceMotion ? .opacity
                    : .offset(x: m(30), y: -m(30) * Slant.lean).combined(with: .opacity))
        }
        .frame(width: size.width * 0.62, height: size.height)
        .position(x: size.width * (mirrored ? 0.4 : 0.6), y: size.height * 0.53)
        .animation(.spring(response: 0.35, dampingFraction: 0.8), value: key)
        .animation(.spring(response: 0.25, dampingFraction: 0.85), value: parallax)
    }

    private func namePlate(_ character: CharacterInfo, color: Color, lean: CGFloat) -> some View {
        let slot = model.selection.players[player]
        let edge: HorizontalAlignment = mirrored ? .trailing : .leading
        return VStack(alignment: edge, spacing: m(8)) {
            CostumeChips(model: model, player: player, character: character, current: slot.costume, m: m)
            HStack(spacing: m(10)) {
                if mirrored { clearButton }
                Text(character.name)
                    .displayStyle(m(40))
                    .lineLimit(1)
                    .minimumScaleFactor(0.5)
                    .foregroundStyle(.white)
                    .shadow(color: .black.opacity(0.6), radius: 3, y: 2)
                if !mirrored { clearButton }
            }
            .padding(.horizontal, m(18))
            .padding(.vertical, m(2))
            .frame(maxWidth: .infinity, alignment: mirrored ? .trailing : .leading)
            .background {
                Slanted(radius: m(6), maxShift: m(12))
                    .fill(LinearGradient(colors: [.black.opacity(0.7), .black.opacity(0.15)],
                                         startPoint: mirrored ? .trailing : .leading,
                                         endPoint: mirrored ? .leading : .trailing))
                    .overlay(alignment: mirrored ? .trailing : .leading) {
                        Slanted(radius: 2).fill(color).frame(width: m(6)).padding(.vertical, m(4))
                    }
            }
        }
        .padding(.leading, mirrored ? m(40) : m(16))
        .padding(.trailing, mirrored ? lean + m(16) : m(40))
        .padding(.bottom, m(16))
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: mirrored ? .bottomTrailing : .bottomLeading)
    }

    private var clearButton: some View {
        Button { model.clear(player: player) } label: {
            Image(systemName: "xmark").font(.system(size: m(10), weight: .heavy))
                .frame(width: m(22), height: m(22))
                .background(Circle().fill(.black.opacity(0.35)))
                .overlay(Circle().strokeBorder(.white.opacity(0.25)))
        }
        .buttonStyle(.plain)
        .foregroundStyle(Palette.textDim)
        .accessibilityLabel("Clear P\(player + 1)'s fighter")
    }

    /// No fighter yet: our emblem, large and faint.
    private func empty(active: Bool, size: CGSize) -> some View {
        ZStack {
            EmblemShape()
                .fill(.white.opacity(active ? 0.10 : 0.05))
                .frame(width: size.height * 0.62, height: size.height * 0.62)
                .position(x: size.width * 0.5, y: size.height * 0.45)
            VStack(spacing: m(6)) {
                Text(active ? "Click a fighter" : "Waiting")
                    .displayStyle(m(26))
                    .foregroundStyle(Palette.text.opacity(active ? 0.9 : 0.4))
                Text(active ? "or press Enter on the grid" : "Tab or click to choose")
                    .font(.ui(m(12)))
                    .foregroundStyle(Palette.textDim.opacity(active ? 1 : 0.6))
            }
            .frame(maxHeight: .infinity, alignment: .bottom)
            .padding(.bottom, m(28))
        }
    }
}

/// Costume swatches: round chips with each costume's stock icon.
struct CostumeChips: View {
    @ObservedObject var model: AppModel
    let player: Int
    let character: CharacterInfo
    let current: Int
    let m: MenuMetrics

    var body: some View {
        let other = model.selection.players[1 - player]
        HStack(spacing: m(6)) {
            ForEach(0..<character.costumeCount, id: \.self) { costume in
                let taken = other.character == character.id && other.costume == costume
                let selected = costume == current
                Button { model.setCostume(player: player, costume: costume) } label: {
                    ZStack {
                        Circle().fill(Color.black.opacity(0.45))
                        if let icon = model.core.stockIcon(character: character.id, costume: costume) {
                            ArtImage(image: icon, pixel: true).frame(width: 24, height: 24)
                        } else {
                            Text("\(costume + 1)").font(.ui(12, .bold))
                        }
                    }
                    .frame(width: m(32), height: m(32))
                    .overlay(Circle().strokeBorder(selected ? Palette.port(player) : .white.opacity(0.18),
                                                   lineWidth: selected ? 2.5 : 1))
                    .shadow(color: selected ? Palette.port(player).opacity(0.7) : .clear, radius: 6)
                    .scaleEffect(selected ? 1.08 : 1)
                }
                .buttonStyle(.plain)
                .disabled(taken)
                .opacity(taken ? 0.3 : 1)
                .accessibilityLabel("Costume \(costume + 1)\(taken ? ", taken" : "")\(selected ? ", selected" : "")")
            }
        }
        .animation(.spring(response: 0.3, dampingFraction: 0.75), value: current)
    }
}

// MARK: Controls

/// The in-game keys for both players, two compact rows bottom right.
struct ControlsLegend: View {
    let m: MenuMetrics
    private static let rows: [[(keys: [String], action: String)]] = [
        [(["W", "A", "S", "D"], "move"), (["K"], "attack"), (["J"], "special"), (["Space"], "jump"),
         (["L"], "shield"), (["I"], "grab")],
        [(["←↑↓→"], "move"), (["N"], "attack"), (["M"], "special"), ([","], "jump"), (["."], "shield"),
         (["/"], "grab")],
    ]

    var body: some View {
        VStack(alignment: .trailing, spacing: m(5)) {
            ForEach(0..<2, id: \.self) { player in
                HStack(spacing: m(4)) {
                    Text("P\(player + 1)").font(.display(m(13))).foregroundStyle(Palette.port(player))
                        .padding(.trailing, m(2))
                    ForEach(Self.rows[player].indices, id: \.self) { index in
                        let item = Self.rows[player][index]
                        ForEach(item.keys, id: \.self) { KeyCap(label: $0, size: m(10)) }
                        Text(item.action).font(.ui(m(11))).foregroundStyle(Palette.textDim)
                            .padding(.trailing, m(4))
                    }
                }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Controls: P1 W A S D move, K attack, J special, Space jump, L shield, I grab. " +
                            "P2 arrows move, N attack, M special, comma jump, period shield, slash grab.")
    }
}

// MARK: Ready banner

/// READY TO FIGHT: a full-width slanted banner on the accent gradient with a
/// light sweep; clicking it (or Enter) goes to stage select.
struct ReadyBanner: View {
    let m: MenuMetrics
    let action: () -> Void
    @State private var hover = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        let shape = Slanted(radius: m(6))
        Button(action: action) {
            HStack(spacing: m(18)) {
                Text("Ready to fight")
                    .displayStyle(m(40))
                    .foregroundStyle(Color(hex: 0x1d0b00))
                HStack(spacing: m(5)) {
                    KeyCap(label: "⏎", size: m(11)).colorScheme(.light)
                    Text("Stage select").font(.ui(m(13), .bold)).foregroundStyle(Color(hex: 0x1d0b00).opacity(0.7))
                }
            }
            .frame(maxWidth: .infinity)
            .frame(height: m(60))
            .background {
                ZStack {
                    shape.fill(Palette.accent)
                    shape.fill(LinearGradient(colors: [.white.opacity(0.4), .clear], startPoint: .top,
                                              endPoint: .center))
                    // Light sweep along the slant (Core Animation: no app work per frame).
                    if !reduceMotion {
                        LightSweep(bandWidth: m(160)).clipShape(shape)
                    }
                }
            }
            .overlay { shape.strokeBorder(.white.opacity(0.55), lineWidth: 1.5) }
            .shadow(color: Palette.accentEnd.opacity(hover ? 0.75 : 0.5), radius: m(hover ? 30 : 22))
            .scaleEffect(hover && !reduceMotion ? 1.015 : 1)
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .animation(.spring(response: 0.3, dampingFraction: 0.75), value: hover)
        .padding(.horizontal, m(32))
        .accessibilityLabel("Ready to fight: choose a stage")
    }
}
