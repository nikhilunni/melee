# Menu design

One visual language for every host (macOS AppKit/SwiftUI, web DOM/CSS). The
hosts draw natively, so the same design is implemented twice; this file is
the contract between them. Screens, tokens and motion here are normative;
pixel values are given in points (macOS) = CSS px (web).

The brief (user, 2026-10-04): Melee's own visual language, made modern and
beautiful. Melee's menus are deep blue space, glossy slanted panels, heavy
italic type, port colours and the Smash emblem; the modern take keeps those
signatures and replaces 2001 bevels with light, depth, glass and motion.

## Assets: disc art is loaded, never committed

Nothing taken from the disc image goes into git: no portraits, icons,
previews, emblems, textures or fonts extracted from it, not even as
placeholders, test fixtures or screenshots (user, 2026-10-04). Disc art is
decoded at runtime by libmelee `art` after the user presents their ISO. Until
it is ready (and if an image is missing) hosts show our own art: what is in
the repo is original work (CSS/SwiftUI shapes, our own vector emblem and
silhouettes) or openly licensed (the OFL fonts below, with their licences).

## Principles

1. **The game's art is the hero.** Portraits, stage previews and emblems
   come from the user's disc (libmelee `art`). Chrome stays quiet around them.
2. **Melee signatures, not pastiche.** Slanted shapes (a 12 degree skew),
   heavy italic display type, port colours, the circle-and-cross emblem,
   "READY TO FIGHT". No fake bevels, no gradients that look like plastic.
3. **Depth through light.** A living backdrop, translucent panels with
   blur, soft coloured glows that follow selection. Never flat grey boxes.
4. **Motion has weight.** Springs, not linear tweens; things slide in on the
   skew axis. Every animation is under 400 ms and respects reduced motion.
5. **Keyboard first, mouse friendly.** Everything is reachable with the
   keyboard (arrows, Enter, Esc, Tab); focus is always visible.

## Tokens

### Colour

| Token | Value | Use |
|---|---|---|
| `bg-deep` | `#05060f` | window background, under the backdrop |
| `bg-indigo` | `#10145a` | backdrop gradient core |
| `bg-violet` | `#3a1a7a` | backdrop gradient accent |
| `glass` | `rgba(18, 22, 48, 0.55)` + 24 px blur | panels |
| `glass-edge` | `rgba(255, 255, 255, 0.10)` 1 px inner border, `rgba(255,255,255,0.22)` top highlight | panel edges |
| `text` | `#f4f6ff` | primary text |
| `text-dim` | `rgba(226, 232, 255, 0.62)` | secondary |
| `p1` | `#ff3b3b` | Player 1 (Melee red) |
| `p2` | `#3b7bff` | Player 2 (Melee blue) |
| `p3` | `#ffc93b` | Player 3 (reserved) |
| `p4` | `#2fd36a` | Player 4 (reserved) |
| `cpu` | `#9aa3b5` | CPU (reserved) |
| `accent` | `#ffb21e` → `#ff5e1e` gradient | primary action, READY TO FIGHT |
| `danger` | `#ff4d6d` | errors |

Port colours glow: the same hue at 45% opacity, 24–40 px blur.

### Type

- **Display:** *Saira Condensed* Black Italic (OFL), all caps, tracking
  +2%. Screen titles 44, player names 34, stage names 30, buttons 20.
  Bundle the font in both hosts (`assets/fonts/`, with its licence) so they
  match; fall back to SF Pro Expanded Heavy Italic / system italic.
- **UI:** *Inter* (OFL) Medium/Semibold, 13–15; numbers with tabular
  figures. Fall back to the system UI font.
- Percentages in the HUD: display face, 44, with a 2 px dark outline and a
  soft drop shadow, the way Melee's damage digits read over any stage.

### Shape

- Slant: panels and buttons are parallelograms skewed −12° (content stays
  upright). Use a skewed background shape, never skewed text.
- Corner radius 14 for glass panels, 10 for tiles, full pill for chips.
- Spacing scale 4 / 8 / 12 / 16 / 24 / 32 / 48.

### Motion

- Spring: response 0.35 s, damping 0.8 (SwiftUI `.spring(response: 0.35,
  dampingFraction: 0.8)`; CSS `cubic-bezier(0.2, 0.9, 0.25, 1.15)` at 320 ms).
- Screen change: outgoing content slides 40 px along the slant and fades;
  incoming slides from the other side, 60 ms stagger per element group.
- Hover/focus on a tile: scale 1.06, glow in the hovering player's colour,
  portrait parallax ±4 px following the pointer.
- `prefers-reduced-motion` / Reduce Motion: cross-fades only.

## Backdrop

A full-window layer behind every menu screen:
- radial gradient `bg-indigo` (centre-left) and `bg-violet` (top-right)
  over `bg-deep`;
- a slow starfield (2 parallax layers of small dots, drifting diagonally,
  ~60 s per cycle);
