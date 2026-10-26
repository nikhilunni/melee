import GameController
import IOKit.hid

// Gamepads: GameController pads and Wii U GameCube adapters (WUP-028, read
// over HID), reported to the core every poll on every screen. The core
// maps them (docs/APP.md "Gamepads"): this file only reads hardware.

final class Gamepads {
    /// Connected GameController pads and the handles the core knows them by.
    private var controllers: [(controller: GCController, handle: UInt32)] = []
    private var nextHandle: UInt32 = 1
    private let adapters = GameCubeAdapters()
    private weak var model: AppModel?
    private var timer: Timer?
    private var lastPoll: Double = 0
    /// Menu events from a pad (on the main thread).
    var onEvents: ((MenuEvents, UInt32) -> Void)?

    private static let prefsKey = "padPrefs"

    init(model: AppModel) {
        self.model = model
        model.core.padLoadPrefs(UserDefaults.standard.string(forKey: Self.prefsKey) ?? "")
        // Pads restore a hidden or minimized window (the app menu can hide it).
        GCController.shouldMonitorBackgroundEvents = true
        for controller in GCController.controllers() { connect(controller) }
        let center = NotificationCenter.default
        center.addObserver(forName: .GCControllerDidConnect, object: nil, queue: .main) { [weak self] note in
            guard let controller = note.object as? GCController else { return }
            self?.connect(controller)
        }
        center.addObserver(forName: .GCControllerDidDisconnect, object: nil, queue: .main) { [weak self] note in
            guard let self, let controller = note.object as? GCController,
                  let index = self.controllers.firstIndex(where: { $0.controller == controller }) else { return }
            self.model?.core.padDisconnect(handle: self.controllers[index].handle)
            self.controllers.remove(at: index)
            self.model?.padsChanged()
        }
        adapters.onChange = { [weak self] in self?.model?.padsChanged() }
        adapters.start()
        // Menus poll at 60 Hz; a running match polls from its display link.
        timer = Timer.scheduledTimer(withTimeInterval: 1.0 / 60, repeats: true) { [weak self] _ in self?.poll() }
        RunLoop.main.add(timer!, forMode: .common)
    }

    private func connect(_ controller: GCController) {
        guard let pad = controller.extendedGamepad, !controllers.contains(where: { $0.controller == controller })
        else { return }
        // The app reads Home, Options and Menu itself (the app menu, pause).
        for button in [pad.buttonHome, pad.buttonOptions, pad.buttonMenu] {
            button?.preferredSystemGestureState = .alwaysReceive
        }
        let handle = nextHandle
        nextHandle += 1
        controllers.append((controller, handle))
        model?.core.padConnect(handle: handle, name: controller.vendorName ?? "", category: controller.productCategory)
        model?.padsChanged()
    }

    /// A GameController pad in the standard layout, by position (GameController's
    /// buttonA is the bottom face button on every pad).
    static func raw(_ pad: GCExtendedGamepad) -> RawPad {
        var raw = RawPad()
        let buttons: [(GCControllerButtonInput?, PadInput)] = [
            (pad.buttonA, .south), (pad.buttonB, .east), (pad.buttonX, .west), (pad.buttonY, .north),
            (pad.leftShoulder, .leftBumper), (pad.rightShoulder, .rightBumper),
            (pad.leftTrigger, .leftTrigger), (pad.rightTrigger, .rightTrigger),
            (pad.buttonOptions, .select), (pad.buttonMenu, .start),
            (pad.leftThumbstickButton, .leftStick), (pad.rightThumbstickButton, .rightStick),
            (pad.dpad.up, .dpadUp), (pad.dpad.down, .dpadDown), (pad.dpad.left, .dpadLeft),
            (pad.dpad.right, .dpadRight), (pad.buttonHome, .home),
        ]
        for (button, input) in buttons {
            guard let button else { continue }
            raw[input] = button.isAnalog ? button.value : (button.isPressed ? 1 : 0)
        }
        raw.axes = [pad.leftThumbstick.xAxis.value, pad.leftThumbstick.yAxis.value,
                    pad.rightThumbstick.xAxis.value, pad.rightThumbstick.yAxis.value]
        return raw
    }

    /// Report every pad; at most once per 4 ms whoever asks (the menu timer
    /// or the match's display link).
    func poll(force: Bool = false) {
        guard let model else { return }
        let now = ProcessInfo.processInfo.systemUptime * 1000
        guard force || now - lastPoll >= 4 else { return }
        lastPoll = now
        for (controller, handle) in controllers {
            guard let pad = controller.extendedGamepad else { continue }
            let events = model.core.padUpdate(handle: handle, Self.raw(pad), now: now)
            if events.rawValue != 0 { onEvents?(events, handle) }
        }
        for (handle, raw) in virtualPads.sorted(by: { $0.key < $1.key }) {
            let events = model.core.padUpdate(handle: handle, raw, now: now)
            if events.rawValue != 0 { onEvents?(events, handle) }
        }
        for (base, report) in adapters.reports() {
            for (port, events) in model.core.padAdapterReport(base: base, report, now: now).enumerated() {
                if let events, events.rawValue != 0 { onEvents?(events, base + UInt32(port)) }
            }
        }
        model.padsPolled()
    }

