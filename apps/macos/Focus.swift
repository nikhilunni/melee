import SwiftUI

// Menu focus for keyboards and pads (DESIGN.md "Keyboard first"): every
// control that can be reached registers its frame and action with
// `.padFocus(...)`; arrows, the D-pad and the stick move among the
// controls of the top layer (the core's melee_focus_next picks the
// neighbour, so the web page moves the same way), and Enter or A presses
// the focused one. Focus is always drawn.

/// The coordinate space focus frames are measured in.
let menuSpace = "menu"

/// Layers: an overlay's controls take focus over what is under them.
enum FocusLayer {
    static let screen = 0
    static let card = 1
    /// The controllers card opens over any screen, the rules card included.
    static let controllers = 2
    static let appMenu = 3
}

struct FocusTarget {
    let id: String
    let frame: CGRect
    let layer: Int
    let action: () -> Void
}

/// Equal when the same controls sit in the same places (actions aside), so
/// `onPreferenceChange` sees controls come and go.
extension FocusTarget: Equatable {
    static func == (a: Self, b: Self) -> Bool { a.id == b.id && a.layer == b.layer && a.frame == b.frame }
}

struct FocusTargetsKey: PreferenceKey {
    static let defaultValue: [FocusTarget] = []
    static func reduce(value: inout [FocusTarget], nextValue: () -> [FocusTarget]) {
        value.append(contentsOf: nextValue())
    }
}

struct PadFocus: ViewModifier {
    @ObservedObject var model: AppModel
    let id: String
    let layer: Int
    let ring: Bool
    let radius: CGFloat
    let action: () -> Void

    func body(content: Content) -> some View {
        let focused = model.focusID == id
        content
            .background {
                GeometryReader { geo in
                    Color.clear.preference(key: FocusTargetsKey.self, value: [
                        FocusTarget(id: id, frame: geo.frame(in: .named(menuSpace)), layer: layer, action: action),
                    ])
                }
            }
            .overlay {
                if ring && focused {
                    RoundedRectangle(cornerRadius: radius)
                        .strokeBorder(Color.white, lineWidth: 2)
                        .padding(-4)
                        .shadow(color: Palette.accentStart.opacity(0.9), radius: 6)
                        .allowsHitTesting(false)
                }
            }
    }
}

extension View {
    /// Make this control reachable by arrows and pads. `ring: false` when
    /// the control draws its own focused state.
    func padFocus(_ id: String, model: AppModel, layer: Int = FocusLayer.screen, ring: Bool = true,
                  radius: CGFloat = 8, action: @escaping () -> Void) -> some View {
        modifier(PadFocus(model: model, id: id, layer: layer, ring: ring, radius: radius, action: action))
    }
}

/// Collects the registered controls into the model on every layout.
struct FocusCollector: ViewModifier {
    let model: AppModel
    func body(content: Content) -> some View {
        content
            .coordinateSpace(name: menuSpace)
            // The sink stores the latest actions on every pass; SwiftUI can skip
            // it when a card appears, which onPreferenceChange does not.
            .backgroundPreferenceValue(FocusTargetsKey.self) { targets in
                FocusSink(model: model, targets: targets)
            }
            .onPreferenceChange(FocusTargetsKey.self) { [model] targets in model.store(targets) }
    }
}

private struct FocusSink: View {
    let model: AppModel
    let targets: [FocusTarget]
    var body: some View {
        // Actions capture the latest state, so the list is replaced on each pass.
        model.store(targets)
        return Color.clear
    }
}

extension AppModel {
    /// The controls of the top layer.
    var activeTargets: [FocusTarget] {
        // An open card owns focus even before its controls report in.
        let open = showingAppMenu ? FocusLayer.appMenu : showingControllers ? FocusLayer.controllers
            : showingRules ? FocusLayer.card : FocusLayer.screen
        let top = max(focusTargets.map(\.layer).max() ?? 0, open)
        return focusTargets.filter { $0.layer == top }
    }

