# Steel thread: one full match, bit-exact

Decided with the user on 2026-09-09 after the stock-take. Tracking lives in
`TRACKER.md` ("Steel thread" section); this file holds the reasoning and
the code shapes so a lane can read them without the conversation.

## Definition

Fox vs Marth on Final Destination, four stocks, eight minutes, human
inputs on both ports, from match start through the GAME banner. The port
must reproduce a Dolphin recording of such a match tick for tick on the 49
gated fighter keys, the RNG ledger and the particle replay. Camera is out
of scope for the gates: it touches neither fighter state nor the RNG.
Acceptance is two-fold: our own recording (`match_fd_foxmarth`) and one
offline Slippi replay of the same matchup recorded by the user. Fox vs Fox
is the fallback if Marth's moveset stalls.

Everything a pad can reach has to exist, because one unported state
anywhere breaks the trace from that tick on. The cost table is in
`TRACKER.md`; the estimate on 2026-09-09 was 30 to 40 Codex tasks after a
consolidation round of about ten.

## Why consolidate first

Movement proved the trait pattern (`CharacterCallbacks` with hooks, shared
states in `melee-ft`, one macro listing characters). Combat strained it:
about a dozen interim `if kind == ..` checks sit inside trait default
bodies, callbacks are dispatched through hand-maintained enums, the effect
layer lives in `melee-sim`, and the tick path allocates. Each of the next
twelve characters and every special move would deepen those grooves. The
consolidation round replaces them with the shapes below, using the 260+
existing gates as the proof that nothing moved.

## How retail structures it (the model we copy)

- Every fighter state is a row in a table: animation id, flags, move id,
  and five callbacks (anim, IASA, physics, collision, camera). See
  `MotionState` in `ft/types.h`.
- The common table `ftData_MotionStateList` has 341 rows shared by every
  character. Each character owns a second table starting at row 341 for
  its specials (`ftFx_Init_MotionStateTable` for Fox). Common code never
  sees the specials; it reaches them through eight per-kind entry points
  (`ftData_SpecialN`, `SpecialAirN`, `SpecialS`, ...) alongside
  `OnLoad`, `OnDeath`, `OnAbsorb`, `OnItemPickup`.
- Falco's specials are Fox's functions. The decomp's `ftFox/` files branch
  on `FTKIND_FALCO` in exactly three ways: which laser and ghost item kind
  to spawn, which sound ids to play, and the attribute values.
- Items mirror fighters: a `SpawnItem` descriptor (owner, kind, position,
  velocity, facing, damage), a per-kind `ItemLogicTable` row holding a
  state table plus about fourteen event callbacks (clanked, reflected,
  absorbed, shield-bounced, hit-shield, ...), and the same subaction script
  format with the item's own collision data. Fox and Falco lasers share one
  state table and one set of callbacks.

## Motion state tables

One row type for both tables. Rows are `Copy`; the common table is a
per-character associated const so it can hold `fn(&mut Fighter<Fox>)`
pointers like the character's own rows. Sketch:

```rust
// melee-ft
pub struct ActionId(pub u16);            // < COMMON_COUNT shared, >= character
pub const COMMON_COUNT: usize = 341;
pub type Callback<C> = fn(&mut Fighter<C>);

#[derive(Clone, Copy)]
pub struct MotionRow<C: CharacterCallbacks> {
    pub animation: AnimationId,
    pub flags: MotionFlags,
    pub move_id: MoveId,
    pub anim: Callback<C>,
    pub iasa: Callback<C>,
    pub physics: Callback<C>,
    pub collision: Callback<C>,
    pub camera: Callback<C>,
}

pub const fn common_table<C: CharacterCallbacks>() -> [MotionRow<C>; COMMON_COUNT] { /* rows */ }

pub enum SpecialSlot { Neutral, Side, Up, Down }

pub trait CharacterCallbacks: Sized + 'static {
    const COMMON: [MotionRow<Self>; COMMON_COUNT] = common_table::<Self>();
    fn special_rows() -> &'static [MotionRow<Self>] { &[] }
    fn enter_special(_f: &mut Fighter<Self>, _slot: SpecialSlot, _airborne: bool) {}
    // ... existing hooks
}

impl<C: CharacterCallbacks> Fighter<C> {
    pub fn row(&self, action: ActionId) -> MotionRow<C> {
        let i = action.0 as usize;
        if i < COMMON_COUNT { C::COMMON[i] } else { C::special_rows()[i - COMMON_COUNT] }
    }
    pub fn change_state(&mut self, action: ActionId, keep: MotionFlags, start_frame: f32, rate: f32) { /* Fighter_ChangeMotionState */ }
    pub fn tick_state(&mut self) {
        let row = self.state.row;
        (row.anim)(self); (row.iasa)(self); (row.physics)(self); (row.collision)(self); (row.camera)(self);
    }
}
```

The shared attack-input check calls `C::enter_special(f, slot, airborne)`;
from there the character moves through its own rows and never touches
shared code again.

## Family crates

Falco must not depend on ft-fox (character crates never depend on each
other), and shared code must not `switch (kind)`. So the shared code goes
in a third crate that neither character owns:

