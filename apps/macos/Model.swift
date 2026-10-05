import AppKit
import Combine

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

    /// Errors and notices for the window to show as alerts.
    var present: ((String, String) -> Void)?
    /// A match fault: the window offers to save the replay.
    var presentFault: ((String) -> Void)?
    var screenChanged: ((Screen) -> Void)?

    private var loadScheduled = false

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
            present?("Melee", notice)
        }
        if screen == .loading && !loadScheduled {
            loadScheduled = true
            DispatchQueue.main.async { self.pumpLoading() }
        }
        if previous != screen { screenChanged?(screen) }
    }

    private func perform(_ title: String = "Melee", _ body: () throws -> Void) {
        do { try body() } catch { present?(title, error.localizedDescription) }
        sync()
    }

    // MARK: Disc

    func openDisc(_ url: URL) {
        let accessing = url.startAccessingSecurityScopedResource()
        defer { if accessing { url.stopAccessingSecurityScopedResource() } }
        do {
            try core.openDisc(path: url.path)
            RecentDiscs.add(url)
            recent = RecentDiscs.load()
        } catch {
            present?("That disc image cannot be used", error.localizedDescription)
        }
        sync()
    }
    func openRecent(_ disc: RecentDisc) {
        guard let url = disc.resolve() else {
            present?("Disc not found", "\(disc.name) has moved or been deleted.")
            RecentDiscs.remove(disc)
            recent = RecentDiscs.load()
            return
        }
        openDisc(url)
    }
    func resumeDisc() { perform { try core.resumeDisc() } }

    // MARK: Character select

    func choose(_ character: CharacterInfo) {
        perform { try core.choose(player: picking, character: character.id) }
        // Move the cursor to the other player while they still need a pick.
        if selection.players[1 - picking].character == nil { picking = 1 - picking }
    }
    func clear(player: Int) {
        perform { try core.choose(player: player, character: nil) }
        picking = player
    }
    func setCostume(player: Int, costume: Int) { perform { try core.setCostume(player: player, costume: costume) } }
    func cycleCostume(player: Int, step: Int32) { perform { try core.cycleCostume(player: player, step: step) } }
    func setStocks(_ stocks: Int) { perform { try core.setStocks(stocks) } }
    func confirmCharacters() { perform { try core.confirmCharacters() } }
    func back() { core.back(); sync() }

    // MARK: Stage select and loading

    func chooseStage(_ stage: StageInfo) { perform { try core.chooseStage(stage.id) } }

    /// Read files in slices of ~16 ms so the progress bar keeps moving.
    private func pumpLoading() {
        loadScheduled = false
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
        let next = core.hud
        if next != hud { hud = next }
        if core.screen != screen { sync() }
    }
    var needsFrame: Bool { core.needsFrame }
    func action(player: Int, _ action: GameAction, down: Bool) { core.action(player: player, action, down: down) }
    func setPaused(_ value: Bool) {
        guard screen == .match else { return }
        core.setPaused(value)
        paused = value
        hud = core.hud
    }
    func togglePause() { setPaused(!paused) }
    func setFocused(_ focused: Bool) { core.setFocused(focused) }
    func restart() { perform { try core.restart() }; paused = false }
    func rematch() { perform { try core.rematch() } }
    func quitToMenu() { core.quitToMenu(); sync() }
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
