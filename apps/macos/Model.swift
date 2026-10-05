import AppKit
import Combine

/// A message for the user, shown as a glass card (DESIGN.md "Errors and notices").
struct Notice: Identifiable, Equatable {
    let id = UUID()
    var title: String
    var message: String
}

/// Keys the menus understand; the window maps key codes to these.
enum MenuKey {
    case left, right, up, down, enter, escape, tab, backTab, costumePrevious, costumeNext, clear, fewerStocks,
         moreStocks
}

/// Observable mirror of the core for the SwiftUI menus. It forwards intent
/// to the core and re-reads the core's state after every call.
final class AppModel: ObservableObject {
    let core = Core()
    let characters = Core.characters
    let stages = Core.stages

    @Published private(set) var screen: Screen = .disc
    @Published private(set) var selection = Selection()
    @Published private(set) var progress = LoadProgress()
    @Published private(set) var hud: Hud?
    @Published private(set) var results: MatchResults?
    @Published private(set) var disc: DiscInfo?
    @Published private(set) var paused = false
    @Published private(set) var recent: [RecentDisc] = RecentDiscs.load()
    /// The player whose pick the next character click sets.
    @Published var picking = 0
    /// The card on top of the menus, if any.
    @Published var notice: Notice?

    // Menu navigation: the character grid cursor (a character id), the
    // stage row cursor (stages, then Random) and the focused button of
    // button-only screens (disc, pause, results).
    @Published var cursor: Int32 = 0
    @Published var stageCursor = 0
    @Published var focus = 0
    /// Bumped when runtime-rendered art (stage previews) becomes available.
    @Published private(set) var artGeneration = 0
    /// The stage the Random tile is shuffling through, while it shuffles.
    @Published private(set) var shuffling: Int?
    /// The stage of the match being loaded or played (loading shows it).
    @Published private(set) var lastStage: UInt32?

    /// Errors and notices: a card on top of the menus.
    lazy var present: (String, String) -> Void = { [weak self] title, message in
        self?.notice = Notice(title: title, message: message)
    }
    /// A match fault: the window offers to save the replay.
    var presentFault: ((String) -> Void)?
    var screenChanged: ((Screen) -> Void)?
    /// Pausing or resuming changes whether the match needs display frames.
    var framesChanged: (() -> Void)?
    var openPanel: (() -> Void)?
    var saveReplayPanel: (() -> Void)?

    // Development only (screenshots): hold the loading screen, and show
    // results or HUD percents over a running match.
    var holdLoading = false
    @Published var previewResults: MatchResults?
    var previewPercents: [Float]?

    private var loadScheduled = false
    private var previewTimer: Timer?

    func character(_ id: Int32?) -> CharacterInfo? {
        guard let id else { return nil }
        return characters.first { $0.id == id }
    }

    func sync() {
        let previous = screen
        screen = core.screen
        selection = core.selection
        disc = core.disc
        progress = core.progress
        results = core.results
        hud = core.hud
        if screen != .match { paused = false }
        if let notice = core.takeNotice() {
            present("Melee", notice)
        }
        if screen == .loading && !loadScheduled && !holdLoading {
            loadScheduled = true
            DispatchQueue.main.async { self.pumpLoading() }
        }
        if previous != screen {
            enter(screen)
            screenChanged?(screen)
        }
    }

    /// Reset navigation for a newly shown screen.
    private func enter(_ screen: Screen) {
        focus = 0
        shuffling = nil
        if screen == .characters, let pick = selection.players[picking].character { cursor = pick }
        if screen == .stages { pollPreviews() } else { previewTimer?.invalidate(); previewTimer = nil }
    }

