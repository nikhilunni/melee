# Revival platform floor entry

`fn_800D5A30` (ft_0D4D.c:367) grants revival protection/color at800D5A4C,
then calls ordinary neutral entry at800D5A54. `ft_80083844` publishes the
resolved map position at800838AC..B8 before invoking that callback. The
shared static revival collision wrapper now keeps that order: resolved
position, existing signed-max/color9 grant, ordinary landing, existing Wait
entry and static character hooks. Rebirth12 retains its stay-in-air ECB pass.
Landing preserves Y self velocity. The existing input and timeout exits share
the extracted grant helper without changing their own entry/grant order.

One existing complete1679-tick retail capture,
`corpus_v3_kj_fox_marth4_e0c8363c7_p1`, ends with the floor callback at1678:
Marth's Rebirth12 at1677 becomes Wait14/Ground at1678, with position
(20,65.67594909667969,0), floor12, jumps0, invincibility120, actual primary
color9 /ffffff80 and name-tag50. RebirthWait13 exists only inside that final
tick after animation completion. This is an **entry-only floor witness**:
there are zero later floor-exit ticks. No new post-entry lifecycle callback
is implemented or inferred from it. The ordinary Wait/protection/color
callbacks are already active and independently exercised by existing data.

The raw scalar regression executes every1679row, checks real protection and
primary color ownership throughout, the actual terminal pose/velocity/ground
state and color program, and both earlier barrel holds/launches. Two existing
900-tick Fox/Marth timeout captures independently protect the shared helper:
all raw protection/color ids are checked through the120->119->1->0 expiry.
They are not additional floor-contact continuation recordings. Strict m5
checks whole fields/items/particles and ordered particle sites, plus every
sequential ledger row, raw LCG result and the port's end seed under that
ledger's own display cadence. The LCG audit does not globally instrument
nonparticle call sites. Actual public step/observe/presentation capture
allocation coverage includes the whole1679tick entry recording.

No new recordings, protected-data writes/copies/symlinks or retail reference
edits. Existing item-landing and other neutral capability boundaries retain their
owners; no new dispatch or character-kind match. Hammer-held floor entry
explicitly faults before ordinary Wait. x2224_b2 is the stamina-death flag
(ft_0C8C.c), not an input-replay mode; supported normal stock setups do not
set it. No new broad import guard is introduced.
The supported Character table excludes Master/Crazy Hand. Peach parasol
cleanup uses WAIT_ARTICLES, held idle uses play_wait_holding_idle, name-tag
uses existing Wait entry, and Link/Young Link tails use WAIT_ENTERED.
The bounded search of70Kongo/revival-referencing captures found no second
floor callback or longer same-seed sibling. Search evidence is in MAIN
`target/wave16/revival-floor-search/`; implementation acceptance is preserved
in MAIN `target/wave17/revival-floor/` with final commit/base provenance.
