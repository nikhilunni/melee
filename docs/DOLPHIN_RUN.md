# Running the game in the Dolphin scripting build

Everything the next person needs to boot GALE01 in Felk's scripting Dolphin,
drive the menus from a shell, save a savestate at the first playable frame of a
match, and record a trace. Verified on 2026-09-08 on macOS arm64 with the build
described in `DOLPHIN_BUILD.md`. Facts marked **verified** were observed on the
real game that day; the rest is how the tooling is meant to be used.

## Local oracle data policy

`cargo gate` and individual Rust oracle tests fail on missing local files and
name the first path plus its recovery command. Extract owned disc files using
`docs/DISC.md`; capture scenarios from the repository root with
`uv run --project harness python harness/record.py harness/scenarios/<name>.toml`.
Ledger600 files need `--ledger-suffix ledger600`; tick bone captures need
`--bones N`, where N is the number of ticks required by that oracle.
Gekko estimate probes use `harness/gekko_probe/run.sh` (see its README), and
the legacy M2 and post-render VI bone captures use the command in `docs/M2_GATE.md`
with the input/output paths and environment settings printed by the preflight.

Contributors without local data can run
`MELEE_ALLOW_MISSING_DATA=1 cargo gate -- --nocapture`. Only the exact value `1`
permits an early return with the same diagnostic; libtest reports that opted-out
test as passed, so this mode is not oracle evidence. `tools/merge-check.sh`
rejects the variable whenever it is set, including empty or `0` values.

A git worktree has no `harness/roms` or `harness/traces` (never symlink or
copy them). To record with a worktree's own harness scripts (a changed
tracer, say), set `MELEE_DATA_ROOT` to the main checkout: `harness/data_root.py`
then reads the disc, savestates and decomp there and writes traces and
bridged scenarios there, in place, e.g.
`cd harness && MELEE_DATA_ROOT=$HOME/Projects/melee uv run python record_many.py $HOME/Projects/melee/harness/scenarios/<name>.toml`.

For testing this policy, `MELEE_TEST_DATA_ROOT` replaces `harness/` in the
shared test preflight only: it expects `roms/` and `traces/` beneath that root.
It does not change scenario loading, production paths or later asset reads,
and is intended solely to simulate missing files without touching real data.
Merge-check rejects this test-only override as well. For example:

```sh
mkdir -p /tmp/melee-c11-empty
MELEE_TEST_DATA_ROOT=/tmp/melee-c11-empty cargo test -p melee-sim --test m4_gate squat_fd_fox_300 -- --exact --nocapture
MELEE_TEST_DATA_ROOT=/tmp/melee-c11-empty MELEE_ALLOW_MISSING_DATA=1 cargo test -p melee-sim --test m4_gate squat_fd_fox_300 -- --exact --nocapture
```

## TL;DR

```sh
cd harness
uv run python dolphin/drive.py --video OGL --ports 2 launch      # boots the disc, runs remote.py
uv run python dolphin/drive.py status                            # frame, fps, seed, fighters
uv run python dolphin/drive.py --sync "hold Start true 90"       # skip the opening movie
uv run python dolphin/drive.py wait-watch 0x804D6718 25 500 && \
uv run python dolphin/drive.py "press Start 5"                   # title -> main menu (see timing note)
uv run python dolphin/drive.py shot /tmp/frame.png               # what the game shows right now
...                                                              # menu sequence below
uv run python dolphin/drive.py save-when-wait /abs/harness/roms/idle_ys_fox.sav
uv run python dolphin/drive.py kill
uv run python dolphin/run_scenario.py scenarios/idle_ys_fox.toml --tick-trace --video OGL --ports 2
```

Existing artefacts (all machine-local, gitignored):

| Path | What |
|---|---|
| `harness/roms/idle_ys_fox.sav` | Savestate, first playable frame of P1 Fox vs P2 Fox (idle human), Yoshi's Story, Stock 1, items off |
| `harness/roms/idle_ys_fox.sav.json` | Sidecar written by `remote.py`: frame, seed, fighter addresses/positions at save time |
| `harness/traces/idle_ys_fox.raw.jsonl` | 600 raw frame records (Fighter bytes) |
| `harness/traces/idle_ys_fox.expected.jsonl` | Decoded canonical trace for `melee-diff` |
| `harness/traces/idle_ys_fox.raw.jsonl.done` | Run summary: frames, fps, savestate load sync check |

**Final Destination is locked on a fresh save** (so is Battlefield); the five
"?" tiles on the stage select are the unlockables. Yoshi's Story was used as
the fallback the task allowed. `idle_fd_fox` needs FD unlocked first (or a
save file that has it).

## Complete game-tick oracle (T0)

`tick_trace.py` defines **one record as the completed state of one invocation
of the GObj proc scheduler**, before rendering or the next pad drain. It uses
the existing `MELEE_SCENARIO` and `MELEE_RAW_OUT` variables; `scenario.frames`
is the number of ticks to record (600 in `idle_ys_fox.toml`). The legacy
`trace_scenario.py` remains a VI begin-field sampler even though its phase is
called `frame_end`. Its existing trace is retained as evidence, not a complete
tick contract.

