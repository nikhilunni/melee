//! Shared ftLk_ states: Link's code, which Young Link runs as well
//! (ftCl_Init_MotionStateTable points at the same callbacks, except its
//! side taunt rows). Character differences are the family trait's
//! constants and typed accessors.
mod attack_s42;
pub mod attributes;
mod common;
pub mod special_hi;

use attributes::LinkAttributes;
use melee_ft::fighter::{
    assets::FighterAssets, state, state::callbacks, ActionId, CharacterCallbacks, CharacterState,
    Fighter, MotionRow, SpecialSlot,
};
use melee_types::CommonMotionState;

/// The kind checks inside the shared callbacks (fp->kind == FTKIND_LINK /
/// FTKIND_CLINK).
pub trait LinkFamily: CharacterCallbacks {
    /// Spin Attack's onAccessory4: the swirl's second joint, a raw parts
    /// index (FtPart_L2ndNa for Link, FtPart_L3rdNa for Young Link).
    const SPIN_TIP_PART: melee_types::FtPart;
    fn attributes(&self) -> &LinkAttributes;
    fn specials(&mut self) -> &mut Specials;
    fn specials_ref(&self) -> &Specials;
}

/// ftLk_FighterVars (fp+222C): the articles Link has out. ftLk_Init_OnDeath
/// and ftCl_Init_OnDeath clear every field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FighterVars {
    /// +222C: a boomerang is out; side-B throws the empty-handed motion.
    pub used_boomerang: bool,
    /// +2230: the boomerang throw was a smash input (on21EC).
    pub smash_boomerang: bool,
}

/// Fighter state the family owns: ftLk_FighterVars, mv.lk and the one-shot
/// callbacks the states install.
#[derive(Clone, Debug, Default)]
pub struct Specials {
    pub vars: FighterVars,
    /// The one-shot accessory4 a state armed.
    pub accessory: Accessory,
    /// mv+4 as the state before a special left it (no ftLk special but the
    /// bow writes it).
    pub retained_word: Option<f32>,
}

/// accessory4_cb while a family state owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// Spin Attack's onAccessory4 (800EBA4C).
    SpinSwirl,
}

/// ftLk_MS_* (ftLink/forward.h): the family's motion states, contiguous
/// from ftCo_MS_Count.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyState {
    AttackS42 = 341,
    AppealSR,
    AppealSL,
    SpecialNStart,
    SpecialNLoop,
    SpecialNEnd,
    SpecialAirNStart,
    SpecialAirNLoop,
    SpecialAirNEnd,
    SpecialS1,
    SpecialS2,
    SpecialS1Empty,
    SpecialAirS1,
    SpecialAirS2,
    SpecialAirS1Empty,
    SpecialHi,
    SpecialAirHi,
    SpecialLw,
    SpecialAirLw,
    AirCatch,
    AirCatchHit,
}
impl FamilyState {
    pub const COUNT: usize = 21;
    pub const fn action(self) -> ActionId {
        ActionId(self as u16)
    }
    /// ftLk_Init_MotionStateTable's submotion column: ftLk_SM_* in table
    /// order from ftCo_SM_Count, skipping the side taunts, which Link's table
    /// leaves at ftCo_SM_None (Young Link's plays the common AppealSR/L).
    pub const fn animation(self) -> i32 {
        const FIRST: i32 = 295; // ftCo_SM_Count
        match self {
            Self::AttackS42 => FIRST,
            Self::AppealSR | Self::AppealSL => -1,
            _ => FIRST + (self as i32 - Self::SpecialNStart as i32) + 1,
        }
    }
    const fn index(self) -> usize {
        self as usize - Self::AttackS42 as usize
    }
}

/// A family motion row.
pub(crate) const fn row(
    state: FamilyState,
    anim: state::AnimFn,
    iasa: state::InputFn,
    physics: state::PhysicsFn,
    collision: state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: state.action(),
        id: CommonMotionState::None,
        animation: state.animation(),
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftLk_Init_MotionStateTable (ftlink.c:26-289). Rows not yet ported fail
/// closed.
pub const fn rows<C: LinkFamily>() -> [MotionRow; FamilyState::COUNT] {
    let mut rows = [state::unimplemented_row(); FamilyState::COUNT];
    let mut i = 0;
    while i < FamilyState::COUNT {
        rows[i].action = ActionId(FamilyState::AttackS42 as u16 + i as u16);
        i += 1;
    }
    rows[FamilyState::AttackS42.index()] = attack_s42::ROW;
    let spin = special_hi::rows::<C>();
    rows[FamilyState::SpecialHi.index()] = spin[0];
    rows[FamilyState::SpecialAirHi.index()] = spin[1];
    rows
}

/// ftLk_Init_MotionStateTable's move ids: the forward smash's second hit,
/// the taunt and tether rows' FtMoveId_Default, each special's own.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; FamilyState::COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; FamilyState::COUNT];
    let mut i = 0;
    while i < FamilyState::COUNT {
        moves[i] = match i {
            0 => Some(M::SideSmash),
            1..=2 => None,
            3..=8 => Some(M::SpecialNeutral),
            9..=14 => Some(M::SpecialSide),
            15..=16 => Some(M::SpecialUp),
            17..=18 => Some(M::SpecialDown),
            _ => None,
        };
        i += 1;
    }
    moves
}

/// ftData_SpecialN/S/Hi/Lw[kind] and the aerial tables.
pub fn enter_special<C: LinkFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    airborne: bool,
    assets: &FighterAssets,
) {
    // The specials write no mv+4 but the bow's charge.
    let retained = f.inherited_scratch_word();
    f.character.get_mut::<C>().specials().retained_word = retained;
    match slot {
        SpecialSlot::Up => special_hi::enter::<C>(f, airborne, assets),
        _ => unimplemented!(
            "ftData_Special{slot:?}[{:?}] (airborne: {airborne}): character special entry",
            f.core.kind
        ),
    }
}

/// Install `accessory` as the state's one-shot accessory4.
pub(crate) fn arm_accessory<C: LinkFamily>(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<C>().specials().accessory = accessory;
    f.core.arm_accessory4();
}

/// Fighter_8006C80C: the state's one-shot accessory4.
pub fn accessory<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let pending = f.character.get::<C>().specials_ref().accessory;
    if !f.run_accessory4(pending != Accessory::None) {
        return;
    }
    f.character.get_mut::<C>().specials().accessory = Accessory::None;
    match pending {
        Accessory::SpinSwirl => special_hi::swirl::<C>(f, assets),
        Accessory::None => unreachable!(),
    }
}

/// mv+4 while `action`, one of the family's special rows, is current.
pub fn retained_scratch_word<C: LinkFamily>(
    state: &CharacterState,
    action: ActionId,
) -> Option<f32> {
    let first = FamilyState::SpecialNStart as u16;
    let last = FamilyState::SpecialAirLw as u16;
    if !(first..=last).contains(&action.0) {
        return None;
    }
    Some(
        state
            .get::<C>()
            .specials_ref()
            .retained_word
            .unwrap_or_else(|| {
                unimplemented!("ftLk specials: mv+4 inherited from an unmodelled scratch word")
            }),
    )
}

/// lbAnim_8001E8F8(ftData_80085E50(fp, 72)): animation 72's end frame, or
/// zero when the kind has no such animation.
pub fn down_air_frames(assets: &FighterAssets) -> f32 {
    assets
        .motions
        .get(&attributes::DOWN_AIR_ANIMATION)
        .map_or(0.0, |motion| motion.animation.frames)
}
