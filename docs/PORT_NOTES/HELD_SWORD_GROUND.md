# Held Beam Sword: ground inputs (2026-10-24)

Twenty-three existing retail captures now gate completely, including item
fields, particle fields and ordered particle RNG: six side C-stick throws,
four vertical C-stick throws, seven normal down-tilt entries and six normal
up-smash entries. They are registered by
`sword_ground_inputs_and_entries_match_retail` in `m5_gate.rs`.

Retail `checkItemThrow` (0x8008C22C, call 0x8008C29C) checks the C-stick
throw edge independently of the main stick that supplied the smash's sign.
The throw preserves fighter facing and selects forward/backward by sign
against facing. Main-stick Sword swings still turn toward the stick.
`ftCo_AttackHi4/Lw4_CheckInput` (0x8008C830/0x8008CB44) similarly throws on
shoulder+A, a throwable item, or a C-stick edge; otherwise the ordinary
vertical attack retains the Sword. `ftCo_AttackLw3_CheckInput` (0x8008BB44)
retains the Sword for its normal down tilt when `ftCo_80094E54` is false.

Only the actual common Sword AttackLw3 (57) and AttackHi4 (63) rows gain
held-state admission. Private character rows sharing a common identity
(such as Game & Watch Manhole, action345/identity57) remain guarded.
Their existing callbacks preserve pickup/repeat/new-attack ordering and
normal hand ownership; the swing-specific hand-drop callbacks do not run.
The down-tilt IASA still checks `checkPadA` before its later throw input.
No character-kind dispatch, new motion row, allocation or callback is added.

These recordings end on entry. Held AttackHi3, AttackLw4, all aerials and
LandingAir remain guarded, as do other unproved item kinds. In particular,
the common aerial's missing throw-first IASA call is not activated by this
work: the 25 existing aerial-entry captures do not witness that later call.
No protected recording, game data or decomp file was changed.

Validation against base bf98d61d: all 23 full captures (21,392 ticks) match
fields, particle dumps and ordered particle RNG. All 38 melee-ft library
unit tests pass without missing-data opt-out, including the private-row
negative audit. The broader worktree crate suite passes 120 test results
with its documented missing-data handling; root acceptance covers actual
disc-backed API allocation tests. Release all-target clippy and the existing
replay-branch, timeout/lightthrow and random-item m5 families pass.

The complete 3,485-scenario sweep has zero regressions and 23 fixes:
3,278 particle-exact, 115 not comparable, 91 existing local failures and one
existing particle fault. Both replay corpora retain their exact path sets,
completion categories and matched frames: 108 complete/964,782 frames and
486 complete/5,102,652 frames. Runtime verification preceded a rebase over
the documentation-only checkpoint 3d443649; runtime source was unchanged
by that checkpoint. Evidence remains in the worktree's target/held-sword.
