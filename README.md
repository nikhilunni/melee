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

Milestone 1 of `docs/PLAN.md`: RNG and fused multiply-add are bit-exact and
tested. `frsqrte`/`fres` are IEEE placeholders pending a decision on the
hardware tables (see `crates/gekko-math/src/estimate.rs`). The oracle
pipeline (`decode.py` to `melee-diff`) is end-to-end tested on synthetic
dumps. The Dolphin driver script is a skeleton awaiting the fighter list
walk.