    // Scripted pads (development, MELEE_APP_PADS): reported every poll like
    // hardware, so a walk goes through the core's mapping, joining and repeat.
    private var virtualPads: [UInt32: RawPad] = [:]
    static let virtualBase: UInt32 = 0xF000_0000
    static func isVirtual(_ handle: UInt32) -> Bool { handle >= virtualBase }
    func connectVirtual(handle: UInt32, name: String, category: String, vendor: UInt16 = 0, product: UInt16 = 0) {
        virtualPads[handle] = RawPad()
        model?.core.padConnect(handle: handle, name: name, category: category, vendor: vendor, product: product)
        model?.padsChanged()
    }
    func setVirtual(handle: UInt32, _ raw: RawPad) {
        guard virtualPads[handle] != nil else { return }
        virtualPads[handle] = raw
        poll(force: true)
    }

    /// A scripted walk keeps its settings to itself: its pads share names
    /// with real ones, and the walk starts from the defaults.
    static var persists = true
    static func savePrefs(_ core: Core) {
        guard persists else { return }
        UserDefaults.standard.set(core.padPrefs, forKey: prefsKey)
    }
}

/// Wii U GameCube adapters over IOHIDManager: started with 0x13, then each
/// 37-byte input report holds four ports (the core parses them).
final class GameCubeAdapters {
    private var manager: IOHIDManager?
    private final class Device {
        let device: IOHIDDevice
        let base: UInt32
        /// The report buffer IOKit fills; lives as long as the device entry.
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: 64)
        var latest: [UInt8]?
        init(device: IOHIDDevice, base: UInt32) { self.device = device; self.base = base }
        deinit { buffer.deallocate() }
    }
    private var devices: [Device] = []
    private var nextBase: UInt32 = 0x1000_0000
    var onChange: (() -> Void)?

    func start() {
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
        let matching: [String: Any] = [kIOHIDVendorIDKey: 0x057e, kIOHIDProductIDKey: 0x0337]
        IOHIDManagerSetDeviceMatching(manager, matching as CFDictionary)
        let context = Unmanaged.passUnretained(self).toOpaque()
        IOHIDManagerRegisterDeviceMatchingCallback(manager, { context, _, _, device in
            guard let context else { return }
            Unmanaged<GameCubeAdapters>.fromOpaque(context).takeUnretainedValue().attach(device)
        }, context)
        IOHIDManagerRegisterDeviceRemovalCallback(manager, { context, _, _, device in
            guard let context else { return }
            Unmanaged<GameCubeAdapters>.fromOpaque(context).takeUnretainedValue().detach(device)
        }, context)
        IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetMain(), CFRunLoopMode.defaultMode.rawValue)
        IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone))
        self.manager = manager
    }

    private func attach(_ device: IOHIDDevice) {
        guard !devices.contains(where: { $0.device === device }) else { return }
        let entry = Device(device: device, base: nextBase)
        nextBase += 4
        devices.append(entry)
        IOHIDDeviceOpen(device, IOOptionBits(kIOHIDOptionsTypeNone))
        let context = Unmanaged.passUnretained(entry).toOpaque()
        IOHIDDeviceRegisterInputReportCallback(device, entry.buffer, 64, { context, result, _, _, _, report, length in
            guard let context, result == kIOReturnSuccess else { return }
            let entry = Unmanaged<Device>.fromOpaque(context).takeUnretainedValue()
            entry.latest = Array(UnsafeBufferPointer(start: report, count: length))
        }, context)
        // Start the reports (Dolphin's GCAdapter writes the same byte).
        var start: UInt8 = 0x13
        IOHIDDeviceSetReport(device, kIOHIDReportTypeOutput, CFIndex(start), &start, 1)
        onChange?()
    }

    private func detach(_ device: IOHIDDevice) {
        guard let index = devices.firstIndex(where: { $0.device === device }) else { return }
        // An empty report: every port of this adapter disconnects.
        devices[index].latest = [0x21] + [UInt8](repeating: 0, count: 36)
        let entry = devices.remove(at: index)
        pendingEmpty.append((entry.base, entry.latest!))
        onChange?()
    }
    private var pendingEmpty: [(UInt32, [UInt8])] = []

    /// The latest report of each adapter (and the empty reports of adapters
    /// just removed), by base handle.
    func reports() -> [(UInt32, [UInt8])] {
        let removed = pendingEmpty
        pendingEmpty.removeAll()
        return removed + devices.compactMap { entry in entry.latest.map { (entry.base, $0) } }
    }
}
