import GameController

// Gamepads through the GameController framework. Each display frame of a
// match reports every connected pad, in connection order, as a GameCube
// controller; the core gives pad n to the match's nth human player.

/// melee_pad_button_e bits.
private enum PadBit: UInt32 {
    case a = 0, b, x, y, z, l, r, dpadLeft, dpadRight, dpadDown, dpadUp
    var mask: UInt32 { 1 << rawValue }
}

final class Gamepads {
    /// Connected pads, oldest first.
    private(set) var controllers: [GCController] = []
    private var reported = 0
    private var startHeld: Set<ObjectIdentifier> = []
    /// A pad's Menu (Start) button was pressed: pause or resume.
    var onStart: (() -> Void)?

    init() {
        controllers = GCController.controllers().filter { $0.extendedGamepad != nil }
        let center = NotificationCenter.default
        center.addObserver(forName: .GCControllerDidConnect, object: nil, queue: .main) { [weak self] note in
            guard let self, let pad = note.object as? GCController, pad.extendedGamepad != nil,
                  !self.controllers.contains(pad) else { return }
            self.controllers.append(pad)
        }
        center.addObserver(forName: .GCControllerDidDisconnect, object: nil, queue: .main) { [weak self] note in
            guard let pad = note.object as? GCController else { return }
            self?.controllers.removeAll { $0 == pad }
        }
    }

    /// A trigger past this counts as clicked in (the GameCube's digital L/R).
    private static let click: Float = 0.95

    static func state(_ pad: GCExtendedGamepad) -> GamepadState {
        var buttons: UInt32 = 0
        let map: [(GCControllerButtonInput, PadBit)] = [
            (pad.buttonA, .a), (pad.buttonB, .b), (pad.buttonX, .x), (pad.buttonY, .y),
            (pad.rightShoulder, .z), (pad.leftShoulder, .z),
            (pad.dpad.left, .dpadLeft), (pad.dpad.right, .dpadRight),
            (pad.dpad.down, .dpadDown), (pad.dpad.up, .dpadUp),
        ]
        for (input, bit) in map where input.isPressed { buttons |= bit.mask }
        if pad.leftTrigger.value >= click { buttons |= PadBit.l.mask }
        if pad.rightTrigger.value >= click { buttons |= PadBit.r.mask }
        var state = GamepadState(buttons: buttons)
        state.stick = (pad.leftThumbstick.xAxis.value, pad.leftThumbstick.yAxis.value)
        state.cstick = (pad.rightThumbstick.xAxis.value, pad.rightThumbstick.yAxis.value)
        state.triggers = (pad.leftTrigger.value, pad.rightTrigger.value)
        return state
    }

    /// Report every pad to the core (once per display frame of a match).
    func poll(_ model: AppModel) {
        for (device, controller) in controllers.enumerated() {
            guard let pad = controller.extendedGamepad else { continue }
            model.gamepad(device: device, Self.state(pad))
            let id = ObjectIdentifier(controller)
            if pad.buttonMenu.isPressed {
                if !startHeld.contains(id) { startHeld.insert(id); onStart?() }
            } else {
                startHeld.remove(id)
            }
        }
        // Pads that went away: their players go neutral.
        for device in controllers.count..<max(reported, controllers.count) {
            model.gamepad(device: device, nil)
        }
        reported = controllers.count
    }
}
