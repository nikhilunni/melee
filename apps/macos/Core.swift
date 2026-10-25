import AppKit
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
    /// CPU players are witnessed here.
    let cpuSupported: Bool
}

/// What drives a slot (melee_controller_e).
enum Controller: UInt32 {
    case off = 0, human, cpu
}

struct Slot: Equatable {
    var controller: Controller = .off
    var character: Int32?
    var costume: Int = 0
    var cpuLevel: Int = 3
    /// 0 red, 1 blue, 2 green.
    var team: Int = 0
    /// 1-9, or 0 for none.
    var handicap: Int = 0
    var active: Bool { controller != .off }
}

/// Four slots, one per controller port.
let slotCount = Int(MELEE_SLOTS)

struct Selection: Equatable {
    var players: [Slot] = [Slot(controller: .human), Slot(controller: .human), Slot(), Slot()]
    var ready = false
    /// Why the picks cannot go on yet.
    var notReady: String?
}

/// How a match is won (melee_mode_e).
enum MatchMode: UInt8 { case stock = 0, time }

/// The Special Melee modes the core offers (melee_special_e).
enum SpecialMode: UInt8, CaseIterable {
    case none = 0, superSuddenDeath, lightning, slomo
    var title: String {
        switch self {
        case .none: "Off"
        case .superSuddenDeath: "Super Sudden Death"
        case .lightning: "Lightning"
        case .slomo: "Slo-Mo"
        }
    }
}

/// Versus rules (melee_rules_s).
struct Rules: Equatable {
    var mode: MatchMode = .stock
    var stocks = 4
    /// 0 for no limit (stock only).
    var timeMinutes = 0
    var teams = false
    var friendlyFire = false
    /// -1 off, 0 very low .. 4 very high.
    var itemFrequency = -1
    /// Tenths, 5-20.
    var damageRatio = 10
    var special: SpecialMode = .none
}

/// Team colours, by team number.
let teamNames = ["Red", "Blue", "Green"]

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
    var cpu = false
    /// 0 red, 1 blue, 2 green; nil outside a team battle.
    var team: Int?
    /// A Time match's KOs minus falls.
    var score = 0
}

struct Hud: Equatable {
    var tick: UInt64
    var paused: Bool
    var faulted: Bool
    var players: [PlayerHud]
    var secondsLeft: Int?
    var timedScores = false
    var suddenDeath = false
}

struct MatchResults: Equatable {
    /// Player index of the winner, or nil for a draw or a team win.
    var winner: Int?
    /// A team battle's winning team.
    var winningTeam: Int? = nil
    var hud: Hud
}

/// A gamepad as a GameCube controller (melee_gamepad_s).
struct GamepadState: Equatable {
    var buttons: UInt32 = 0
    var stick = (x: Float(0), y: Float(0))
    var cstick = (x: Float(0), y: Float(0))
    var triggers = (l: Float(0), r: Float(0))
    static func == (a: Self, b: Self) -> Bool {
        a.buttons == b.buttons && a.stick == b.stick && a.cstick == b.cstick && a.triggers == b.triggers
    }
}

struct DiscInfo: Equatable {
    var gameID: String
    var title: String
    var revision: Int
    var cachedFiles: Int
    var cachedBytes: UInt64
}

/// Menu art kinds (melee_art_e). Character kinds take a character id,
/// stage kinds a stage id.
enum ArtKind: UInt32 {
    case portrait = 0, face, stock, characterEmblem, stageIcon, stageName, stageEmblem, stagePreview
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
    let all = [raw.players.0, raw.players.1, raw.players.2, raw.players.3]
    let players = all.prefix(Int(raw.player_count)).map {
        PlayerHud(port: Int($0.port), character: Int32($0.character), costume: Int($0.costume),
                  percent: $0.percent, stocks: Int($0.stocks), cpu: $0.cpu,
                  team: $0.team < 0 ? nil : Int($0.team), score: Int($0.score))
    }
    return Hud(tick: raw.tick, paused: raw.paused, faulted: raw.faulted, players: Array(players),
               secondsLeft: raw.seconds_left < 0 ? nil : Int(raw.seconds_left),
               timedScores: raw.timed_scores, suddenDeath: raw.sudden_death)
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
        return StageInfo(id: id, name: String(cString: info.name), key: String(cString: info.key),
                         cpuSupported: info.cpu_supported)
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
    func openDisc(path: String) throws {
        artCache.removeAll()
        try check(melee_app_open_disc(app, path))
    }
    func resumeDisc() throws { try check(melee_app_resume_disc(app)) }
    var disc: DiscInfo? {
        var info = melee_disc_info_s()
        guard melee_app_disc_info(app, &info) else { return nil }
        return DiscInfo(gameID: text(info.game_id), title: text(info.title), revision: Int(info.revision),
                        cachedFiles: Int(info.cached_files), cachedBytes: info.cached_bytes)
    }

