# Refusing unsupported mixed item contacts

`ftColl_80077C60` clears the victim's pending phantom log at retail
0x800780B4 (`ftcoll.c:1233`), before ordinary item victim recording,
invincibility and shell handling. The port's ordinary damage logger already
performs that clear. Invincibility and an intact shell return before the
logger, however. A bounded existing-recording search found complete ordinary
item contacts but no complete witness of an earlier pending phantom followed
by one of these item early returns. See coordinator scratch
`target/wave20/item-phantom-search/plan.md` for exact search limits; this is
not an exhaustive archive or reachability claim.

Until that combined response is witnessed, these two paths explicitly refuse
with `unimplemented!` and the retail C lines. They do not implement the clear
or admit a new item kind/state. The guard examines only the current pending
log, after pure damage scaling and before ordinary history/effect/health
changes. Its shell lookahead is the single subtraction at 0x80078200 in the
same operand order as `through_shell`; only a negative result breaks the
shell, so zero and unordered results refuse. Ordinary damage and shell break
still reach the existing logger. Empty pending logs, phantom-only contacts
and an older resolved phantom/active lockout remain supported.

The collision regression uses an existing CBeam ordinary-contact pose plus a
synthetic pending entry. It checks both refusal messages, exact-zero and NaN
shell boundaries, unchanged ordinary response state, normal damage/shell
break controls and an actual shallow item phantom contact. This is refusal
coverage, not a retail witness of the combined response; collision queries
and legitimate earlier phantom contacts are not rolled back.

Four unchanged complete retail captures protect the supported item paths:
`kirby_stone_lasers_fd_fox4` (460),
`kirby_stone_laser_break_fd_fox4` (520),
`corpus_v3_fd_kirby_captainfalcon4_ee42e8dad_p0` (6001) and
`corpus_v3_ys_kirby_fox4_ea4d5d9ec_p1` (5542). The raw regression consumes
every row at its actual display cadence and compares shell health/flag,
phantom lockout/knockback, own hitlag and positions. The strict oracle checks
all fields/items/particles, ordered particle call sites and ledger-cadence
seeds; a separate raw LCG audit starts from each genuine initial observation,
validated against its boundary sample and importer seed checks. Ordered
particle-site comparison is not a global RNG caller tracer. The existing
four complete fighter Stone/phantom controls retain their original checks.

Validation artifacts and final revision/binary provenance are retained under
coordinator `target/wave20/item-phantom-guard/`. No recording, protected data,
retail expectation or baseline is replaced by this change.
