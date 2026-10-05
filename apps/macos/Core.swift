import Foundation

// Swift face of libmelee (crates/melee-platform/include/melee_platform.h).
// Every call goes to the shared Rust core; nothing here decides game or
// menu rules.

enum Screen: UInt32 {
    case disc = 0, characters, stages, loading, match, results
}

enum GameAction: UInt32 {
    case left = 0, right, up, down, attack, special, jump, shield, grab
}

struct CharacterInfo: Identifiable, Hashable {
    let id: Int32
    let name: String
    let key: String
    let costumeCount: Int
    let row: Int
    let column: Int
}

struct StageInfo: Identifiable, Hashable {
    let id: UInt32
    let name: String
    let key: String
}

struct Slot: Equatable {
    var character: Int32?
    var costume: Int
}

struct Selection: Equatable {
    var players: [Slot] = [Slot(character: nil, costume: 0), Slot(character: nil, costume: 0)]
    var stocks: Int = 4
    var ready = false
}

struct LoadProgress: Equatable {
    var filesDone = 0, filesTotal = 0
    var bytesDone: UInt64 = 0, bytesTotal: UInt64 = 0
    var fraction: Double { bytesTotal == 0 ? 1 : Double(bytesDone) / Double(bytesTotal) }
}

struct PlayerHud: Equatable {
    var port: Int
    var character: Int32
    var costume: Int
    var percent: Float
    var stocks: Int
}

struct Hud: Equatable {
    var tick: UInt64
    var paused: Bool
    var faulted: Bool
    var players: [PlayerHud]
}

struct MatchResults: Equatable {
    /// Player index of the winner, or nil for a draw.
    var winner: Int?
    var hud: Hud
}

struct DiscInfo: Equatable {
    var gameID: String
    var title: String
    var revision: Int
    var cachedFiles: Int
    var cachedBytes: UInt64
}

struct CoreError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

private func text<T>(_ tuple: T) -> String {
    withUnsafeBytes(of: tuple) { raw in
        let bytes = raw.prefix { $0 != 0 }
        return String(decoding: bytes, as: UTF8.self)
    }
}

private func makeHud(_ raw: melee_hud_s) -> Hud {
    let players = [raw.players.0, raw.players.1].map {
        PlayerHud(port: Int($0.port), character: Int32($0.character), costume: Int($0.costume),
                  percent: $0.percent, stocks: Int($0.stocks))
    }
    return Hud(tick: raw.tick, paused: raw.paused, faulted: raw.faulted, players: players)
}

final class Core {
    private let app: OpaquePointer

    static let characters: [CharacterInfo] = (0..<melee_character_count()).compactMap { id in
        var info = melee_character_info_s()
        guard melee_character_info(id, &info) else { return nil }
        return CharacterInfo(id: Int32(id), name: String(cString: info.name), key: String(cString: info.key),
                             costumeCount: Int(info.costume_count), row: Int(info.grid_row),
                             column: Int(info.grid_column))
    }
    static let stages: [StageInfo] = (0..<melee_stage_count()).compactMap { id in
        var info = melee_stage_info_s()
        guard melee_stage_info(id, &info) else { return nil }
        return StageInfo(id: id, name: String(cString: info.name), key: String(cString: info.key))
    }

    init() {
        precondition(melee_api_version() == UInt32(MELEE_API_VERSION), "libmelee API version mismatch")
        app = melee_app_new()
    }
    deinit { melee_app_free(app) }

    private func lastError() -> String {
        let length = melee_app_last_error(app, nil, 0)
        var buffer = [CChar](repeating: 0, count: length + 1)
        melee_app_last_error(app, &buffer, buffer.count)
        return String(cString: buffer)
    }
    private func check(_ ok: Bool) throws {
        if !ok { throw CoreError(message: lastError()) }
    }

    var screen: Screen { Screen(rawValue: melee_app_screen(app).rawValue) ?? .disc }

