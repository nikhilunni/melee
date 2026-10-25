# HeavyGet terminal entry

This ports the shared, non-looping HeavyGet entry and the remaining same-tick
common physics, collision and camera phases. Row 93 uses retail submotion 89.
Its next Animation and IASA callbacks explicitly refuse continuation. No item
attachment, carry, drop, throw, or later HeavyGet lifecycle is admitted.

## Retail order and required prerequisites

`ftpickupitem_80094694` calls the model-Y rate helper before clearing the whole
throw-flag word (800946E0), changing motion (800946FC), and stepping the animation
once (80094740..44). It then publishes the first scratch word and installs the
common damage callback (80094754..60). The supported unit player scale yields
rate 1 in the literal `CalcYScaledKnockback` arithmetic; no scaled rate is ported.
Raw warm imports now reject unsupported model scale rather than silently
normalizing it to 1. The existing typed `model_width` depth override remains
supported, independently of player scale and animated joint scale.

The common callback's lifetime is explicit: motion changes clear it alongside
other installed callbacks, and HeavyGet installs it after the entry animation
step. Inferring ownership from retained move scratch was incorrect because Wait
can preserve that scratch. The common callback replaces the character hook,
including its article-free side effect. Retail's empty primary hand is a literal
no-op; a nonempty hand still refuses the unported drop.

A prerequisite fixes throw scratch inheritance. Retail `ftCo_800DD398` clears
mv+4/+8 at 800DD3E4/E8 before motion entry 3F4 and animation step 3FC. The existing
`ThrowFlight::frozen` already models the real mv+4 writes, including 1 on airborne
freezing and 0 on landing. A typed Throw owner now exposes that word to ordinary
scratch inheritance and preserves it through Wait. The predecessor's untouched
mv+14/+18 image is folded before replacing its typed owner. This changes neither
throw arithmetic nor the existing flag writers.

## Existing complete witnesses

| Original scenario | Ticks | Terminal actor/item |
|---|---:|---|
| corpus_v3_bf_items_sword_e5993e1bc_p0 |259| Marth/Taru |
| corpus_v3_fd_items_sword_e5de865a8_p1 |257| Fox/Box |
| corpus_v3_fd_items_sword_e6af4a7bb_p2 |240| Fox/Box |
| corpus_v3_fd_items_sword_efc6a328f_p2 |241| Fox/Box |
| corpus_v3_fd_items_taru_e1895a33d_p0 |295| Marth/Taru |

These are five original complete captures totaling 1,292 rows, with HeavyGet only
on each final row. Each retains both NULL hands and the same unowned REST item.
The raw regression uses the actual display cadence and compares all-row motion
and position, terminal animation/rate/velocity/ground/floor/throw flags/callback,
retained scratch, and item identity/state/owner. The fifth capture additionally
checks every scratch word from ThrowF entry 242 through Wait and HeavyGet 294.
Complete 560-row forward/back Kirby throw controls check the real 0→1→0 landing
sequence and every Throw-owned word, including retention through Wait.

The strict gate retains every original field/item/particle row and ordered
particle LR sites. A separate raw audit starts from independently recorded
initial, metadata and boundary seeds, including each genuine row-zero setup
draw, checks every ordered raw global LCG PC/seed edge and each port tick-end
seed under the ledger cadence. This does not instrument every nonparticle port
RNG call. The five raw ledgers contain 58,265 draws. The trace adapter's initial
observation remains an observation; no setup draw is inferred from port output.

The actual API test measures step, observe and presentation capture, and clones
before the terminal entry so both independent matches enter. Clone construction
is outside the allocation counter. The actual next API step must refuse the
unported Animation callback; no later retail continuation is claimed.

## Validation status

Focused raw/ownership/negative-guard tests pass 3/0; the imported-scale guard
passes 1/0, strict HeavyGet group 1/0, actual API allocation/clone test 1/0,
shared-path groups 14/0, and touched melee-ft unit tests 38/0. Workspace release
all-target clippy is clean. The final fixed-binary sweep against the coordinator's 72aa5d9a baseline has
3,490 rows, zero regressions and exactly the five terminal-entry fixes: 3,321
EXACT, 115 NOTCMP, 53 existing FAIL and one existing FAULT. Both replay corpora
preserve every path's category, matched count and frame count: public-v3.7 is
108/108 complete with 964,782 matched frames; public-v3.7-b is 486/502 complete
with 5,102,652 matched frames. The normal CLI was built after all tests/clippy,
preserved as a fixed executable and checked against unchanged source/scenario
manifests throughout the proof. The documentation-only rebase to ee522621
preserved every Rust/Cargo source hash. Coordinator integration checks remain
the coordinator's responsibility. Detailed logs and final provenance live under
`target/wave20/heavy-entry/sourceportnote/`; superseded failed fixture/compile
attempts remain there and are not final acceptance evidence.
