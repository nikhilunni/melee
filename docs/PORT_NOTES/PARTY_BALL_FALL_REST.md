# Party Ball: neutral FALL and REST

This ports only the existing `start_fd_items_kusudama` capture's left-facing,
unowned, unheld, zero-velocity spawn, FALL 2, flat-floor bounce and REST 0.
The warm and cold TOMLs check the **same complete 600-tick retail capture**;
they are two initialization paths, not independent recordings. Spawn is at
461, bounce at 480, settle at 495 and REST continues through 599. No game data
or recording was written.

`itKusudama_Logic4_Spawned` (0x80289784), `it_8028A3CC` and the FALL/REST rows
keep the common item animation/lifetime processing. Both rows continue
`it_802897C8`'s pitch easing, with the literal ten-degree snap distance and
five-degree step; retail uses separate subtract/add operations, no FMA.
`fn_8026C88C` initializes all three spawn velocity components to zero
(itspawn.c:170; stores 0x8026C930/34/38). The first raw Party Ball snapshot,
after its first gravity proc, has velocity (0, -0.1199999973, 0).

The ordinary `air_collision_with_landing` helper keeps bounce reflection,
spread draws, sound and settling order. `it_8026DD5C` resets the root's spin
before `it_8028A190`; the latter changes motion at 0x8028A1CC and writes pitch
at 0x8028A228. `Item_80268E5C` flushes efAsync at 0x80268F44. The Party Ball queues
the existing scoped `Resume` for the REST pitch tail after changing motion,
so the prepared bounce `JointEffect` resolves the pre-pitch root first.
No shared drain ordering changed. The later facing-yaw and outer collision
translation writes are idempotent for this neutral left-facing article-1
path. This does not establish a generic continuation across scripts, models,
owner transitions or differing facing.

The recorded tick 495 spark is (75, 0.6751003265, 0), with y bits `0x3f2cd360`.
An actual simulation regression asserts that retail constant and final REST
pitch, common lifetime through 599 and continuing REST easing. A split-proc
regression checks zero pre-flush root pitch, `JointEffect` before `Resume`,
false early-phase origin flags and the exact spread prefix:
`0x6b9922fb -> 0xd396e1d2 -> 0x3ed3c14d -> 0xc6eacedc`.
The existing Box/Barrel early/late phase-and-restoration regression remains.

Verification terminology is deliberately precise: full fighter/item fields
include each tick's RNG seed, and strict checks compare **ordered particle
RNG call sites over all 600 ticks**. The raw global ledger and retail asm are
a separate audit: all 23,294 recorded transitions across 600 rows form the
LCG stream from the cold setup seed 2269040402, and contacts 480/495 start
with spread sites 0x80278A30/54/78,
then particle sites. The tooling does not globally instrument every
non-particle HSD RNG call, so this is not a general global call-site ledger
comparison. The named strict test also compares each port tick's seed
against the ledger under its own display cadence and audits every captured
LCG transition. No prefix truncation or particle-state waiver is used.

Positive facing, owned/held or nonzero-velocity spawns, slopes, wall/ceiling
contacts, sliding/floor departure, pickup/throw/drop, damage/shield/clank/
reflect, destruction and all other states remain guarded. Only supported
article-1 scripts are loaded; unsupported opening/color scripts 24/25 remain
unavailable behind callback/entry guards. Those empty asset slots must not
become a fallback for future state transitions.

Release acceptance evidence lives under
`target/wave16/partyball/partyball-*`: both complete 600-tick
capture checks and ordered particle-site checks, all 37 prior random-item
witnesses, actual API step/observe/presentation capture allocation checks,
three continuation regressions, item unit tests and all-target clippy.
Against baseline `486aaba1420d923734948f09def10e23cae5d660`, the original-base
full sweep checks 3,485 scenarios: zero regressions, two fixes (the warm/cold
Party Ball pair), 3,292 EXACT, 115 NOTCMP, 77 existing failures and one existing
expected FAULT. Replay A remains 108/108 complete with 964,782 matched frames;
B remains 486/502 complete with 5,102,652 matched frames. Every path preserves
its category and matched count. These broad runs precede integration with
Kongo's separate runtime change; the combined checkpoint belongs to the
coordinator.
