import AppKit

// Keyboard and pad navigation for the menus (DESIGN.md "Keyboard first",
// docs/APP.md "Pad parity"). The window turns key presses and pad menu
// events into MenuKeys; arrows move focus between the controls of the top
// layer (Focus.swift), Enter or A presses the focused one, and each screen
// adds its own keys (costumes, players, stocks, back).

/// The buttons of the disc screen, in order.
enum DiscAction: Hashable {
    case open
    case resume(String)
    case recent(RecentDisc)
}

/// The pause card's buttons, in order.
enum PauseAction: CaseIterable { case resume, restart, saveReplay, quit }

/// The results screen's buttons, in order.
enum ResultsAction: CaseIterable { case rematch, characters, stages, saveReplay }

/// The app menu's items: the menu bar's commands for pads.
enum AppMenuAction: CaseIterable {
    case controllers, fullScreen, minimize, hide, hideOthers, help, about, quit
}

extension MenuKey {
    /// A pad's menu events as keys, in a fixed order (docs/APP.md "Pad
    /// parity"): directions, A Enter, B back, Start, X/Y costumes, L/R the
    /// previous/next player (Shift-Tab/Tab), Z and Select/Home the app menu.
    static func keys(for events: MenuEvents) -> [MenuKey] {
        let table: [(MenuEvents, MenuKey)] = [
            (.up, .up), (.down, .down), (.left, .left), (.right, .right),
            (.confirm, .enter), (.back, .back), (.start, .start),
            (.x, .costumeNext), (.y, .costumePrevious), (.l, .backTab), (.r, .tab),
            (.z, .appMenu), (.menu, .appMenu),
        ]
        var keys: [MenuKey] = []
        for (event, key) in table where events.contains(event) && !keys.contains(key) { keys.append(key) }
        return keys
    }
}

extension AppModel {
    var discActions: [DiscAction] {
        var actions: [DiscAction] = [.open]
        if let disc { actions.append(.resume(disc.gameID)) }
        // The disc already open is the Continue button; offer the others.
        for item in recent where disc == nil || item != recent.first {
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
        if showingAppMenu { return cardKey(key) { showingAppMenu = false } }
        if notice != nil {
            if [.enter, .escape, .back, .start].contains(key) { notice = nil; return true }
            if screen == .match && !paused { return false }
        }
        if key == .appMenu {
            if screen == .match && !paused && !showsResults { setPaused(true) }
            showingAppMenu = true
            focusID = nil
            return true
        }
        if showingControllers { return controllersKey(key) }
        if showsResults { return resultsKey(key) }
        switch screen {
        case .disc: return buttonKey(key)
        case .characters:
            if showingRules { return cardKey(key) { showingRules = false } }
            return characterKey(key)
        case .stages: return stageKey(key)
        case .loading:
            if key == .escape || key == .back { back(); return true }
            return buttonKey(key)
        case .match:
            guard paused else { return false }
            if key == .escape || key == .back || key == .start { setPaused(false); return true }
            return buttonKey(key)
        case .results: return resultsKey(key)
        }
    }

    /// Arrows move focus; Enter (A) and Start press the focused control.
    private func buttonKey(_ key: MenuKey) -> Bool {
        switch key {
        case .up: moveFocus(0)
        case .down: moveFocus(1)
        case .left, .backTab: moveFocus(2)
        case .right, .tab: moveFocus(3)
        case .enter, .start: activateFocus()
        default: return false
        }
        return true
    }

    /// A card over the screen (rules, app menu): Esc, B and Start close it.
    private func cardKey(_ key: MenuKey, close: () -> Void) -> Bool {
        switch key {
        case .escape, .back, .start, .appMenu: close(); focusID = nil
        default: _ = buttonKey(key)
        }
        return true
    }

    private func controllersKey(_ key: MenuKey) -> Bool {
        if let pad = settingsPad, pads.first(where: { $0.handle == pad })?.listening != nil {
            // Waiting for a press on the pad: Esc stops waiting.
            if key == .escape { core.padListen(handle: pad, target: nil); padsChanged() }
            return true
        }
        switch key {
        case .escape, .back, .start: closeControllers()
        case .backTab, .tab:
            // L and R pick the controller to edit.
            guard let index = pads.firstIndex(where: { $0.handle == settingsPad }), !pads.isEmpty else { break }
            settingsPad = pads[(index + (key == .tab ? 1 : pads.count - 1)) % pads.count].handle
        default: _ = buttonKey(key)
        }
        return true
    }

    private func resultsKey(_ key: MenuKey) -> Bool {
        switch key {
        case .escape, .back: quitToMenu()
        case .start: perform(ResultsAction.rematch)
        default: return buttonKey(key)
        }
        return true
    }

    private func characterKey(_ key: MenuKey) -> Bool {
        let onTile = focusID?.hasPrefix("tile.") ?? false
        switch key {
        case .up, .down, .left, .right: _ = buttonKey(key)
        case .enter:
            if onTile && selection.ready {
                confirmCharacters()
            } else if onTile, let character = character(cursor) {
                choose(character)
            } else {
                activateFocus()
            }
        case .start:
            if selection.ready { confirmCharacters() }
        case .tab: activate(player: (picking + 1) % slotCount)
        case .backTab: activate(player: (picking + slotCount - 1) % slotCount)
        case .costumePrevious, .costumeNext:
            guard selection.players[picking].character != nil else { return true }
            cycleCostume(player: picking, step: key == .costumeNext ? 1 : -1)
        case .clear: clear(player: picking)
        case .back:
            // Melee's B: take the pick back, then leave.
            if selection.players[picking].character != nil { clear(player: picking) } else { back() }
        case .fewerStocks: setStocks(max(1, rules.stocks - 1))
        case .moreStocks: setStocks(min(99, rules.stocks + 1))
        case .escape: back()
        case .appMenu: break
        }
        return true
    }

    // MARK: Stage select

    /// Stages, then the Random tile.
    var stageTileCount: Int { stages.count + 1 }

    func activateStageTile(_ index: Int) {
        if index < stages.count {
            if let reason = stageUnavailable(stages[index]) {
                return present(stages[index].name, "Not with these players and rules: \(reason).")
            }
            chooseStage(stages[index])
        } else {
            chooseRandomStage(reduceMotion: NSWorkspace.shared.accessibilityDisplayShouldReduceMotion)
        }
    }

    private func stageKey(_ key: MenuKey) -> Bool {
        guard shuffling == nil else { return true }
        switch key {
        case .escape, .back: back()
        case .tab: setFocus("stage.\((stageCursor + 1) % stageTileCount)")
        case .backTab: setFocus("stage.\((stageCursor + stageTileCount - 1) % stageTileCount)")
        default: return buttonKey(key)
        }
        return true
    }
}