The watched word is **`gm_80479D58.unk_0`, `0x80479D58` (+0)**. In
`third_party/melee-decomp/src/melee/gm/gm_1A45.c:340-342`, the driver calls
`HSD_GObj_80390CFC`, then increments this counter unless it has saturated at
`0xFFFFFFFE`. Retail GALE01 1.02 confirms:

```text
801A4FA0  481EBD5D  bl HSD_GObj_80390CFC
801A4FA4  80790000  lwz r3, 0(r25)
801A4FA8  3C030001  addis r0, r3, 1
801A4FAC  2800FFFE  cmplwi r0, 0xfffe
801A4FB0  4182000C  beq 0x801A4FBC
801A4FB4  38030001  addi r0, r3, 1
801A4FB8  90190000  stw r0, 0(r25)  # r25 = 0x80479D58
```

That store runs once per iteration of the inner pad-queue loop, including
when multiple ticks precede one render. The only other assignment to this
counter is its scene-entry reset (`gm_1A45.c:274`, retail `0x801A4D60`), before
the tick loop. The sampler installs only after loading the match savestate,
checks the code words above, rejects counter discontinuities and saturation,
and never treats a scene reset as a new capture epoch. Pausing still permits
scheduler invocations with masked procs; the idle validator will reject frozen
animations. The counter measures scheduler invocations, not unpaused fighter
updates.

This is an **end-of-tick** sample: `lb_800198E0` has drained this tick's pad
sample, and `lb_80019900` has copied it on the normal active-match path. Pad
master/copy buffers therefore belong to the just-executed tick; a start sample
after the next drain would instead pair previous-tick fighters with newly
drained pad data. No pad data is rewritten by the sampler. Only neutral input
scenarios are supported because per-VI overrides cannot schedule arbitrary
input changes against individual entries of Melee's pad queue. Port 0's full
neutral input is re-issued every frameadvance; port 1 remains the idle second
controller configured by the launch flags.

Source-verified against `~/Projects/dolphin-scripting/src` (2026-09-08):

- `python-stubs/dolphin/memory.pyi` omits memchecks. The actual Python bindings
  in `Source/Core/Scripting/Python/Modules/memorymodule.cpp:60-81,94-95` expose
  `memory.add_memcheck(addr)` and `memory.remove_memcheck(addr)`, each taking
  one positional u32 address and returning None. `Core/API/Memory.cpp:13-25`
  creates a single-address check with no pause/log flags. Reads also emit
  events; the sampler filters `is_write` and the exact address.
- `event.on_memorybreakpoint(callback)` calls
  `callback(is_write: bool, addr: int, value: int)` synchronously.
  `Core/API/Events.h:82-104` asserts the CPU thread and invokes listeners
  inline; `Scripting/Python/Modules/eventmodule.cpp:151-173,260-263` calls
  Python directly. Guest CPU execution does not advance during the callback.
  `Core/PowerPC/MMU.cpp:711-714` invokes the memcheck **before** the u32
  store, and `BreakPoints.cpp:389` emits the event. Thus a memory read sees
  the **old counter**, while callback `value` is the **pending new counter**;
  all scheduler procs have already returned. Host reads use HostRead and do
  not themselves trigger guest memchecks. No asynchronous coroutine is used.
- `registers.pyi` exposes GPR/FPR reads and writes, but no PC read. The
  sampler verifies the static retail store instructions and counter sequence;
  `store_pc` in `.done` is the audited write site, not a captured PC.
- Listeners accumulate in this source revision (contrary to older notes
  saying replacement). Completed/error callbacks park behind guards. Removing
  a memcheck inside its callback would invalidate the `TMemCheck` still used
  by `Action` after event dispatch; cleanup is deferred to the next VI callback.

Savestate loading still occurs synchronously on the first frameadvance. The
load itself produces **no record**. Ordinal/frame 0 is the first scheduler-end
store after loading; the loaded VI may be partway through that tick. Initialize
a simulator from this new record 0, then advance once per adjacent record.
Do not align its ordinal 0 to the old trace's loaded-state ordinal 0.

Every raw and decoded record retains `frame` (zero-based capture ordinal) and
`phase: "frame_end"`, plus the same seed and Fighter fields as before. These
diagnostics remain outside canonical `state`:

| Key | Meaning |
|---|---|
| `tick` | Pending incremented game counter supplied by the callback |
| `vi_frame` | Script VI ordinal, 0 at the load callback; may repeat or skip between records |
| `watch_address` | `0x80479D58`, encoded as an integer |
| `watch_value` | Old word observed in memory; must equal `tick - 1` modulo 2^32 |

Duplicate counter callbacks and re-entrancy are suppressed and counted in
`.done`. A counter gap, wrong callback/store ordering, wrong retail code,
sidecar seed mismatch, missing fighters, or 120 VI fields without a tick
writes `.err`. `.done` is written after exactly N records and safe memcheck
removal on the following VI. It proves capture completion; the validator
separately checks animation and RNG behavior. If emulation itself stops and
no VI callback fires, the shell runner's wall-clock timeout terminates the run.

