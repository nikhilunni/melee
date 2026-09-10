# C15: one concrete fighter shell with static character callbacks

Design only. Schedule after S1/S4 and the in-flight combat work merge. P1 binds
existing typed tables at construction to stop Yoshi's cross-crate fanout; it does
not implement this concrete-shell design. Re-measure the merged combat tree
before implementing C15; the numbers below describe P1 on cd6cfad.

## Measured problem and budget

`cargo llvm-lines -p <crate> --release --lib`, using the existing PERF substring
census, reports 2,015 melee-ft copies / 135,809 IR lines compiled by melee-sim,
535 / 77,989 by melee-ft, and 22 / 1,105 by ft-yoshi after P1. Before P1 those
were 2,017 / 137,843, 532 / 77,080, and 228 / 14,783 respectively. Other character
crates contribute 19 copies combined. These are emitted IR definitions, including
closures and some standard-library helpers; they are not linked text bytes.

The remaining sim definitions fall into these concrete-shell candidates:

| Current sim group | Function labels | Copies | IR lines | One-body arithmetic estimate: copies / IR lines |
|---|---:|---:|---:|---:|
| Inherent `Fighter<C>` methods | 104 | 716 | 84,526 | 104 / 12,157 |
| `state::callbacks::*` motion callbacks | 101 | 695 | 22,827 | 101 / 3,315 |
| `capture_pair`, `enter_back_throw`, `release_back_throw`, `detect_hit` | 4 | 196 | 22,344 | 4 / 456 |
| Selected groups total | 209 | 1,607 | 129,697 | 209 / 15,928 |

The estimate divides each label's measured lines by its actual copy count;
it is sizing arithmetic, not a predicted optimizer result. It suggests removing
1,398 redundant definitions and approximately 113,769 IR lines from these groups,
with the one shared implementation moving to melee-ft. Initialization wrappers,
character adapters, special/family logic and unrelated generic libraries remain.
A practical first C15 budget is at most **900** substring-census copies in
melee-sim (617 remaining after that arithmetic plus 283 adapter/transition copies),
**one definition per common shell/callback label across compiling crates**, and
**one per common pair helper**, with separately reported character adapters.
The total across all crates must fall; moving the same duplicates between crates
is not a pass. Rebaseline melee-ft's concrete-body count explicitly after review.

Seven scene characters currently produce 49 copies of each of the four pair
helpers. With 26 characters that product would become 676 each. A concrete pair
signature removes the product entirely. The common table has 341 rows and five
function-pointer fields: sharing it removes six sets of 1,705 pointer slots,
81,840 bytes of pointer fields on a 64-bit target before metadata/alignment and
link-time folding. Per-character hook tables should remain around a few hundred
bytes each; measure `size_of` and the emitted data section instead of assuming
that every hook below needs an individual stored pointer.

## Ownership and dependency direction

Use a non-generic `Fighter` containing the existing `FighterCore`, a typed
`CharacterState` enum, an installed non-generic `MotionRow`, and a reference to a
static `CharacterVTable`. Keep the installed row: callbacks must not be looked up
again on each phase, and the callback-substitution regression must still pass.

`CharacterState` belongs below the character implementation crates. Place its
data-only variants and per-move state structures under new
`melee-ft/src/character_state/` modules. Move character-specific attribute *type*
definitions there as needed; archive readers and gameplay remain in ft-*/family
crates. melee-ft must not acquire dependencies on those implementation crates.
This avoids both a crate cycle and unsafe opaque-payload casts. It also follows
the existing rule that character scratch is typed, not an untyped union.

Keep local character marker types in ft-* crates implementing the construction
trait, with an associated state type/factory. Moving both a trait and its
implementing type into melee-ft would make external implementations violate the
orphan rule; local marker types avoid that. Character adapters alone destructure
their expected enum variant. Shared gameplay never branches on fighter kind.
A mismatched payload/table pair must fail during construction, not silently
select another character's behavior.

The simulator can own two concrete fighters in initialization-time unique
storage. No dyn, Rc, RefCell, per-tick Box allocation, payload allocation, or
unsafe lifetime/type erasure is needed. Variable-sized archive/model owners
remain allocated once, including P1's revival platform. Common physics continues
to take `FighterCore` directly; only hook-calling state behavior needs `Fighter`.

## Callback signatures and dispatch

All existing resource arguments, result types and invocation order stay intact.
The five phase contexts in `state/phase.rs` retain their current borrowed fields.

