# Swing42 original command-owner invariant

This changes only the missing static followup diagnostic from `unimplemented!`
to an explicit invariant panic. Captain's existing private Sword row341 and
all entry, rate, script, callback, hand and input behavior are unchanged. No
item kind is admitted and no newly ported gameplay/function gain is claimed.
Malformed/custom scripts or state still fail loudly.

## Source closure

`ftswing.c` get_anim_id (retail0x800CD034..800CD080) selects the private arm
only for input3; only Captain kind2 owns341+itemclass, otherwise retail asserts
at C97. The five actual callers of `ftCo_Attack_800CCF58` are Attack1(0),
AttackS3(1), AttackS4(2), AttackDash(4), and `ftCo_800CD204`(3). All six item
swing IASAs delegate to CD204. Callback tables reference those IASAs in the24
common swing rows and Captain's six private rows only. Ganondorf's similarly
named private comments have NULL animations/callbacks, not executable IASAs.
CPU behavior uses the same entry/input paths, not another input3 writer.

All six retail wrappers enter via CD140. `0x800CD170` clears the entire throw
word before ChangeMotionState at0x800CD17C and direct script stepping.
NULL borrow-source/frame0 replace an earlier capture/thrower's command
pointer and loop/return stack with actor x24[anim_id].xC (`fighter.c1252..1266`).
Parasol entry800CD4E0 passes0x400; the other wrappers pass0. That flag does
not select borrowed commands (Harisen800CD5A8 passes0). The only positive b3
writer is ftAction800718A4 opcode20/index0 (`Command::GrabRelease`); the Rust
latches clear together. CD204 tests/clears b3 at0x800CD228..23C and tests the
continuous-A scratch at0x800CD254..25C before requesting3 at0x800CD260..264.
A release cannot be undone by a later press. Hitlag, seek and skipped steps
cannot manufacture a writer absent from the entire graph. Reentry clears
again; leaving the motion replaces its callback owner.

Ordinary initialization sets x24 to actor ftData.xC (`fighter.c727..735`).
Nana's ftData80085FD4 model/animation fallback to Popo does not install Captain
commands; both Nana and Popo are in the audit. All25 PlKbCp data roots are real
copy-specific roots, but do not replace Kirby's ordinary x24 swing command
table; PlKb's common scripts are audited. Costumes add no command owner.

## Demo/menu/cutscene fence

The alternate x24 assignment in `ftDemo_CreateFighter` (ftdemo.c68..72) is
not silently excluded. Demo creation registers animation1/update4/map5/
accessory9/camera16, but not Input/IASA link3 or CPU0 (ftdemo.c117..121).
The sole direct input_cb dispatch is inside Fighter_Spaghetti_8006AD10
(fighter.c2136); its only production registration is ordinary Fighter_Create
at900. No demo handoff registers or directly calls it on an existing demo.
Ordinary creation allocates a new GObj. All23 demo rows (14 common plus
2Mario/2Luigi/4Kirby/1Giga private) have NULL input and no swing IASA; the three
nonNULL common animation callbacks800BED84/BEF00/BEFD0 are empty. Private
rows only supply the listed physics callbacks. All16 on_create_fighter slots
and helpers enter demo/dead/wait/walk rows, suppression flags or articles,
never a normal swing entry/CD204/CCF58. Demo state14 is not common Wait14.
The only x1C/x20 table writers are ordinary initialization and this demo
override; ordinary reinitialization also restores actor x24 before publishing
an ordinary owner. input_cb has one assignment from the resolved motion row.
Thus no unreset demo-command-to-normal-swing-Input path exists. This uses
callgraph/table ownership, not unsupported-mode or corpus-absence reasoning.

## Reproducible authored-data property

Run release `melee-ft` test
`original_swing_command_owners_only_captain_requests_followup` with
MELEE_DATA_ROOT pointing to the main checkout. It enumerates Pl*.dat/.usd
public headers using hsd-archive, separating all33 ordinary ftData roots from
all25 ftDataKirbyCopy roots; .usd adds none. The descriptor owns row layout;
the test reuses production read_script/decode/word_count to read all reachable
Call/Goto bodies and complete loop bodies across animations108..131 (all six
retail item classes),792 graphs/744 nonnull. Repeating a finite body cannot
introduce another writer. Exactly PlCa110/114/118/122/126/130 contain
GrabRelease. No script words or game assets are committed. The independent
bounded execution audit also checked finite control flow:16649 executed
records,35 finite loop entries, zero parse/control errors or animation loops.
Its per-file hashes/counts remain in coordinator scratch wave24 artifacts.

A new command-source import, dynamic table mutation, demo handoff or custom
script admission invalidates this closure and requires re-audit.

## Behavioral acceptance and limits

The new unit test runs complete existing Captain520 and Peach1400 with their
actual raw display cadence. Captain must execute authored common smash to
private341 with animation295 and followup scratch. Genuine live swing clones
exercise real static IASA: Captain positive, nonCaptain malformed/defaultNone
specific panic, A-release/no-rearm and whole-word/script/control-stack reset
through real ground input entry. These isolated mutations are controls, not
new retail recordings or claims that Peach requests followup.

Mechanical acceptance additionally requires all seven existing swing gates,
strict matrix/random/Sword ground/aerial/catch and human cross-sections,
touched release tests/clippy, all3490 full sweep rows byte-identical to the
canonical55 baseline and both610 replay tuples unchanged. Final logs and
source/binary/manifest/commit binding live in coordinator scratch
`target/wave24/swing42-invariant`; no protected data is changed. The
coordinator owns the integrated full workspace/WASM/performance checkpoint.

## Outgoing motion-change callback fence

The flag clear alone is not the whole temporal proof: fighter.c1211 invokes
old x21EC inside ChangeMotionState, before the new command pointer is installed.
A complete source assignment/callee audit covers31 positive installations and
18 distinct file-qualified targets. Nine Fox/KirbyFox/DK/Yoshi/Purin/
KirbyPurin/MarsS/common-jab/common-down-tilt wrappers call only892A0/89824
attack-instance/statistics tails. Six Luigi/Pikachu/PeachS/Captain/MarsN/
KirbyMars hooks clear command variables or update typed scratch/velocity
(Luigi also draws Randi). One Link/YoungLink hook updates boomerang/stat
metadata. Barrel and Thrown hooks call8007B8CC, whose body (ftcoll.c3391)
only writes thrown-owner/team/player association. Complete bodies/callees
contain no script step/seek/execute, nested motion/swing entry, or positive
b3/throwword write. Thus this pre-install callback cannot resume old borrowed
commands after CD140's clear. The later callback-clear block at
fighter.c1370..1389 removes old accessory/damage/hitlag writers separately.
This closure depends on the original callback set; a new x21EC writer or
dynamic callback requires re-audit. The independent31/18 catalog and per-body
source hashes are retained in coordinator wave24/swing42-review artifacts.
