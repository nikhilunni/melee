import Foundation

// Swift face of libmelee's gamepads (melee_app_pad_*): the core maps every
// pad onto a GameCube controller, assigns ports and turns presses into menu
// events; the host only reads its pads and draws.

/// Menu events of one pad poll (melee_menu_event_e).
struct MenuEvents: OptionSet {
    let rawValue: UInt32
    static let up = MenuEvents(rawValue: 1 << 0)
    static let down = MenuEvents(rawValue: 1 << 1)
    static let left = MenuEvents(rawValue: 1 << 2)
    static let right = MenuEvents(rawValue: 1 << 3)
    static let confirm = MenuEvents(rawValue: 1 << 4)
    static let back = MenuEvents(rawValue: 1 << 5)
    static let x = MenuEvents(rawValue: 1 << 6)
    static let y = MenuEvents(rawValue: 1 << 7)
    static let l = MenuEvents(rawValue: 1 << 8)
    static let r = MenuEvents(rawValue: 1 << 9)
    static let z = MenuEvents(rawValue: 1 << 10)
    static let start = MenuEvents(rawValue: 1 << 11)
    static let menu = MenuEvents(rawValue: 1 << 12)
    static let joined = MenuEvents(rawValue: 1 << 16)
    static let bound = MenuEvents(rawValue: 1 << 17)
}

/// Physical controls by position (melee_pad_input_e).
enum PadInput: Int, CaseIterable {
    case south = 0, east, west, north, leftBumper, rightBumper, leftTrigger, rightTrigger, select, start,
         leftStick, rightStick, dpadUp, dpadDown, dpadLeft, dpadRight, home
}

/// GameCube controls a pad's inputs map to (melee_pad_target_e; 0 is none).
enum PadTarget: Int, CaseIterable {
    case none = 0, a, b, x, y, z, l, r, start, dpadUp, dpadDown, dpadLeft, dpadRight
    var name: String { String(cString: melee_pad_target_name(UInt32(rawValue))) }
}

/// A pad's state in the W3C standard layout (melee_raw_pad_s).
struct RawPad {
    var buttons = [Float](repeating: 0, count: PadInput.allCases.count)
    /// Left x, y and right x, y; up positive.
    var axes: [Float] = [0, 0, 0, 0]
    subscript(input: PadInput) -> Float {
        get { buttons[input.rawValue] }
        set { buttons[input.rawValue] = newValue }
    }
}

/// A connected pad as the core sees it (melee_pad_info_s).
struct PadInfo: Identifiable, Equatable {
    let handle: UInt32
    var id: UInt32 { handle }
    var family: UInt32
    var name: String
    var port: Int?
    var listening: PadTarget?
    var bindings: [PadTarget]
    var deadzone: Float
    var triggerClick: Float
    var buttons: UInt32
    var stick: [Int]
    var cstick: [Int]
    var triggers: [Int]

    var familyName: String { String(cString: melee_pad_family_name(family)) }
    /// What this pad prints on a control.
    func label(_ input: PadInput) -> String {
        String(cString: melee_pad_input_label(family, UInt32(input.rawValue)))
    }
    static func position(_ input: PadInput) -> String {
        String(cString: melee_pad_input_position(UInt32(input.rawValue)))
    }
    /// The controls doing `target`, by their labels.
    func controls(for target: PadTarget) -> [PadInput] {
        PadInput.allCases.filter { bindings[$0.rawValue] == target }
    }
}

