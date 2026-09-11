import AppKit
import QuartzCore
import Darwin

// Native shell only. Logical actions, fixed ticks, camera, asset loading and
// rendering are shared Rust behavior behind melee_platform.h.
final class GameView: NSView {
    private var session: OpaquePointer?
    private var displayLink: CADisplayLink?
    private var frames = 0
    private let hud = NSTextField(labelWithString: "")
    override var acceptsFirstResponder: Bool { true }

    init(frame: NSRect, directory: String) throws {
        super.init(frame: frame)
        var error = [CChar](repeating: 0, count: 2048)
        session = directory.withCString { melee_session_create($0, &error, error.count) }
        guard session != nil else { throw failure(String(cString: error)) }
        wantsLayer = true
        let metalLayer = CAMetalLayer()
        metalLayer.isOpaque = true
        layer = metalLayer
        hud.font = NSFont.monospacedSystemFont(ofSize: 12, weight: .medium)
        hud.textColor = .white
        hud.backgroundColor = NSColor.black.withAlphaComponent(0.7)
        hud.drawsBackground = true
        hud.translatesAutoresizingMaskIntoConstraints = false
        addSubview(hud)
        NSLayoutConstraint.activate([hud.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12), hud.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12), hud.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12)])
        let size = convertToBacking(bounds).size
        guard melee_session_attach_macos(session, Unmanaged.passUnretained(metalLayer).toOpaque(), UInt32(size.width), UInt32(size.height)) else {
            melee_session_error(session, &error, error.count)
            melee_session_destroy(session); session = nil
            throw failure(String(cString: error))
        }
        updateHUD()
    }
    required init?(coder: NSCoder) { fatalError("GameView requires an asset directory") }
    deinit { displayLink?.invalidate(); melee_session_destroy(session) }
    func shutdown() { displayLink?.invalidate(); displayLink = nil; melee_session_destroy(session); session = nil }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window != nil && displayLink == nil {
            displayLink = self.displayLink(target: self, selector: #selector(drawFrame(_:)))
            displayLink?.add(to: .main, forMode: .common)
            window?.makeFirstResponder(self)
        }
    }
    override func layout() {
        super.layout()
        // A paused/finished session still repaints once when its window resizes.
        if window != nil, let link = displayLink, link.isPaused { drawFrame(link) }
    }
    @objc private func drawFrame(_ link: CADisplayLink) {
        let size = convertToBacking(bounds).size
        guard size.width > 0 && size.height > 0 else { return }
        layer?.contentsScale = window?.backingScaleFactor ?? 1
        if !melee_session_frame(session, UInt32(size.width), UInt32(size.height)) {
            link.isPaused = true
            var error = [CChar](repeating: 0, count: 2048)
            melee_session_error(session, &error, error.count)
            NSAlert(error: failure(String(cString: error))).runModal()
            return
        }
        frames += 1
        if frames % 15 == 0 { updateHUD(); synchronizeDisplayLink() }
    }
    func smokePosition() -> Float {
        var status = MeleeStatus()
        guard melee_session_status(session, &status) else { return .nan }
        return status.position.0.0
    }
    func smokeKey(_ down: Bool) {
        let event = NSEvent.keyEvent(with: down ? .keyDown : .keyUp, location: .zero,
            modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: window?.windowNumber ?? 0, context: nil, characters: "d",
            charactersIgnoringModifiers: "d", isARepeat: false, keyCode: 2)!
        if down { keyDown(with: event) } else { keyUp(with: event) }
    }
    func smokeTick() -> UInt64 {
        var status = MeleeStatus()
        return melee_session_status(session, &status) ? status.tick : 0
    }
    private func updateHUD() {
        var status = MeleeStatus()
        guard melee_session_status(session, &status) else { return }
        let state = status.paused != 0 ? "Paused" : (status.running == 0 ? "Finished · ⌘R to restart" : "Tick \(status.tick)")
        hud.stringValue = "Fox \(Int(status.damage.0))% · \(status.stocks.0) stocks    Marth \(Int(status.damage.1))% · \(status.stocks.1) stocks    \(state)\nP1: WASD · K attack · J special · Space jump · L shield · I grab    P2: arrows · N attack · M special · , jump · . shield · / grab"
    }
    func setFocused(_ value: Bool) { melee_session_focus(session, value); synchronizeDisplayLink(); updateHUD() }
    @objc func togglePause(_ sender: Any?) { melee_session_toggle_pause(session); synchronizeDisplayLink(); updateHUD() }
    private func synchronizeDisplayLink() {
        var status = MeleeStatus()
        if melee_session_status(session, &status) { displayLink?.isPaused = status.needs_frame == 0 }
    }
    @objc func restart(_ sender: Any?) {
        if !melee_session_reset(session) { return }
        synchronizeDisplayLink(); updateHUD()
    }
    private func action(for key: UInt16) -> (UInt32, UInt32)? {
        // macOS physical virtual-key codes stay here. Logical mapping is shared.
        let p1 = Self.p1
        let p2 = Self.p2
        if let action = p1[key] { return (0, UInt32(action.rawValue)) }
        if let action = p2[key] { return (1, UInt32(action.rawValue)) }
        return nil
    }
    private static let p1: [UInt16: MeleeAction] = [0:MELEE_LEFT, 2:MELEE_RIGHT, 13:MELEE_UP, 1:MELEE_DOWN, 40:MELEE_ATTACK, 38:MELEE_SPECIAL, 49:MELEE_JUMP, 37:MELEE_SHIELD, 34:MELEE_GRAB]
    private static let p2: [UInt16: MeleeAction] = [123:MELEE_LEFT, 124:MELEE_RIGHT, 126:MELEE_UP, 125:MELEE_DOWN, 45:MELEE_ATTACK, 46:MELEE_SPECIAL, 43:MELEE_JUMP, 47:MELEE_SHIELD, 44:MELEE_GRAB]
    override func keyDown(with event: NSEvent) {
        guard !event.isARepeat else { return }
        if let (player, action) = action(for: event.keyCode) { melee_session_action(session, player, action, true) }
        else { super.keyDown(with: event) }
    }
    override func keyUp(with event: NSEvent) {
        if let (player, action) = action(for: event.keyCode) { melee_session_action(session, player, action, false) }
        else { super.keyUp(with: event) }
    }
}
func failure(_ text: String) -> NSError { NSError(domain: "Melee", code: 1, userInfo: [NSLocalizedDescriptionKey: text]) }

