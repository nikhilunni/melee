//! Static character dispatch and checked inline ownership of typed move scratch.
use super::*;
use std::{
    any::TypeId,
    mem::{align_of, size_of, MaybeUninit},
};

/// Fixed inline capacity; construction rejects larger or more-aligned payloads.
/// Resource vectors inside a payload retain their existing initialization owners.
const PAYLOAD_BYTES: usize = 1024;
#[repr(C, align(16))]
struct Payload([MaybeUninit<u8>; PAYLOAD_BYTES]);

/// One immutable table per character crate. Common motion rows never belong here.
pub struct CharacterTable {
    clone_payload: fn(&CharacterState) -> CharacterState,
    type_id: fn() -> TypeId,
    pub kind: fn(&CharacterState) -> FighterKind,
    pub descriptor: fn() -> &'static assets::CharacterDescriptor,
    pub enter_aerial: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>,
    pub special_rows: &'static [MotionRow],
    pub specials_keep_held_item: bool,
    pub special_moves: &'static [Option<melee_types::combat::StaleMove>],
    pub enter_special: fn(&mut Fighter, SpecialSlot, bool, &assets::FighterAssets),
    pub accessory: fn(&mut Fighter, &assets::FighterAssets, &mut gekko_math::HsdRng),
    pub special_grab: SpecialGrab,
    pub take_damage: Option<fn(&mut Fighter)>,
    /// Fighter.deal_dmg_cb: Fighter_ProcessHit's damage-dealt branch.
    pub deal_damage: Option<fn(&mut Fighter)>,
    pub death: Option<fn(&mut Fighter)>,
    pub hurtbox_detect: Option<fn(&mut Fighter, &assets::FighterAssets, damage::InertTouch)>,
    pub item_muzzle: fn(&mut Fighter, &assets::FighterAssets) -> Option<(Vec3, f32)>,
    pub item_owner: fn(&mut Fighter, &assets::FighterAssets) -> melee_it::ItemOwner,
    pub forward_smash_variant: fn(&CharacterState),
    pub forward_smash: Option<super::RngEntry>,
    pub catch_variant: fn(&mut CharacterState),
    pub mouth_capture_scale: fn(&CharacterState) -> Option<f32>,
    pub throw_variant: fn(&CharacterState),
    pub knockback_enter: fn(&mut Fighter, &assets::FighterAssets),
    pub knockback_exit: fn(&mut Fighter, &assets::FighterAssets),
    pub throw_animation: fn(&mut Fighter, &assets::FighterAssets),
    pub catch_pull_start: fn(&mut Fighter, &assets::FighterAssets, f32) -> f32,
    pub enter_taunt: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>,
    pub defense_contact: Option<DefenseContact>,
    pub reflector_contact: Option<super::reflection::CharacterContact>,
    pub reflect_hit: Option<super::reflection::CharacterResponse>,
    pub process_defense_hit: Option<DefenseHit>,
    pub item_defense_contact: Option<ItemDefenseContact>,
    pub jab_variant: fn(&CharacterState),
    pub third_jab_state: fn(&CharacterState) -> melee_types::CommonMotionState,
    pub restore_saved: fn(&mut CharacterState, &[u8]),
    pub on_load: fn(&mut CharacterState, &mut Capabilities),
    pub on_reset: fn(&mut CharacterState),
    pub on_costume_loaded: fn(&mut CharacterState, &hsd_archive::Archive, u8) -> assets::Result<()>,
    pub on_resources_loaded: fn(&mut CharacterState, &assets::FighterAssets, &PlayerSlot),
    pub on_grounded_motion: fn(&mut CharacterState),
    pub on_motion_change: fn(&mut CharacterState),
    pub dynamics_first_force_bone: fn(&CharacterState, usize, usize) -> usize,
    pub air_dodge_tether: fn(&CharacterState),
    pub on_landing: fn(&mut CharacterState, bool),
    pub retained_scratch_word: fn(&CharacterState, ActionId) -> Option<f32>,
    pub guard_variant: fn(&CharacterState, &mut commands::CommandState),
    pub escape_variant: fn(&mut Fighter, &assets::FighterAssets, bool) -> assets::Result<()>,
    pub check_float_input: fn(
        &CharacterState,
        &crate::input::FighterInput,
        &assets::FighterAssets,
        f32,
        FloatInputPhase,
    ) -> bool,
    pub enter_float: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>,
    pub aerial_jump_style: fn(&CharacterState) -> AerialJumpStyle,
    pub multi_jump_attributes: fn(&CharacterState) -> Option<&multi_jump::MultiJumpAttributes>,
    pub multi_jump_family: fn(&CharacterState) -> usize,
    pub multi_jump_animation: fn(&CharacterState, usize) -> i32,
    pub aerial_jump_entered: fn(&mut Fighter),
    pub aerial_jump_animated: fn(&mut Fighter),
    pub action_id: fn(&CharacterState, melee_types::CommonMotionState) -> i32,
    pub animated_shield: fn(&CharacterState) -> bool,
    pub enter_shield: fn(&mut Fighter, &assets::FighterAssets, bool) -> Option<assets::Result<()>>,
    pub animate_shield: fn(&mut Fighter, &assets::FighterAssets) -> Option<assets::Result<()>>,
    pub input_shield: fn(&mut Fighter, &assets::FighterAssets) -> Option<assets::Result<()>>,
    pub enter_guard_hold: fn(&mut Fighter, &assets::FighterAssets) -> Option<assets::Result<()>>,
    pub enter_guard_off: fn(&mut Fighter, &assets::FighterAssets) -> Option<assets::Result<()>>,
    pub enter_shield_stun: fn(
        &mut Fighter,
        &super::shield::ShieldImpact,
        &assets::FighterAssets,
    ) -> Option<assets::Result<()>>,
    pub escape_finished: fn(&mut Fighter, &assets::FighterAssets) -> Option<assets::Result<()>>,
    pub escape_animated: fn(&mut Fighter),
    pub special_parasol: fn(&CharacterState) -> Option<parasol::SpecialParasol>,
    pub set_parasol_animation: fn(&mut Fighter, usize, f32),
    pub article_destroyed: fn(&mut Fighter, melee_types::ItemKind),
    pub landing_articles: fn(&mut Fighter, bool),
}
impl CharacterTable {
    pub const fn new<C: CharacterCallbacks>() -> Self {
        Self {
            clone_payload: |state| state.clone_typed::<C>(),
            type_id: TypeId::of::<C>,
            kind: |state| state.get::<C>().kind(),
            descriptor: C::descriptor,
            special_rows: C::SPECIAL_ROWS,
            specials_keep_held_item: C::SPECIALS_KEEP_HELD_ITEM,
            special_moves: C::SPECIAL_MOVES,
            enter_aerial: C::ENTER_AERIAL,
            enter_special: C::enter_special,
            accessory: C::accessory,
            special_grab: C::SPECIAL_GRAB,
            take_damage: C::TAKE_DAMAGE,
            deal_damage: C::DEAL_DAMAGE,
            death: C::DEATH,
            hurtbox_detect: C::HURTBOX_DETECT,
            item_muzzle: C::item_muzzle,
            item_owner: C::item_owner,
            forward_smash_variant: |state| state.get::<C>().forward_smash_variant(),
            forward_smash: C::FORWARD_SMASH,
            catch_variant: |state| state.get_mut::<C>().catch_variant(),
            mouth_capture_scale: |state| state.get::<C>().mouth_capture_scale(),
            throw_variant: |state| state.get::<C>().throw_variant(),
            knockback_enter: C::KNOCKBACK_ENTER,
            knockback_exit: C::KNOCKBACK_EXIT,
            throw_animation: C::THROW_ANIMATION,
            catch_pull_start: C::CATCH_PULL_START,
            enter_taunt: C::ENTER_TAUNT,
            defense_contact: C::DEFENSE_CONTACT,
            reflector_contact: C::REFLECTOR_CONTACT,
            reflect_hit: C::REFLECT_HIT,
            process_defense_hit: C::PROCESS_DEFENSE_HIT,
            item_defense_contact: C::ITEM_DEFENSE_CONTACT,
            jab_variant: |state| state.get::<C>().jab_variant(),
            third_jab_state: |state| state.get::<C>().third_jab_state(),
            restore_saved: |state, raw_fighter| state.get_mut::<C>().restore_saved(raw_fighter),
            on_load: |state, capabilities| state.get_mut::<C>().on_load(capabilities),
            on_reset: |state| state.get_mut::<C>().on_reset(),
            on_costume_loaded: |state, archive, costume| {
                state.get_mut::<C>().on_costume_loaded(archive, costume)
            },
            on_resources_loaded: |state, assets, player| {
                state.get_mut::<C>().on_resources_loaded(assets, player)
            },
            on_grounded_motion: |state| state.get_mut::<C>().on_grounded_motion(),
            on_motion_change: |state| state.get_mut::<C>().on_motion_change(),
            dynamics_first_force_bone: |state, set, count| {
                state.get::<C>().dynamics_first_force_bone(set, count)
            },
            air_dodge_tether: |state| state.get::<C>().air_dodge_tether(),
            on_landing: |state, allow_interrupt| state.get_mut::<C>().on_landing(allow_interrupt),
            retained_scratch_word: C::RETAINED_SCRATCH_WORD,
            guard_variant: |state, commands| state.get::<C>().guard_variant(commands),
            escape_variant: C::escape_variant,
            check_float_input: |state, input, assets, vertical_velocity, phase| {
                state
                    .get::<C>()
                    .check_float_input(input, assets, vertical_velocity, phase)
            },
            enter_float: C::ENTER_FLOAT,
            aerial_jump_style: |state| state.get::<C>().aerial_jump_style(),
            multi_jump_attributes: |state| state.get::<C>().multi_jump_attributes(),
            multi_jump_family: |state| state.get::<C>().multi_jump_family(),
            multi_jump_animation: |state, jump| state.get::<C>().multi_jump_animation(jump),
            aerial_jump_entered: C::aerial_jump_entered,
            aerial_jump_animated: C::aerial_jump_animated,
            action_id: |payload, state| payload.get::<C>().action_id(state),
            animated_shield: |state| state.get::<C>().animated_shield(),
            enter_shield: C::enter_shield,
            animate_shield: C::animate_shield,
            input_shield: C::input_shield,
            enter_guard_hold: C::enter_guard_hold,
            enter_guard_off: C::enter_guard_off,
            enter_shield_stun: C::enter_shield_stun,
            escape_finished: C::escape_finished,
            escape_animated: C::escape_animated,
            special_parasol: |state| state.get::<C>().special_parasol(),
            set_parasol_animation: C::SET_PARASOL_ANIMATION,
            article_destroyed: C::ARTICLE_DESTROYED,
            landing_articles: C::LANDING_ARTICLES,
        }
    }
}

