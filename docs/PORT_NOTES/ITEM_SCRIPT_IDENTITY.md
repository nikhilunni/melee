# Running item command identity

`Item_80268E5C` changes an item's motion and article descriptor without
replacing its command pointer unless `ITEM_ANIM_UPDATE` or `ITEM_CMD_UPDATE`
is set. Retail 0x80269314..28 and 0x80269340..54 reset that pointer, timer and
control stack together. With neither flag, execution continues the old
program; a negative animation id clears it. `it_802799E4` follows the live
command pointer rather than looking up the current motion's script again.

The port flattened each motion's program separately, but preserved only its
instruction offset through a non-resetting transition. A rolling Barrel's
drop (it_80287F20, flags 0x15 at 0x80287F40) changed motion 5 to 3. Its suspended
PC 2 then addressed motion 3's appended subroutine rather than motion 5's delayed
SetVariable command. The later Return correctly exposed an empty call stack.

ItemCore now retains the source motion of its active immutable program and
uses it for both commands and parallel hitbox flags. Actual animation/command
resets replace the source; no-animation and explicit static-pose clears
remove it. Clone copies source and execution state together. End retains the
existing inactive execution state. Generic ScriptState and all item motion
admissions remain unchanged.

The existing whole captures `corpus_v3_bf_items_sword_e5993e1bc_p2` (398 ticks)
and `corpus_v3_fd_items_taru_e17844051_p1` (369 ticks) observe the rolling drop,
continued original pointer/timer and delayed variable 0=1 followed by NULL.
BF opens and creates a Sword on its final tick; that endpoint does not prove
later opening cleanup or Sword flight. The raw regression checks each owned
capture's actual display cadence. Strict oracle tests retain complete fields,
items and particles, ordered particle call sites and ledger-cadence seeds,
with a separate raw ordered LCG audit. Each row-zero setup draw also has an
independently captured incoming seed, recorded before the first tick and
checked against the reused boundary. This is not a comparison of every
nonparticle port caller identity.

Synthetic tests reproduce the old Return failure, preserve nested call/loop
state across clone, distinguish the command program's hitbox flags, and check
animation reset, command-only reset and clear semantics. The public API test
steps/captures both full recordings without allocation and clones the actual
suspended program after each drop, comparing independent continuations.

Evidence and final validation/provenance are under
`target/wave18/item-script-identity/` in the coordinator checkout. These two
match-start recordings do not establish arbitrary running-item warm imports,
new item kinds, heavy or held aerial rows, or a broader ownership lifecycle.
