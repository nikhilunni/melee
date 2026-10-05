import AppKit
import QuartzCore
import SwiftUI
import UniformTypeIdentifiers

// The macOS app runtime: windows, menus, panels, drag and drop, the Metal
// layer and keyboard. Everything else is libmelee (Core.swift).

/// The CAMetalLayer-backed view the core draws the match into.
final class GameView: NSView {
    weak var model: AppModel?
    private var displayLink: CADisplayLink?
    private var attached = false
    override var acceptsFirstResponder: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        let metal = CAMetalLayer()
        metal.isOpaque = true
        layer = metal
    }
    required init?(coder: NSCoder) { fatalError("not used") }

    func shutdown() {
        displayLink?.invalidate()
        displayLink = nil
        model?.core.detachSurface()
        attached = false
    }

    private var pixelSize: (UInt32, UInt32) {
        let size = convertToBacking(bounds).size
        return (UInt32(max(size.width, 1)), UInt32(max(size.height, 1)))
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard let window, let model, !attached, let metal = layer as? CAMetalLayer else { return }
        metal.contentsScale = window.backingScaleFactor
        let (width, height) = pixelSize
        do {
            try model.core.attach(layer: Unmanaged.passUnretained(metal).toOpaque(), width: width, height: height)
            attached = true
        } catch {
            model.present("Metal is unavailable", error.localizedDescription)
        }
        displayLink = self.displayLink(target: self, selector: #selector(step(_:)))
        displayLink?.add(to: .main, forMode: .common)
        displayLink?.isPaused = true
    }
    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        layer?.contentsScale = window?.backingScaleFactor ?? 1
    }
    override func layout() {
        super.layout()
        // A paused or finished match still redraws when the window resizes.
        if let link = displayLink, link.isPaused, model?.screen == .match || model?.screen == .results {
            draw()
        }
    }

    /// Run the display link only while the match needs frames.
    func synchronize() {
        displayLink?.isPaused = !(model?.needsFrame ?? false)
        blurred = model.map { $0.screen == .match && ($0.paused || $0.showsResults) } ?? false
    }

    /// Paused: the stage blurs behind the pause card (a Core Image filter
    /// composited by the window server; the paused game draws nothing new).
    private var blurred = false {
        didSet {
            guard blurred != oldValue, let layer else { return }
            layerUsesCoreImageFilters = true
            if blurred, let blur = CIFilter(name: "CIGaussianBlur") {
                blur.setValue(10, forKey: kCIInputRadiusKey)
                layer.filters = [blur]
            } else {
                layer.filters = nil
            }
        }
    }

    @objc private func step(_ link: CADisplayLink) {
        draw()
        synchronize()
    }
    private func draw() {
        guard attached, let model else { return }
        let (width, height) = pixelSize
        model.frame(width: width, height: height)
    }

    // macOS virtual key codes for the two keyboard players; the logical
    // actions are the core's.
    private static let p1: [UInt16: GameAction] = [0: .left, 2: .right, 13: .up, 1: .down, 40: .attack,
                                                   38: .special, 49: .jump, 37: .shield, 34: .grab]
    private static let p2: [UInt16: GameAction] = [123: .left, 124: .right, 126: .up, 125: .down, 45: .attack,
                                                   46: .special, 43: .jump, 47: .shield, 44: .grab]
    private static let escape: UInt16 = 53

    private func binding(_ key: UInt16) -> (Int, GameAction)? {
        if let action = Self.p1[key] { return (0, action) }
        if let action = Self.p2[key] { return (1, action) }
        return nil
    }
    override func keyDown(with event: NSEvent) {
        guard let model, model.screen == .match else { return super.keyDown(with: event) }
        if event.keyCode == Self.escape {
            if !event.isARepeat { model.togglePause(); synchronize() }
            return
        }
        guard let (player, action) = binding(event.keyCode) else { return super.keyDown(with: event) }
        if !event.isARepeat { model.action(player: player, action, down: true) }
    }
    override func keyUp(with event: NSEvent) {
        guard let model, let (player, action) = binding(event.keyCode) else { return super.keyUp(with: event) }
        model.action(player: player, action, down: false)
    }
}