/// Owns exactly one typed character value without allocating or exposing raw bytes.
/// The type tag, destructor and table are installed together and cannot be replaced.
/// Accessors check type identity before creating a reference; table hooks may change
/// motion state synchronously, so callers must end scratch borrows first.
pub struct CharacterState {
    payload: Payload,
    type_id: TypeId,
    drop_payload: unsafe fn(&mut Payload),
    table: &'static CharacterTable,
}
impl CharacterState {
    #[inline(always)]
    fn clone_typed<C: CharacterCallbacks>(&self) -> Self {
        let value = self.get::<C>().clone();
        let mut cloned = Self {
            payload: Payload([MaybeUninit::uninit(); PAYLOAD_BYTES]),
            type_id: self.type_id,
            drop_payload: self.drop_payload,
            table: self.table,
        };
        // SAFETY: get::<C> checks the source type; its construction already
        // checked C's size/alignment. This is a typed write of a cloned owner
        // into fresh storage, retaining the matching table and destructor.
        unsafe {
            cloned.payload.0.as_mut_ptr().cast::<C>().write(value);
        }
        cloned
    }
    pub fn new<C: CharacterCallbacks>(value: C) -> Self {
        assert!(
            size_of::<C>() <= PAYLOAD_BYTES,
            "character payload exceeds inline capacity"
        );
        assert!(
            align_of::<C>() <= align_of::<Payload>(),
            "character payload exceeds inline alignment"
        );
        let table = C::table();
        assert_eq!(
            (table.type_id)(),
            TypeId::of::<C>(),
            "character payload/table mismatch"
        );
        let mut state = Self {
            payload: Payload([MaybeUninit::uninit(); PAYLOAD_BYTES]),
            type_id: TypeId::of::<C>(),
            drop_payload: drop_payload::<C>,
            table,
        };
        // SAFETY: checked capacity/alignment; storage is uninitialized and now
        // takes ownership of value. The tag and destructor both describe C.
        unsafe {
            state.payload.0.as_mut_ptr().cast::<C>().write(value);
        }
        state
    }
    pub fn table(&self) -> &'static CharacterTable {
        self.table
    }
    pub fn kind(&self) -> FighterKind {
        (self.table.kind)(self)
    }
    pub fn get<C: CharacterCallbacks>(&self) -> &C {
        assert_eq!(
            self.type_id,
            TypeId::of::<C>(),
            "character payload/table mismatch"
        );
        // SAFETY: construction initialized this exact type; shared borrow of
        // the owner prevents mutation or destruction while the reference lives.
        unsafe { &*self.payload.0.as_ptr().cast::<C>() }
    }
    pub fn get_mut<C: CharacterCallbacks>(&mut self) -> &mut C {
        assert_eq!(
            self.type_id,
            TypeId::of::<C>(),
            "character payload/table mismatch"
        );
        // SAFETY: same type/layout invariant as get, with exclusive ownership.
        unsafe { &mut *self.payload.0.as_mut_ptr().cast::<C>() }
    }
    pub fn forward_smash_variant(&self) {
        (self.table.forward_smash_variant)(self)
    }
    pub fn catch_variant(&mut self) {
        (self.table.catch_variant)(self)
    }
    pub fn throw_variant(&self) {
        (self.table.throw_variant)(self)
    }
    pub fn mouth_capture_scale(&self) -> Option<f32> {
        (self.table.mouth_capture_scale)(self)
    }
    pub fn jab_variant(&self) {
        (self.table.jab_variant)(self)
    }
    pub fn third_jab_state(&self) -> melee_types::CommonMotionState {
        (self.table.third_jab_state)(self)
    }
    pub fn restore_saved(&mut self, raw_fighter: &[u8]) {
        (self.table.restore_saved)(self, raw_fighter)
    }
    pub fn on_load(&mut self, capabilities: &mut Capabilities) {
        (self.table.on_load)(self, capabilities)
    }
    pub fn on_reset(&mut self) {
        (self.table.on_reset)(self)
    }
    pub fn on_costume_loaded(
        &mut self,
        archive: &hsd_archive::Archive,
        costume: u8,
    ) -> assets::Result<()> {
        (self.table.on_costume_loaded)(self, archive, costume)
    }
    pub fn on_resources_loaded(&mut self, assets: &assets::FighterAssets, player: &PlayerSlot) {
        (self.table.on_resources_loaded)(self, assets, player)
    }
    pub fn on_grounded_motion(&mut self) {
        (self.table.on_grounded_motion)(self)
    }
    pub fn on_motion_change(&mut self) {
        (self.table.on_motion_change)(self)
    }
    pub fn dynamics_first_force_bone(&self, set: usize, count: usize) -> usize {
        (self.table.dynamics_first_force_bone)(self, set, count)
    }
    pub fn air_dodge_tether(&self) {
        (self.table.air_dodge_tether)(self)
    }
    pub fn on_landing(&mut self, allow_interrupt: bool) {
        (self.table.on_landing)(self, allow_interrupt)
    }
    pub fn retained_scratch_word(&self, action: ActionId) -> Option<f32> {
        (self.table.retained_scratch_word)(self, action)
    }
    pub fn guard_variant(&self, commands: &mut commands::CommandState) {
        (self.table.guard_variant)(self, commands)
    }
    pub fn check_float_input(
        &self,
        input: &crate::input::FighterInput,
        assets: &assets::FighterAssets,
        vertical_velocity: f32,
        phase: FloatInputPhase,
    ) -> bool {
        (self.table.check_float_input)(self, input, assets, vertical_velocity, phase)
    }
    pub fn aerial_jump_style(&self) -> AerialJumpStyle {
        (self.table.aerial_jump_style)(self)
    }
    pub fn multi_jump_attributes(&self) -> Option<&multi_jump::MultiJumpAttributes> {
        (self.table.multi_jump_attributes)(self)
    }
    pub fn multi_jump_family(&self) -> usize {
        (self.table.multi_jump_family)(self)
    }
    pub fn multi_jump_animation(&self, jump: usize) -> i32 {
        (self.table.multi_jump_animation)(self, jump)
    }
    pub fn action_id(&self, state: melee_types::CommonMotionState) -> i32 {
        (self.table.action_id)(self, state)
    }
    pub fn animated_shield(&self) -> bool {
        (self.table.animated_shield)(self)
    }
}
impl Drop for CharacterState {
    fn drop(&mut self) {
        // SAFETY: constructor pairs this destructor with the initialized value;
        // no API moves that value out or changes its type. Drop runs exactly once.
        unsafe {
            (self.drop_payload)(&mut self.payload);
        }
    }
}
unsafe fn drop_payload<C: CharacterCallbacks>(payload: &mut Payload) {
    // SAFETY: called only by the owner whose constructor stored C here.
    unsafe {
        payload.0.as_mut_ptr().cast::<C>().drop_in_place();
    }
}

