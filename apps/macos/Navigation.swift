import AppKit

// Keyboard navigation for the menus (DESIGN.md "Keyboard first"). The
// window turns key presses into MenuKeys; each screen moves its cursor or
// focus and Enter does what a click on the focused element would.

/// The buttons of the disc screen, in focus order.
enum DiscAction: Hashable {
    case open
    case resume(String)
    case recent(RecentDisc)
}

/// The pause card's buttons, in focus order.
enum PauseAction: CaseIterable { case resume, restart, saveReplay, quit }

/// The results screen's buttons, in focus order.
enum ResultsAction: CaseIterable { case rematch, characters, stages, saveReplay }

extension AppModel {
    var discActions: [DiscAction] {
        var actions: [DiscAction] = [.open]
        if let disc { actions.append(.resume(disc.gameID)) }
        // The disc already open is the Continue button; offer the others.
        for item in recent.prefix(disc == nil ? 2 : 1) where disc == nil || item != recent.first {
            actions.append(.recent(item))
        }
        return actions
    }

    func perform(_ action: DiscAction) {
        switch action {
        case .open: openPanel?()
        case .resume: resumeDisc()
        case .recent(let disc): openRecent(disc)
        }
    }

    func perform(_ action: PauseAction) {
        switch action {
        case .resume: setPaused(false)
        case .restart: restart()
        case .saveReplay: saveReplayPanel?()
        case .quit: quitToMenu()
        }
    }

    func perform(_ action: ResultsAction) {
        switch action {
        case .rematch: rematch()
        case .characters: quitToMenu()
        case .stages: toStageSelect()
        case .saveReplay: saveReplayPanel?()
        }
    }

    var showsResults: Bool { screen == .results || previewResults != nil }

    /// Handle a menu key; false lets the key through (to the game).
    func handle(_ key: MenuKey) -> Bool {
        if notice != nil {
            if key == .enter || key == .escape { notice = nil; return true }
            if screen == .match && !paused { return false }
        }
        if showsResults { return resultsKey(key) }
        switch screen {
        case .disc: return buttonKey(key, count: discActions.count) { perform(discActions[$0]) }
        case .characters: return characterKey(key)
        case .stages: return stageKey(key)
        case .loading:
            if key == .escape { back(); return true }
            return false
        case .match:
            guard paused else { return false }
            if key == .escape { setPaused(false); return true }
            return buttonKey(key, count: PauseAction.allCases.count) { perform(PauseAction.allCases[$0]) }
        case .results: return resultsKey(key)
        }
    }

    private func resultsKey(_ key: MenuKey) -> Bool {
        if key == .escape { quitToMenu(); return true }
        return buttonKey(key, count: ResultsAction.allCases.count) { perform(ResultsAction.allCases[$0]) }
    }

    /// Arrow/Tab focus over a row or column of buttons; Enter presses.
    private func buttonKey(_ key: MenuKey, count: Int, press: (Int) -> Void) -> Bool {
        guard count > 0 else { return false }
        switch key {
        case .left, .up, .backTab: focus = (focus + count - 1) % count
        case .right, .down, .tab: focus = (focus + 1) % count
        case .enter: press(min(focus, count - 1))
        default: return false
        }
        return true
    }

    // MARK: Character select

    /// Visual column of a grid cell: the 7-wide bottom row is centred under the 9s.
    private func visualColumn(_ character: CharacterInfo) -> Int {
        let rowLength = characters.filter { $0.row == character.row }.count
        let widest = characters.reduce(into: [Int: Int]()) { $0[$1.row, default: 0] += 1 }.values.max() ?? rowLength
        return character.column + (widest - rowLength) / 2
    }

    private func moveCursor(dx: Int, dy: Int) {
        guard let current = character(cursor) ?? characters.first else { return }
        if dx != 0 {
            let row = characters.filter { $0.row == current.row }.sorted { $0.column < $1.column }
            guard let index = row.firstIndex(of: current) else { return }
            cursor = row[(index + dx + row.count) % row.count].id
            return
        }
        let rows = Set(characters.map(\.row)).sorted()
        guard let rowIndex = rows.firstIndex(of: current.row) else { return }
        let target = rows[(rowIndex + dy + rows.count) % rows.count]
        let column = visualColumn(current)
        let candidates = characters.filter { $0.row == target }
        if let best = candidates.min(by: { abs(visualColumn($0) - column) < abs(visualColumn($1) - column) }) {
            cursor = best.id
        }
    }

    private func characterKey(_ key: MenuKey) -> Bool {
        switch key {
        case .left: moveCursor(dx: -1, dy: 0)
        case .right: moveCursor(dx: 1, dy: 0)
        case .up: moveCursor(dx: 0, dy: -1)
        case .down: moveCursor(dx: 0, dy: 1)
        case .enter:
            if selection.ready {
                confirmCharacters()
            } else if let character = character(cursor) {
                choose(character)
            }
        case .tab, .backTab: activate(player: 1 - picking)
        case .costumePrevious, .costumeNext:
            let player = selection.players[picking].character != nil ? picking : 1 - picking
            guard selection.players[player].character != nil else { return true }
            cycleCostume(player: player, step: key == .costumeNext ? 1 : -1)
        case .clear: clear(player: picking)
        case .fewerStocks: setStocks(max(1, selection.stocks - 1))
        case .moreStocks: setStocks(min(99, selection.stocks + 1))
        case .escape: back()
        }
        return true
    }

    // MARK: Stage select

    /// Stages, then the Random tile.
    var stageTileCount: Int { stages.count + 1 }

    func activateStageTile(_ index: Int) {
        if index < stages.count {
            chooseStage(stages[index])
        } else {
            chooseRandomStage(reduceMotion: NSWorkspace.shared.accessibilityDisplayShouldReduceMotion)
        }
    }

    private func stageKey(_ key: MenuKey) -> Bool {
        guard shuffling == nil else { return true }
        switch key {
        case .left, .up, .backTab: stageCursor = (stageCursor + stageTileCount - 1) % stageTileCount
        case .right, .down, .tab: stageCursor = (stageCursor + 1) % stageTileCount
        case .enter: activateStageTile(stageCursor)
        case .escape: back()
        default: return false
        }
        return true
    }
}
