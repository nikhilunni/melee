# T9 human input port

The public entry points are `input_source`, `run_cpu_input_proc`,
`update_input` / `update_human_input`, and `wait_iasa`. The caller loads
`InputCommonData::read(&plco_archive)`, supplies the match/fighter gates, applies
returned Player joystick-count increments, then dispatches the input callback
when `InputEffects.run_input_callback` is true. T10 owns that callback wiring
and the actions represented by `WaitTransition`. No RNG is accepted or drawn.

## Ported functions

| Retail function | Address | Rust |
|---|---|---|
| `Player_8003248C` | 8003248C | `resolve_player_kind` |
| `ftCo_800A2040` | 800A2040 | `input_source` (does not read PlCo) |
| `Fighter_8006ABA0`, s_link 2 | 8006ABA0 | `run_cpu_input_proc`; CPU execution explicitly unimplemented, melee-cpu M5 |
| `Fighter_Spaghetti_8006AD10`, s_link 3 | 8006AD10 | human sampling, history, edges, analog/button timers, reset gates |
| `Fighter_ResetInputData_80068854` / `Fighter_UnkInitLoad_80068914_Inner1` | 80068854 / inlined | input defaults / `clear_current_and_buffers` (saved history preserved) |
| `Fighter_UnkIncrementCounters_8006ABEC` | 8006ABEC | jump and four special buffers |
| `ftCommon_8008031C` | 8008031C | joystick statistics events and their timer resets |
| `lb_8000D148` | 8000D148 | origin-centered stick-circle crossing, exact math retained |
| `HSD_PadClampCheck3` | 80376E90 | Melee configuration: min=0, max=80, shift=1 |
| `HSD_PadScale` in `HSD_PadRenewMasterStatus` | 8037750C | stick division by 80, shoulder division by 140 |
| `ftCo_Wait_IASA` | 8008A4D4 | all 21 ordered predicates for item-free Wait, enum transition result |

The Wait list is **side special, up special, neutral special, down special,
grab, side/up/down smash, side/up/down tilt, jab, escape, shield, Fox taunt,
taunt, jump, dash, squat, turn, walk**. Despite its name,
`ftCo_Attack100_CheckInput` checks **up special**. The per-predicate addresses
are beside the implementation. Neutral predicates do not call trigonometry:
the A-button conditions short-circuit the tilt-angle queries.

## Fighter fields

Internal structures group related state, rather than copying retail layout.
Every public field carries its C name/offset in its doc comment.

| C fields | Fighter offsets | Meaning |
|---|---|---|
| input.lstick[0/1/2].x/y | 620/624, 628/62C, 630/634 | current, previous, saved main stick |
| input.cstick[0/1/2].x/y | 638/63C, 640/644, 648/64C | current, previous, saved C-stick |
| input.triggers[0/1/2] | 650, 654, 658 | maximum shoulder analog |
| input.held_buttons[0/1/2] | 65C, 660, 664 | current, previous, saved buttons |
| input.pressed_buttons / released_buttons | 668, 66C | rising/falling edges |
| x670_timer_lstick_tilt_x, x671_timer_lstick_tilt_y, trigger_analog_timer | 670, 671, 672 | tilt/press age |
| x673, x674, x675 | 673, 674, **675** | repeated analog hold buffers |
| x676_x, x677_y, x678 | 676, 677, 678 | ages since threshold crossings |
| x679_x, x67A_y, x67B | 679, 67A, 67B | activity buffers consumed by Player statistics |
| x67C, x67D, x67E, x67F, x680, x681, x682 | 67C–682 | A, B, XY, LR, digital L/R, up, down ages |
| x683, x684 | 683, 684 | previous A and digital L/R press ages |
| x685, x686, x687, x688, x689 | 685–689 | jump, up/down/side/neutral special ages |
| x68A, x68B | 68A, 68B | previous jump/up-special input ages |
| x221D_b3, x2228_b7, x2229_b0 | 221D, 2228, 2229 | previous-history source and last smash directions |
| cpu.buttons, cpu.lstick_x/y | 1A88, 1A8C, 1A8D | untouched on human path |
| cpu.xC, cpu.level, cpu.x18, cpu.x7C | 1A94, 1A98, 1AA0, 1B04 | mode, level, behavior, random timer; untouched |

`fighter.generated.yaml` inherits the C header's stale +674 comment for x675.
The actual field is +675, confirmed by retail `stb` at **8006B4D0**. The test
asserts this known schema discrepancy and compares the actual byte at +675;
neither the header nor schema was edited.

## Neutral pad evidence

