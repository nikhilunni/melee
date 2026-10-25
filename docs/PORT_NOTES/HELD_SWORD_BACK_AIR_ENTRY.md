# Held Sword common Back Air entry

The original 621-tick `corpus_v3_fd_items_sword_nocont_e8f0de8c6_p2` capture
ends with Marth entering common action 67 while still holding the same Sword.
Only Sword plus actual resolved common action 67 is admitted. Other common
aerials, private character rows and LandingAir remain guarded.

Retail entry ftCo_AttackAir_EnterFromMsid clears interrupt/command0/throw
flags, calls Fighter_ChangeMotionState at 0x8008CFF0 and directly steps
ftAnim_8006EBA4 at 0x8008CFF8. This occurs inside the preceding Jump IASA at
scheduler link 3, after link 1 Animation. The new row's Animation/IASA callback
is not redispatched during entry. Existing common physics, collision and
held-item work finish the observed tick unchanged.

The next actual common Animation callback refuses before step_animation
(ftCo_AttackAir.c135-144), and IASA refuses before the interrupt check
(ftCo_AttackAir.c147-169). No frame counter, timer or source-phase exception
selects this boundary. In particular the missing held-item throw-first IASA
call at 0x8008D2CC is not implemented from an entry witness. This is not
Back Air continuation, hit, landing or interruption support.

Verification consumes every original expected/raw/particle/meta/ledger row
at its actual cadence, all 29272 raw LCG edges from genuine initial 4067602209,
ordered particle sites and ledger endseeds. Raw checks preserve held handle,
fighter owner identity, part 40, visibility, state 2, lifetime 1400 and the
stale raw Item.pos(0,7.020100117,0), plus fighter motion/commands. Raw Item.pos
is not a hand/root matrix sample: no bone/cache exactness or broader held
attachment lifecycle is claimed, and no hand/cache/effect code changes.

Direct next-Animation/IASA tests assert local pre-body state preservation;
public next-step faults use the existing API fault contract, not a whole-tick
rollback guarantee. The public step/observe/capture test covers independent
continuation after cloning before entry; clone construction is outside the
allocation counter. Public live display cadence and the raw trace cadence
are verified separately. Evidence is retained at coordinator
`target/wave21/held-bair-entry/`; no retail data or expectations are replaced.

The raw interrupt flag comparison follows its actual callback ownership:
common attack-aerial rows consume it and compare bit-exact, including the
terminal entry. Retail Fall 29 at tick 336 retains the previous unheld forward
aerial's true bit, but Fall entry/Animation/IASA never consume or write it;
existing reset_motion normalizes that unused port control to false. An
explicit regression preserves both values there. This is semantic control
projection, not physical fp2218 parity throughout every prefix state; the
global reset and raw retail expectation remain unchanged.

Pickup attachment assertions apply to the actual held Sword kind. The
capture also contains Fox's held blaster (kind 74 at tick 472): its existing
static HELD_PART presentation path does not represent raw xDC4 in
ItemCore::holder_part. All-item kind, state, owner, held flag, lifetime and
position comparisons remain; no raw attachment-part parity for unrelated
articles is claimed. No blaster or attachment runtime code changed.

Six sibling original whole captures use the same terminal common Back Air
entry: 1294, 969, 1545, 1387, 1103 and 974 rows. Together with the primary
621-row capture, strict checks cover 7893 rows and 290079 genuine initial
LCG edges, full particles and ordered particle sites. Each capture ends at
its first held aerial row; this adds no later-callback witness. The primary
621-row test separately checks raw Sword ownership and held metadata at
actual raw display cadence. Public API allocation and clone continuation
proof are separate from that raw-cadence check.

The continuation guard uses immutable move-entry ownership in the typed
Aerial payload, captured only by actual aerial entry before its direct
script step. It is never advanced or cleared as a tick flag. A blanket
current-hand guard initially regressed the existing 1344-row
`corpus_v3_fd_items_sword_e0144d061_p1`: Marth enters Back Air empty-handed,
then catches a Sword at tick 1293 without changing motion. Retail and port
remain in that existing move at tick 1294. That catch preserves entry
ownership; a subsequent actual entry with the same hand recomputes it,
even though the airtime-wide catch lock persists. The real callback order
and scheduling remain unchanged.

The old mid-aerial catch capture has its interrupt flag unlocked at ticks
1293 and 1294. Its complete existing input sequence remains exact; this
does not prove the omitted held-item throw-first query for other inputs.
No new throw-routing body or catch lifecycle admission is included. The
new held-on-entry contract refuses its next real callbacks independently
of the interrupt flag, frame, timer or scheduler phase.
