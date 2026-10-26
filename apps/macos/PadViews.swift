import SwiftUI

// Pad-facing views: button hints, the controllers card (per-pad mapping,
// deadzone, trigger click, port, live readout), the app menu (the menu
// bar's commands for pads) and the join toast.

/// A GameCube button glyph: a disc for A, B, X, Y, Z; a pill for the rest.
struct PadGlyph: View {
    let label: String
    var size: CGFloat = 12

    var body: some View {
        let colour: Color = switch label {
        case "A": Color(hex: 0x2fbf9a)
        case "B": Color(hex: 0xe0484f)
        case "Z": Color(hex: 0x7a5cff)
        default: Color.white.opacity(0.22)
        }
        Text(label)
            .font(.system(size: size * 0.82, weight: .heavy, design: .rounded))
            .foregroundStyle(.white)
            .padding(.horizontal, label.count > 1 ? size * 0.45 : 0)
            .frame(minWidth: size * 1.5, minHeight: size * 1.5)
            .background(Capsule().fill(colour))
            .accessibilityLabel("\(label) button")
    }
}

/// "Ⓐ Pick  Ⓑ Back ..." in GameCube terms, shown while a pad is connected.
struct PadHints: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics
    let hints: [(String, String)]

    var body: some View {
        if !model.pads.isEmpty {
            HStack(spacing: m(12)) {
                ForEach(hints.indices, id: \.self) { index in
                    HStack(spacing: m(4)) {
                        PadGlyph(label: hints[index].0, size: m(11))
                        Text(hints[index].1).font(.ui(m(12))).foregroundStyle(Palette.textDim)
                    }
                }
            }
            .accessibilityElement(children: .combine)
        }
    }
}

/// A short message near the top (a pad joined).
struct ToastView: View {
    let text: String
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "gamecontroller.fill").foregroundStyle(Palette.accentStart)
            Text(text).font(.ui(14, .semibold))
        }
        .padding(.horizontal, 16)
        .frame(height: 38)
        .glass(Capsule())
        .shadow(color: .black.opacity(0.4), radius: 12, y: 4)
    }
}

// MARK: Controllers

