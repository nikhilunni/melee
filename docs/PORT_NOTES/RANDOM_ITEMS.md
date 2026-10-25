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

## Empty Box and Barrel blasts (wave 15)

The deferred continuation after the boards retains the originating item
callback's phase. Retail `it_802790C0` checks the current scheduler object's
`s_link > 11` (8027951C), then immediately positions a newly created hitbox
through `it_8027129C` (8027952C). That sets its current and previous positions
and advances state 1 to 2. The port's normal script path already does this;
the Box/Barrel continuation previously lost the late callback context before
running its blast script. Capsule runs its blast inline and was unaffected.

`ItemEvent::Resume` now carries both hitbox-refresh and immediate efAsync
context. The scene restores the issuing context only while executing the
continuation, then restores the drain's previous context. The original
Effect 0x427, resume/empty-roll, Effect 0x410 and script event order remains.
A regression exercises early and late issuers under opposite drain contexts,
including first-position initialization and restoration of both flags.

Box `it_80286AA4` and Barrel `it_802885C8` hide, become intangible and
ungrabbable, set their explosion lifetime, zero x/y velocity, set
opened/despawn fields, enable owner/kindred hits, issue the gust, and enter
motion 6 with flags 2 (retail 80286B58 / 8028867C). They use the existing
common explosion machinery; no hitbox phase is assigned by kind code.

The complete primary recordings contain three Box blasts across 4,711 ticks
(`corpus_v3_fd_items_sword_e7254fba4_p2`) and two Barrel blasts across 4,669
ticks (`corpus_v3_fd_items_taru_ee6ff560c_p0`). Eight complete recordings are
registered in `RANDOM_ITEM_WITNESSES`. They start from warm match boundaries;
they do not claim import of an already exploding item. Existing warm/cold
start witnesses remain independent construction coverage. The separate
`...sword_e17844051_p1` and `...sword_eee14f173_p2` recordings also pass in
full after the held-Sword ground entry integration. Both are registered;
the strict random-item family now contains 37 complete witnesses.

## Fail closed (follow-up worklist)

- Item Switch setups outside two-player Final Destination and Battlefield
  remain fail-closed until recorded. Warm imports preserve the actual saved
  item rules even when scenario metadata omits them; clone/reset retains
  those rules.

- Thrown / dropped Box (reached only through HeavyGet), sliding Capsule and
  Box, barrel state 4 (entered by nothing).
- Barrel article-owner transfers outside the witnessed primary-owner to
  unowned REST recipient case (`BARREL_OWNER_TRANSFER.md`).
- Monsters from crates (it_8026F6BC, 1 in ItCo +124).
- Poke Balls (it_8026C704 / Item_804A0C64.x1C).
- Party Ball (itkusudama.c): ported then withdrawn. Its fall matches; on the
  settling landing retail's bounce spark (it_80277C40, flushed at link 9)
  is placed by a root matrix without the pitch it_8028A190 sets
  (start_fd_items_kusudama, tick 495; the recording is in harness/traces).
- Egg: weight 0 on FD and Battlefield (COVERAGE_AUDIT).
- Fighter side, the bulk of explorer faults: HeavyGet / LiftWait / LiftWalk /
  LiftTurn / HeavyThrow (ftCo_Lift.c, ftCo_ItemThrow.c) for crates and
  barrels; held Sword aerials/landings, ordinary Hi3/Lw4 and private rows,
  plus unproved battering-item kinds. The witnessed common ground rows
  57/63 and Sword C-stick throws are admitted (`HELD_SWORD_GROUND.md`).
- Every other random item kind fails closed by name at its spawn
  (scene_items::spawn_random_item / spawn_released_item).
- `command return without call` in the item script engine on a barrel
  dropped off a roll (corpus_v3_fd_items_taru_e17844051_p1).
