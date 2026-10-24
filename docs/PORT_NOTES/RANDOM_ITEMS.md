# Random items (wave 13)

Matches with the Item Switch on, from boundary to spawner to containers.

## Boundaries

`make_boundary.py --item-frequency {off,very-low,low,medium,high,very-high}`
pokes the save data's frequency (`gmm_x1CB0.item_freq`, 0x8045C370) and
`--items NAME...` its switch (`item_mask`, 0x8045C378; bit i is item kind
`lbl_803B7844[i]`, gm_1601.c:3648). Containers (Capsule, Box, Barrel, Egg)
have no switch entry: VS rules start from an all-ones mask (gm_1601.c:3534),
so they spawn whenever items are on. `--no-containers` / `--containers Taru`
replace the match's mask (StartMeleeRules.x20 of gmVsMelee_StartData) at the
creation hook. The match's rules (xB, x20) are read back after creation and
written to both scenarios (`item_frequency`, `item_mask`), the registry and
the recording format (`Config.items`); `replay_to_scenario.py` matches them.

Registered: `start_fd_items_sword_nocont` (Sword only), `start_fd_items_sword`
(the menus' mask: Sword and containers), `start_fd_items_taru` (Sword and
Barrels), `start_bf_items_sword`.

The wave-14 integration registers 27 exact scenarios: these four warm/cold
boundary pairs and 19 recorded explorer cases. Eight additional explorer
cases fixed by the foundation were promoted after the full sweep. The m5
check uses each boundary's recorded `ledger600` pass explicitly, including
cold construction, and retains complete per-tick RNG-site assertions.

## The spawner (itspawn.c)

- it_8026D018 builds the pick tables from the stage weights
  (Ground_801C28CC: grGroundParam +0x6A times the StageParam row's +0x1A)
  with the 0.99 rounding fmadds (8026CADC, 8026CCF4), and draws the first
  interval (ItCo +FC by frequency, fmadds 8026D1F4, times Ground_801C2AE8).
  Its draw comes **before** the stage's creation draws (it runs between
  Stage_802251E8 and Stage_8022524C): checked on FD and Battlefield.
- fn_8026C88C, GObj 5/7 proc at s_link 0: countdown, it_8026C75C (Poke
  Ball exclusion), Stage_80224FDC marker, mpColl_8004D024, facing, item,
  efAsync 0x420 (generator 0x159), then the next interval.
- Savestate import reads the countdown from it_804A0E30 and checks the
  rebuilt table's total and mask against memory.
- Every new item is grabbable (Item_80267130 calls it_8026B390), which also
  runs a common item's lifetime from its creation.

## Containers (it-container)

The public API allocation regression replays three recorded FD/Battlefield
matches through step, observe and presentation capture. It requires live
Capsules, Boxes and Barrels and zero allocations after construction.
Imported matches retain the saved item rules through clone/reset, including
warm scenarios that omit item metadata. Boundary CLI tests cover combined
team/item options without starting Dolphin.

- Capsule: fall, rest, hold, throw, drop, open (one item) or explode.
- Box: fall, rest, break (efAsync 0x427's six boards, model 0x1B with
  efLib_Cb_Fall_FromParamY) into one to three items (it_80286248 with
  it_8026F6BC's monster roll and the same-kind roll).
- Barrel: fall with bounce, rest, roll (it_802874F0), struck rolling with the
  striker as owner (it_8027236C), break open into one item.
- Release (it_8026F3D4 / it_8026F5C8 / it_8026F53C) runs in the scene on
  `ItemEvent::Release` / `CrateContents`; a callback that draws an effect
  before going on resumes through `ItemLogic::resumed`.
- Heavy items push each other apart (it_80271F78).

## Fail closed (follow-up worklist)

- Item Switch setups outside two-player Final Destination and Battlefield
  remain fail-closed until recorded, including saved-state imports whose
  scenario does not explicitly name the saved item rules.

- Box / Barrel broken empty (it_80286AA4, it_802885C8): the blast hitbox's
  state is 1 in the port where retail has 2 (explorer cases
  corpus_v3_fd_items_sword_e7254fba4_p2, corpus_v3_fd_items_taru_ee6ff560c_p0).
  The Capsule's blast (it_8027CFE8) is exact.
- Thrown / dropped Box (reached only through HeavyGet), sliding Capsule and
  Box, barrel state 4 (entered by nothing).
- A barrel struck by an item (it_8027236C's xCF0 owner).
- Monsters from crates (it_8026F6BC, 1 in ItCo +124).
- Poke Balls (it_8026C704 / Item_804A0C64.x1C).
- Party Ball (itkusudama.c): ported then withdrawn. Its fall matches; on the
  settling landing retail's bounce spark (it_80277C40, flushed at link 9)
  is placed by a root matrix without the pitch it_8028A190 sets
  (start_fd_items_kusudama, tick 495; the recording is in harness/traces).
- Egg: weight 0 on FD and Battlefield (COVERAGE_AUDIT).
- Fighter side, the bulk of explorer faults: HeavyGet / LiftWait / LiftWalk /
  LiftTurn / HeavyThrow (ftCo_Lift.c, ftCo_ItemThrow.c) for crates and
  barrels; Beam Sword smashes, tilts, aerials and C-stick smashes with a
  battering item (attack.rs, item_pickup.rs HELD_ITEM_STATES).
- Every other random item kind fails closed by name at its spawn
  (scene_items::spawn_random_item / spawn_released_item).
- `command return without call` in the item script engine on a barrel
  dropped off a roll (corpus_v3_fd_items_taru_e17844051_p1).