```rust
// crates/ft-fox-family  (retail ftFx_)
pub trait FoxFamily: CharacterCallbacks {
    const LASER: ItemKind;       // Fox_Laser / Falco_Laser
    const GHOST: ItemKind;       // Fox_Illusion / Falco_Phantasm
    const SOUNDS: FamilySounds;  // blaster fire, holster, throw-fire ids
    fn attributes(&self) -> &FoxAttributes;             // same layout, different values
    fn special_neutral(&mut self) -> &mut SpecialNeutral; // typed scratch (retail fp->mv union)
}

#[repr(u16)]
pub enum FamilyState { SpecialNStart = COMMON_COUNT as u16, SpecialNLoop, SpecialNEnd, /* ... */ }

pub const fn rows<C: FoxFamily>() -> [MotionRow<C>; FamilyState::COUNT] { /* generic callbacks */ }
pub fn enter_special<C: FoxFamily>(f: &mut Fighter<C>, slot: SpecialSlot, airborne: bool) { /* ... */ }

pub mod special_n {
    pub fn loop_anim<C: FoxFamily>(f: &mut Fighter<C>) {
        if f.subaction_flag(0) && f.character.special_neutral().shots_fired == 0 {
            let attrs = f.character.attributes();
            let pos = f.bone_position(FoxBone::RightThumb, attrs.blaster_hold_offset);
            f.items().spawn(SpawnItem::ray(C::LASER, f.slot(), pos, attrs.blaster_velocity, f.facing()));
            f.play_sound(C::SOUNDS.fire[f.facing_left() as usize]);
            f.character.special_neutral().shots_fired += 1;
        }
        if f.animation_finished() {
            f.change_state(FamilyState::SpecialNEnd.into(), MotionFlags::NONE, 0.0, 1.0);
        }
    }
}
```

Each character is then a page of data:

```rust
// crates/ft-fox
impl FoxFamily for Fox {
    const LASER: ItemKind = ItemKind::FoxLaser;
    const GHOST: ItemKind = ItemKind::FoxIllusion;
    const SOUNDS: FamilySounds = FamilySounds { fire: [110_100, 110_101], holster: 110_109, /* ... */ };
    fn attributes(&self) -> &FoxAttributes { &self.attributes }
    fn special_neutral(&mut self) -> &mut SpecialNeutral { &mut self.special_neutral }
}
impl CharacterCallbacks for Fox {
    fn special_rows() -> &'static [MotionRow<Fox>] {
        static ROWS: [MotionRow<Fox>; FamilyState::COUNT] = ft_fox_family::rows::<Fox>();
        &ROWS
    }
    fn enter_special(f: &mut Fighter<Fox>, slot: SpecialSlot, airborne: bool) {
        ft_fox_family::enter_special(f, slot, airborne)
    }
}
```

`ft-falco` is the same with Falco's constants. A character with no sibling
(Marth until Roy) keeps its rows in its own crate as
`[MotionRow<Marth>; N]` with non-generic callbacks; promoting that to a
family crate later is mechanical.

## Items

- One engine crate (`melee-it`): the common loop (animation, physics,
  collision, script interpreter, owner as a player slot index), a
  `SpawnItem` matching retail's, fixed pools.
- One trait per item kind mirroring the logic row: a state table plus the
  event callbacks, all defaulting to "no reaction". Kinds are listed in one
  macro like `scene_characters!`, giving an enum rather than boxed trait
  objects, because items are heterogeneous at run time where fighters are
  not.
- Two extractions from `melee-ft` first: the subaction command interpreter
  (`melee-cmd`) and the hitbox/hurtbox machinery (`melee-coll`), so fighter
  vs item hits and shield reflection have one implementation.
- The laser as its own small crate (`it-foxlaser`), depended on by the Fox
  family crate.
- The tracer must record the item list as new compared keys before the
  first item task (C9).

## Cost of the abstractions, measured

| Baseline 2026-09-09 | |
|---|---|
| Release `melee-sim`, stripped, 7 characters, 4 stages, CLI deps | 3.9 MB |
| Text segment | 3.6 MB |
| `gate start_fd_fox`, 600 ticks, CPU time incl. savestate load and compare | 0.25 s |

Retail's executable is about 4.3 MB. What each piece costs:

- Associated consts, attribute accessors, typed scratch, the family trait:
  nothing; resolved at compile time and inlined.
- Fn-pointer tables: five indirect calls per fighter per tick, identical
  to retail's design; inlining across the call is lost and irrelevant.
- The character enum: one match per fighter per tick.
- Monomorphization: the real cost. Every generic function is compiled once
  per character that instantiates it; binary size and build time grow
  with roster size times generic surface. The fix is a concrete core with
  a thin generic shell (C2) and an instantiation budget (`cargo
  llvm-lines`).

The actual performance debt found on 2026-09-09 is not the abstractions:
the effect flush allocates four vectors per motion change, the frame loop
holds an `Rc<RefCell<Vec>>` for RNG-call tracking, and several bone-oracle
modules `collect()` per frame. C5 removes them and locks it in with an
allocation-count test. Bit-exactness and optimization do not fight: Rust
never reassociates or contracts floats, so fat LTO, one codegen unit and
native CPU targeting are safe.

## Order

1. C1 table refactor (pure restructuring; gates unchanged) in the core
   lane; C6/C5-sim/C10 in the perf lane; C9 in the harness lane.
2. C2 concrete core, then C7 kind-check cleanup, on the core lane.
3. C4 `melee-ef` and C8 `melee-cmd`/`melee-coll` once C1/C2 have merged.
4. S4 Fox laser end to end (spawn, travel, hit Marth, hit a shield,
   despawn) to force the item crate into existence against one case; C3
   family crate lands with it.
5. The rest of the S table, breadth paused until the thread closes.