- the emblem (our own circle-and-cross vector, or the disc's at runtime)
  very large (120% of window height), 4% opacity, slowly rotating 0.5°/s;
- a soft light sweep across the slant every ~8 s;
- a faint film grain (2–3% noise) for depth.
It must be cheap: CSS gradients and transforms / SwiftUI Canvas or Core
Animation, never a per-frame JS/Swift loop drawing pixels.

## Screens

### Disc

Centered stack: the Smash emblem (large, glowing white), the title
"SUPER SMASH BROS. MELEE" in display type with "PORT" in small UI caps
underneath, then a glass drop zone (dashed slanted border that brightens on
drag-over) reading "Drop your Melee disc image" with an "Open Disc…" accent
button and the recent disc as a secondary button ("Open GALE01.iso"). A
one-line footnote: "NTSC-U 1.02 (GALE01, revision 2) · nothing leaves your
machine". Validation errors appear as a danger-tinted glass card under the
zone, with what was found.

### Character select

Layout (landscape, scales down to 1024×640):
- **Top bar:** Back (chevron, slanted pill), title "CHOOSE YOUR CHARACTER",
  stocks stepper as a slanted pill on the right ("STOCK ◀ 4 ▶").
- **Grid:** the retail 9/9/7 layout, centred. Each tile is a slanted card
  showing the CSS face icon (or the portrait cropped to the face) filling the
  tile, the name in display type along the bottom edge on a dark gradient.
  Tiles chosen by a player wear that player's coloured ring and a small
  "P1"/"P2" coin token (circle, port colour, display type) in the corner.
- **Player panels** (bottom, one per player, P1 left, P2 right): tall glass
  cards tinted with the port colour, a big "P1" in the corner. They show the
  selected costume's **portrait** large (the CSP, scaled up crisply, with a
  soft port-coloured glow behind it), the character name in display type at
  the bottom (Melee's panel name plate), and costume swatches as a row of
  round chips, each a tiny crop of that costume's portrait or stock icon. An
  empty panel shows a dim silhouette and "PRESS A KEY / CLICK A FIGHTER".
- **Picking model:** the active player is highlighted (its panel glows); click
  or Enter on a tile assigns it to the active player and advances to the next
  player without a pick. Clicking a panel makes that player active.
  Keyboard: arrows move the cursor across the grid, Enter picks, Tab switches
  player, `[`/`]` or Q/E cycles costume, Backspace clears.
- **Ready banner:** when both players have picked, a full-width slanted
  banner slides in across the middle-bottom: "READY TO FIGHT" in display type
  on the `accent` gradient with a light sweep; clicking it or pressing Enter
  goes to stage select. This replaces a plain "Choose Stage" button.
- Controls legend: small, dim, bottom-left, in UI type with key caps.

### Stage select

- **Hero preview** (top 55%): the hovered/focused stage's preview art large
  in a slanted glass frame with a port-neutral glow, the stage name in display
  type overlapping the frame's lower edge, the series emblem faint behind it.
  Cross-fade between stages (150 ms).
- **Stage row** (bottom): the six stages as slanted tiles with their
  icon/preview art, name underneath in UI caps; hover/focus lifts and glows.
  Arrow keys move, Enter starts, Esc goes back. A small "RANDOM" tile at the
  end picks one with a quick shuffle animation.

### Loading

Over the stage preview, dimmed: the two portraits facing each other with a
slanted "VS" between them in display type, and a thin accent progress bar
along the slant under them (files and MB in UI type). Brief, it's fast.

### Match HUD

Bottom centre, Melee's arrangement: one plate per player. Each plate shows
the stock icons (from the disc) as a row, the character's **portrait face
crop** (or stock icon large) on a port-coloured slanted plate, and the
percentage in display type with outline. The percent tints from white to
red as damage rises (0% white, 100% orange, 200%+ deep red), with a quick
shake when it jumps. Never cover the stage centre; keep plates compact.

Pause: the game dims and blurs; a glass card "PAUSED" with Resume, Restart,
Save Replay, Quit to Character Select, as slanted buttons.

### Results

Winner's portrait large on the left with a port-coloured light burst and the
series emblem behind it, "WINNER" / "NO CONTEST" in display type, the name
plate, and a small table of final stocks and percent per player. Buttons:
Rematch (accent), Character Select, Stage Select.

### Errors and notices

Glass cards tinted `danger`, the message in UI type, copyable details,
and one obvious action. Never a raw alert where a card fits (on macOS a sheet
for faults that offer Save Replay is fine).

## Platform notes

- **macOS:** SwiftUI views hosted in the AppKit window. Glass =
  `.ultraThinMaterial` tinted with `glass`; display font registered from the
  app bundle (`ATSApplicationFontsPath` or CTFontManager). Images from
  libmelee as `CGImage` with `.interpolation(.high)` for scale-ups over 2×
  and `.none` for icons drawn at integer multiples. Respect Reduce Motion
  and Increase Contrast.
- **Web:** plain DOM + CSS, no framework. Glass = `backdrop-filter:
  blur(24px) saturate(140%)` with the `glass` fill. Fonts via `@font-face`
  from `www/fonts/`. Disc images become `ImageBitmap`s drawn to small
  `<canvas>` elements or `blob:` URLs on `<img>`; `image-rendering: auto` for
  portraits, `pixelated` for tiny icons scaled by integers. Respect
  `prefers-reduced-motion`.
- Both: the game view keeps rendering behind overlays (pause, results) where
  it does today; menus never block the render loop.