final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    private var window: NSWindow?
    private var game: GameView?
    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            let directory: String
            if let argument = CommandLine.arguments.dropFirst().first(where: { !$0.hasPrefix("--") }) { directory = argument }
            else {
                let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
                panel.message = "Choose your extracted Melee game files folder."
                guard panel.runModal() == .OK, let url = panel.url else { NSApp.terminate(nil); return }
                directory = url.path
            }
            let rect = NSRect(x: 0, y: 0, width: 1280, height: 720)
            let view = try GameView(frame: rect, directory: directory)
            let window = NSWindow(contentRect: rect, styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
            window.title = "Melee"; window.minSize = NSSize(width: 640, height: 400)
            window.contentView = view; window.delegate = self; window.center()
            self.window = window; self.game = view
            let menu = NSMenu(); let item = NSMenuItem(); menu.addItem(item)
            let application = NSMenu(); item.submenu = application
            application.addItem(withTitle: "Pause / Resume", action: #selector(GameView.togglePause(_:)), keyEquivalent: "p").target = view
            application.addItem(withTitle: "Restart Match", action: #selector(GameView.restart(_:)), keyEquivalent: "r").target = view
            application.addItem(.separator())
            application.addItem(withTitle: "Quit Melee", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
            NSApp.mainMenu = menu
            window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
            if CommandLine.arguments.contains("--smoke") {
                var before = Float.nan
                var pausedTick: UInt64 = 0
                DispatchQueue.main.asyncAfter(deadline: .now() + 1) { view.togglePause(nil); pausedTick = view.smokeTick() }
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.05) { view.setFocused(false) }
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.1) { view.setFocused(true) }
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
                    guard view.smokeTick() == pausedTick else { print("native smoke FAILED: focus cleared user pause"); exit(1) }
                    view.togglePause(nil); window.setContentSize(NSSize(width: 1100, height: 680))
                }
                DispatchQueue.main.asyncAfter(deadline: .now() + 3) { before = view.smokePosition(); view.smokeKey(true) }
                DispatchQueue.main.asyncAfter(deadline: .now() + 3.25) { view.smokeKey(false) }
                DispatchQueue.main.asyncAfter(deadline: .now() + 4) {
                    let after = view.smokePosition(); let tick = view.smokeTick()
                    guard tick > 120 && before.isFinite && after.isFinite && before != after else {
                        print("native smoke FAILED: tick \(tick), x \(before) -> \(after)"); exit(1)
                    }
                    print("native smoke passed: resize, focus, pause/resume, keyboard; tick \(tick), x \(before) -> \(after)")
                    NSApp.terminate(nil)
                }
            }
        } catch { NSAlert(error: error).runModal(); NSApp.terminate(nil) }
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func windowWillClose(_ notification: Notification) { game?.shutdown(); game = nil }
    func applicationWillTerminate(_ notification: Notification) { game?.shutdown(); game = nil }
    func windowDidResignKey(_ notification: Notification) { game?.setFocused(false) }
    func windowDidBecomeKey(_ notification: Notification) { game?.setFocused(true) }
}
let application = NSApplication.shared
application.setActivationPolicy(.regular)
let delegate = AppDelegate()
application.delegate = delegate
application.run()
