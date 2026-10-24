> **Branch** `worktree-agent-ad07370e21111fa2e` (tip `23fc6bb4`), not merged into main. Based on bad0df46; to merge, rebase onto main (conflicts in the shared Kirby lists, m4/m5_gate, boundaries.toml), then run a full sweep and replay-batch (the effect bank table grew to 33 and a swallowed Nana now gives an ability). All commits are verified; none is WIP.

# Handoff: Kirby's copied neutral specials of Zelda, Bowser, the Ice Climbers and Sheik

Branch `worktree-agent-ad07370e21111fa2e`, rebased onto main `bad0df46`.

## Task and scope

Wave 13 (breadth). Port Kirby's copied neutral specials, one kind per commit,
following `docs/PORT_NOTES/KIRBY.md` "Where this stands and how to resume",
item 2 (pattern: the original's states made generic over a small trait in a
family crate; a `Kirby<Kind>` marker in `crates/ft-kirby/src/copied/`):

1. Zelda's Nayru's Love (reflector) - **done**
2. Bowser's Fire Breath (it-koopaflame, the fuels) - **done**
3. The Ice Climbers' Ice Shot (it-climbersice) - **done**
4. Sheik's Needle Storm (a charge kind; needs `ftKb_Init_UnkMotionStates4`,
   which is now on main in `crates/ft-kirby/src/charge_flash.rs`, its Sheik
   arm `xB4 == 6`, colour 87, left empty for this task) - **not started**
   (only its start boundary exists).

Witnesses go in `KIRBY_WITNESSES` (`crates/melee-sim/tests/m5_gate.rs`);
boundaries in `start_fd_kirby_copy_wave13_boundaries_600` (`m4_gate.rs`).

## Done and verified (every commit exact against retail)

All witnesses gate exact (items and particles) on their first gate and show
0 differing particle-site ticks.

- `143243ac` ft-kirby: the copied Nayru's Love. New `crates/ft-zelda-family`
  (`nayrus_love.rs`, `common.rs` moved from ft-zelda). Witnesses
  `kirby_copy_nayru_fd_zelda4` (900), `_edge_` (780), `_reflect_` (800,
  a Sheik needle reflected: Zelda transforms after the swallow; Din's Fire
  has no hitbox in flight), `_struck_` (700). Boundary
  `start_fd_kirby_zelda4`.
- `7eadfb1b` ft-kirby: the copied Fire Breath. New `crates/ft-koopa-family`
  (`fire_breath.rs`, `common.rs` moved from ft-koopa). `it-koopaflame` is
  generic (`Flame<const KIRBY>`). Witnesses `kirby_copy_firebreath_fd_bowser4`
  (1100), `_edge_` (780), `_air_` (800). Boundary `start_fd_kirby_bowser4`.
- `bf3226dd` ft-kirby: the copied Ice Shot. New `crates/ft-iceclimbers-family`
  (`ice_shot.rs`, `common.rs` holding the helpers from climber.rs).
  `it-climbersice` generic (`Ice<const KIRBY>`). Also: a swallowed Nana gives
  Popo's ability (`melee-ft` `capture_kirby::ability`,
  ftCo_ThrownKirby.c:57-59), and a warm-import fix (below). Witnesses
  `kirby_copy_iceshot_fd_iceclimbers4` (800), `_air_` (760), `_edge_` (780),
  `_struck_` (700). Boundary `start_fd_kirby_iceclimbers4`.
- `c9629363` harness: boundary `start_fd_kirby_sheik4` (warm and cold exact),
  for the Needle Storm.

No WIP commits.

### `start_fd_kirby_iceclimbers4` and initial_state/fighter.rs:131

