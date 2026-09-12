# Up-special from jump squat

`ftCo_KneeBend_IASA` (800CB5FC) checks up-special, grab, up-smash and then
short-hop bookkeeping. The port recognized the up-special transition but
panicked instead of entering it.

The misleadingly named `ftCo_Attack100_CheckInput` (800D695C) checks whether
the character has a grounded up-special and whether the up-special buffer is
fresh, then directly calls `ftData_SpecialHi[kind]`. This entry must select the
Up slot explicitly. Re-running ordinary grounded special selection could
choose Side when diagonal input has made both buffers eligible.

Six 300-tick Fox–Marth schedules cover each fighter, competing A/Z inputs,
and opposing diagonal input with simultaneous side/up buffers. They jump at
frame 30 and request up-special at frame 31, producing KneeBend at tick 31
and grounded special entry at tick 32 (Fox action 353, Marth action 367). The intended change uses the existing
character table and does not add physics or alter jump timing.

Acceptance: `jumpcancel_upb_{fd,priority_fd,diagonal_fd}_{fox,marth}` fighter
and ordered-particle gates, zero simulated-tick allocations, existing movement/combat regressions,
workspace debug/release gates and
all-target clippy. Status and final results belong in TRACKER.md. Local
oracle data remains ignored and is backed up outside the repository.

The priority captures establish that up-special wins when A/Z are also
present. They do not separately exercise grab or up-smash winning during
jump squat; those unchanged branches remain part of the wider coverage audit.

Changed files: `crates/melee-ft/src/fighter/jump.rs`,
`crates/melee-sim/tests/{m5_gate.rs,alloc_gate.rs}`, six scenario TOMLs,
TRACKER, the matchup inventory and this note.

Verified: six focused combat gates and the combined zero-allocation check
pass. Full debug and release `cargo gate` each passed 1,230 tests, with zero
failures and three existing ignores. All-target clippy, formatting and 220
harness tests pass. Native smoke and its exported headless replay pass at
232 ticks. The stripped simulator remains 3,926,784 bytes (no file-size
increase from this packet); existing performance/size debt is unchanged.
All 78 new capture files match their external backup.
