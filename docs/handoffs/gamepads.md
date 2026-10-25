> **Branch** `worktree-agent-af22012d14b4ab307` (tip `0055ed7c`), not merged into main. Both commits are WIP (70fc0dd8 core, tested; 0055ed7c macOS, never compiled). API 5 removes the old gamepad calls, so neither can merge alone: finish macOS main.swift and the web host first.

# Handoff: gamepads that just work, full pad parity (wave 26)

Task spec: `target/wave26/QUEUED_GAMEPADS.md` (main checkout). Stopped on the
coordinator's usage-limit order. No sim change; no sweep was run.

## Done (commit "WIP: melee-platform: gamepads in the core")

All in `crates/melee-platform`, unit-tested (`cargo test --release -p
melee-platform --lib`: 42 pass; `--test disc_flow` with `MELEE_DATA_ROOT`: 6
pass; clippy `-D warnings` clean native and wasm32):

- `pads/family.rs`: family detection from USB ids (web `Gamepad.id`, Chromium
  and Firefox formats, `parse_web_id`) or names (macOS `vendorName` /
  `productCategory`): GameCube (WUP-028, NSO GC, Mayflash), Switch Pro,
  Joy-Con, DualSense, DualShock 4, Xbox, 8BitDo, generic. Per-family labels
  and default deadzones.
- `pads/mapping.rs`: W3C standard layout `Input` -> GameCube `Target` by
  position (south A, west B, east X, north Y, bumpers Z, triggers L/R, Start,
  d-pad); bind (swaps so nothing is lost), swap A/B, deadzone, trigger click;
  `encode`/`decode` text.
- `pads/calibration.rs`: radial deadzone + octagonal gate (Dolphin's 87 main
  / 74 C radii); full tilt hits Melee's rim, diagonal notch = (0.7, 0.7)
  through `normalize_stick`. GameCube sticks one to one, origin-calibrated.
- `pads/nav.rs`: menu events (directions with 400/90 ms repeat, stick
  hysteresis, button edges, MENU from Select/Home).
- `pads/adapter.rs`: WUP-028 37-byte report parse.
- `pads/mod.rs`: `Pads` registry: connect/disconnect, read (rebind listening
  with 5 s timeout), settings, ports (assign swaps), prefs blob
  (`identity\tsettings` lines).
- `app.rs`: `pad_connect/disconnect/update/assign_pad/pad_adapter_report`;
  "press a button to join" on character select (first human without a pad,
  else first Off slot -> Human; consumes the press, `JOINED`); in a match an
  unjoined pad takes the first human port without a pad. `input::Gamepad` is
  now GameCube bytes (`[i8;2]` sticks, `[u8;2]` triggers).
- `focus.rs`: spatial focus `next(frames, from, dir)` shared by both hosts.
- C API 5 (`include/melee_platform.h`, `src/ffi/pads.rs`):
  `melee_app_pad_*`, `melee_pad_*` labels, `melee_focus_next`. Old
  `melee_app_gamepad` removed.
- `crates/melee-web/src/lib.rs`: `WebApp.pad_*`, `pads()`, `pad_labels()`,
  `focus_next()`. Old `gamepad`/`gamepad_lost` removed.

## Partial (commit "WIP: macOS ...", never compiled)

`apps/macos`: `PadCore.swift` (Swift wrappers), `Gamepads.swift` (rewritten:
GameController by position + IOHID WUP-028 reader, prefs in UserDefaults
`padPrefs`, 60 Hz timer on every screen), `Focus.swift` (`.padFocus` registry
via preferences, layers, `moveFocus`/`activateFocus` via `melee_focus_next`),
`Navigation.swift` (rewritten: MenuKey `.back/.start/.appMenu`, cards, B
clears then backs out, Start = ready/resume/rematch), `PadViews.swift`
(PadHints, ControllersCard, AppMenuCard, toast), focus ids wired into disc,
character select (top bar incl. new Controllers button, tiles, ready banner,
slot controls, chips, rules card), stages, loading, pause, results;
RootView overlays + `FocusCollector`.

Not yet done on macOS:
- `main.swift`: GameView still creates `Gamepads()` with the old API. Needs:
  one `Gamepads(model:)` owned by WindowController; `onEvents` dispatch
  (JOINED -> `padsChanged` + toast "P<n>: <name> joined"; BOUND ->
  `padsChanged`; inactive/minimized -> restore on MENU/START/CONFIRM;
  attached sheet: NSAlert A=first button, Y=second, B cancels panels;
  unpaused match: START toggles pause, MENU pauses + app menu; else map to
  MenuKeys with `model.padInput = true`); display link calls `poll()` in a
  match; `model.appMenuAction` (controllers, toggleFullScreen + `fullScreen`
  flag via window delegate, miniaturize, hide, hideOtherApplications, help
  as a `model.present` notice instead of NSAlert, about, terminate);
  `saveReplayPanel` with `padInput` saves to `~/Documents/Melee Replays`
  without a panel; `MELEE_APP_KEYS` print uses `model.focus` (now
  `focusID`); add `MELEE_APP_PADS=` scripted fake-pad walk (feed MenuEvents
  through the same dispatch) for the pad-only acceptance walk.
- Build (`tools/build-macos.sh`) and fix compile errors; real-click check
  with `MELEE_APP_CLICK`.

## Not started

- Web host `crates/melee-web/www/app.js`: replace `pollGamepads` (uses the
  removed `app.gamepad`) with: connect/disconnect by `Gamepad.index` and
  `id`, `pad_update` every rAF on every screen, prefs in localStorage
  (try/catch), spatial focus with `focus_next` over the visible focusables
  of the top layer (dialogs first), A clicks, B/Start/X/Y/L/R/Z per the
  macOS Navigation semantics, controllers dialog, pad hints, `?pads=` fake
  Gamepad walk for testing.
- docs/APP.md: "Gamepads" and "Pad parity" sections (every keyboard
  shortcut of both hosts with its pad binding; host limits: NSOpenPanel and
  the web file picker / permission prompts cannot be driven by a pad, the
  remembered disc and recent discs can).
- Final: `--filter start_` sweep vs the baseline, clippy, rebase.
