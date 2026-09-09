# Capturing retail particle simulation state

`harness/particle_dump.py` reads memory without importing Dolphin or writing
memory. `harness/dolphin_particle_snippet.py` subclasses the existing
`tick_trace.TickTracer`. Importing either module starts no tracer. It captures
the loaded savestate separately, then N completed scheduler ticks, including
multiple ticks within a single VI field. The inherited sampler checks the
retail boundary instructions, seed sidecar, counter continuity, duplicates,
reentrancy, saturation, and missing-tick watchdog. Memchecks are removed on
the next VI callback after the last sample.

Run from the repository root with the scripting Dolphin build described in
[DOLPHIN_BUILD.md](DOLPHIN_BUILD.md), with its memory-breakpoint API enabled:

```sh
MELEE_PARTICLES_SAVESTATE="$PWD/harness/roms/idle_fd_fox.sav" \
MELEE_PARTICLES_OUT="$PWD/harness/traces/idle_fd_fox.particles.jsonl" \
MELEE_PARTICLES_TICKS=3 \
"$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin" \
  -e "$PWD/harness/roms/GALE01.iso" \
  --script "$PWD/harness/dolphin_particle_snippet.py" \
  -v OGL \
  -C Dolphin.Core.EmulationSpeed=0 \
  -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
  -C Dolphin.Core.SIDevice2=0 -C Dolphin.Core.SIDevice3=0
```

Wait for `harness/traces/idle_fd_fox.particles.jsonl.done`, or inspect the
adjacent `.err` file if capture fails. The snippet parks its listeners after
completion; close Dolphin afterwards. It does not terminate the emulator.
Set `MELEE_PARTICLES_TICKS` to the desired positive count. All outputs stay
under the ignored `harness/traces/` directory and must not be committed.

For output path `X`, the files are:

| File | Contents |
|---|---|
| `X` | N canonical melee-diff records, phase `particles`, frames `0..N-1` |
| `X.meta.jsonl` | Tick/fighter diagnostics plus particle structures, raw bytes and pointers |
| `X.initial.jsonl` | One initial-state record, captured after load, before the next scheduler tick |
| `X.initial.jsonl.meta.json` | Initial tick counter and full initial particle snapshot |
| `X.done` | Completion summary, initial/final ticks, duplicate count and timing |
| `X.err` | Failure traceback; stale success/error markers are cleared at startup |

The initial state is **not** completed tick zero. Start a replay from the
initial snapshot; compare its first completed update to frame zero in `X`.
The savestate may itself be at a VI boundary, so its point within the game's
scheduling must still be considered when integrating a whole-game replay.
Sampling occurs at tick-counter store `0x801A4FB8`, after `HSD_GObj_80390CFC`
returns; the counter's pending new value appears in metadata `tick`.
See [DOLPHIN_RUN.md](DOLPHIN_RUN.md) for that contract.

## Runtime layout and list order

All offsets below are 32-bit big-endian Gekko offsets, from
`third_party/melee-decomp/src/sysdolphin/baselib/psstructs.h`:
`HSD_Particle` lines 128–189; `HSD_Generator` lines 243–277;
`HSD_psAppSRT` lines 91–126. `HSD_SList` is in `list.h:6–9`.
Generator size (`0x94`) and AppSRT size (`0xA4`) include ABI tail alignment;
particle size (`0x98`) is explicitly documented in the header.

| Global | Retail address | Meaning |
|---|---|---|
| `hsd_804D78FC` | `0x804D78FC` | Live generator-list head pointer |
| `hsd_804D0908` | `0x804D0908` | Sixteen particle-list head pointers |
| `hsd_804D78F4` | `0x804D78F4` | Pending generator SList head; node next `+0`, generator `+4` |
| `psCmdListArray` | `0x804D0D58` | 65 exclusive upper descriptor IDs |
| `ptclref_804D0E5C` | `0x804D0E5C` | 65 pointers to relocated descriptor-pointer arrays |
| `hsd_804D08E8` | `0x804D08E8` | Eight point-JObj references |
| `lbl_804D6368` | `0x804D6368` | u16 family-ID allocator counter |

Addresses are resolved through `harness/symbols.py` at runtime:

```sh
cd harness
uv run python symbols.py hsd_804D78FC hsd_804D0908 hsd_804D78F4 psCmdListArray ptclref_804D0E5C hsd_804D08E8
uv run python -m pytest -q tests/test_particle_dump.py tests/test_tick_trace.py
```

There is **no particle-list head in a generator**. The walker reads generator
head then `next`, and particle links 0 through 15, each head then `next`.
Particle `gen` at `+0x88` supplies the optional generator association. The
canonical stream normalizes it to the generator's current list ordinal,
and AppSRT references to first-encounter order. Allocator addresses, raw bytes
and pointers remain in metadata. Corrupt pointers, shared particle nodes,
cycles, excessive lists and mismatched particle links fail the capture.