/// The overlay host. Mouse clicks on empty overlay space reach the game view.
final class OverlayHostingView<Content: View>: NSHostingView<Content> {
    override func hitTest(_ point: NSPoint) -> NSView? {
        let hit = super.hitTest(point)
        return hit === self ? nil : hit
    }
}

final class WindowController: NSWindowController, NSWindowDelegate {
    let model: AppModel
    let game: GameView
    private var overlay: OverlayHostingView<RootView>!
    /// The menus' living backdrop, between the game and the SwiftUI overlay.
    let backdrop = BackdropView(frame: NSRect(x: 0, y: 0, width: 1280, height: 800))
    private var keyMonitor: Any?

    init(model: AppModel) {
        self.model = model
        game = GameView(frame: NSRect(x: 0, y: 0, width: 1280, height: 800))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1280, height: 800),
                              styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
                              backing: .buffered, defer: false)
        window.title = "Melee"
        // The menus draw under a transparent title bar.
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.minSize = NSSize(width: 1024, height: 640)
        window.backgroundColor = NSColor(srgbRed: 5 / 255, green: 6 / 255, blue: 15 / 255, alpha: 1)
        window.tabbingMode = .disallowed
        window.collectionBehavior.insert(.fullScreenPrimary)
        super.init(window: window)
        game.model = model
        let container = NSView()
        game.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(game)
        backdrop.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(backdrop)
        overlay = OverlayHostingView(rootView: RootView(model: model))
        model.openPanel = { [weak self] in self?.openDiscPanel() }
        model.saveReplayPanel = { [weak self] in self?.saveReplay() }
        // The window sets its own size; SwiftUI's ideal size must not shrink it.
        overlay.sizingOptions = []
        overlay.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(overlay)
        for view in [game, backdrop, overlay!] {
            NSLayoutConstraint.activate([
                view.leadingAnchor.constraint(equalTo: container.leadingAnchor),
                view.trailingAnchor.constraint(equalTo: container.trailingAnchor),
                view.topAnchor.constraint(equalTo: container.topAnchor),
                view.bottomAnchor.constraint(equalTo: container.bottomAnchor),
            ])
        }
        window.contentView = container
        window.delegate = self
        window.setFrameAutosaveName("MeleeMain")
        if !window.setFrameUsingName("MeleeMain") { window.center() }

        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, let key = self.menuKey(event) else { return event }
            guard self.model.handle(key) else { return event }
            self.game.synchronize()
            return nil
        }
        model.framesChanged = { [weak self] in self?.game.synchronize() }
        model.presentFault = { [weak self] message in self?.fault(message) }
        model.screenChanged = { [weak self] screen in self?.screenChanged(screen) }
        screenChanged(model.screen)
    }
    required init?(coder: NSCoder) { fatalError("not used") }

    /// Menu keys: arrows (and WASD), Enter/Space, Esc, Tab, Q/E or [ ] for
    /// costumes, Backspace to clear, - and = for stocks. Nil while a sheet
    /// is up, with Command held, or for keys the menus do not use.
    private func menuKey(_ event: NSEvent) -> MenuKey? {
        guard let window, event.window === window, window.attachedSheet == nil,
              event.modifierFlags.intersection([.command, .control, .option]).isEmpty else { return nil }
        // During play only Esc and the pause card's keys are the menus'.
        let inPlay = model.screen == .match && !model.paused && !model.showsResults
        if inPlay && model.notice == nil { return nil }
        let shift = event.modifierFlags.contains(.shift)
        switch event.keyCode {
        case 123, 0: return .left
        case 124, 2: return .right
        case 126, 13: return .up
        case 125, 1: return .down
        case 36, 76, 49: return .enter
        case 53: return .escape
        case 48: return shift ? .backTab : .tab
        case 12, 33: return .costumePrevious
        case 14, 30: return .costumeNext
        case 51, 117: return .clear
        case 27: return .fewerStocks
        case 24: return .moreStocks
        default: return nil
        }
    }

    private func screenChanged(_ screen: Screen) {
        let inGame = screen == .match || screen == .results
        game.isHidden = !inGame
        backdrop.isHidden = inGame
        if !inGame { window?.makeFirstResponder(overlay) }
        if screen == .match {
            window?.makeFirstResponder(game)
        }
        game.synchronize()
    }

    func alert(_ title: String, _ message: String) {
        guard let window else { return }
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = .warning
        if let sheet = window.attachedSheet { window.endSheet(sheet) }
        alert.beginSheetModal(for: window)
    }

    private func fault(_ message: String) {
        guard let window else { return }
        game.synchronize()
        let alert = NSAlert()
        alert.messageText = "The match stopped on a fault"
        alert.informativeText = message
        alert.addButton(withTitle: "Quit to Character Select")
        alert.addButton(withTitle: "Save Replay…")
        alert.beginSheetModal(for: window) { [weak self] response in
            if response == .alertSecondButtonReturn {
                DispatchQueue.main.async { self?.saveReplay(); self?.model.quitToMenu() }
            } else {
                self?.model.quitToMenu()
            }
        }
    }

    @objc func openDiscPanel() {
        guard let window else { return }
        let panel = NSOpenPanel()
        panel.message = "Choose a Super Smash Bros. Melee (NTSC-U 1.02) disc image"
        panel.allowedContentTypes = ["iso", "gcm"].compactMap { UTType(filenameExtension: $0) }
        panel.allowsOtherFileTypes = true
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let url = panel.url else { return }
            self?.model.openDisc(url)
        }
    }

    @objc func saveReplay() {
        guard let window, model.core.hud != nil else { return }
        let wasPaused = model.paused
        if model.screen == .match { model.setPaused(true); game.synchronize() }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        panel.nameFieldStringValue = "melee-replay-\(stamp).json"
        panel.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            if response == .OK, let url = panel.url { self.model.saveReplay(to: url) }
            if self.model.screen == .match && !wasPaused { self.model.setPaused(wasPaused) }
            self.game.synchronize()
        }
    }

    // Menu actions
    @objc func togglePause(_ sender: Any?) { model.togglePause(); game.synchronize() }
    @objc func restartMatch(_ sender: Any?) { model.restart(); game.synchronize() }
    @objc func quitToMenu(_ sender: Any?) { model.quitToMenu() }
    @objc func saveReplayMenu(_ sender: Any?) { saveReplay() }
    @objc func openDiscMenu(_ sender: Any?) { openDiscPanel() }
    @objc func openRecentMenu(_ sender: NSMenuItem) {
        if let disc = sender.representedObject as? RecentDisc { model.openRecent(disc) }
    }

    func validate(_ item: NSMenuItem) -> Bool {
        switch item.action {
        case #selector(togglePause(_:)):
            item.title = model.paused ? "Resume" : "Pause"
            return model.screen == .match
        case #selector(restartMatch(_:)): return model.screen == .match
        case #selector(quitToMenu(_:)): return model.screen == .match || model.screen == .results
        case #selector(saveReplayMenu(_:)): return model.screen == .match || model.screen == .results
        case #selector(openDiscMenu(_:)), #selector(openRecentMenu(_:)):
            return model.screen != .match && model.screen != .loading
        default: return true
        }
    }

    func windowDidResignKey(_ notification: Notification) { model.setFocused(false); game.synchronize() }
    func windowDidBecomeKey(_ notification: Notification) {
        model.setFocused(true)
        if model.screen == .match { window?.makeFirstResponder(game) }
        game.synchronize()
    }
    func windowWillClose(_ notification: Notification) { game.shutdown() }
}

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate, NSMenuItemValidation {
    let model = AppModel()
    var controller: WindowController!
    private let recentMenu = NSMenu(title: "Open Recent")

    func applicationDidFinishLaunching(_ notification: Notification) {
        Fonts.register()
        controller = WindowController(model: model)
        NSApp.mainMenu = buildMenu()
        controller.showWindow(nil)
        NSApp.activate(ignoringOtherApps: true)
        model.sync()
        Autostart.run(model: model)
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        guard let url = urls.first else { return }
        if model.screen == .match || model.screen == .loading {
            model.present("Finish the match first", "Quit to character select before changing discs.")
            return
        }
        model.openDisc(url)
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ notification: Notification) { controller?.game.shutdown() }

    func validateMenuItem(_ item: NSMenuItem) -> Bool { controller?.validate(item) ?? false }

    func menuNeedsUpdate(_ menu: NSMenu) {
        guard menu === recentMenu else { return }
        menu.removeAllItems()
        for disc in model.recent {
            let item = NSMenuItem(title: disc.name, action: #selector(openRecent(_:)), keyEquivalent: "")
            item.representedObject = disc
            item.target = self
            menu.addItem(item)
        }
        if model.recent.isEmpty {
            menu.addItem(withTitle: "No Recent Discs", action: nil, keyEquivalent: "").isEnabled = false
        }
    }

    // Forward menu actions to the window controller.
    @objc func togglePause(_ sender: Any?) { controller.togglePause(sender) }
    @objc func restartMatch(_ sender: Any?) { controller.restartMatch(sender) }
    @objc func quitToMenu(_ sender: Any?) { controller.quitToMenu(sender) }
    @objc func saveReplayMenu(_ sender: Any?) { controller.saveReplayMenu(sender) }
    @objc func openDiscMenu(_ sender: Any?) { controller.openDiscMenu(sender) }
    @objc func openRecent(_ sender: NSMenuItem) { controller.openRecentMenu(sender) }

    private func buildMenu() -> NSMenu {
        let main = NSMenu()
        func submenu(_ title: String) -> NSMenu {
            let item = NSMenuItem()
            let menu = NSMenu(title: title)
            item.submenu = menu
            main.addItem(item)
            return menu
        }
        func add(_ menu: NSMenu, _ title: String, _ action: Selector?, _ key: String,
                 _ modifiers: NSEvent.ModifierFlags = .command) {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
            item.keyEquivalentModifierMask = modifiers
            if let action, responds(to: action) { item.target = self }
            menu.addItem(item)
        }

        let app = submenu("Melee")
        add(app, "About Melee", #selector(NSApplication.orderFrontStandardAboutPanel(_:)), "")
        app.addItem(.separator())
        let services = NSMenuItem(title: "Services", action: nil, keyEquivalent: "")
        services.submenu = NSMenu(title: "Services")
        NSApp.servicesMenu = services.submenu
        app.addItem(services)
        app.addItem(.separator())
        add(app, "Hide Melee", #selector(NSApplication.hide(_:)), "h")
        add(app, "Hide Others", #selector(NSApplication.hideOtherApplications(_:)), "h", [.command, .option])
        add(app, "Show All", #selector(NSApplication.unhideAllApplications(_:)), "")
        app.addItem(.separator())
        add(app, "Quit Melee", #selector(NSApplication.terminate(_:)), "q")

        let file = submenu("File")
        add(file, "Open Disc…", #selector(openDiscMenu(_:)), "o")
        let recent = NSMenuItem(title: "Open Recent", action: nil, keyEquivalent: "")
        recentMenu.delegate = self
        recent.submenu = recentMenu
        file.addItem(recent)
        file.addItem(.separator())
        add(file, "Save Replay…", #selector(saveReplayMenu(_:)), "s")
        file.addItem(.separator())
        add(file, "Close Window", #selector(NSWindow.performClose(_:)), "w")

        let match = submenu("Match")
        add(match, "Pause", #selector(togglePause(_:)), "p")
        add(match, "Restart", #selector(restartMatch(_:)), "r")
        add(match, "Quit to Character Select", #selector(quitToMenu(_:)), ".")

        let window = submenu("Window")
        add(window, "Minimize", #selector(NSWindow.performMiniaturize(_:)), "m")
        add(window, "Zoom", #selector(NSWindow.performZoom(_:)), "")
        add(window, "Enter Full Screen", #selector(NSWindow.toggleFullScreen(_:)), "f", [.command, .control])
        NSApp.windowsMenu = window

        let help = submenu("Help")
        add(help, "Melee Help", #selector(showHelp(_:)), "?")
        NSApp.helpMenu = help
        return main
    }

    @objc func showHelp(_ sender: Any?) {
        controller.alert("Playing", """
            Open your Super Smash Bros. Melee NTSC-U 1.02 disc image (.iso), pick two fighters \
            and a stage.

            P1: W A S D move · K attack · J special · Space jump · L shield · I grab
            P2: arrows move · N attack · M special · , jump · . shield · / grab
            Esc pauses. Gamepads are not supported yet.
            """)
    }
}

/// Development smoke test: MELEE_APP_AUTOSTART=/path/to.iso[:Fox:Marth[:FinalDestination]]
/// walks the menus; MELEE_APP_SMOKE_SECONDS=N then prints the HUD and quits.
///
/// For screenshots: MELEE_APP_WINDOW_SIZE=WxH sizes the window, and
/// MELEE_APP_SCREEN stops the walk on a screen: `disc` (the disc screen
/// with the disc open), `stages` (with iso:P1:P2), `loading` (held, with a
/// full autostart), `pause` or `results` (over the running match, after
/// MELEE_APP_SCREEN_DELAY seconds, default 1.5; results are a preview with
/// P1 winning). MELEE_APP_PREVIEW_PERCENTS=57,142 shows those HUD percents.
enum Autostart {
    static func run(model: AppModel) {
        let env = ProcessInfo.processInfo.environment
        if let size = env["MELEE_APP_WINDOW_SIZE"], let window = NSApp.windows.first {
            let parts = size.split(separator: "x").compactMap { Double($0) }
            if parts.count == 2 {
                window.setContentSize(NSSize(width: parts[0], height: parts[1]))
                window.center()
            }
        }
        if env["MELEE_APP_PRINT_WINDOW"] != nil, let window = NSApp.windows.first {
            // For screencapture -l during development.
            print("window \(window.windowNumber)")
            fflush(stdout)
        }
        if env["MELEE_APP_SCREEN"] == "disc-empty" { model.hideRecentDiscs() }
        guard let spec = env["MELEE_APP_AUTOSTART"] else { return }
        // Fewer fields stop earlier: "iso" at an empty character select, "iso:P1:P2" with the picks made.
        let parts = spec.split(separator: ":").map(String.init)
        guard (1...4).contains(parts.count), parts.count != 2 else {
            return fail("MELEE_APP_AUTOSTART wants iso[:P1:P2[:Stage]]")
        }
        model.openDisc(URL(fileURLWithPath: parts[0]))
        if env["MELEE_APP_SMOKE_SECONDS"] != nil {
            // Menu art decodes from the disc: report a portrait and a stage icon.
            let portrait = model.core.portrait(character: 0, costume: 0)
            let icon = model.core.stageIcon(0)
            print("art: ready \(model.core.artReady) portrait \(portrait.map { "\($0.width)x\($0.height)" } ?? "none") " +
                  "stage icon \(icon.map { "\($0.width)x\($0.height)" } ?? "none")")
            fflush(stdout)
            reportStagePreviews(model: model, since: Date())
        }
        let screen = env["MELEE_APP_SCREEN"]
        if let keys = env["MELEE_APP_KEYS"] { return press(keys, model: model) }
        if screen == "disc" { return model.back() }
        guard parts.count >= 3 else { return }
        for (player, key) in parts[1...2].enumerated() {
            guard let character = model.characters.first(where: { $0.key == key }) else {
                return fail("unknown character \(key); keys: \(model.characters.map(\.key))")
            }
            model.picking = player
            model.choose(character)
        }
        if screen == "stages" { return model.confirmCharacters() }
        guard parts.count == 4 else { return }
        model.confirmCharacters()
        guard let stage = model.stages.first(where: { $0.key == parts[3] }) else {
            return fail("unknown stage \(parts[3]); keys: \(model.stages.map(\.key))")
        }
        model.holdLoading = screen == "loading"
        model.previewPercents = env["MELEE_APP_PREVIEW_PERCENTS"].map {
            $0.split(separator: ",").compactMap { Float($0) }
        }
        model.chooseStage(stage)
        if screen == "pause" || screen == "results" {
            let delay = env["MELEE_APP_SCREEN_DELAY"].flatMap(Double.init) ?? 1.5
            DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
                model.setPaused(true)
                guard screen == "results", var hud = model.core.hud else { return }
                for index in hud.players.indices {
                    hud.players[index].percent = model.previewPercents?[safe: index] ?? hud.players[index].percent
                }
                hud.players[1].stocks = 0
                model.previewResults = MatchResults(winner: 0, hud: hud)
            }
        }
        guard let seconds = env["MELEE_APP_SMOKE_SECONDS"].flatMap(Double.init) else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + seconds) {
            let hud = model.core.hud
            let line = "smoke: screen \(model.core.screen) tick \(hud?.tick ?? 0) " +
                (hud?.players.map { "P\($0.port + 1) \($0.percent)% \($0.stocks) stocks" }.joined(separator: ", ") ?? "")
            print(line)
            fflush(stdout)
            exit(model.core.screen == .match && (hud?.tick ?? 0) > 0 ? 0 : 1)
        }
    }
    /// Poll until the stage previews are rendered and say how long it took.
    private static func reportStagePreviews(model: AppModel, since start: Date) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            guard model.core.stagePreviewsReady else {
                return reportStagePreviews(model: model, since: start)
            }
            let sizes = model.stages.map { stage in
                model.core.stagePreview(stage.id).map { "\($0.width)x\($0.height)" } ?? "none"
            }
            print(String(format: "previews: ready after %.2f s: %@", Date().timeIntervalSince(start),
                         sizes.joined(separator: " ")))
            fflush(stdout)
        }
    }
    /// MELEE_APP_KEYS=right,right,enter,...: menu keys fed to the model 0.25 s
    /// apart (development: the keyboard paths without synthetic events).
    private static func press(_ spec: String, model: AppModel) {
        let names: [String: MenuKey] = [
            "left": .left, "right": .right, "up": .up, "down": .down, "enter": .enter, "escape": .escape,
            "tab": .tab, "backtab": .backTab, "q": .costumePrevious, "e": .costumeNext, "clear": .clear,
            "minus": .fewerStocks, "plus": .moreStocks,
        ]
        for (index, name) in spec.split(separator: ",").enumerated() {
            guard let key = names[String(name)] else { return fail("unknown key \(name); keys: \(names.keys.sorted())") }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25 * Double(index + 1)) {
                let handled = model.handle(key)
                print("key \(name): \(handled ? "handled" : "passed") screen \(model.screen) cursor \(model.cursor) " +
                      "picking P\(model.picking + 1) stage \(model.stageCursor) focus \(model.focus)")
                fflush(stdout)
            }
        }
    }

    private static func fail(_ message: String) {
        print("autostart: \(message)")
        fflush(stdout)
        exit(2)
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? { indices.contains(index) ? self[index] : nil }
}

let application = NSApplication.shared
application.setActivationPolicy(.regular)
let delegate = AppDelegate()
application.delegate = delegate
application.run()
