# T7 fighter animation playback

The FD playback test matches **600/600 recorded frame-end states for P0 and
600/600 for P1**, including the submotion id, frame bits, remainder, speed,
blend duration and blend progress. Frame 0 initializes P0 at 5.0, blend 6/6,
and P1 at 0.0, blend 6/1. The test consumes FIRST_PLAY once during restoration
and compares frames 0 through 599; later state is advanced, not loaded from
the trace. Both Wait1 and Wait2 resources and the choice table are read from
the owned disc.

## Ports and integration

| C entry / retail address | Rust implementation |
| --- | --- |
| ftAnim_8006E9B4, 0x8006E9B4 | FighterAnimation main step, active AObj frame and loop remainder |
| ftAnim_8006EBA4, 0x8006EBA4 | step / step_with_hooks: main step, commands, part blends, accessories |
| ftAnim_8006EBE8, 0x8006EBE8 | set_animation: remove, restore, attach, request, loop/rate, blend reset |
| ftAnim_8006F4C8 / ftAnim_8006FCE4 / ftAnim_8006FE08 | attach_motion: same-kind part masks or source-to-destination part remapping |
| ftAnim_8006E7B8, 0x8006E7B8 | animate_parts: eligible joints bounded by preorder depth |
| ftAnim_8006F3DC, 0x8006F3DC | current_aobj: first eligible main AObj or first blend-tree AObj |
| ftAnim_IsFramesRemaining, 0x8006F238 | frames_remaining: eligible AOBJ_NO_ANIM checks on selected tree |
| ftAnim_800707B0, 0x800707B0 | advance_parts: five independent part blend states |
| ftAnim_8006FE9C / ftAnim_8006FF74 | per-part blend/copy routing from the secondary skeleton |
| ftAnim_8006FA58 / ftAnim_8006FB88 | restore descriptor SRT below TopN, preserving rotation/part ownership |
| ftAnim_8006F0FC / ftAnim_SetAnimRate, 0x8006F190 | set_rate: both trees or deferred saved rate |
| ftAnim_8006E054, 0x8006E054 | RootMotion: translation histories, scale, secondary extraction and compensation |
| lb_8000C490 / lbCopyJObjSRT, 0x8000C7BC | blend_pose / copy_pose: fused S/T and quaternion routing |
| fn_8001E60C / lbAnim_8001E7E8 | melee-lb translation-filtered track attachment |
| ftCo_8008A7A8, 0x8008A7A8 | choose_wait_animation / update_wait |
| ftCo_8008A6D8, 0x8008A6D8 | animation-only restart, including immediate evaluation |
| ftCo_Wait_Anim, 0x8008A494 | animation-only Wait callback, run after the main step |

The hexadecimal suffixes above are the retail addresses where no second
address is written. ftAnim frame stepping reuses HSD_AObjInterpretAnim
(0x80364190), rather than independently incrementing Fighter.cur_anim_frame.
A completed blend retains its duration and secondary skeleton.

T6 owns ftAction_80073240 and command/dynamics resets. Use step_with_hooks
and update_wait_with_restart to execute those at the original call sites,
including inside a Wait restart. The plain step/update_wait methods run
animation only. ftCo_800DB500 has an accessory hook. T10 owns fighter state
transitions and the item predicate; pass None for the no-choice branch.
The FD test proves clocks/choice/hidden blend state, not 600-tick bone-matrix
parity or command-script execution. Active part blending is separately tested.
TObj/PObj animation remains outside the existing hsd-anim representation.

## Fighter fields

| Offset | C / Rust |
| --- | --- |
| +014 | anim_id / motion_id (submotion; action-state id is +010) |
| +590 | x590 / Motion.animation, externally owned FigaTree |
| +594 | x594_s32 / MotionFlags: root, loop, remainder, secondary-root and scale bits; 13-bit bone mask |
| +5E8 | parts / AnimationPart: joint, part flags (+8), depth (+C), conditional mask |
| +68C, +698, +6A4, +6B0 | primary translation position, previous position, delta, previous delta |
| +6C0, +6CC, +6D8, +6E4 | corresponding secondary translation history |
| +894 | cur_anim_frame / frame |
| +898 | x898_unk / remainder |
| +89C | frame_speed_mul / speed |
| +8A0 | x8A0_unk / saved_speed |
| +8A4 | x8A4_animBlendFrames / blend_duration |
| +8A8 | x8A8_anim_frame / blend_progress |
| +8AC | x8AC_animSkeleton / blend_tree |
| +8B0..+913 | five PartAnimation entries, stride 0x14; state, duration/progress/rate, previous/current selections |