    func takeNotice() -> String? {
        var buffer = [CChar](repeating: 0, count: 8192)
        let length = melee_app_take_notice(app, &buffer, buffer.count)
        return length == 0 ? nil : String(cString: buffer)
    }

    // Disc
    func openDisc(path: String) throws { try check(melee_app_open_disc(app, path)) }
    func resumeDisc() throws { try check(melee_app_resume_disc(app)) }
    var disc: DiscInfo? {
        var info = melee_disc_info_s()
        guard melee_app_disc_info(app, &info) else { return nil }
        return DiscInfo(gameID: text(info.game_id), title: text(info.title), revision: Int(info.revision),
                        cachedFiles: Int(info.cached_files), cachedBytes: info.cached_bytes)
    }

    // Character select
    var selection: Selection {
        var raw = melee_selection_s()
        melee_app_selection(app, &raw)
        let slots = [raw.players.0, raw.players.1].map {
            Slot(character: $0.character < 0 ? nil : $0.character, costume: Int($0.costume))
        }
        return Selection(players: slots, stocks: Int(raw.stocks), ready: raw.ready)
    }
    func choose(player: Int, character: Int32?) throws {
        try check(melee_app_choose_character(app, UInt32(player), character ?? -1))
    }
    func setCostume(player: Int, costume: Int) throws {
        try check(melee_app_set_costume(app, UInt32(player), UInt8(costume)))
    }
    func cycleCostume(player: Int, step: Int32) throws {
        try check(melee_app_cycle_costume(app, UInt32(player), step))
    }
    func setStocks(_ stocks: Int) throws { try check(melee_app_set_stocks(app, UInt8(clamping: stocks))) }
    func confirmCharacters() throws { try check(melee_app_confirm_characters(app)) }
    func back() { melee_app_back(app) }

    // Stage select and loading
    func chooseStage(_ stage: UInt32) throws {
        try check(melee_app_choose_stage(app, stage, UInt32.random(in: .min ... .max)))
    }
    var progress: LoadProgress {
        var raw = melee_load_progress_s()
        melee_app_load_progress(app, &raw)
        return LoadProgress(filesDone: Int(raw.files_done), filesTotal: Int(raw.files_total),
                            bytesDone: raw.bytes_done, bytesTotal: raw.bytes_total)
    }
    /// Read one file; true while more remain.
    func loadStep() throws -> Bool {
        let result = melee_app_load_step(app)
        if result < 0 { throw CoreError(message: lastError()) }
        return result > 0
    }
    func finishLoading() throws { try check(melee_app_finish_loading(app)) }

    // Surface and match
    func attach(layer: UnsafeMutableRawPointer, width: UInt32, height: UInt32) throws {
        try check(melee_app_attach_metal_layer(app, layer, width, height))
    }
    func detachSurface() { melee_app_detach_surface(app) }
    /// Advance and draw one display frame; throws the fault message.
    func frame(width: UInt32, height: UInt32) throws { try check(melee_app_frame(app, width, height)) }
    var needsFrame: Bool { melee_app_needs_frame(app) }
    func action(player: Int, _ action: GameAction, down: Bool) {
        melee_app_action(app, UInt32(player), melee_action_e(rawValue: action.rawValue), down)
    }
    func setPaused(_ paused: Bool) { melee_app_set_paused(app, paused) }
    func setFocused(_ focused: Bool) { melee_app_set_focused(app, focused) }
    var hud: Hud? {
        var raw = melee_hud_s()
        return melee_app_hud(app, &raw) ? makeHud(raw) : nil
    }
    var results: MatchResults? {
        var raw = melee_results_s()
        guard melee_app_results(app, &raw) else { return nil }
        return MatchResults(winner: raw.winner < 0 ? nil : Int(raw.winner), hud: makeHud(raw.hud))
    }
    func restart() throws { try check(melee_app_restart(app)) }
    func rematch() throws { try check(melee_app_rematch(app, UInt32.random(in: .min ... .max))) }
    func quitToMenu() { melee_app_quit_to_menu(app) }
    func saveReplay(to path: String) throws { try check(melee_app_save_replay(app, path)) }
}