    /// A card closed (`open` false): its controls leave focus at once, before
    /// its exit animation ends (SwiftUI may not report the smaller list).
    func closed(_ open: Bool, _ layer: Int) {
        guard !open else { return }
        focusTargets.removeAll { $0.layer == layer }
        if let focusID, !activeTargets.contains(where: { $0.id == focusID }) { self.focusID = nil }
    }

    func store(_ targets: [FocusTarget]) {
        // A closing card still reports its controls while it animates out.
        let closedLayers = [(showingRules, FocusLayer.card), (showingControllers, FocusLayer.controllers),
                            (showingAppMenu, FocusLayer.appMenu)].filter { !$0.0 }.map(\.1)
        focusTargets = targets.filter { !closedLayers.contains($0.layer) }
        // Keep focus on something visible (after the layout pass), then run
        // a key that waited for these controls.
        DispatchQueue.main.async { [weak self] in
            self?.ensureFocus()
            self?.replayPendingKey()
        }
    }

    /// Where focus lands when the focused control went away: the grid
    /// cursor, the stage cursor, or the first control of the layer.
    private var defaultFocus: String? {
        let active = activeTargets
        let preferred: [String] = switch screen {
        case .characters: ["tile.\(cursor)"]
        case .stages: ["stage.\(stageCursor)"]
        default: []
        }
        if let id = preferred.first(where: { id in active.contains { $0.id == id } }) { return id }
        return active.min { ($0.frame.minY, $0.frame.minX) < ($1.frame.minY, $1.frame.minX) }?.id
    }

    func ensureFocus() {
        let active = activeTargets
        if let focusID, active.contains(where: { $0.id == focusID }) { return }
        if let next = defaultFocus, next != focusID { setFocus(next) }
    }

    /// Focus a control; the grid and stage cursors follow their tiles, and
    /// focusing a player's panel makes that player the one picking.
    func setFocus(_ id: String) {
        focusID = id
        let parts = id.split(separator: ".")
        if parts.first == "tile", let value = parts.last.flatMap({ Int32($0) }) { cursor = value }
        if parts.first == "stage", let value = parts.last.flatMap({ Int($0) }) { stageCursor = value }
        if ["slot", "chip"].contains(parts.first), parts.count > 1, let player = Int(parts[1]), player != picking {
            picking = player
        }
    }

    /// The pointer entered a control. Only a pointer that moved takes focus:
    /// one resting over the window sees controls slide under it as screens
    /// change and the keys or a pad move through the menus.
    func hoverFocus(_ id: String) {
        let pointer = NSEvent.mouseLocation
        guard pointer != lastPointer else { return }
        lastPointer = pointer
        setFocus(id)
    }

    /// Move focus: 0 up, 1 down, 2 left, 3 right.
    @discardableResult
    func moveFocus(_ direction: Int) -> Bool {
        let active = activeTargets
        guard let focusID, let from = active.firstIndex(where: { $0.id == focusID }) else {
            // A screen or card that just opened reports its controls a moment
            // later; the press waits for them rather than getting lost.
            if !waitForTargets([.up, .down, .left, .right][direction]) { ensureFocus() }
            return true
        }
        if let next = Core.focusNext(active.map(\.frame), from: from, direction: direction) {
            setFocus(active[next].id)
        }
        return true
    }

    /// Press the focused control.
    @discardableResult
    func activateFocus() -> Bool {
        guard let focusID, let target = activeTargets.first(where: { $0.id == focusID }) else {
            if !waitForTargets(.enter) { ensureFocus() }
            return true
        }
        target.action()
        return true
    }

    /// Hold a key until the controls of what just opened report in (at most
    /// a second); false when nothing is opening and the key is spent.
    private func waitForTargets(_ key: MenuKey) -> Bool {
        let opening = focusID != nil || activeTargets.isEmpty
        guard opening else { return false }
        pendingKey = (key, Date())
        return true
    }

    /// Replay a held key once the controls are in.
    func replayPendingKey() {
        guard let (key, at) = pendingKey else { return }
        if Date().timeIntervalSince(at) > 1 { pendingKey = nil; return }
        guard let focusID, activeTargets.contains(where: { $0.id == focusID }) else { return }
        pendingKey = nil
        _ = handle(key)
    }
}