    /// Stage previews arrive after the menus open; poll until all are drawn.
    private func pollPreviews() {
        previewTimer?.invalidate()
        previewTimer = nil
        guard !stages.allSatisfy({ core.stagePreview($0.id) != nil }) else { return }
        var seen = 0
        previewTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] timer in
            guard let self else { return timer.invalidate() }
            let ready = self.stages.filter { self.core.stagePreview($0.id) != nil }.count
            if ready != seen { seen = ready; self.artGeneration &+= 1 }
            if ready == self.stages.count || self.screen != .stages {
                timer.invalidate()
                self.previewTimer = nil
            }
        }
    }

    private func perform(_ title: String = "Melee", _ body: () throws -> Void) {
        do { try body() } catch { present(title, error.localizedDescription) }
        sync()
    }

    // MARK: Disc

    func openDisc(_ url: URL) {
        let accessing = url.startAccessingSecurityScopedResource()
        defer { if accessing { url.stopAccessingSecurityScopedResource() } }
        notice = nil
        do {
            try core.openDisc(path: url.path)
            RecentDiscs.add(url)
            recent = RecentDiscs.load()
        } catch {
            present("That disc image cannot be used", error.localizedDescription)
        }
        sync()
    }
    func openRecent(_ disc: RecentDisc) {
        guard let url = disc.resolve() else {
            present("Disc not found", "\(disc.name) has moved or been deleted.")
            RecentDiscs.remove(disc)
            recent = RecentDiscs.load()
            return
        }
        openDisc(url)
    }
    /// Development (screenshots of a first launch): show no recent discs.
    func hideRecentDiscs() { recent = [] }
    func resumeDisc() { perform { try core.resumeDisc() } }

    // MARK: Character select

    func choose(_ character: CharacterInfo) {
        cursor = character.id
        perform { try core.choose(player: picking, character: character.id) }
        // Move the cursor to the other player while they still need a pick.
        if selection.players[1 - picking].character == nil { picking = 1 - picking }
    }
    func clear(player: Int) {
        perform { try core.choose(player: player, character: nil) }
        picking = player
    }
    /// Make a player the one picking; the cursor jumps to their fighter.
    func activate(player: Int) {
        picking = player
        if let pick = selection.players[player].character { cursor = pick }
    }
    func setCostume(player: Int, costume: Int) { perform { try core.setCostume(player: player, costume: costume) } }
    func cycleCostume(player: Int, step: Int32) { perform { try core.cycleCostume(player: player, step: step) } }
    func setStocks(_ stocks: Int) { perform { try core.setStocks(stocks) } }
    func confirmCharacters() { perform { try core.confirmCharacters() } }
    func back() { core.back(); sync() }

    // MARK: Stage select and loading

    func chooseStage(_ stage: StageInfo) {
        lastStage = stage.id
        perform { try core.chooseStage(stage.id) }
    }

    /// The Random tile: a quick shuffle along the row, then the pick.
    func chooseRandomStage(reduceMotion: Bool) {
        guard shuffling == nil, !stages.isEmpty else { return }
        let target = Int.random(in: 0..<stages.count)
        if reduceMotion { return chooseStage(stages[target]) }
        let steps = stages.count + target
        for step in 0...steps {
            // Ease out: each hop a little slower than the last; under a second.
            let t = Double(step) / Double(steps)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.45 * t * t + 0.25 * t) { [weak self] in
                guard let self, self.screen == .stages else { return }
                self.shuffling = step % self.stages.count
                self.stageCursor = step % self.stages.count
                guard step == steps else { return }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                    guard self.screen == .stages else { return }
                    self.shuffling = nil
                    self.chooseStage(self.stages[target])
                }
            }
        }
    }
    /// From results: back to stage select with the same picks.
    func toStageSelect() {
        previewResults = nil
        core.quitToMenu()
        perform { try core.confirmCharacters() }
    }

    /// Read files in slices of ~16 ms so the progress bar keeps moving.
    private func pumpLoading() {
        loadScheduled = false
        if holdLoading { return }
        guard core.screen == .loading else { return sync() }
        let start = Date()
        do {
            while try core.loadStep() {
                if Date().timeIntervalSince(start) > 0.016 {
                    progress = core.progress
                    loadScheduled = true
                    DispatchQueue.main.async { self.pumpLoading() }
                    return
                }
            }
            progress = core.progress
            // Let the full bar draw before the match build blocks briefly.
            DispatchQueue.main.async {
                do { try self.core.finishLoading() } catch {}
                self.sync()
            }
        } catch {
            sync()
        }
    }

    // MARK: Match

    /// One display frame: advance, draw, refresh the HUD.
    func frame(width: UInt32, height: UInt32) {
        guard core.screen == .match else { return }
        do {
            try core.frame(width: width, height: height)
        } catch {
            let message = core.takeNotice() ?? error.localizedDescription
            hud = core.hud
            presentFault?(message)
            return
        }
        // Republish only what the HUD draws (not the tick): the overlay then
        // re-renders on damage and stock changes, not every frame.
        let next = core.hud
        if next?.players != hud?.players || next?.paused != hud?.paused || next?.faulted != hud?.faulted {
            hud = next
        }
        if core.screen != screen { sync() }
    }
    var needsFrame: Bool { core.needsFrame }
    func action(player: Int, _ action: GameAction, down: Bool) { core.action(player: player, action, down: down) }
    func setPaused(_ value: Bool) {
        guard screen == .match else { return }
        core.setPaused(value)
        paused = value
        hud = core.hud
        framesChanged?()
    }
    func togglePause() { setPaused(!paused) }
    func setFocused(_ focused: Bool) { core.setFocused(focused) }
    func restart() { perform { try core.restart() }; paused = false; framesChanged?() }
    func rematch() { previewResults = nil; perform { try core.rematch() } }
    func quitToMenu() { previewResults = nil; core.quitToMenu(); sync() }
    func saveReplay(to url: URL) { perform("Could not save the replay") { try core.saveReplay(to: url.path) } }
}

/// Discs opened before, as bookmarks so they survive renames and moves.
struct RecentDisc: Identifiable, Hashable {
    let bookmark: Data
    let name: String
    var id: Data { bookmark }

    func resolve() -> URL? {
        var stale = false
        for options: URL.BookmarkResolutionOptions in [.withSecurityScope, []] {
            if let url = try? URL(resolvingBookmarkData: bookmark, options: options, relativeTo: nil,
                                  bookmarkDataIsStale: &stale),
               FileManager.default.fileExists(atPath: url.path) {
                return url
            }
        }
        return nil
    }
}

enum RecentDiscs {
    private static let key = "recentDiscs"
    private static let limit = 5

    static func load() -> [RecentDisc] {
        let entries = UserDefaults.standard.array(forKey: key) as? [[String: Any]] ?? []
        return entries.compactMap { entry in
            guard let data = entry["bookmark"] as? Data, let name = entry["name"] as? String else { return nil }
            return RecentDisc(bookmark: data, name: name)
        }
    }
    private static func store(_ discs: [RecentDisc]) {
        UserDefaults.standard.set(discs.prefix(limit).map { ["bookmark": $0.bookmark, "name": $0.name] },
                                  forKey: key)
    }
    static func add(_ url: URL) {
        let bookmark = (try? url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil,
                                              relativeTo: nil))
            ?? (try? url.bookmarkData())
        guard let bookmark else { return }
        let others = load().filter { $0.resolve()?.standardizedFileURL != url.standardizedFileURL }
        store([RecentDisc(bookmark: bookmark, name: url.lastPathComponent)] + others)
    }
    static func remove(_ disc: RecentDisc) {
        store(load().filter { $0 != disc })
    }
}
