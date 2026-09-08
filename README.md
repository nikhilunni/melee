# melee

A Rust port of Super Smash Bros. Melee (NTSC-U 1.02), built function by
function against a bit-exact oracle: the retail game running in Dolphin.

The port is verified, not trusted. Every change must reproduce the retail
game's state to the bit over recorded scenarios before it is merged.

## Layout

| Path | What |
|---|---|
| `crates/` | Rust workspace, layered for build speed. See `CLAUDE.md` for the layering rules. |
| `harness/` | Dolphin oracle: symbol resolver, state schema, memory decoder, scenario runner. Python, managed with `uv`. |
| `third_party/melee-decomp` | The doldecomp `melee` repo as a submodule, pinned. Read-only reference: C source, retail assembly, symbol map. |
| `docs/` | Plan, oracle design, ISO requirements. |

## Quick start

```sh
git submodule update --init
cargo gate                      # build + tests for the whole workspace
cd harness && uv sync && uv run python symbols.py seed HSD_GObj_Entities
```

You need your own disc image to run the oracle or the simulator. See
`docs/ISO.md` for exactly which one and how to verify it. No game data is
or ever will be committed to this repository.

## Status

Milestone 1 of `docs/PLAN.md` is complete except for the `frsqrte`/`fres`
hardware tables (see `crates/gekko-math/src/estimate.rs`). Archive parsing,
the shared type crates, the Slippi replay parser, and the oracle harness
(fighter-list walk, schema generator, `melee-diff`) are done and tested.
Blocked on a disc image and a scripting-capable Dolphin build for the first
real trace.
