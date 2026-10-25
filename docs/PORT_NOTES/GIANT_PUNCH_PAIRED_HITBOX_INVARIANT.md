# Giant Punch paired-hitbox invariant

The partial Giant Punch callback (ftDk_SpecialN_Anim, 0x8010E930;
ftDk_SpecialAirN_Anim, 0x8010EDD8, and Kirby's copied twins
ftKb_DkSpecialN_Anim, 0x800FFC94, and ftKb_DkSpecialAirN_Anim,
0x80100128) tests capsule 0 before reading
both capsules' base damage (ftdonkeyspecialn.c:165). Replace the missing
capsule unimplemented diagnostic with an explicit invariant panic. No
arithmetic, damage, callback, dispatch or capability changes. Retail
0x8010E9D0..0x8010E9D8 tests capsule 0; 0x8010EA3C reads capsule 1
damage without a separate capsule-state check. The named callback addresses
were individually checked against the symbol map and asm.py disassembly:
full ground 0x8010EB0C and full air 0x8010EF7C as well as the partial
callbacks above. Corrected the shared callback doc comment accordingly.

Original command owners, independently checked against live archive bytes:

| Archive / public table | Submotions | SHA256 |
| --- | --- | --- |
| PlDk.dat / ftDataDonkey | 322,323,327,328 | c6cd090eda7e96da6fbd6d5dd4296a91eab7cab99c5b007244b090f5de799d6f |
| PlKb.dat / ftDataKirby | 393,394,398,399 | bae86007bbdcdd6bea67ddc0309280c47f7e94557de0f7d5469b5c19d0755cd7 |

All 212 executed command records across the eight finite paths were checked;
all 56 conservative wait/frame/end boundaries have both-or-neither 0/1.
Spawns are unconditional group 0 (no throw-owner requirement), paired in one
immediate batch; removal is clear-all, with no single-clear opcode. Full
scripts loop exactly three times. Full punches carry script damage; only
partial punches call the charge helper.

Fresh entry uses flags 0 and clears earlier active capsules before the real
script step. Ground/air conversion uses SkipHit + UpdateCmd: it retains the
pair and advances control without executing owner commands. Ordinary seek
skips creation and clearing together. Collision history updates do not
remove resident capsules, and the callback runs after the complete batch.
This closure is actor-scoped: Ness's real unregistered per-index revival
must survive motion changes. Only typed Ness yo-yo dispatch writes that
revival state; DK and Kirby cannot run it, and copied Ness uses PK Flash.
Clone preserves the registered actors' invariant.

This proves reachability only for registered DK/Kirby with original assets,
not arbitrary GiantPunchUser clients, modified archives or caller-mutated
command state. Such malformed states still panic; no stale/disabled damage
is invented. Scaling and smash-charge refusals, integer/float conversion,
staling and every actual unported branch remain unchanged. No new recording,
script words or game data are committed. Focused and final validation
artifacts live under target/wave22/giant-punch-invariant; reported acceptance
must remain separate from this source/data proof.
