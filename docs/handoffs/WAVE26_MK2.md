# Mushroom Kingdom II (grinishie2.c): not started, notes for the next agent

Wave 26 stopped on the usage limit before any MK2 code was written. Part 1 of
the task (Yoshi's Island, Mushroom Kingdom and Onett allocating mid-match)
landed as its own commit. What follows is what was read so far.

## Data in hand

- Boundary recorded: `start_mk2_fox_marth4` and `_cold` scenarios and traces
  are in the main checkout; the savestate's seed is 1915543760
  (`harness/roms/start_mk2_fox_marth4.sav.json`). The boundary still needs its
  `[[boundary]]` entry in `harness/boundaries.toml` (stage
  "MushroomKingdom2", players Fox and Marth, stocks 4).
- GrI2.dat `yakumono_param` (offset 81128):
  - unk0/2 = 50/450: lift A's wait, `50 + rand(450)`
  - unk4/6 = 600/300: lift B's wait
  - unk8/A = 600/600: Birdo's wait
  - unkC/E = 600/600: the carpet's wait
  - unk10 = [265, 410]: the lifts' delay before mpJointListAdd
  - unk14 = (±30, 28.5, 0): Birdo's spawn offset from the blast zones
  - unk2C = 0: Birdo's platform height above her
  - unk30 = (±30, 80, 0): the carpet's spawn offset
  - unk48/4A = 5/3: the carpet's turn odds
- Decomp items live under `src/melee/it/kinds/` (itkyasarin.c,
  itkyasarinegg.c), not `it/items/`. The worktree's submodule is not
  checked out; read the main checkout's copy.

## Shape of the port

- Setup draws: map 2's on_init (grInishie2_801FCE04) calls D018 (two draws),
  D368 (one) and D744 (one), so four `test_random` draws when every range is
  nonzero. on_start disables joints 0, 2..0xE.
- GrJoint `{x: joint, y: map, z: bone}`:
  - joint 0 is on map 1 (the carpet)
  - joints 12 and 13 are on maps 3 and 4 (the lifts)
  - joint 14 is on map 15 (Birdo's platform)
  - joints 2..11 are on maps 5..14, which have no callbacks and are never
    created, so those joints stay disabled
- Maps 1, 3, 4 and 15 are created and destroyed mid-match
  (Ground_GetStageGObj / Ground_801C4A08). Kongo Jungle's event maps are the
  template: `scene_stage/kongo.rs` `create_object`, `remove_object` and
  `load_reserved` (models loaded hidden at load, so the tick does not
  allocate), with tagged procs at `last::TAG_BASE + map * 3 + i`.
- Map 1 (the carpet, D824/D9EC) draws Randi(2) at creation, switches between
  whole-model animations 0 and 1 (grAnime_801C8138; prepare them at load),
  and leaves past the blast zones, restarting the timer through
  grInishie2_801FDFB4.
- Birdo (it_802ECD3C) is a zako item (it_8027B5B0, hold_kind 4), like
  it-heiho and it-klap; the zako KO paths it_8027BA54 / it_8027B964 /
  it_8027C794 / it_8027C79C are already used by it-monsters/redead.rs. Her
  assets come from GrI2.dat's `itemdata` (`ItemAssets::from_stage_item`, as
  Klap and Heiho do). Her damage callback is it_802EDDC0. When she is
  destroyed, itKyasarin_Logic25_Destroyed calls grInishie2_801FD448 (restart
  the timer, remove map 15). When she falls, she calls 801FD4CC (joint 0xE
  off).
- Birdo's egg (it_802EFA44) can be picked up and thrown. On its hit, clank,
  shield or reflect callbacks it spawns efSync 0x4D0, then either hatches
  (Randi(x10) == 0, it_802EFD84) or breaks through it_8026F3D4. Check what
  it_8026F3D4 does with items off before porting.

## Not done

- No MK2 code.
- Great Bay, Princess Peach's Castle and Rainbow Cruise were not started.
