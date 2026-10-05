# melee

A Rust port of Super Smash Bros. Melee (NTSC-U 1.02), written function by
function from the [doldecomp/melee](https://github.com/doldecomp/melee)
decompilation and checked against the original game running in Dolphin. A
match is meant to produce the same bits as the console, tick for tick.

It is two things:

- a headless, deterministic simulator with a small API (`crates/melee-lib`)
  for AI and game research: create a match, step it with controller inputs,
  inspect or clone its state;
- a playable app on macOS and in the browser (WebGPU), with menus, a HUD and
  replays.

This repository contains no game data. You need your own copy of the game.

**Play in the browser:** https://nikhilunni.github.io/melee/ (Chrome, Edge
or Safari with WebGPU). Drop in your disc image; it is read locally and never
uploaded.

## Status

- 25 of 26 characters (all but Kirby) and the six tournament stages
  (Battlefield, Final Destination, Dream Land, Yoshi's Story, Fountain of
  Dreams, Pokémon Stadium) match the original game on every recorded test
  scenario.
- 106 of 108 public Slippi replays in the test corpus replay to the end
  without a single divergence from the recorded game, including the random
  number stream.
- Not implemented: Kirby, CPU opponents, random item spawns, single-player
  modes. Unported branches fail loudly
  (`unimplemented!`) rather than diverging silently.
- The renderer is a display approximation; only the simulation is exact.

`TRACKER.md` has the current state in detail.

## Performance

The simulator runs the game logic only (no rendering or audio), so it is much
faster than emulating the console. Measured on an Apple M2 Max (8 performance
+ 4 efficiency cores), release build, random controller inputs over full
matches:

| Match | µs per tick | ticks/s, one core | × real time (60 fps) |
|---|---:|---:|---:|
| Samus vs Link, Yoshi's Story | 48 | 21,000 | 350× |
| Fox vs Marth, Final Destination | 53 | 19,000 | 317× |
| Fox vs Marth, Battlefield | 69 | 14,500 | 242× |
| Fox vs Marth, Pokémon Stadium | 77 | 12,900 | 215× |
| Zelda vs Pikachu, Fountain of Dreams | 97 | 10,400 | 173× |
| Ice Climbers vs Peach, Pokémon Stadium | 118 | 8,500 | 142× |

Independent matches scale across cores: 12 Fox vs Marth matches on 12 threads
run 142,000 ticks/s together, about 2,400× real time.

Against Dolphin on the same machine (headless, Null video, no audio,
unlimited speed, the same Fox vs Marth match from a savestate), the
simulator is about 20× faster per core:

| | Battlefield | Pokémon Stadium |
|---|---:|---:|
| Dolphin | 750 fps (12.5×) | 660 fps (11×) |
| This simulator | 14,500–17,100 ticks/s (242–285×) | 12,900–15,200 ticks/s (215–254×) |

Sizes and costs:

| | |
|---|---:|
| Minimal headless simulator binary, stripped | 4.5 MB (1.8 MB gzipped) |
| Simulator compiled to WebAssembly | 4.0 MB (1.2 MB gzipped) |
| Browser app, renderer included | 5.0 MB (1.6 MB gzipped) |
| Disc files read for one match | 16–21 MB of the 1.4 GB image |
| Memory per match | 96 MB |
| Create, reset or clone a match | 8–10 ms |

A match's state is large (it mirrors the game's memory pools), so cloning
costs about as much as 180 ticks; plan tree search around that. The numbers
were taken with other applications running and vary by about 3%. To
reproduce: `cargo run --release -p melee-lib --example sim_bench --
throughput <disc files dir>` (also `parallel` and `lifecycle`).

## What you need

- A disc image of Super Smash Bros. Melee for the GameCube, NTSC-U version
  1.02 (game ID `GALE01`, revision 2), as an uncompressed `.iso` or `.gcm`
  dumped from a disc you own. Other versions and compressed formats (RVZ,
  CISO, NKit) are rejected with a message saying what was found.
  `docs/ISO.md` lists the checksums to verify your dump against.
- To build: Rust (stable), and Xcode command line tools for the macOS app.

## Running

```sh
git submodule update --init        # the decompilation, used as reference
tools/run-macos.sh                 # build and launch the macOS app
tools/run-web.sh                   # build the web app and serve it on :8080
cargo run --release -p melee-lib --example headless   # the simulator alone
```

The apps ask for the disc image on first launch and read everything they
need from it at runtime: character models, stages, portraits, menu art.

## How it is verified

Scenarios (scripted inputs, recorded human matches, random-input
"explorer" matches, Slippi replays) are played on the original game in a
headless build of Dolphin with a scripting fork, which records the game's
memory every tick. The port plays the same inputs and the two traces are
compared field by field, floats by bit pattern. Floating-point code follows
the original PowerPC instruction sequence, including fused multiply-adds,
through `crates/gekko-math`. `docs/ORACLE.md` describes the setup and
`CLAUDE.md` the working rules.

## Layout

| Path | |
|---|---|
| `crates/gekko-math` | PowerPC float semantics, the game's math library |
| `crates/hsd-*` | HAL's engine: archives, scene graph, animation, particles |
| `crates/melee-*`, `ft-*`, `it-*` | game subsystems, one crate per character and item family |
| `crates/melee-lib` | the simulator API |
| `crates/melee-platform` | the app core shared by every platform, with a C API |
| `apps/macos`, `crates/melee-web` | the macOS (AppKit/SwiftUI) and browser hosts |
| `crates/melee-sim`, `harness/` | verification tooling (Rust and Python) |

## Legal

This is an unofficial, non-commercial project for research and
educational purposes. It is not affiliated with, endorsed by or sponsored by
Nintendo, HAL Laboratory or any of the companies whose characters appear in
the game. Super Smash Bros. Melee and all related names, characters and
marks are trademarks of their respective owners.

The repository does not contain, and the apps do not download, any of the
game's code, data, graphics, models, audio or other assets. Everything the
apps show from the game is read at runtime from a disc image the user
provides. Please do not ask for or share disc images here; dump your own.

There is deliberately no license file, following doldecomp: the port is
written from a decompilation of copyrighted software, and no rights to that
software are granted or implied. Third-party material in this repository
keeps its own license: the Barlow fonts (SIL Open Font License,
`assets/fonts/OFL.txt`) and Slippi replay test files from
[peppi](https://github.com/hohav/peppi) (MIT, `crates/slp/tests/data/NOTICE`).
The decompilation is included as a git submodule, not copied.

If you are a rights holder with a concern, please open an issue.