From the repository root, this command loads **`harness/roms/idle_ys_fox.sav`**,
captures **600 ticks**, waits for completion, stops Dolphin, decodes, and
validates, returning nonzero on any failure:

```sh
cd harness
uv run python dolphin/run_scenario.py scenarios/idle_ys_fox.toml --tick-trace --video OGL --ports 2
```

Outputs are `harness/traces/idle_ys_fox.tick.raw.jsonl` (with `.done` or `.err`),
`harness/traces/idle_ys_fox.tick.expected.jsonl`, and
`harness/traces/idle_ys_fox.tick.dolphin.out`. The older VI files are untouched.
Equivalent direct capture command, run from the repository root:

```sh
MELEE_SCENARIO="$PWD/harness/scenarios/idle_ys_fox.toml" \
MELEE_RAW_OUT="$PWD/harness/traces/idle_ys_fox.tick.raw.jsonl" \
~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
  -e "$PWD/harness/roms/GALE01.iso" --script "$PWD/harness/dolphin/tick_trace.py" \
  -v OGL -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
  -C Dolphin.Core.SIDevice2=0 -C Dolphin.Core.SIDevice3=0 \
  -C Dolphin.Core.EmulationSpeed=0
# After .done appears, close Dolphin, then:
cd harness
uv run python decode.py traces/idle_ys_fox.tick.raw.jsonl traces/idle_ys_fox.tick.expected.jsonl
uv run python validate_ticks.py traces/idle_ys_fox.tick.expected.jsonl
```

The validator compares canonical f32 bits, allows animation time to advance
by exactly one or restart at a smaller value, and checks each adjacent seed
against 0..64 retail LCG draws (`--max-draws K` changes the bound). This is
specific to idle/unit-rate animations, not general hitlag or variable-rate
actions. An RNG pass establishes reachability within the bound, not the
correct consumer schedule or RNG call sites.

The existing VI trace produces exit status 1 and exactly:

```text
records: 600; transitions: 599
LCG draw-count histogram (0..64): {0: 183, 1: 374, 2: 13, 3: 3, 4: 20, 5: 5, 7: 1}
ordinal 486: p0.cur_anim_frame 11 -> 13 (expected +1 or restart)
ordinal 487: p0.cur_anim_frame 13 -> 13 (expected +1 or restart)
ordinal 488: p0.cur_anim_frame 13 -> 15 (expected +1 or restart)
ordinal 496: p1.cur_anim_frame 15 -> 17 (expected +1 or restart)
ordinal 497: p1.cur_anim_frame 17 -> 17 (expected +1 or restart)
ordinal 498: p1.cur_anim_frame 17 -> 19 (expected +1 or restart)
FAIL: 6 violations
```

The tick sampler is source-verified and tested with fake CPU events. A live
Dolphin capture remains to be run outside the Codex sandbox; no new trace is
claimed as verified until that command succeeds.

## Items

The tick tracer (and its RNG-ledger subclass) always records `items: []` or
the live `HSD_GObj_Entities->items` list in retail order. Each entry contains
`gobj`, `base` (Item pointer), numeric `kind`, generated `kind_name`, `owner`
(fighter player slot, or `null` for an unresolved/non-fighter owner), and
the complete **0xFCC-byte** Item image as hex. An empty list costs two memory
reads. There is no capture flag; old traces without `items` remain valid.

`decode.py` retains each entry and adds its own typed `state`: position,
collision previous position, velocity, facing, kind/spawn kind, spawn ID,
motion/animation IDs, raw owner GObj, active hitbox count and first damage,
and lifetime. Movement diagnostics also decode flags, extra displacement,
callbacks and hit references. Fox lasers additionally expose
`laser.prev_pos`, saved by their physics callback; collision `prev_pos` is
not necessarily a previous-tick position. Float values retain exact bits.
These keys stay outside the existing 49-key fighter/RNG `state`.

Identity/finite-position checks run whenever items are present. Add
`--items` to the **validator** to print spawn/despawn ticks and motion-check
coverage:

```sh
python harness/validate_ticks.py harness/traces/laser_fd_fox.tick.expected.jsonl --scripted --max-draws 256 --items
```

The velocity diagnostic checks unaffected Fox laser flight with f32
tolerance; hitlag, attachment, changed owner/velocity, other movement, and
unaudited kinds/callbacks are reported in the skipped count. It is not a
general item-physics comparator. Items already present at capture start
are labelled; capture end is not a despawn. `record.py` needs no changes.
The proposed 300-tick laser input script, offset sources, compatibility
evidence and exact validation commands are in
[`PORT_NOTES/C9_ITEM_ORACLE.md`](PORT_NOTES/C9_ITEM_ORACLE.md).

## One-time machine setup