Model scale is supplied from co_attrs.model_scaling (+19C). Root-motion
configuration carries the selected TransN/secondary/model joints and the
x2221_b2 && !x2226_b2 compensation predicate.

## Retail arithmetic and filtered tracks

Re-ran `UV_CACHE_DIR=/tmp/melee-t7-uv uv run python asm.py <symbol> --fused`
from harness for the step, attach/request, part blend, root extraction and
HSD_AObjInterpretAnim functions. None has a fused instruction. lb_8000C490
has six fmadds at 8000C4C0, 8000C4D4, 8000C4E8, 8000C4FC, 8000C510 and
8000C524. Each computes target * weight + an already-rounded existing *
inverse product. Source comments cite these sites; a cancellation regression
separates the fused result (-2^-46) from the unfused result (zero).

The filter increments the track pointer at 8001E6A0 only after acceptance;
the iteration counter increments unconditionally at 8001E6AC. A rejected
translation track (type 5, 6 or 7) is therefore reconsidered for all remaining
iterations. The attached list is the accepted prefix, not a conventional
filter over the whole list. With a nonempty list starting with translation,
no FObj is allocated: C's final FObj variable is uninitialized, and retail's
8001E6BC store uses the incoming track pointer. That corrupting input returns
NoAcceptedTracks in Rust. A zero-count node remains a no-op.

## Wait-choice ledger evidence

Each listed draw is at lr-4 == 0x8008A8BC and is the first draw of its tick.
The previous tick's final seed supplies its pre-draw state. Particles and
other consumers remain in the recorded seed history; no fighter-specific RNG
stream is invented. Each of the nine choices consumes exactly one draw.

| Trace frame | Player | Chosen submotion | Seed before | Seed after |
| --- | --- | --- | --- | --- |
| 115 | P0 | 2 | 0ca17126 | 0fc26351 |
| 120 | P1 | 3 | 5a638186 | e3f5b231 |
| 235 | P0 | 2 | 4e8b7361 | 794d08a0 |
| 240 | P1 | 2 | 1540e3b1 | a7f1f7b0 |
| 355 | P0 | 2 | a846c113 | 15d4678a |
| 360 | P1 | 2 | 16b7d845 | 1a4069f4 |
| 475 | P0 | 2 | 1e9fa85b | 0610d1b2 |
| 480 | P1 | 3 | c4062a23 | d5ae6c5a |
| 595 | P0 | 3 | f5c3dbec | b909baff |

Wait2 cannot repeat. Fixed-seed unit cases exercise retry separately: from
DEADBEEF, roll 78 selects Wait2; if current is Wait2, it is rejected, roll 26
selects Wait1, and the final seed is 42185CE1 after exactly two draws.
The tests also pin cumulative 70/71 boundaries and the repeatable id 31.

## Validation and file list

Latest focused run: 18 T7 tests pass (two choice, two C-oracle, thirteen
behavior, one live FD replay). The oracle compares 100,000 inputs with zero
bit differences and verifies its two C excerpts against the pinned decomp.
It covers finite fractional/negative rates, first play, stopped animations,
loop rewind, overshoot, blend progress and loop remainder. FObj/SRT calls are
stubbed in this pure-arithmetic oracle; scalar clocks execute the original C.

`CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo gate` passed on the final tree.
`CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets -- -D warnings`
passed. An earlier clippy run found six unused_parens errors in the other
task's input/iasa.rs. After that task's file changed, clippy was rerun and
passed; T7 made no edits to input/. No tests or expected values were weakened.

T7 changes:

- crates/melee-ft/src/anim/{mod,attach,blend,playback,root_motion,wait_choice}.rs
- crates/melee-ft/src/anim/README.md
- crates/melee-ft/src/desc/{mod,playback}.rs
- crates/melee-lb/src/anim.rs
- crates/melee-ft/tests/{fighter_animation,real_fox_wait_playback,animation_ref_oracle}.rs
- crates/melee-ft/tests/ref/animation/{aobj.c.inc,ftanim.c.inc,driver.c,NOTICE}
- crates/melee-ft/Cargo.toml and Cargo.lock (serde_json dev dependency)

No commits. No changes to lib.rs, input/, TRACKER.md, CLAUDE.md, root
Cargo.toml, third_party/ or harness/. No game data added.
