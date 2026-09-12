# Diagonal forward smash

Fox and Marth previously panicked when either stick had a vertical component
during forward-smash entry. Retail probes authored angled animation variants
and falls back to straight forward smash when they are absent. Both characters
have only submotion 62. The loader now retains the availability of submotions
60, 61, 63 and 64 independently of which animations the port has loaded.

Entry also preserves retail input priority: qualifying main-stick+A wins over
an opposing C-stick. Dash entry keeps its separate facing/age rules and passes
the selected stick into common entry. Shared angle selection follows strict
PlCo thresholds and retains character hooks. Other characters' unported selected
motion rows remain explicit errors rather than silently becoming straight smash.

Retail: `ftCo_AttackS4_CheckInput` (8008BFC4), dash check (8008C114), `doEnter`
(8008C3E0), angle helpers (8007D964/8007D99C). The local `doEnter` symbol is
ambiguous in the name resolver, so its assembly was read from the owning
`ftCo_AttackS4.s` unit. The submotion probes 60/61/64/63 select actions
58/59/62/61 respectively; action and submotion numbers must not be confused.

Four 300-tick directed gates cover Fox/Marth, both facings, main-stick and
C-stick diagonals, opposing simultaneous stick inputs, and forward-dash entry:
`fsmash_diagonal_fd_{fox,marth}` and `fsmash_dash_diagonal_fd_{fox,marth}`.
The captured dash cases reach action 60 at tick 9; the multi-smash cases reach
it at ticks 31, 111 and 191 with the expected facing. Gates compare fighter bits
and ordered particle RNG. Allocation acceptance is
`diagonal_smash_allocation_budget`; full profile/clippy results are in TRACKER.md.

Changed files: `crates/melee-ft/src/fighter/{assets.rs,attack.rs,smash.rs}`,
`crates/melee-sim/tests/{m5_gate.rs,alloc_gate.rs}`, the four scenario TOMLs,
`TRACKER.md`, `docs/MATCHUP_COMPLETENESS.md` and this note. Local oracle data is
ignored and mirrored externally. No expected-value, decomp or threshold changes.