1. Dolphin config lives in `~/Library/Application Support/Dolphin/` (the default
   user dir; the fork's own `-u ~/Projects/dolphin-scripting/user` also works).
   Pre-create `Config/Dolphin.ini` so first launch shows no dialogs:

   ```ini
   [Analytics]
   PermissionAsked = True
   Enabled = False
   [Core]
   EnableCheats = False
   [Interface]
   UsePanicHandlers = False
   ConfirmStop = False
   ```

   and `Config/Logger.ini` with `[Options] WriteToFile = True`, `Verbosity = 4`,
   `[Logs] Scripting = True` so `print()` from scripts lands in
   `~/Library/Application Support/Dolphin/Logs/dolphin.log`. **Verified:** with
   this config the first boot shows no Dolphin dialog at all; the game itself
   shows "Creating new Game Data" once (fresh memory card, saved as a `.gci`
   under `GC/USA/Card A/`).
2. `defaults write org.dolphin-emu.dolphin NSAppSleepDisabled -bool YES`
   (App Nap). Harmless; the real fix for background slowdowns is the OGL
   backend, see below.
3. `screencapture` needs Screen Recording permission for your terminal, but you
   do not need it: `drive.py shot` reads the emulator's own frame.

## Things about this Dolphin build you must know

All **verified** on 2026-09-08.

- **Use `-v OGL`.** With the default Metal backend the emulation drops to
  3-9 fps as soon as another window covers the Dolphin window (the presenter
  blocks on the occluded drawable). With `-v OGL` it holds 60 fps covered or
  not, and ran at unlimited speed for the trace.
- **Do not click or type in the Dolphin window while a script is driving.**
  The emulated pad is the keyboard-mapped default profile, so stray keys are
  real inputs; during this session an accidental keypress in the window
  changed the menu state mid-run. Keep the window unfocused (it can be fully
  hidden behind other windows; `shot` still works).
- `__file__` is undefined in `--script` files. Scripts recover their path from
  `sys._getframe().f_code.co_filename` (see the top of `remote.py`).
- Exceptions in a frame callback are printed to the Dolphin log and the
  callback keeps firing. `sys.exit()` does not stop anything.
  `event.on_frameadvance(None)` raises `ValueError`. The source audit above
  corrects the old replacement claim: listeners accumulate, so callback
  guards must park completed scripts.
- `await event.framedrawn()` is a one-shot listener (removed after it fires);
  that is how `shot` works without paying the per-frame readback cost. The
  frame dump readback only runs while a listener exists, so the first frames
  after registering return the *previous* shot's pixels; `remote.py` awaits
  three frames and keeps the last.
- `savestate.load_from_file` called **inside** the frameadvance callback is
  synchronous (`State::LoadAs` -> `Core::RunOnCPUThread` runs inline on the
  CPU thread); memory read right after the call is the saved frame boundary.
  Called from top-level script code (core not running yet) it is a no-op, so
  `trace_scenario.py` loads on its first frame callback.
- `controller.set_gc_buttons(port, {...})` sticks are floats -1..1, triggers
  0..1, buttons bool; an override lasts one frame. The game sees overrides in
  `HSD_PadMasterStatus[port]` the same frame (verified: `button` showed
  `0x1000` during a held Start).
- Dolphin's default SI config is one Standard Controller in port 1 and nothing
  in ports 2-4. `drive.py launch --ports 2` passes
  `-C Dolphin.Core.SIDevice1=6` so the game sees a second (idle) pad.
- Savestates are ~19 MB and load fine into a fresh Dolphin process one frame
  after boot, as long as the SI devices match (`run_scenario.py --ports`).

## The tools

All under `harness/dolphin/`, run with `cd harness && uv run python dolphin/<tool>`.

- `remote.py` runs inside Dolphin (`--script`). Every frame it applies
  commands from `harness/roms/.remote/cmd.json`, re-issues queued pad inputs,
  and every 10 frames writes `status.json` (frame counter, wall-clock fps,
  RNG `seed`, fighters found by `walk.py` with kind/position/motion, raw pad
  state, watched memory). Errors go to `log.txt`.
- `drive.py` is the shell side. Grammar (in `remote_proto.parse_command`):

  | Command | Effect |
  |---|---|
  | `press A [N]`, `press A+Start 3` | hold buttons N frames (default 2), then release |
  | `hold StickX 1.0 30`, `hold Start true 5` | hold one input |
  | `stick X Y N` | main stick, floats -1..1 (or raw 0..255) |
  | `... @1` | any input command on controller port 1 (P2) |
  | `wait N` | N neutral frames |
  | `clear` | drop the queue |
  | `save P`, `load P`, `save-when-wait P` | savestates (absolute paths); `save-when-wait` fires at the first frame every fighter is in `ftCo_MS_Wait` |
  | `shot [P]` | PNG of the emulator frame (blocks until written) |
  | `watch ADDR N`, `unwatch ADDR|all` | show N bytes at ADDR in every status |
  | `status`, `wait-idle`, `wait-watch ADDR LO HI`, `kill`, `launch` | local verbs |

  Several commands can be sent in one batch (`"press A 3; wait 40"`); a batch
  is applied on the next frame. `--sync` blocks until the batch is applied and
  the queue is empty.
- `run_scenario.py scenario.toml` launches Dolphin with `trace_scenario.py`
  (unlimited speed), waits for the `.done` marker, kills Dolphin and runs
  `decode.py`. Use `--keep` to leave Dolphin open, `--speed 1` for realtime.
- `tests/test_remote.py` and `tests/test_walk.py` cover the protocol, queue,
  PNG writer, tracer state machine and the fighter walk without Dolphin.

## Menu navigation, verified input sequence

Menus ignore a button for ~40 frames after a transition; `press X 3` with a
`wait 40` afterwards is reliable. All positions below are in the 640x527
emulator frame `shot` produces.

1. **Opening movie -> title.** `hold Start true 90`. The movie also exits on a
   short press but only once it accepts input; a long hold is simplest.
2. **Title -> main menu.** The title (`gm_Scene_Title_OnFrame`) ignores input
   for `countdown_timer` = 20 frames, then exits on a Start *edge*, and after
   `frame_count` > 600 it leaves for the attract demo on its own. Read
   `frame_count` at `0x804D6718` and press while it is in range:
   `drive.py wait-watch 0x804D6718 25 500 && drive.py "press Start 5"`.
   If the title has already timed out (`frame_count` stuck at 0x259 or the
   demo/movie playing), a `press Start 5` bounces back to a fresh title;
   repeat. **The title -> menu transition takes ~10 s** (the game writes the
   memory card first); do not press anything meanwhile.
3. **Main menu -> Custom Rules (one-time, saved to the card):**
   `press Down 3; wait 60; press A 3; wait 90` (VS. Mode), `press Down 3;
   wait 40` x3, `press A 3; wait 120` (Custom Rules). Rules row: `press Right 3`
   TIME -> STOCK. Stock row (`press Down 3`): Left decrements and wraps
   3 -> 2 -> 1 -> 99, Right wraps 99 -> 1. Item Switch (`press Down 3` x4,
   `press A 3; wait 100`): `press Up 3` moves onto the frequency bar, then
   `press Right 3` x3 MEDIUM -> LOW -> VERY LOW -> NONE; `press B 3` saves.
   `press B 3` again returns to VS. Mode, `press Up 3` x3 highlights Melee.
4. **Character select.** `press A 3; wait 240` from VS. Mode/Melee. The P1
   hand starts at about (60, 495) carrying its token, P2's at (200, 495); Fox
   is the portrait at about (128, 170). Hands move ~12 px/frame.
   `stick 0 1 20; stick 1 0 6; stick 0 1 6` puts P1 over Fox (the card
   previews the hovered character); `press A 3 @0` drops the token.
   P2: `stick 0 1 26 @1; stick -1 0 6 @1; press A 3 @1`. "READY TO FIGHT"
   appears; `press Start 3; wait 150`.
5. **Stage select.** Free crosshair, starts at (320, 405), accelerates
   (roughly 10-20 px/frame; nudge in 3-6 frame steps and read the hover name).
   Verified tiles: row 1 = Peach's Castle, Kongo Jungle, Great Bay,
   **Yoshi's Story** (~(355, 85)), Fountain of Dreams, Corneria; row 2 col 3 =
   Temple; row 3 col 2 = Onett; bottom row = five locked "?" including
   Battlefield and Final Destination. `press A 3` on the tile starts the match.
6. **Match start.** Arm `save-when-wait /abs/path.sav` before pressing A on
   the stage. Entry animations end with every fighter in `ftCo_MS_Wait`
   (motion_id 14); the save fires on that frame and writes the sidecar JSON.
   For `idle_ys_fox.sav` this was Dolphin frame 45970, seed `0x0EE0B933`
   (249636915), Fox P1 at `0x80C6CE60` pos (-42.0, 23.4501, 0), Fox P2 at
   `0x80CB1A20` pos (42.0, 23.4501, 0), `stage_info.grkind` (`0x8049E750`) =
   0x0A (`Gr_Kind_Story`). y = 23.45 is the height of Yoshi's Story's side
   platforms, where a 2-player match spawns.

## Useful addresses (symbols.txt, retail 1.02)

| Symbol | Address | Use |
|---|---|---|
| `seed` | `0x804D5F90` | RNG; changes every frame in a match, constant on the title |
| `HSD_GObj_Entities` | `0x804D782C` | fighter list root (`walk.py`) |
| `HSD_PadMasterStatus` | `0x804C1FAC` | raw pad state per port (0x2C each), `button` at +0 |
| `HSD_PadCopyStatus` | `0x804C20BC` | per-frame copy with `trigger` at +8 |
| `controller_map` | `0x80479C30` | merged menu input; entry 4 (`+0xF0`) is "any pad" |
| `countdown_timer` / `frame_count` | `0x804D6714` / `0x804D6718` | title screen timers |
| `stage_info.grkind` | `0x8049E750` | current stage `GrKind` |

## Observed performance

- Title/menus/match at realtime: 59.9-60.1 fps (Dolphin paces to 60).
- Traced run at unlimited speed (`-C Dolphin.Core.EmulationSpeed=0`, OGL,
  two fighters): **131.6 fps** for the 600 frames (4.56 s), 6.8 s wall
  including Dolphin boot and the savestate load. Each traced frame reads the
  full 0x23EC-byte Fighter struct per fighter through `memory.read_u32`
  (about 2300 calls per fighter per frame), which is the main cost; a bulk
  read in the fork would raise this a lot.
- The savestate load one frame after boot is synchronous and exact: the
  `.done` file records `seed_after_load == sidecar_seed` (`synced: true`).

## Determinism check (2026-09-08)

Two independent `run_scenario.py` runs of `idle_ys_fox` (fresh Dolphin
process each, savestate loaded one frame after boot) produced byte-identical
`expected.jsonl` files: all 600 seeds, positions and every schema field agree.
The *raw* dumps differed in two places, both on P2's Fighter struct: bytes
`0x14BC-0x1520` at frame 519 and the word at `0x5B0` at frame 559. Those
offsets are not in `harness/schema/fighter.yaml`, so they do not reach the
canonical trace; before adding fields there, check them against another pair
of runs (they are the first known Fighter bytes that are not a pure function
of the savestate; heap pointers or audio/timing state are the usual suspects).

## Known gaps / next steps

- Final Destination: unlock legitimately (all event matches) or import a
  save with it unlocked; a memory poke of the unlock flags would work but is
  save-data modification and was not done.
- `trace_scenario.py` still records only `frame_end`; intra-frame phases need
  code breakpoints (no Python API yet, see `DOLPHIN.md`).
- Menu navigation is scriptable end-to-end from this document; a
  `drive.py`-driven `navigate.py` that replays it and checks each step with
  `watch`/`status` would remove the manual screenshots.


## RNG ledger (who draws from `seed`, per tick)

Requires a Dolphin fork patch that exposes the program counter and link
register to Python: `docs/patches/0002-scripting-read-pc-lr.patch` (applied to
`~/Projects/dolphin-scripting/src`, rebuilt with `ninja` in ~15 s). `HSD_Rand`,
`HSD_Randi` and `HSD_Randf` all inline the LCG and store `seed` without saving
LR, so at the store `registers.read_lr()` is the direct caller.

```sh
cd harness
OUT="$PWD/traces/idle_fd_fox.ledger.raw.jsonl"
MELEE_SCENARIO=scenarios/idle_fd_fox.toml MELEE_RAW_OUT="$OUT" \
  ~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
  -v OGL -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
  -e "$PWD/roms/GALE01.iso" --script "$PWD/dolphin/rng_ledger.py"
uv run python rng_ledger_report.py "$OUT" --ticks 3
```

Each raw record gains `rng_draws: [{pc, lr, seed}]` in draw order; the report
maps `lr - 4` (the `bl`) to `function+offset` via symbols.txt. Result for
`idle_fd_fox` (2026-09-08): per tick, one draw at `hsd_8039EE24+0xDC`
(particle generator update) plus six per live particle (`hsd_8039DAD4+0x10A0/
+0x10F8`, `hsd_8039930C+0x1D7C/+0x1DE8/+0x1E54/+0x1EC0`), i.e. the 6k+1
pattern; `grLast_8021ADD0+0x270` and `ftCo_8008A7A8+0x114` (Wait1/Wait2
choice) fire rarely. Note: scripts under `harness/dolphin/` only start their
tracer when run as the entry script (`__name__ == "__main__"`), so they can be
imported by other tracers.

## Scripted-input scenarios (Milestone 4)

`tick_trace.py` accepts a scenario `inputs` list: each step
`{ frame = N, port = 0, buttons = { StickX = 0.35 } }` holds those pad keys
(`remote_proto.GC_KEYS`; unnamed keys are neutral) on that port from VI
callback `N` (counted from the savestate load) until the port's next step.
**`inputs` must be a top-level TOML key, written before the first
`[[fighters]]` table**; a key after a table header belongs to that table, and
both the tracer and `melee-sim`'s `Scenario` reject the misplaced form.

VI timing cannot be aligned to the game's pad queue, so every tick record also
carries `pad_game`, the `HSD_PadGameStatus[4]` array (0x44 bytes per port)
that the tick consumed: `lb_80019900` renews it right before
`HSD_GObj_80390CFC` runs the tick (gm_1A45.c:306-340). `decode.py` emits it as
`record["inputs"]["pN"]` beside, not inside, the compared `state`; the port
replays it (`crates/melee-sim/src/inputs.rs`). `run_scenario.py --tick-trace`
adds `--scripted` to `validate_ticks.py` for scripted scenarios (animation
rates are not unit while moving).

Stick values: Dolphin takes floats -1..1 mapped to raw 0..255 around 128.
HSD clamps to radius 80 and divides by 80, so `StickX = 0.5` (raw 64) reaches
the game as 0.8 (the dash threshold) and `0.35` (raw 45) as 0.5625. Recording:

```sh
cd harness
uv run python dolphin/run_scenario.py scenarios/walk_fd_fox.toml --tick-trace --video OGL --ports 2
# RNG ledger for the same scenario (rng_ledger.py extends the tick tracer):
OUT="$PWD/traces/walk_fd_fox.ledger.raw.jsonl"
MELEE_SCENARIO="$PWD/scenarios/walk_fd_fox.toml" MELEE_RAW_OUT="$OUT" \
  ~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
  -v OGL -C Dolphin.Core.EmulationSpeed=0 -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
  -e "$PWD/roms/GALE01.iso" --script "$PWD/dolphin/rng_ledger.py"   # kill Dolphin once $OUT.done exists
uv run python rng_ledger_report.py "$OUT" --ticks 300
```

## Unlocks and rules are RAM-only pokes (per session)

The memory card still holds the stock save: no unlockables, time mode
2 minutes, 3 stock. Every recording session re-applies these after the
title screen has loaded the save (all addresses are inside the save-data
block at `gmMainLib_804D3EE0` -> 0x8045A6C0; nothing is written back to
the card):

```sh
cd harness
uv run python dolphin/drive.py "poke-or 0x8045BF2A u16 0xC0"    # stages: Battlefield + Final Destination
uv run python dolphin/drive.py "poke-or 0x8045BF28 u16 0xFFFF"  # all characters (Marth is row 3, ~(450,240))
uv run python dolphin/drive.py "poke 0x8045BF12 u8 1; poke 0x8045BF14 u8 1"  # GameRules: stock mode, 1 stock
```

Character select with everyone unlocked: P1's hand reaches Marth with
`stick 0 1 20 @0; stick 1 0 33 @0`, P2's reaches Fox with
`stick 0 1 26 @1; stick -1 0 6 @1`; confirm with `shot` (the card previews
the hovered character) before `press A 3 @0; press A 3 @1; press Start 3`.
On the stage select `stick 1 0 5; stick 0 1 3` from the start position
lands on Final Destination.

## One-command recording

`harness/record.py scenarios/<name>.toml [--bones N] [--no-ledger] [--no-particles] [--ledger-suffix ledger600]`
records the tick trace, the RNG ledger, the particle dump and optionally an
N-tick bone dump for a scenario in sequence (one Dolphin at a time), then
prints P1's motion transitions and any RNG sites beyond the idle set. The
particle and bone snippets replay the scenario's scripted inputs
(`MELEE_PARTICLES_SCENARIO` / `MELEE_BONES_SCENARIO`).

### Compressed traces (2026-09-27)

After every capture has finished and validated, `record.py` (and a standalone
`dolphin/run_scenario.py`) replaces each JSONL output of 1 MiB or more with
`<file>.jsonl.zst` (zstd level 12, every core). The compressed file is
decompressed and compared byte for byte with the plain one before the plain
file is removed; `.done` markers, `.json` sidecars, logs and small files stay
plain. `--no-compress` keeps everything plain. On `idle_fd_fox` and `jab_fd_fox` the
whole capture set shrank 47x (239 MB to 5 MB): particle dumps ~60x, their
metadata ~35x, ledgers ~35x, raw tick traces ~45x, expected traces 140-220x.

Readers name the plain `.jsonl` path and accept either form, preferring the
plain file: Rust through `crates/melee-trace-io` (tests reach it as
`melee_test_support::trace`; `require_files` counts a `.zst` sibling as
present), Python through `harness/trace_io.py`. Inspect one by hand with
`zstd -dc <file>.jsonl.zst | head`. Existing plain traces migrate with
`cd harness && uv run python compress_traces.py` (dry run; `--estimate`
samples the ratio) and then `--apply`; it skips files modified in the last
ten minutes and can be interrupted and rerun at any point.

### Headless recording (the default since 2026-09-26)

Every scripted capture (`record.py`, `dolphin/run_scenario.py`) launches the
headless build: no window, no host audio, Null video. Build or refresh it with

    tools/build-headless-dolphin.sh

which applies `docs/patches/0003-nogui-scripting-backend.patch` to the
scripting fork, builds `dolphin-nogui` and wraps it in
`~/Projects/dolphin-scripting/build/Binaries/DolphinHeadless.app`. Two fixes
make it usable:

- The fork's no-GUI frontend parsed `--script` but never started a scripting
  backend. The patch creates it after `BootCore` (as DolphinQt does) and
  destroys it after `Core::Shutdown`.
- On macOS Dolphin locates `Sys` through the app bundle. A bare executable
  finds none, so it loses `Sys/GameSettings/GALE01.ini` and the DSP
  resampling coefficients; every savestate load then fails silently inside
  the AX ucode (the tracer reports "savestate seed does not match sidecar").
  The bundle links `Contents/Resources/Sys` to the windowed app's copy, so both
  frontends run with identical data.

**Verified 2026-09-26:** with default settings, `laser_reflect_fresh_fd_marth`
(tick, ledger, particles, 150 bone ticks) and `powershield_ftilt_fd_marth`
(tick, ledger, particles), recorded under temporary names, were byte-identical
to their windowed OGL recordings. A full four-pass capture takes about 13 s.

Selection (`harness/dolphin_config.py`):

| Situation | Dolphin |
|---|---|
| Scripted scenes, replays of human pad logs, ledgers, particles, bones | headless (default) |
| Live human play (`record.py` on a scene with `controller = "human"`) | windowed, automatically |
| Match-start boundaries (`make_boundary.py`) | headless |
| Interactive menu driving (`dolphin/drive.py`, screenshots) | windowed |
| `DOLPHIN_GUI=1` | windowed everywhere |
| `DOLPHIN_BIN=<path>` | that executable |
| `DOLPHIN_AUDIO=1` | keep host audio (muted by default; it cannot affect game state) |

Headless capture validates the deterministic simulation oracle only. It does
not validate native rendering or replace graphics smoke tests. Always run
recordings serially.

### Match-start boundaries in one command (2026-09-27)

    cd harness && uv run python make_boundary.py --stage Battlefield --players Marth Fox [--stocks 4]

boots the disc in headless Dolphin (private user folder, unlimited speed) and
runs `dolphin/boundary_script.py`, which reads the game's own state every
frame instead of timing inputs or reading screenshots: it pulses Start until
the scene machine (`0x80479D30`) reaches the main menu, applies the RAM-only
save-data pokes below (plus items off), walks the main menu by its hovered
item (`mn_804A04F0`), places each port's CSS hand over the character's icon
(bounds from the CSS icon table) and presses A with that port's controller,
then sets the stage screen's `force_stage_id` (the field retail's random and
ordered stage rules use). It saves at the first frame with an initialised
roster, the `save-when-fighters` predicate. The tool then writes
`<name>.toml` and `<name>_cold.toml`, records, gates, and registers the
boundary in `boundaries.toml` when the port matches (about 30 s in all; the
menus take 4 s). A layout the port does not yet reproduce stays unregistered
with its savestate and traces in place.