On main (before `bf3226dd`) the warm import of this boundary panics at
`crates/melee-lib/src/initial_state/fighter.rs:131` (the script-pointer
lookup's `unwrap`). Cause: the savestate holds Nana before her entry's
motion change finished: x3EC is stale heap bytes (0xECEBEEF3), as are her
callbacks (0x219C..0x21F8), flags (0x2218/0x221C) and mv words
(0x236C..0x2380). Every other fighter's saved bytes equal the tick-zero row
of the tick trace. Fix in `bf3226dd`
(`crates/melee-lib/src/initial_state/mod.rs`): a secondary fighter whose
saved x3EC is non-zero and not a main-RAM address is imported from the
tick-zero row. With it the warm boundary gates 600 ticks exact. Simply
treating that x3EC as "no script" was tried first: it still diverges at
tick 11 (Nana's y), because the mv words are stale too.

## Where I stopped

- The final full sweep against `/Users/nikhilunni/Projects/melee/target/sweeps/26fee4c7.tsv`
  was queued behind the sweep lock and had not run. Earlier filtered sweeps
  (before the rebase onto bad0df46) showed 0 regressions for `--filter
  zelda`, `bowser`, `koopa`, `iceclimbers`, `kirby`. After the rebase all 12
  witnesses and the Sheik boundary re-gated exact, and
  `cargo check --release --workspace` and clippy (`-D warnings`) on the
  touched crates were clean.
- `replay-batch` over `~/melee-data/replays/public-v3.7` was not run. The
  changes reach replays (melee-ef's bank table grew to 33 entries; the
  `CharacterBanks` array now builds with `std::array::from_fn` since
  `Default` stops at 32; a swallowed Nana now gives an ability).

## Next steps

1. Full sweep:
   `cargo run -q --release -p melee-sim -- sweep --scenarios /Users/nikhilunni/Projects/melee/harness/scenarios --out target/after.tsv --baseline /Users/nikhilunni/Projects/melee/target/sweeps/<main commit>.tsv --lock /Users/nikhilunni/Projects/melee/target/sweep.lock`
   (zero regressions required; the warm-import change touches every
   savestate scenario, though it only fires for a partially created partner).
2. `replay-batch` over public-v3.7 against the coordinator's latest
   `target/sweeps/*.replays.jsonl`; no game may match fewer frames.
3. Sheik's Needle Storm: `ftkirbyspecialseak.c` (entries
   `ftKb_SpecialNSk_8010612C` / `801061E4`, put-away arm
   `ftKb_SpecialNSk_8010603C`), rows 467..474 (SM 402..409), attributes
   +290..+2A0 (`specialn_sk_*`), items `KirbySeakNeedleThrow` (0x98) /
   `KirbySeakNeedleHeld` (0x99) from `PlKbCpSk.dat`. Make `ft-seak`'s
   `special_n.rs` generic in a new `ft-seak-family` (as the three above),
   fill `charge_flash.rs`'s Sheik arm, add the NEUTRAL entry, the
   put-away arm in `ability.rs`, and `reset_copied_scratch` words.
4. Witnesses `kirby_copy_needles_*` from `start_fd_kirby_sheik4`: grounded
   charge and throw, aerial, cancel, a hit while charging, losing the
   ability (taunt, death). Kirby's swallow script (frames 130-346 of any
   `kirby_copy_*` scenario) works on this boundary layout.

## Commands

```sh
# build / check (release only, CARGO_INCREMENTAL=0)
CARGO_INCREMENTAL=0 cargo clippy --release -p ft-kirby -p melee-sim --all-targets -- -D warnings
# gate one witness (from a worktree, against the main checkout's data)
cargo run -q --release -p melee-sim -- gate /Users/nikhilunni/Projects/melee/harness/scenarios/kirby_copy_iceshot_fd_iceclimbers4.toml
cargo run -q --release -p melee-sim -- particle-sites /Users/nikhilunni/Projects/melee/harness/scenarios/kirby_copy_iceshot_fd_iceclimbers4.toml --to 800
# dry-run a draft scenario from a boundary's recording
MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee cargo run -q --release -p melee-sim -- dry-run draft.toml --state-from /Users/nikhilunni/Projects/melee/harness/scenarios/start_fd_kirby_sheik4.toml --out o.jsonl
# record (scenario TOMLs copied into the main checkout's harness/scenarios first)
cd /Users/nikhilunni/Projects/melee/harness && uv run python record_many.py scenarios/<a>.toml ... --jobs 4
```

## Boundaries and recordings

- Boundaries (make_boundary.py --no-register; entries in this branch's
  `harness/boundaries.toml`): `start_fd_kirby_zelda4` (seed 1495875259),
  `start_fd_kirby_bowser4` (2211013753), `start_fd_kirby_iceclimbers4`
  (1197273965), `start_fd_kirby_sheik4` (3825146497). Savestates in the
  main checkout's `harness/roms/`; scenario TOMLs (warm and `_cold`) are
  committed.
- Recordings of all 11 witnesses and the boundaries are in the main
  checkout's `harness/traces/` (`<name>.tick.*`, `.ledger.*`,
  `.particles.*`). The witness TOMLs are committed and also present in
  the main checkout's `harness/scenarios/`.

## Gotchas

- In the worktree the decomp submodule is empty: read
  `/Users/nikhilunni/Projects/melee/third_party/melee-decomp/src/...`.
- Kirby's Zelda ground/air changes (decomp names them
  `ftKb_SpecialNSk_80105E8C / 80105F3C`) do not reinstall the crystal's
  accessory; they restore hitlag callbacks if x2219_b0.
- Kirby's aerial Fire Breath start lands in the grounded *loop* (444),
  not the start (asm `li r4, 0x1bc` at 800FB218).
- Kirby's flame leaves from parts[12] with offset (0, 0.5, 3); fuels refill
  through an `EVERY_FRAME` hook only with Bowser's / Giga Bowser's hat.
- Kirby's Ice Shot `let_go` clears only u.kb.xC0; his damage callbacks stay
  (unlike Popo's 8011F16C).
- The Kirby/IC dry-run catches Nana (she trails Popo), so the witnesses
  copy through Nana; a Popo swallow is not witnessed.
- Popo's own `ARTICLE_DESTROYED` ignores `IceClimberIce`, so Popo's `ice`
  flag is not cleared if a held block is destroyed by something else (not
  changed here; possible latent divergence in ft-iceclimbers).
- Rebase conflicts in `resources.rs` / `scene_items.rs` / `ability.rs` /
  `KIRBY.md` are all additive; array lengths (`CHARACTER_EFFECT_FILES`,
  `KIRBY_COPY_ARCHIVES`) must be recounted.

## Divergences outside this area

None found.