The SDK's `SPEC2_MakeStatus` subtracts 128 from wire stick coordinates and then
applies origin calibration (`extern/dolphin/src/dolphin/pad/pad.c:954-965`).
HSD receives **signed zero**, not 128. Its disconnected-controller branch also
zeros button, stick, trigger, and normalized fields (`controller.c:388-407`).
`controller.h` and retail's `addi r26,r26,0x44` at 8037798C establish an HSD
status stride of **0x44**, not the older docs/DOLPHIN_RUN.md table's 0x2C.
Master/Copy/Game array bases are 804C1FAC / 804C20BC / 804C21CC, each size 0x110.
The fighter reads GameStatus normalized floats, not CopyStatus's edge words.

In **all 600 records, both fighters** in `idle_fd_fox.tick.raw.jsonl` have:

- +620..+66F: all bytes zero, including all three histories and both edges;
- +670..+67B: all bytes FE;
- +67C..+68B: all bytes FF;
- the seven canonical CPU values: **0, 0, 0, 4, 1, 1, 2** for p0 and
  **0, 0, 0, 4, 1, 1, 0** for p1.

Those observed zeros are the *Fighter input block*. This raw trace has no HSD
pad-memory capture, so it cannot independently reveal pre-deadzone stick bytes,
wire coordinates, or whether a controller was connected. The zero PadSample
is established by the neutral scenario and SDK/HSD processing, then verified
against every recorded Fighter input byte. The CPU timer 4/0 in the older M3
plan describes the Yoshi's Story capture, not this FD capture's 2/0.

## State owned elsewhere

`InputContext` makes x221F_b3, x2224_b2, x2219_b5, x221D_b4 and the debug/match
pad gates explicit. The port retains accumulated hitlag edges, saved-history
selection, C-stick suppression, the special single-button mode, Z macro
suppression, and save/reset ordering. `x221D_b3` and the last-direction bits
are updated on FighterInput; the owner mirrors them if it stores other flags.

Capture (`x1980 != NULL`, ftCommon_8007FFD8) and active smash charging
(ftCo_800DF0D0, PreCharge/Charging) assert if reached outside hitlag. Wait IASA
asserts that there is no held item/hammer, active tether, or jab countdown
(`hitlag_mul` at +196C); these require their state owners. Special callback
availability, shield health, facing, and Fox/Falco stage-taunt eligibility are
explicit WaitContext inputs. An active CPU arm panics with its M5 owner.
No unsupported state is silently entered. The trace test checks the raw
capture/item/charge/jab and proc gates on **each** record.

## Fusion audit and verification

Run from `harness`, with `UV_CACHE_DIR=/tmp/melee-t9-uv` if the default uv cache
is sandbox-blocked:

```
uv run python asm.py Fighter_Spaghetti_8006AD10 --fused
uv run python asm.py HSD_PadRenewMasterStatus --fused
uv run python asm.py HSD_PadClampCheck3 --fused
uv run python asm.py lb_8000D148 --fused
uv run python asm.py ftCo_Wait_IASA --calls
```

The first two have no fused sites. HSD clamping keeps separate squared
products/addition; only the existing gekko-math square-root Newton steps fuse.
The stick-circle helper has three single fused sites (8000D164, 8000D168,
8000D1E8), three double Newton sites, and deliberately unfused endpoint-distance
sums. Each is cited in Rust and in the retail-faithful native C oracle.

The new tests cover threshold neighbors, all **65,536 signed-byte stick pairs**,
**65,542** old/new stick-circle queries, button edges, hitlag accumulation,
freeze/reset history, byte wrapping, activity consumption, human/CPU routing,
and a real-input table visiting all 21 Wait predicates. A native C compilation
of the actual Wait IASA caller checks neutral traversal and **441 simultaneous
predicate pairs**. Copied reference functions are checked against the decomp.
The live test runs 600 zero samples for both fighters and compares all 108
input bytes plus all seven canonical CPU fields every tick; it skips only
when its local traces or PlCo archive are absent.

Changed files: `src/input/{mod,common,pad,state,human,geometry,iasa}.rs`, this
README; `tests/{human_input,human_idle_input_600,input_oracle}.rs`,
`tests/input_support/mod.rs`, and `tests/ref/input/{clamp,scale,circle,
circle_retail,driver,wait,wait_driver}.c` plus `decode_idle.py`.
No changes to lib.rs, other crates, harness, third_party, or tracker; no commit.

Final validation after the interrupted run: all 14 T9 tests passed, including
the live 600-tick test (not skipped). `cargo gate` passed, and
`cargo clippy --workspace --all-targets -- -D warnings` passed with no warnings.
Cargo commands ran sequentially with `CARGO_BUILD_JOBS=1`; tests also used
`RUST_TEST_THREADS=1` to limit memory. Concurrent T7 edits were left untouched.
