//! Shared ftLk_ states: Link's code, which Young Link runs as well
//! (ftCl_Init_MotionStateTable points at the same callbacks, except its
//! side taunt rows). Character differences are the family trait's
//! constants and typed accessors.
mod attack_s42;
pub mod attributes;
mod common;
pub mod special_hi;
pub mod special_s;

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
    /// +2234 != NULL: the boomerang exists.
    pub boomerang_out: bool,
    /// The boomerang hangs in the hand (its state 0, it_8029FDBC).
    pub boomerang_in_hand: bool,
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
    /// take_dmg_cb / death2_cb = ftLk_800EAF58 for this motion: the articles
    /// go when the fighter is hit.
    pub removal_armed: bool,
}

/// accessory4_cb while a family state owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// Spin Attack's onAccessory4 (800EBA4C).
    SpinSwirl,
    /// The boomerang throw's onAccessory4 (800EC210), every frame while
    /// installed.
    BoomerangThrow,
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
    let boomerang = special_s::rows::<C>();
    let mut i = 0;
    while i < boomerang.len() {
        rows[FamilyState::SpecialS1.index() + i] = boomerang[i];
        i += 1;
    }
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

/// ftLk_Init_MotionStateTable's x4_flags column (ftLk_MF_*, ftLink/forward.h),
/// which Fighter.x2070 takes on entry. Young Link's side taunts are
/// ftCl_MF_Zair (0x71) where Link's rows are empty.
pub const fn motion_flags(young: bool) -> [u32; FamilyState::COUNT] {
    let taunt = if young { 0x71 } else { 0 };
    [
        0x0024_0009, // AttackS42
        taunt,
        taunt,
        0x0034_0111, // SpecialNStart
        0x003C_0111, // SpecialNLoop
        0x0034_0111, // SpecialNEnd
        0x0034_0511, // SpecialAirNStart
        0x003C_0511, // SpecialAirNLoop
        0x0034_0511, // SpecialAirNEnd
        0x0034_0112, // SpecialS1
        0x0034_0112, // SpecialS2
        0x0034_0112, // SpecialS1Empty
        0x0034_0512, // SpecialAirS1
        0x0034_0512, // SpecialAirS2
        0x0034_0512, // SpecialAirS1Empty
        0x0034_0213, // SpecialHi
        0x0034_0213, // SpecialAirHi
        0x0034_0014, // SpecialLw
        0x0034_0414, // SpecialAirLw
        0x0020_0000, // AirCatch
        0x00C0_0000, // AirCatchHit
    ]
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
        SpecialSlot::Side => special_s::enter::<C>(f, airborne, assets),
        _ => unimplemented!(
            "ftData_Special{slot:?}[{:?}] (airborne: {airborne}): character special entry",
            f.core.kind
        ),
    }
}

/// ftCo_800C3B10's Link/Young Link arm once the common tests passed:
/// ftCo_800C3BE8 enters ftLk_MS_AirCatch in the air (on the ground it
/// only spends the tether), which throws the hookshot.
pub fn air_tether<C: LinkFamily>(f: &mut Fighter, _assets: &FighterAssets) -> bool {
    if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        unimplemented!("ftCo_800C3BE8: ftLk_MS_AirCatch and the hookshot (itlinkhookshot.c)");
    }
    true
}

/// The standing and dash grabs (ftCo_0D8E.c fn_800D8EC8 / fn_800D9228)
/// throw the hookshot article and grab through it.
pub fn catch_variant() {
    unimplemented!("ftCo_0D8E.c:43-173: the Links' hookshot grab (itlinkhookshot.c)");
}

/// ftLk_800EAF58 (800EAF58), take_dmg_cb and death2_cb while armed: the
/// boomerang goes (the hookshot, arrow, bow and milk are not ported).
pub fn take_damage<C: LinkFamily>(f: &mut Fighter) {
    if f.character.get::<C>().specials_ref().removal_armed {
        special_s::remove_boomerang::<C>(f);
    }
}

/// ftCo_800D331C's death callbacks: death2_cb (ftLk_800EAF58, while armed)
/// and death3_cb (ftLk_800EAF38, installed with the boomerang and never
/// removed): the boomerang goes either way.
pub fn death<C: LinkFamily>(f: &mut Fighter) {
    special_s::remove_boomerang::<C>(f);
}

/// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
/// callbacks go.
pub fn motion_changed(specials: &mut Specials) {
    specials.removal_armed = false;
}

/// An owned article reached back (it_802A07B4 / Logic18_Destroyed's
/// ftLk_SpecialS_RemoveBoomerang0).
pub fn article_destroyed<C: LinkFamily>(f: &mut Fighter, kind: melee_types::ItemKind) {
    use melee_types::ItemKind as K;
    if matches!(kind, K::LinkBoomerang | K::CLinkBoomerang) {
        special_s::forget_boomerang::<C>(f);
    }
}

/// An owned article's request (the boomerang's catch).
pub fn article_request<C: LinkFamily>(
    f: &mut Fighter,
    assets: &FighterAssets,
    kind: melee_types::ItemKind,
    request: melee_it::OwnerRequest,
) -> Option<u8> {
    use melee_types::ItemKind as K;
    match (kind, request) {
        (K::LinkBoomerang | K::CLinkBoomerang, melee_it::OwnerRequest::Catch) => {
            special_s::catch::<C>(f, assets)
        }
        (K::LinkBoomerang | K::CLinkBoomerang, melee_it::OwnerRequest::Released) => {
            special_s::forget_boomerang::<C>(f);
            None
        }
        _ => unimplemented!("{kind:?} asked {request:?} of a Link"),
    }
}

/// What the Links' articles read of their owner: the boomerang homes on
/// ftLk_SpecialHi_GetPosWithAdjustedY (cur_pos raised by x28, fadds).
pub fn item_owner<C: LinkFamily>(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
    let height = f.character.get::<C>().attributes().hookshot_height;
    let position = f.physics.position;
    melee_it::ItemOwner {
        illusion: None,
        position,
        facing: f.physics.facing,
        hold_position: position,
        blaster_action: 9,
        remove_blaster: true,
        motion: f.motion_state.action.0,
        motion_flags: f.motion_flags(),
        in_hitlag: f.core.in_hitlag(),
        anchor: hsd_types::Vec3::new(position.x, position.y + height, position.z),
        articles_fired: 0,
        charge: None,
        holds_needles: false,
        stick: hsd_types::Vec2::new(f.input.current.stick.x, f.input.current.stick.y),
        steering_article: false,
        detonating_article: false,
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
    if pending == Accessory::BoomerangThrow {
        if f.core.accessory4_armed {
            special_s::throw_accessory::<C>(f, assets);
        }
        return;
    }
    if !f.run_accessory4(pending != Accessory::None) {
        return;
    }
    f.character.get_mut::<C>().specials().accessory = Accessory::None;
    match pending {
        Accessory::SpinSwirl => special_hi::swirl::<C>(f, assets),
        Accessory::None | Accessory::BoomerangThrow => unreachable!(),
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