### Corpus bridge: any port recording as a retail oracle (2026-09-26)

A `melee-replay` recording (corpus explorer, native app export) replays in
Dolphin when its match starts from a registered boundary:

    uv run python replay_to_scenario.py <recording.json> --name <scenario>
    uv run python record.py scenarios/<scenario>.toml [--bones N]
    uv run python replay_to_scenario.py <recording.json> --name <scenario> --verify

- `harness/boundaries.toml` lists match-start savestates whose cold
  construction the port reproduces exactly (initial scene, particles and 600
  gated ticks, `registered_boundaries_cold_600` in `cold_tests.rs`). The
  recording must use the boundary's stage, players, stocks and seed. Add one
  per stage and character layout with `make_boundary.py` (below).
- Scenarios use `input_clock = "tick"`: step frames count tick records and
  carry raw PADStatus values. `HSD_PadRenewMasterStatus` dequeues one raw poll
  per tick and repeats the last status when the queue is empty, so VI-frame
  scheduling occasionally lands a step one tick late. The tick tracer instead
  rewrites the raw queue at every tick end (game stopped in the memcheck
  callback) to hold exactly the next tick's pad. Every capture pass inherits
  it from `TickTracer`.
- Analog values must be retail-reachable: sticks k/80 with radius <= 80,
  triggers k/140. Dolphin's float for each value comes from
  `dolphin/pad_calibration.json`, measured by `calibrate_pads.py`.