    // Menu art, decoded by the core from the open disc. Images are at their
    // native size (portrait 136x188, face 64x56, stock 24x24, emblem 80x64,
    // stage icon 64x56, stage name 224x56); draw them with
    // `.interpolation(.none)` to keep the pixels. nil until the art is read,
    // and where retail has none (Sheik's portrait and face).
    private struct ArtKey: Hashable { let kind: ArtKind; let id: UInt32; let costume: UInt8 }
    private var artCache: [ArtKey: CGImage] = [:]

    var artReady: Bool { melee_app_art_ready(app) }
    var artProgress: LoadProgress {
        var raw = melee_load_progress_s()
        melee_app_art_progress(app, &raw)
        return LoadProgress(filesDone: Int(raw.files_done), filesTotal: Int(raw.files_total),
                            bytesDone: raw.bytes_done, bytesTotal: raw.bytes_total)
    }
    /// Read the menu archives again (open-disc already tries once).
    func loadArt() throws { try check(melee_app_load_art(app)) }

    func art(_ kind: ArtKind, id: UInt32, costume: Int = 0) -> CGImage? {
        let key = ArtKey(kind: kind, id: id, costume: UInt8(clamping: costume))
        if let cached = artCache[key] { return cached }
        var raw = melee_image_s()
        guard melee_app_art(app, melee_art_e(rawValue: kind.rawValue), id, key.costume, &raw),
              let pixels = raw.rgba else { return nil }
        // Copy: the core's pointer is valid only until the next call.
        let data = Data(bytes: pixels, count: raw.len) as CFData
        guard let provider = CGDataProvider(data: data),
              let image = CGImage(width: Int(raw.width), height: Int(raw.height),
                                  bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: Int(raw.width) * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
                                  provider: provider, decode: nil,
                                  // Stage previews are rendered pictures: smooth when scaled.
                                  shouldInterpolate: kind == .stagePreview,
                                  intent: .defaultIntent)
        else { return nil }
        artCache[key] = image
        return image
    }
    func artImage(_ kind: ArtKind, id: UInt32, costume: Int = 0) -> NSImage? {
        art(kind, id: id, costume: costume).map { NSImage(cgImage: $0, size: NSSize(width: $0.width, height: $0.height)) }
    }
    func portrait(character: Int32, costume: Int) -> CGImage? { art(.portrait, id: UInt32(character), costume: costume) }
    func face(character: Int32) -> CGImage? { art(.face, id: UInt32(character)) }
    func stockIcon(character: Int32, costume: Int) -> CGImage? { art(.stock, id: UInt32(character), costume: costume) }
    func emblem(character: Int32) -> CGImage? { art(.characterEmblem, id: UInt32(character)) }
    func stageIcon(_ stage: UInt32) -> CGImage? { art(.stageIcon, id: stage) }
    func stageName(_ stage: UInt32) -> CGImage? { art(.stageName, id: stage) }
    func stageEmblem(_ stage: UInt32) -> CGImage? { art(.stageEmblem, id: stage) }

    // Stage previews (1920x1080, opaque), rendered by the core from the disc
    // in the background once it is open; nil until each is ready. Poll when
    // redrawing and show a placeholder meanwhile.
    var stagePreviewsReady: Bool { melee_app_stage_previews_ready(app) }
    func stagePreview(_ stage: UInt32) -> CGImage? { art(.stagePreview, id: stage) }