| API today | C15 signature / placement |
|---|---|
| `AnimFn<C>` | `fn(&mut Fighter, AnimationPhase<'_>) -> Result<Option<WaitChoice>>` |
| `InputFn<C>` | `fn(&mut Fighter, InputPhase<'_>)` |
| `PhysicsFn<C>` | `fn(&mut Fighter, PhysicsPhase<'_>)` |
| `CollisionFn<C>` | `fn(&mut Fighter, CollisionPhase<'_>) -> Result<()>` |
| `CameraFn<C>` | `fn(&mut Fighter, CameraPhase<'_>)` |
| `COMMON`, `common_table<C>()`, `MotionRow<C>`, `MotionState::new<C>` | One static common table, non-generic row and scalar-row constructor |
| `SPECIAL_ROWS` / `special_rows()` | Static non-generic character table referenced by the character vtable |
| `enter_special(&mut Fighter<Self>, SpecialSlot, bool)` | `fn(&mut Fighter, SpecialSlot, bool)` |
| `escape_variant(&mut Fighter<Self>, &FighterAssets, bool) -> Result<()>` | Same arguments/results with concrete `Fighter` |
| `enter_shield(&mut Fighter<Self>, &FighterAssets, bool) -> Option<Result<()>>` | Same arguments/results with concrete `Fighter`; preserve None fallback |
| `animate_shield`, `input_shield`, `enter_guard_hold`, `enter_guard_off`, `escape_finished` | Each `fn(&mut Fighter, &FighterAssets) -> Option<Result<()>>` |
| `aerial_jump_entered`, `aerial_jump_animated`, `escape_animated` | Each `fn(&mut Fighter)` |
| `on_reset`, `on_grounded_motion`, `catch_variant` | Each `fn(&mut CharacterState)` |
| `on_landing(&mut self, bool)` | `fn(&mut CharacterState, bool)` |
| `guard_variant(&self, &mut CommandState)` | `fn(&CharacterState, &mut CommandState)` |
| `forward_smash_variant`, `throw_variant`, `check_hurtbox_interaction`, `jab_variant`, `air_dodge_tether` | Each `fn(&CharacterState)` |
| `action_id(&self, CommonMotionState) -> i32` | `fn(&CharacterState, CommonMotionState) -> i32` |
| `animated_shield`, `aerial_jump_style`, `multi_jump_family` | Typed queries over `&CharacterState`, returning bool / AerialJumpStyle / usize |
| `multi_jump_attributes` | Lifetime-preserving `for<'a> fn(&'a CharacterState) -> Option<&'a MultiJumpAttributes>` |
| `multi_jump_animation(&self, usize) -> i32` | `fn(&CharacterState, usize) -> i32` |
| `dynamics_first_force_bone(&self, usize, usize) -> usize` | `fn(&CharacterState, usize, usize) -> usize` |
| `check_float_input(&self, &FighterInput, &FighterAssets, f32, FloatInputPhase)` | Same inputs with `&CharacterState` replacing self |
| `kind`, `descriptor`, `from_archive`, `restore_saved`, `on_load`, `on_costume_loaded`, `on_resources_loaded` | Construction/import factories over the marker's associated state; loaded kind/capabilities/descriptors remain immutable data. Death calls only on_reset. |
| Generic pair entry/reaction helpers | Two concrete `&mut Fighter` parameters, preserving receiver-first order and each participant's own vtable |

Do not reduce the number of hooks by combining calls across a state change,
resource mutation, RNG draw, or collision candidate. Preserve the existing
begin-motion-change → grounded hook → reset/install row → animated-shield hook
→ playback order. Read a copied static function pointer before borrowing the
fighter mutably. A hook may synchronously change state through the same concrete
shell; no adapter may retain a borrow into character scratch across that call.

Common rows point directly at common concrete callbacks. Character rows point at
small character or family adapters. Fox/Falco shared specials may still use a
family marker trait to select constants/accessors in the family crate; their
callbacks accept concrete `Fighter`, and instantiating a family move must never
pull the common state graph back into that crate. Scalar descriptor properties
can become vtable data when their current semantics are immutable. This is a
measured tradeoff: common shell code loses cross-hook inlining and adds indirect
hook calls; performance must be verified, not asserted to improve from IR size.

## Files and staged acceptance

1. Introduce data-only character states in `melee-ft/src/character_state/`;
   adapt state construction in `ft-{fox,falco,mars,captain,peach,purin,yoshi}/src/`
   and the merged family crates. Reconcile any new combat scratch/traits after
   S1/S4 merge. No state or oracle changes in this step.
2. Change `melee-ft/src/fighter/mod.rs`, `state/{phase,row,common_table}.rs`,
   `state/callbacks/*.rs`, `state/special.rs` and `spawn.rs` to concrete rows and
   static vtables; preserve installed-row and action-ID mapping tests.
3. Convert hook callers in `fighter/{procs,life,shield,escape,jump,multi_jump,
   fall,landing,ledge,walk,turn,turn_run,squat,dash,run,attack*,damage,grab,
   grab_throw,air_dodge}.rs` and any merged specials. The attack/damage and family
   files are future C15 scope, not P1 edits. Hook-free helpers stay on FighterCore.
4. Update `melee-sim/src/{scene_fighter,frame}.rs`, `frame/{grab_pairs,combat}.rs`,
   `initial_state/{fighter,saved_pose}.rs` and affected import/replay tests to the
   concrete type. Keep kind selection at composition, including adding a roster
   entry through the existing single-list convention. Update macro-generated
   adapters together with the character enum so mismatches are impossible.
5. Run both full workspace profiles, M4 261, the post-combat M5 gate list,
   `check-release-math.sh`, clippy and fmt. Require all 49 KO keys and ordered
   particle RNG draws, the five allocation ceilings and P1's repeated-revival
   zero-allocation test. Never change expected words to accommodate refactoring.
6. Run the full PERF census per compiling crate, preserve original/raw symbols,
   and verify no common Yoshi body is duplicated downstream. Capture load time,
   simulate-only time, stripped bytes, text and common-table data before/after
   on an otherwise idle machine. Preserve the current time/size limits; accept
   the proposed sim-copy budget only after all semantic gates pass. A callback
   API migration without the measured reduction is unfinished C15 work.