impl Clone for CharacterState {
    fn clone(&self) -> Self {
        (self.table.clone_payload)(self)
    }
}

#[cfg(test)]
mod tests;

// Concrete defaults belong to the shared library, not each character's generic
// table construction. Prevent automatic cross-crate inlining from cloning the
// no-op body into every table owner.
#[inline(never)]
pub(super) fn no_animation(_fighter: &mut Fighter, _assets: &assets::FighterAssets) {}
/// ftCommon_8007E83C without a special parasol: retail asserts
/// (ftcommon.c:1069) unless the Parasol item is held, which is unported.
pub(super) fn unsupported_parasol_animation(_fighter: &mut Fighter, index: usize, frames: f32) {
    unimplemented!("ftCommon_8007E83C({index}, {frames}): Parasol item animation")
}
pub(super) fn no_article(_fighter: &mut Fighter, _kind: melee_types::ItemKind) {}
pub(super) fn no_landing_articles(_fighter: &mut Fighter, _allow_interrupt: bool) {}

#[inline(never)]
pub(super) fn catch_frame(
    _fighter: &mut Fighter,
    _assets: &assets::FighterAssets,
    frame: f32,
) -> f32 {
    frame
}

#[inline(never)]
pub(super) fn no_retained_scratch_word(_state: &CharacterState, _action: ActionId) -> Option<f32> {
    None
}

pub(super) fn unsupported_float(
    _fighter: &mut Fighter,
    _assets: &assets::FighterAssets,
) -> assets::Result<()> {
    unreachable!("float entry for a character whose float predicate never matches");
}

pub(super) fn unsupported_special_grab(
    _captor: &mut Fighter,
    _victim: &mut Fighter,
    _captor_assets: &assets::FighterAssets,
    _victim_assets: &assets::FighterAssets,
) -> assets::Result<()> {
    unimplemented!("fighter.c:2602-2603: special grab_cb / grabbed_cb");
}

pub(super) fn unsupported_taunt(
    _fighter: &mut Fighter,
    _assets: &assets::FighterAssets,
) -> assets::Result<()> {
    unimplemented!("ftCo_800DEA28: character taunt entry");
}