- The first tick's pads were renewed before the savestate was taken and are
  neutral; the explorer holds neutral on tick 0 and the bridge rejects
  anything else.
- `--verify` compares every recorded tick's pads with the recording. Verified
  on `corpus_v2_s0_e1_p0` (894 ticks): pads exact, and the port replaying the
  retail trace matches through tick 893, where it reaches the same
  unimplemented phantom hit as the generated run.

Bulk bridging: every recording in an explorer output directory can be bridged,
recorded, pad-verified and gated in one pass (one Dolphin at a time); the
2026-09-26 session drove this from a scratch script looping over
`replay_to_scenario.py`, `record.py`, `--verify` and `melee-sim gate`.

### Dry runs: search inputs before recording (2026-09-27)

A tick-clock scenario (`input_clock = "tick"`) can run in the port alone
from any recorded scenario of the same savestate:

```sh
cargo run -q --release -p melee-sim -- dry-run harness/scenarios/<new>.toml \
  --state-from harness/scenarios/<recorded>.toml --out /tmp/dry.jsonl
```

The pads come from the scenario's own schedule (raw PADStatus, HSD's
virtual stick directions added), the initial state and display clock from
the reference recording; each record also carries the item state. Nothing
is compared: use it to find inputs that reach a path (a panic names an
unported one), then record only the winner. Replaying a recorded
scenario's own schedule this way reproduces its trace exactly.

### Diagnostics for a diverging scenario (2026-09-26)

- `melee-sim bones-diff <scenario> [--limit N] [--tick T]`: replays against
  `<name>.bones.jsonl` and prints the first differing local SRT word per
  fighter and tick (or every differing word of tick T). Record a window cheaply
  with `record.py <scenario> --no-ledger --no-particles --bones N --bones-from T`
  (the dump still runs from tick 0 but writes only ticks >= T).
- `melee-sim particle-sites <scenario> --from A --to B`: ordered particle RNG
  call sites per tick, port versus the retail ledger (the order the M5 gate
  checks).
- `melee-sim particles-diff <scenario> --from A --to B`: first differing
  particle-system field per tick against `<name>.particles.jsonl`, apart from
  AppSRT display caches.
- `record.py --camera N` (`dolphin_camera_tick_snippet.py`): per-tick raw
  `game_camera`, main CObj with eye/interest positions, CmSubject list and
  ifMagnify state, the oracle for the camera port.

Recorder timeouts now apply to scripted and replay tick captures as well as
auxiliary passes. A process that exits before its completion marker is reported
immediately, including a zero exit code without a completed capture.