| Generator offset | Type | State |
|---|---|---|
| `00`, `04` | pointer, u32 | next, kind/flags |
| `08`, `0C` | f32 | random emission rate, accumulated emission count |
| `10` | pointer | attached JObj |
| `14`, `16` | u16 | generator lifetime, type/flags |
| `18..1A` | u8 | bank, list link, texture group |
| `1C`, `1E` | u16 | family ID, particle lifetime |
| `20` | pointer | bytecode start |
| `24..2C`, `30..38` | 3 × f32 | position, velocity |
| `3C`, `40`, `44`, `48`, `4C` | f32 | gravity, friction, size, radius, angle |
| `50` | u32 | retained child count |
| `54`, `58`, `5C` | pointer | AppSRT, user functions, callback |
| `60..90` | union | active auxiliary shape parameters |

The auxiliary member follows `type & 15` in `hsd_8039F05C`: disc for 0/3/4,
line for 1, tornado for 2, rect for 5, cone for 6/7, sphere for 8. Rect has
12 floats at `60..8C` and u16 flags at `90`. Only the active member enters
canonical records; the full raw union remains in metadata.

| Particle offset | Type | State |
|---|---|---|
| `00`, `04` | pointer, u32 | next, kind/flags |
| `08..0B` | u8 | bank, texture group, pose, palette |
| `0C`, `0E`, `10` | u16 | size, primary-color, environment-color counters |
| `12..15`, `16..19` | 4 × u8 | primary/environment RGBA |
| `1A` | u16 | command wait |
| `1C`, `1D`, `1E` | u8, u8, u16 | loop count, link, family ID |
| `20`, `24`, `26`, `28` | pointer, 3 × u16 | program start, PC, mark PC, loop PC |
| `2A` | u16 | remaining particle lifetime |
| `2C..34`, `38`, `3C`, `40..48` | f32 | velocity, gravity, friction, position |
| `4C`, `50`, `54..5F` | f32, counters/bytes | size, rotation, alpha/material/ambient/rotation counters |
| `60`, `64` | f32 | target size and rotation |
| `68..83` | u16/u8 | remaining interpolation counters and target colors/alpha/material/ambient bytes |
| `84` | f32 | trail |
| `88`, `8C`, `90`, `94` | pointer | generator, AppSRT, user data, callback |

Every named particle member is decoded, including rendering-related
interpolation counters because bytecode updates them. All float words enter
canonical records as `{t:"f32",v:{bits,approx}}`; no `read_f32` conversion
can quiet signaling NaNs or discard signed zero. PCs remain byte offsets.
`program_kind` resolves `cmdList` against `descriptor + 0x3C` in the relocated
command bank. Version-zero banks have IDs `0..count` and a header eight
bytes before the pointer table. For versions `0x40..0x43`, including FD's
`0x42` bank, `psInitDataBankLoad` stores an exclusive upper ID and biases the
pointer table backward: `table = header + 12 - first_id * 4`. FD's first ID
is 30000 and its count is 5, so the valid IDs are 30000 through 30004.
The dump recovers the header by scanning backward from
`table + upper_id * 4`, checking version, first ID, count, and the complete
bounds relation. It then dereferences only entries within that validated ID
range; addresses preceding the real table are never treated as descriptor
pointers. Failure to recover a consistent header raises an explicit error.
Unresolved/custom program pointers are marked `program_resolved=0`
and null kind, with the pointer retained in metadata. Program bytes should
be read from the stage's owned disc archive by the Rust bank loader.

Referenced AppSRT captures include translation `08`, quaternion `14`, scale
`24`, status/frame/use count `30/31/32`, matrix `34`, scale factors `64/68`,
all twelve unknown floats `6C..98`, callback `9C`, ID `A0`, and byte `A2`.
Attached and point-JObj cached SRT/matrices are included in metadata; the
dump does not run matrix setup. Allocation/peak/family counters and
callback-global addresses are also retained in metadata. The u16 allocator
counter `lbl_804D6368` is additionally emitted as
`particles.family_id_counter` for restoring future family-ID assignments.

## Verification and remaining work

Fake-memory tests verify literal offsets, signed-zero/NaN word retention,
list order and associations, rect auxiliary data, shared AppSRT, program
resolution (including the shifted v42 bank with IDs 30000..30004),
family-ID width, malformed lists/banks, initial capture, two scheduler ticks in
one VI, deferred memcheck removal, and startup marker cleanup. They do not
establish live Dolphin parity. No live capture was run for this change.

Opaque callback/user-data pointers are recorded without executing callbacks
or guessing user-data allocation size. JObj hierarchy/animation state beyond
the referenced cached nodes, bank bytecode bytes and renderer state are not
serialized. Callback-defined auxiliary layouts for generator types beyond
8 remain raw metadata. A Rust state-restoration/trace adapter must map the
normalized IDs and float bits into owned objects before a bit-exact Dolphin
comparison can be claimed. Stage/fighter RNG draws elsewhere in the tick
remain part of `rng.seed` and must be handled by the whole-game integration
or separated using the RNG ledger.