extension Core {
    func padConnect(handle: UInt32, name: String, category: String, vendor: UInt16 = 0, product: UInt16 = 0) {
        melee_app_pad_connect(app, handle, name, category, vendor, product)
    }
    func padDisconnect(handle: UInt32) { melee_app_pad_disconnect(app, handle) }
    func padUpdate(handle: UInt32, _ raw: RawPad, now: Double) -> MenuEvents {
        var c = melee_raw_pad_s()
        withUnsafeMutableBytes(of: &c.buttons) { buffer in
            let floats = buffer.bindMemory(to: Float.self)
            for (index, value) in raw.buttons.enumerated() where index < floats.count { floats[index] = value }
        }
        c.axes = (raw.axes[0], raw.axes[1], raw.axes[2], raw.axes[3])
        return MenuEvents(rawValue: melee_app_pad_update(app, handle, &c, now))
    }
    /// One Wii U GameCube adapter report; each port's events (nil: empty port).
    func padAdapterReport(base: UInt32, _ report: [UInt8], now: Double) -> [MenuEvents?] {
        var events: [UInt32] = [0, 0, 0, 0]
        let connected = report.withUnsafeBufferPointer { bytes in
            melee_app_pad_adapter_report(app, base, bytes.baseAddress, bytes.count, now, &events)
        }
        return (0..<4).map { connected & (1 << $0) != 0 ? MenuEvents(rawValue: events[$0]) : nil }
    }
    var pads: [PadInfo] {
        (0..<melee_app_pad_count(app)).compactMap { index in
            let handle = melee_app_pad_handle(app, index)
            var raw = melee_pad_info_s()
            guard melee_app_pad_info(app, handle, &raw) else { return nil }
            let bindings = withUnsafeBytes(of: raw.bindings) { Array($0) }.map { PadTarget(rawValue: Int($0)) ?? .none }
            let name = withUnsafeBytes(of: raw.name) { bytes in
                String(decoding: bytes.prefix { $0 != 0 }, as: UTF8.self)
            }
            return PadInfo(handle: handle, family: raw.family, name: name,
                           port: raw.port < 0 ? nil : Int(raw.port),
                           listening: raw.listening < 0 ? nil : PadTarget(rawValue: Int(raw.listening)),
                           bindings: bindings, deadzone: raw.deadzone, triggerClick: raw.trigger_click,
                           buttons: raw.buttons, stick: [Int(raw.stick.0), Int(raw.stick.1)],
                           cstick: [Int(raw.cstick.0), Int(raw.cstick.1)],
                           triggers: [Int(raw.triggers.0), Int(raw.triggers.1)])
        }
    }
    func padBind(handle: UInt32, input: PadInput, target: PadTarget) {
        melee_app_pad_bind(app, handle, UInt32(input.rawValue), UInt32(target.rawValue))
    }
    func padSwapAB(handle: UInt32) { melee_app_pad_swap_ab(app, handle) }
    func padSetDeadzone(handle: UInt32, _ value: Float) { melee_app_pad_set_deadzone(app, handle, value) }
    func padSetTriggerClick(handle: UInt32, _ value: Float) { melee_app_pad_set_trigger_click(app, handle, value) }
    func padReset(handle: UInt32) { melee_app_pad_reset(app, handle) }
    func padListen(handle: UInt32, target: PadTarget?) {
        melee_app_pad_listen(app, handle, Int32(target?.rawValue ?? -1))
    }
    func padAssign(handle: UInt32, port: Int?) { melee_app_pad_assign(app, handle, Int32(port ?? -1)) }
    var padPrefs: String {
        let length = melee_app_pad_prefs(app, nil, 0)
        var buffer = [CChar](repeating: 0, count: length + 1)
        melee_app_pad_prefs(app, &buffer, buffer.count)
        return String(cString: buffer)
    }
    func padLoadPrefs(_ prefs: String) { melee_app_pad_load_prefs(app, prefs) }

    /// The control focus moves to (melee_focus_next): direction 0 up, 1 down, 2 left, 3 right.
    static func focusNext(_ frames: [CGRect], from: Int, direction: Int) -> Int? {
        let flat = frames.flatMap { [Float($0.minX), Float($0.minY), Float($0.width), Float($0.height)] }
        let next = melee_focus_next(flat, UInt32(frames.count), UInt32(from), UInt32(direction))
        return next < 0 ? nil : Int(next)
    }
}
