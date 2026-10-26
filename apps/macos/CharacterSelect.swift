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
            .overlay {
                if model.showingRules {
                    ZStack {
                        Color.black.opacity(0.45).ignoresSafeArea()
                            .onTapGesture { model.showingRules = false }
                        RulesCard(model: model, m: m)
                    }
                    .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.15), value: model.showingRules)
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
                .buttonStyle(SlantButtonStyle(kind: .glass, focused: model.focusID == "chars.back", size: m(15)))
                .padFocus("chars.back", model: model, ring: false, action: model.back)
                .accessibilityLabel("Back to the disc screen")
                Spacer()
                HStack(spacing: m(10)) {
                    StockStepper(model: model, m: m)
                    Button(action: openRules) {
                        HStack(spacing: m(6)) {
                            Image(systemName: "slider.horizontal.3").font(.system(size: m(13), weight: .bold))
                            Text("Rules")
                        }
                    }
                    .buttonStyle(SlantButtonStyle(kind: .glass, focused: model.focusID == "chars.rules", size: m(15)))
                    .padFocus("chars.rules", model: model, ring: false, action: openRules)
                    .accessibilityLabel("Rules: \(RulesCard.summary(model.rules))")
                    Button(action: model.openControllers) {
                        HStack(spacing: m(6)) {
                            Image(systemName: "gamecontroller").font(.system(size: m(13), weight: .bold))
                            Text("Controllers")
                        }
                    }
                    .buttonStyle(SlantButtonStyle(kind: .glass, focused: model.focusID == "chars.controllers",
                                                  size: m(15)))
                    .padFocus("chars.controllers", model: model, ring: false, action: model.openControllers)
                    .accessibilityLabel("Controllers: \(model.pads.count) connected")
                }
            }
        }
        .frame(height: m(54))
    }

    private func openRules() {
        model.showingRules = true
        model.focusID = nil
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
        HStack(spacing: m(18)) {
            ForEach(0..<slotCount, id: \.self) { player in
                PlayerPanel(model: model, player: player, m: m)
            }
        }
        .frame(minHeight: m(200), maxHeight: m(420))
    }

    @ViewBuilder private func readyBanner(_ m: MenuMetrics) -> some View {
        if model.selection.ready {
            ReadyBanner(m: m, focused: model.focusID == "chars.ready", action: model.confirmCharacters)
                .padFocus("chars.ready", model: model, ring: false, action: model.confirmCharacters)
                .transition(reduceMotion ? .opacity
                    : .asymmetric(insertion: .offset(x: -m(120), y: 0).combined(with: .opacity),
                                  removal: .opacity))
        } else {
            // Who picks next, in their colour, or what is missing.
            HStack(spacing: m(10)) {
                PortCoin(port: model.picking, size: m(26))
                Text(model.selection.players[model.picking].character == nil || model.selection.notReady == nil
                     ? "Choose your fighter" : (model.selection.notReady ?? "").capitalizedFirst)
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
            legendItem(["Tab"], "Next player", m)
            legendItem(["Q", "E"], "Costume", m)
            legendItem(["⌫"], "Clear", m)
                        legendItem(["Esc"], "Back", m)
            Spacer(minLength: m(16))
            if model.pads.isEmpty {
                ControlsLegend(m: m)
            } else {
                PadHints(model: model, m: m, hints: [("A", "Pick"), ("B", "Back"), ("X", "Y"), ("Y", "Costume"),
                                                     ("L", "R"), ("R", "Player"), ("Start", "Ready"), ("Z", "Menu")])
            }
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
        let timed = model.rules.mode == .time
        let stocks = timed ? model.rules.timeMinutes : model.rules.stocks
        let set: (Int) -> Void = timed ? { minutes in model.update { $0.timeMinutes = minutes } } : model.setStocks
        HStack(spacing: m(10)) {
            Text(timed ? "Time" : "Stock").font(.displayBold(m(15))).tracking(m(1)).textCase(.uppercase)
                .foregroundStyle(Palette.textDim)
            arrow("arrowtriangle.left.fill", enabled: stocks > 1) { if stocks > 1 { set(stocks - 1) } }
                .padFocus("chars.stocks.down", model: model, radius: m(11)) { if stocks > 1 { set(stocks - 1) } }
                .accessibilityLabel(timed ? "Fewer minutes" : "Fewer stocks")
            Text("\(stocks)")
                .font(.display(m(26)))
                .monospacedDigit()
                .frame(minWidth: m(30))
                .contentTransition(.numericText(value: Double(stocks)))
                .animation(.spring(response: 0.3, dampingFraction: 0.8), value: stocks)
            arrow("arrowtriangle.right.fill", enabled: stocks < 99) { if stocks < 99 { set(stocks + 1) } }
                .padFocus("chars.stocks.up", model: model, radius: m(11)) { if stocks < 99 { set(stocks + 1) } }
                .accessibilityLabel(timed ? "More minutes" : "More stocks")
            if timed { Text("min").font(.ui(m(12))).foregroundStyle(Palette.textDim) }
        }
        .padding(.horizontal, m(20))
        .frame(height: m(40))
        .glass(Slanted(radius: m(20)))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(timed ? "Time \(stocks) minutes" : "Stocks \(stocks)")
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
        let pickedBy = (0..<slotCount).filter {
            model.selection.players[$0].active && model.selection.players[$0].character == character.id
        }
        let focused = model.focusID == "tile.\(character.id)"
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
        .padFocus("tile.\(character.id)", model: model, ring: false) { model.choose(character) }
        .onHover { if $0 { model.hoverFocus("tile.\(character.id)") } }
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

    private var mirrored: Bool { player % 2 == 1 }

    var body: some View {
        let slot = model.selection.players[player]
        let character = slot.active ? model.character(slot.character) : nil
        let active = model.picking == player
        let color = model.rules.teams && slot.active ? Palette.team(slot.team)
            : slot.active ? Palette.port(player) : Palette.cpu
        let shape = Slanted(radius: m(14), maxShift: m(34))
        GeometryReader { geo in
            let lean = shape.shift(height: geo.size.height)
            ZStack {
                portMark(height: geo.size.height, lean: lean, active: active, color: color)
                if let character {
                    portrait(character, costume: slot.costume, color: color, size: geo.size)
                    namePlate(character, color: color, lean: lean)
                } else {
                    empty(active: active, slot: slot, size: geo.size)
                }
                SlotControls(model: model, player: player, m: m)
                    .padding(.top, m(10))
                    .padding(.leading, mirrored ? lean + m(12) : 0)
                    .padding(.trailing, mirrored ? 0 : m(12))
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: mirrored ? .topLeading : .topTrailing)
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
        .opacity(slot.active ? 1 : 0.6)
        .overlay { shape.strokeBorder(color.opacity(active ? 0.9 : 0.0), lineWidth: 2) }
        .shadow(color: active ? color.opacity(0.45) : .black.opacity(0.3), radius: active ? m(22) : m(10))
        .contentShape(shape)
        .onTapGesture { model.activate(player: player) }
        .animation(.spring(response: 0.35, dampingFraction: 0.8), value: active)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Player \(player + 1)\(active ? ", choosing" : "")" +
                            (slot.controller == .cpu ? ", CPU level \(slot.cpuLevel)" : slot.active ? "" : ", off"))
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
        .padFocus("slot.\(player).clear", model: model, radius: m(11)) { model.clear(player: player) }
        .accessibilityLabel("Clear P\(player + 1)'s fighter")
    }

    /// No fighter yet: our emblem, large and faint.
    private func empty(active: Bool, slot: Slot, size: CGSize) -> some View {
        ZStack {
            EmblemShape()
                .fill(.white.opacity(active ? 0.08 : 0.04))
                .frame(width: size.height * 0.62, height: size.height * 0.62)
                .position(x: size.width * 0.5, y: size.height * 0.45)
            VStack(spacing: m(6)) {
                Text(!slot.active ? "Not playing" : active ? "Click a fighter" : "Waiting")
                    .displayStyle(m(26))
                    .lineLimit(1)
                    .minimumScaleFactor(0.6)
                    .foregroundStyle(Palette.text.opacity(active ? 0.9 : 0.4))
                Text(!slot.active ? "Pick a fighter or set HMN/CPU to join" :
                        active ? "or press Enter (A) on the grid" : "Tab (L/R) or click to choose")
                    .multilineTextAlignment(.center)
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
        let others = model.selection.players.indices.filter { $0 != player }.map { model.selection.players[$0] }
        HStack(spacing: m(4)) {
            ForEach(0..<character.costumeCount, id: \.self) { costume in
                let taken = others.contains { $0.active && $0.character == character.id && $0.costume == costume }
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
                    .frame(width: m(28), height: m(28))
                    .overlay(Circle().strokeBorder(selected ? Palette.port(player) : .white.opacity(0.18),
                                                   lineWidth: selected ? 2.5 : 1))
                    .shadow(color: selected ? Palette.port(player).opacity(0.7) : .clear, radius: 6)
                    .scaleEffect(selected ? 1.08 : 1)
                }
                .buttonStyle(.plain)
                .disabled(taken)
                .padFocus("chip.\(player).\(costume)", model: model, radius: m(14)) {
                    if !taken { model.setCostume(player: player, costume: costume) }
                }
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
            Text("Keyboards drive the human players in port order; a gamepad joins with a button press")
                .font(.ui(m(11))).foregroundStyle(Palette.textDim)
            ForEach(0..<2, id: \.self) { player in
                HStack(spacing: m(4)) {
                    Text(player == 0 ? "1st" : "2nd").font(.display(m(13))).foregroundStyle(Palette.port(player))
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
    var focused = false
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
                    PadGlyph(label: "Start", size: m(11))
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
            .overlay { shape.strokeBorder(.white.opacity(focused ? 1 : 0.55), lineWidth: focused ? 3 : 1.5) }
            .shadow(color: Palette.accentEnd.opacity(hover || focused ? 0.75 : 0.5),
                    radius: m(hover || focused ? 30 : 22))
            .scaleEffect((hover || focused) && !reduceMotion ? 1.015 : 1)
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .animation(.spring(response: 0.3, dampingFraction: 0.75), value: hover)
        .padding(.horizontal, m(32))
        .accessibilityLabel("Ready to fight: choose a stage")
    }
}

extension String {
    var capitalizedFirst: String { prefix(1).uppercased() + dropFirst() }
}

// MARK: Slot controls

/// A panel's top row: HMN / CPU / OFF, the CPU level, the team colour in a
/// team battle and the handicap when one is set.
struct SlotControls: View {
    @ObservedObject var model: AppModel
    let player: Int
    let m: MenuMetrics

    var body: some View {
        let slot = model.selection.players[player]
        VStack(alignment: .trailing, spacing: m(6)) {
            HStack(spacing: m(2)) {
                ForEach([Controller.human, .cpu, .off], id: \.self) { controller in
                    let selected = slot.controller == controller
                    Button { model.setController(player: player, controller) } label: {
                        Text(label(controller))
                            .font(.displayBold(m(13))).tracking(m(0.5))
                            .padding(.horizontal, m(7)).frame(height: m(22))
                            .background(Capsule().fill(selected ? (controller == .off ? Palette.cpu : Palette.port(player)).opacity(0.85)
                                                       : .black.opacity(0.35)))
                            .foregroundStyle(selected ? .white : Palette.textDim)
                    }
                    .buttonStyle(.plain)
                    .padFocus("slot.\(player).\(label(controller).lowercased())", model: model, radius: m(11)) {
                        model.setController(player: player, controller)
                    }
                    .accessibilityLabel("P\(player + 1) \(label(controller))\(selected ? ", selected" : "")")
                }
            }
            if slot.controller == .cpu {
                stepper("LV", id: "level", value: slot.cpuLevel, range: 1...9, name: "CPU level") {
                    model.setCpuLevel(player: player, level: $0)
                }
            }
            if slot.active && model.rules.teams {
                Button { model.cycleTeam(player: player) } label: {
                    Text(teamNames[min(slot.team, teamNames.count - 1)])
                        .font(.displayBold(m(13))).textCase(.uppercase)
                        .padding(.horizontal, m(9)).frame(height: m(22))
                        .background(Capsule().fill(Palette.team(slot.team)))
                        .foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .padFocus("slot.\(player).team", model: model, radius: m(11)) { model.cycleTeam(player: player) }
                .accessibilityLabel("P\(player + 1) team \(teamNames[min(slot.team, teamNames.count - 1)])")
            }
            if slot.controller == .human {
                // Which pad drives this player, or how to join one.
                let pad = model.pad(port: player)
                if pad != nil || !model.pads.isEmpty {
                    HStack(spacing: m(4)) {
                        Image(systemName: "gamecontroller.fill").font(.system(size: m(10)))
                        Text(pad?.name ?? "Press a button to join").lineLimit(1)
                    }
                    .font(.ui(m(11), .semibold))
                    .foregroundStyle(pad == nil ? Palette.textDim : Palette.port(player))
                    .padding(.horizontal, m(8)).frame(height: m(20))
                    .background(Capsule().fill(.black.opacity(0.35)))
                }
            }
            if slot.active && model.showsHandicap {
                stepper("HC", id: "handicap", value: slot.handicap, range: 0...9, name: "handicap") {
                    model.setHandicap(player: player, handicap: $0)
                }
            }
        }
    }

    private func label(_ controller: Controller) -> String {
        switch controller {
        case .human: "HMN"
        case .cpu: "CPU"
        case .off: "OFF"
        }
    }

    private func stepper(_ title: String, id: String, value: Int, range: ClosedRange<Int>, name: String,
                         set: @escaping (Int) -> Void) -> some View {
        let lower = { if value > range.lowerBound { set(value - 1) } }
        let raise = { if value < range.upperBound { set(value + 1) } }
        return HStack(spacing: m(4)) {
            Text(title).font(.displayBold(m(12))).foregroundStyle(Palette.textDim)
            Button(action: lower) { Image(systemName: "minus").frame(width: m(18), height: m(18)) }
                .buttonStyle(.plain).disabled(value <= range.lowerBound)
                .padFocus("slot.\(player).\(id).down", model: model, radius: m(9), action: lower)
                .accessibilityLabel("Lower P\(player + 1) \(name)")
            Text(value == 0 ? "–" : "\(value)").font(.display(m(16))).monospacedDigit().frame(minWidth: m(14))
            Button(action: raise) { Image(systemName: "plus").frame(width: m(18), height: m(18)) }
                .buttonStyle(.plain).disabled(value >= range.upperBound)
                .padFocus("slot.\(player).\(id).up", model: model, radius: m(9), action: raise)
                .accessibilityLabel("Raise P\(player + 1) \(name)")
        }
        .font(.system(size: m(10), weight: .heavy))
        .padding(.horizontal, m(6)).frame(height: m(24))
        .background(Capsule().fill(.black.opacity(0.35)))
    }
}

// MARK: Rules

/// The rules card over character select: mode, stocks or time, teams,
/// items, damage ratio and Special Melee. One row per setting, so match
/// modes the simulator gains later add a row each.
struct RulesCard: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics

    static func summary(_ rules: Rules) -> String {
        var parts = [rules.mode == .time ? "Time \(rules.timeMinutes) min" : "\(rules.stocks) stock"]
        if rules.teams { parts.append("teams") }
        if rules.itemFrequency >= 0 { parts.append("items") }
        if rules.special != .none { parts.append(rules.special.title) }
        return parts.joined(separator: ", ")
    }

    private static let frequencies = ["Off", "Very low", "Low", "Medium", "High", "Very high"]

    var body: some View {
        let rules = model.rules
        VStack(alignment: .leading, spacing: m(14)) {
            ScreenTitle(text: "Rules", size: m(40)).padding(.bottom, m(4))
            row("Mode") {
                choice("mode", ["Stock", "Time"], selected: Int(rules.mode.rawValue)) { index in
                    model.update {
                        $0.mode = index == 1 ? .time : .stock
                        if index == 1 && $0.timeMinutes == 0 { $0.timeMinutes = 2 }
                    }
                }
            }
            if rules.mode == .stock {
                row("Stocks") { number("stocks", rules.stocks, range: 1...99) { v in model.update { $0.stocks = v } } }
            }
            row(rules.mode == .time ? "Time" : "Time limit") {
                number("time", rules.timeMinutes, range: (rules.mode == .time ? 1 : 0)...99, zero: "None", unit: "min") { v in
                    model.update { $0.timeMinutes = v }
                }
            }
            row("Teams") {
                choice("teams", ["Off", "On"], selected: rules.teams ? 1 : 0) { index in model.update { $0.teams = index == 1 } }
            }
            if rules.teams {
                row("Friendly fire") {
                    choice("friendly", ["Off", "On"], selected: rules.friendlyFire ? 1 : 0) { index in
                        model.update { $0.friendlyFire = index == 1 }
                    }
                }
            }
            row("Items") {
                choice("items", Self.frequencies, selected: rules.itemFrequency + 1) { index in
                    model.update { $0.itemFrequency = index - 1 }
                }
            }
            row("Damage ratio") {
                number("ratio", rules.damageRatio, range: 5...20, format: { String(format: "%.1f×", Double($0) / 10) }) { v in
                    model.update { $0.damageRatio = v }
                }
            }
            row("Handicap") {
                choice("handicap", ["Off", "On"], selected: model.showsHandicap ? 1 : 0) { index in
                    model.setHandicapRule(index == 1)
                }
            }
            row("Special") {
                choice("special", SpecialMode.allCases.map(\.title), selected: Int(rules.special.rawValue)) { index in
                    model.update { $0.special = SpecialMode(rawValue: UInt8(index)) ?? .none }
                }
            }
            Text("Items and CPU players are offered where the simulator is verified; stage select says where.")
                .font(.ui(m(12))).foregroundStyle(Palette.textDim)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                PadHints(model: model, m: m, hints: [("A", "Choose"), ("B", "Done")])
                Spacer()
                Button("Done", action: close)
                    .buttonStyle(SlantButtonStyle(kind: .accent, focused: model.focusID == "rules.done", size: m(16),
                                                  minWidth: m(140)))
                    .padFocus("rules.done", model: model, layer: FocusLayer.card, ring: false, action: close)
            }
        }
        .padding(.horizontal, m(36))
        .padding(.vertical, m(26))
        .frame(maxWidth: m(760))
        .glass(Slanted(radius: m(16), maxShift: m(30)))
        .shadow(color: .black.opacity(0.5), radius: m(30), y: m(10))
    }

    private func close() {
        model.showingRules = false
        model.focusID = nil
    }

    private func row<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        HStack(spacing: m(16)) {
            Text(title).font(.displayBold(m(17))).textCase(.uppercase).tracking(m(0.6))
                .foregroundStyle(Palette.textDim)
                .frame(width: m(150), alignment: .leading)
            content()
            Spacer(minLength: 0)
        }
    }

    private func choice(_ id: String, _ options: [String], selected: Int, set: @escaping (Int) -> Void) -> some View {
        HStack(spacing: m(4)) {
            ForEach(options.indices, id: \.self) { index in
                let on = index == selected
                Button { set(index) } label: {
                    Text(options[index]).font(.ui(m(13), .bold))
                        .padding(.horizontal, m(10)).frame(height: m(28))
                        .background(Capsule().fill(on ? AnyShapeStyle(Palette.accent) : AnyShapeStyle(Color.black.opacity(0.3))))
                        .foregroundStyle(on ? Color(hex: 0x1d0b00) : Palette.text)
                }
                .buttonStyle(.plain)
                .padFocus("rules.\(id).\(index)", model: model, layer: FocusLayer.card, radius: m(14)) { set(index) }
                .accessibilityLabel("\(options[index])\(on ? ", selected" : "")")
            }
        }
    }

    private func number(_ id: String, _ value: Int, range: ClosedRange<Int>, zero: String? = nil, unit: String = "",
                        format: ((Int) -> String)? = nil, set: @escaping (Int) -> Void) -> some View {
        let lower = { if value > range.lowerBound { set(value - 1) } }
        let raise = { if value < range.upperBound { set(value + 1) } }
        return HStack(spacing: m(8)) {
            Button(action: lower) { Image(systemName: "arrowtriangle.left.fill") }
                .buttonStyle(.plain).disabled(value <= range.lowerBound)
                .padFocus("rules.\(id).down", model: model, layer: FocusLayer.card, radius: m(8), action: lower)
            Text(value == 0 && zero != nil ? zero! : (format?(value) ?? "\(value)\(unit.isEmpty ? "" : " " + unit)"))
                .font(.display(m(22))).monospacedDigit().frame(minWidth: m(70))
            Button(action: raise) { Image(systemName: "arrowtriangle.right.fill") }
                .buttonStyle(.plain).disabled(value >= range.upperBound)
                .padFocus("rules.\(id).up", model: model, layer: FocusLayer.card, radius: m(8), action: raise)
        }
        .font(.system(size: m(12)))
        .foregroundStyle(Palette.accentStart)
    }
}