struct ControllersCard: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics

    private let layer = FocusLayer.controllers

    var body: some View {
        let pad = model.pads.first { $0.handle == model.settingsPad } ?? model.pads.first
        VStack(alignment: .leading, spacing: m(12)) {
            ScreenTitle(text: "Controllers", size: m(36))
            if let pad {
                header(pad)
                Text("Buttons map by position, as on a GameCube controller: the bottom face button is A, the left " +
                     "one B, the right X, the top Y (on a Nintendo pad the button labelled B is the GameCube's A).")
                    .font(.ui(m(12))).foregroundStyle(Palette.textDim)
                    .fixedSize(horizontal: false, vertical: true)
                portRow(pad)
                tuning(pad)
                bindings(pad)
                live(pad)
            } else {
                Text("No controllers connected. Connect a controller over USB or Bluetooth (Switch Pro, Joy-Con, " +
                     "DualSense, DualShock 4, Xbox, 8BitDo) or a Wii U GameCube adapter; it appears here.")
                    .font(.ui(m(14))).foregroundStyle(Palette.text.opacity(0.85))
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                PadHints(model: model, m: m, hints: [("A", "Choose"), ("B", "Done"), ("L", "R"), ("R", "Controller")])
                Spacer()
                button("Done", id: "pads.done", accent: true, action: model.closeControllers)
            }
        }
        .padding(.horizontal, m(32))
        .padding(.vertical, m(22))
        .frame(maxWidth: m(820))
        .glass(Slanted(radius: m(16), maxShift: m(30)))
        .shadow(color: .black.opacity(0.5), radius: m(30), y: m(10))
    }

    private func button(_ title: String, id: String, accent: Bool = false, selected: Bool = false,
                        action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title).font(.ui(m(13), .bold))
                .padding(.horizontal, m(10)).frame(height: m(28))
                .background(Capsule().fill(accent || selected ? AnyShapeStyle(Palette.accent)
                                                              : AnyShapeStyle(Color.black.opacity(0.3))))
                .foregroundStyle(accent || selected ? Color(hex: 0x1d0b00) : Palette.text)
        }
        .buttonStyle(.plain)
        .padFocus(id, model: model, layer: layer, radius: m(14), action: action)
    }

    private func header(_ pad: PadInfo) -> some View {
        HStack(spacing: m(10)) {
            if model.pads.count > 1 {
                button("◀", id: "pads.prev") { step(-1) }
            }
            VStack(alignment: .leading, spacing: 0) {
                Text(pad.name).font(.displayBold(m(20)))
                Text(pad.familyName + (model.pads.count > 1 ?
                     "  ·  \((model.pads.firstIndex(of: pad) ?? 0) + 1) of \(model.pads.count)" : ""))
                    .font(.ui(m(12))).foregroundStyle(Palette.textDim)
            }
            if model.pads.count > 1 {
                button("▶", id: "pads.next") { step(1) }
            }
        }
    }
    private func step(_ by: Int) {
        guard let index = model.pads.firstIndex(where: { $0.handle == model.settingsPad }) else { return }
        model.settingsPad = model.pads[(index + by + model.pads.count) % model.pads.count].handle
    }

    private func row<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        HStack(spacing: m(12)) {
            Text(title).font(.displayBold(m(15))).textCase(.uppercase).tracking(m(0.5))
                .foregroundStyle(Palette.textDim)
                .frame(width: m(150), alignment: .leading)
            content()
            Spacer(minLength: 0)
        }
    }

    private func portRow(_ pad: PadInfo) -> some View {
        row("Player") {
            button("None", id: "pads.port.none", selected: pad.port == nil) {
                model.editPad { $0.padAssign(handle: $1, port: nil) }
            }
            ForEach(0..<slotCount, id: \.self) { port in
                button("P\(port + 1)", id: "pads.port.\(port)", selected: pad.port == port) {
                    model.editPad { $0.padAssign(handle: $1, port: port) }
                }
            }
        }
    }

    private func tuning(_ pad: PadInfo) -> some View {
        VStack(alignment: .leading, spacing: m(8)) {
            row("Stick deadzone") {
                button("−", id: "pads.deadzone.down") {
                    model.editPad { $0.padSetDeadzone(handle: $1, pad.deadzone - 0.02) }
                }
                Text("\(Int((pad.deadzone * 100).rounded()))%").font(.display(m(18))).monospacedDigit()
                    .frame(minWidth: m(52))
                button("+", id: "pads.deadzone.up") {
                    model.editPad { $0.padSetDeadzone(handle: $1, pad.deadzone + 0.02) }
                }
            }
            row("Trigger click") {
                button("−", id: "pads.click.down") {
                    model.editPad { $0.padSetTriggerClick(handle: $1, pad.triggerClick - 0.05) }
                }
                Text("\(Int((pad.triggerClick * 100).rounded()))%").font(.display(m(18))).monospacedDigit()
                    .frame(minWidth: m(52))
                button("+", id: "pads.click.up") {
                    model.editPad { $0.padSetTriggerClick(handle: $1, pad.triggerClick + 0.05) }
                }
                button("Swap A and B", id: "pads.swap") { model.editPad { $0.padSwapAB(handle: $1) } }
                button("Reset to defaults", id: "pads.reset") { model.editPad { $0.padReset(handle: $1) } }
            }
        }
    }

    private func bindings(_ pad: PadInfo) -> some View {
        let targets = PadTarget.allCases.filter { $0 != .none }
        let half = (targets.count + 1) / 2
        return HStack(alignment: .top, spacing: m(24)) {
            ForEach([Array(targets[..<half]), Array(targets[half...])], id: \.first) { column in
                VStack(alignment: .leading, spacing: m(6)) {
                    ForEach(column, id: \.self) { target in
                        bindingRow(pad, target)
                    }
                }
            }
        }
    }

    private func bindingRow(_ pad: PadInfo, _ target: PadTarget) -> some View {
        let controls = pad.controls(for: target).map { pad.label($0) }
        let waiting = pad.listening == target
        return HStack(spacing: m(8)) {
            PadGlyph(label: glyph(target), size: m(11)).frame(width: m(64), alignment: .leading)
            Text(waiting ? "Press a control…" : controls.isEmpty ? "Unbound" : controls.joined(separator: ", "))
                .font(.ui(m(13), waiting ? .bold : .regular))
                .foregroundStyle(waiting ? Palette.accentStart : Palette.text)
                .lineLimit(1)
                .frame(width: m(170), alignment: .leading)
            button("Change", id: "pads.bind.\(target.rawValue)") {
                model.editPad { $0.padListen(handle: $1, target: target) }
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("GameCube \(target.name): \(controls.joined(separator: ", "))")
    }

    private func glyph(_ target: PadTarget) -> String {
        switch target {
        case .dpadUp: "D ↑"
        case .dpadDown: "D ↓"
        case .dpadLeft: "D ←"
        case .dpadRight: "D →"
        default: target.name
        }
    }

    private func live(_ pad: PadInfo) -> some View {
        let held = ["A", "B", "X", "Y", "Z", "L", "R", "←", "→", "↓", "↑"].enumerated()
            .filter { pad.buttons & (1 << UInt32($0.offset)) != 0 }.map(\.element)
        return Text("Stick \(pad.stick[0]), \(pad.stick[1])   C \(pad.cstick[0]), \(pad.cstick[1])   " +
                    "L \(pad.triggers[0])  R \(pad.triggers[1])   \(held.joined(separator: " "))")
            .font(.system(size: m(12), design: .monospaced))
            .foregroundStyle(Palette.textDim)
            .accessibilityLabel("Live input")
    }
}

// MARK: App menu

/// The menu bar's commands for a pad (Z, Select or Home opens it).
struct AppMenuCard: View {
    @ObservedObject var model: AppModel
    let m: MenuMetrics

    var body: some View {
        VStack(spacing: m(10)) {
            ScreenTitle(text: "Melee", size: m(36)).padding(.bottom, m(4))
            ForEach(Array(AppMenuAction.allCases.enumerated()), id: \.offset) { index, action in
                Button(title(action)) { run(action) }
                    .buttonStyle(SlantButtonStyle(kind: index == 0 ? .accent : .glass,
                                                  focused: model.focusID == "menu.\(index)", size: m(16),
                                                  minWidth: m(300)))
                    .padFocus("menu.\(index)", model: model, layer: FocusLayer.appMenu, ring: false) { run(action) }
                    .onHover { if $0 { model.hoverFocus("menu.\(index)") } }
            }
            PadHints(model: model, m: m, hints: [("A", "Choose"), ("B", "Close")]).padding(.top, m(4))
        }
        .padding(.horizontal, m(40))
        .padding(.vertical, m(26))
        .glass(Slanted(radius: m(16), maxShift: m(30)))
        .shadow(color: .black.opacity(0.5), radius: m(30), y: m(10))
    }

    private func run(_ action: AppMenuAction) {
        model.showingAppMenu = false
        model.focusID = nil
        model.appMenuAction?(action)
    }

    private func title(_ action: AppMenuAction) -> String {
        switch action {
        case .controllers: "Controllers…"
        case .fullScreen: model.fullScreen ? "Exit Full Screen" : "Enter Full Screen"
        case .minimize: "Minimize"
        case .hide: "Hide Melee"
        case .hideOthers: "Hide Others"
        case .help: "Help"
        case .about: "About Melee"
        case .quit: "Quit Melee"
        }
    }
}