    /// A reason string the core copies out (0 length: none).
    private func reason(_ copy: (UnsafeMutablePointer<CChar>?, Int) -> Int) -> String? {
        var buffer = [CChar](repeating: 0, count: 256)
        let length = copy(&buffer, buffer.count)
        return length == 0 ? nil : String(cString: buffer)
    }

    // Character select
    var selection: Selection {
        var raw = melee_selection_s()
        melee_app_selection(app, &raw)
        let all: [melee_slot_s] = [raw.players.0, raw.players.1, raw.players.2, raw.players.3]
        let slots = all.map { (s: melee_slot_s) -> Slot in
            let character: Int32? = s.character < 0 ? nil : s.character
            return Slot(controller: Controller(rawValue: s.controller) ?? .off, character: character,
                        costume: Int(s.costume), cpuLevel: Int(s.cpu_level), team: Int(s.team),
                        handicap: Int(s.handicap))
        }
        let notReady = reason { melee_app_characters_not_ready(app, $0, $1) }
        return Selection(players: slots, ready: raw.ready, notReady: notReady)
    }
    func setController(player: Int, _ controller: Controller) throws {
        try check(melee_app_set_controller(app, UInt32(player), melee_controller_e(rawValue: controller.rawValue)))
    }
    func setCpuLevel(player: Int, level: Int) throws {
        try check(melee_app_set_cpu_level(app, UInt32(player), UInt8(clamping: level)))
    }
    func setTeam(player: Int, team: Int) throws {
        try check(melee_app_set_team(app, UInt32(player), UInt8(clamping: team)))
    }
    func setHandicap(player: Int, handicap: Int) throws {
        try check(melee_app_set_handicap(app, UInt32(player), UInt8(clamping: handicap)))
    }
    var rules: Rules {
        var raw = melee_rules_s()
        melee_app_rules(app, &raw)
        return Rules(mode: MatchMode(rawValue: raw.mode) ?? .stock, stocks: Int(raw.stocks),
                     timeMinutes: Int(raw.time_minutes), teams: raw.teams, friendlyFire: raw.friendly_fire,
                     itemFrequency: Int(raw.item_frequency), damageRatio: Int(raw.damage_ratio_tenths),
                     special: SpecialMode(rawValue: raw.special) ?? .none)
    }
    func setRules(_ rules: Rules) throws {
        var raw = melee_rules_s(mode: rules.mode.rawValue, stocks: UInt8(clamping: rules.stocks),
                                time_minutes: UInt8(clamping: rules.timeMinutes), teams: rules.teams,
                                friendly_fire: rules.friendlyFire, item_frequency: Int8(clamping: rules.itemFrequency),
                                damage_ratio_tenths: UInt8(clamping: rules.damageRatio),
                                special: rules.special.rawValue)
        try check(melee_app_set_rules(app, &raw))
    }
    /// Why a stage cannot host the current picks, or nil.
    func stageUnavailable(_ stage: UInt32) -> String? {
        reason { melee_app_stage_unavailable(app, stage, $0, $1) }
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
    /// Gamepad `device` (connection order) drives the match's nth human; nil
    /// when it disconnects.
    func gamepad(device: Int, _ state: GamepadState?) {
        guard let state else { return melee_app_gamepad(app, UInt32(device), nil) }
        var raw = melee_gamepad_s(buttons: state.buttons, stick_x: state.stick.x, stick_y: state.stick.y,
                                  cstick_x: state.cstick.x, cstick_y: state.cstick.y,
                                  trigger_l: state.triggers.l, trigger_r: state.triggers.r)
        melee_app_gamepad(app, UInt32(device), &raw)
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
        return MatchResults(winner: raw.winner < 0 ? nil : Int(raw.winner),
                            winningTeam: raw.winning_team < 0 ? nil : Int(raw.winning_team), hud: makeHud(raw.hud))
    }
    func restart() throws { try check(melee_app_restart(app)) }
    func rematch() throws { try check(melee_app_rematch(app, UInt32.random(in: .min ... .max))) }
    func quitToMenu() { melee_app_quit_to_menu(app) }
    func saveReplay(to path: String) throws { try check(melee_app_save_replay(app, path)) }
}
