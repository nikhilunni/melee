//! Shared ftLk_ states: Link's code, which Young Link runs as well
//! (ftCl_Init_MotionStateTable points at the same callbacks, except its
//! side taunt rows). Character differences are the family trait's
//! constants and typed accessors.
mod air_catch;
pub mod attack_air;
mod attack_s42;
pub mod attributes;
mod common;
pub mod hookshot;
pub mod hylian;
pub mod milk;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
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
    /// x5F4_arr[2].prev: model group 2's selection (0: the shield on the arm).
    fn shield_model_group(&self) -> i32;
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
/// callbacks the states install. The hookshot's chain is allocated with the
/// fighter and reused by every throw.
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
    /// fp->u.lk.xC: the hookshot while it is out.
    pub hookshot: Option<hookshot::Hookshot>,
    /// The hookshot's links (its ItemLink list).
    pub chain: it_link::hookshot::chain::Chain,
    /// mv+0 of the grabs and the aerial hookshot: frames since entry.
    pub hookshot_timer: f32,
    /// The down aerial's bounce (ftLk_AttackAir_Enter's callbacks).
    pub down_air: attack_air::DownAir,
    /// The Hylian shield's volume while standing or crouching.
    pub hylian: hylian::HylianShield,
    /// The bow special's scratch and articles.
    pub bow: special_n::Bow,
    /// u.lk.x18: Young Link's taunt milk is out.
    pub milk_out: bool,
}
impl Specials {
    /// ftLk_Init_OnDeath's clears, keeping the chain's allocation.
    pub fn reset(&mut self) {
        self.vars = FighterVars::default();
        self.accessory = Accessory::None;
        self.retained_word = None;
        self.removal_armed = false;
        self.hookshot = None;
        self.hookshot_timer = 0.0;
        self.down_air.armed = false;
        self.down_air.frame_start = 0.0;
        self.down_air.hits.fill(None);
        self.hylian = hylian::HylianShield::default();
        self.bow = special_n::Bow::default();
        self.milk_out = false;
    }
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
    /// spawnBomb (800EB7C8), every frame of the pull while installed.
    PullBomb,
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
    rows[FamilyState::AirCatch.index()] = air_catch::motion_row::<C>();
    let bomb = special_lw::rows::<C>();
    rows[FamilyState::SpecialLw.index()] = bomb[0];
    rows[FamilyState::SpecialAirLw.index()] = bomb[1];
    let bow = special_n::rows::<C>();
    let mut i = 0;
    while i < bow.len() {
        rows[FamilyState::SpecialNStart.index() + i] = bow[i];
        i += 1;
    }
    rows
}

/// ftCl_Init_MotionStateTable: Link's rows with the side taunts' milk.
pub const fn young_rows<C: LinkFamily>() -> [MotionRow; FamilyState::COUNT] {
    let mut rows = rows::<C>();
    let taunt = milk::rows::<C>();
    rows[FamilyState::AppealSR.index()] = taunt[0];
    rows[FamilyState::AppealSL.index()] = taunt[1];
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
        SpecialSlot::Neutral => special_n::enter::<C>(f, airborne, assets),
        SpecialSlot::Down => special_lw::enter::<C>(f, airborne, assets),
    }
}

/// ftCo_800C3B10's Link/Young Link arm once the common tests passed: no
/// hookshot out (accessory2, death1 and accessory3 unset), then
/// ftCo_800C3BE8, which enters ftLk_MS_AirCatch in the air (on the ground
/// it only spends the tether).
pub fn air_tether<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) -> bool {
    if f.character.get::<C>().specials_ref().hookshot.is_some() {
        return false;
    }
    if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        air_catch::enter::<C>(f, assets).expect("AirCatch assets");
    }
    true
}

/// ftCommon_8007DB58's callbacks: ftLk_800EAF58 (take_dmg_cb while armed:
/// the boomerang and the hookshot go; the arrow, bow and milk are not
/// ported), then death1_cb (the hookshot's it_802A7AAC).
pub fn take_damage<C: LinkFamily>(f: &mut Fighter) {
    if f.character.get::<C>().specials_ref().removal_armed {
        special_s::remove_boomerang::<C>(f);
        // ftCo_800D94D8.
        hookshot::remove::<C>(f);
        // ftLk_SpecialN_ProcessFv10 / ProcessFv14.
        special_n::remove_articles::<C>(f);
        // ftCl_Init_80149268.
        milk::put_away::<C>(f);
    }
    // ftCommon_8007DB58 then runs death1_cb, it_802A7AAC while the
    // hookshot is out.
    hookshot::remove::<C>(f);
}

/// ftCo_800D331C's death callbacks: death2_cb (ftLk_800EAF58, while armed)
/// and death3_cb (ftLk_800EAF38, installed with the boomerang and never
/// removed): the boomerang goes either way.
pub fn death<C: LinkFamily>(f: &mut Fighter) {
    // death1_cb = it_802A7AAC while the hookshot is out.
    hookshot::remove::<C>(f);
    special_s::remove_boomerang::<C>(f);
    // death2_cb's ftCl_Init_80149268 while armed (the milk's).
    if f.character.get::<C>().specials_ref().removal_armed {
        milk::put_away::<C>(f);
    }
}

/// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
/// callbacks go.
pub fn motion_changed(specials: &mut Specials) {
    specials.removal_armed = false;
    specials.down_air.armed = false;
    specials.hylian.raised = false;
}

/// An owned article reached back (it_802A07B4 / Logic18_Destroyed's
/// ftLk_SpecialS_RemoveBoomerang0).
pub fn article_destroyed<C: LinkFamily>(f: &mut Fighter, kind: melee_types::ItemKind) {
    use melee_types::ItemKind as K;
    match kind {
        K::LinkBoomerang | K::CLinkBoomerang => special_s::forget_boomerang::<C>(f),
        // The thrower removed it (it_802A2B10) and already let go.
        K::LinkHShot | K::CLinkHShot => {}
        // itCLinkMilk_NotifyParent -> ftCl_Init_801492C4: u.lk.x18 clears.
        K::CLinkMilk => f.character.get_mut::<C>().specials().milk_out = false,
        _ => special_n::article_destroyed::<C>(f, kind),
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
        (K::LinkHShot | K::CLinkHShot, melee_it::OwnerRequest::ArticleStep(step)) => {
            hookshot::install_step::<C>(f, step);
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
        article_stage: special_n::stage::<C>(f),
        model_scale: f.player.scale * f.attributes.size.model_scaling,
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
    if pending == Accessory::PullBomb {
        // spawnBomb stays installed: it runs every frame of the pull.
        let action = f.motion_state.action;
        let pulling = action == FamilyState::SpecialLw.action()
            || action == FamilyState::SpecialAirLw.action();
        if f.core.accessory4_armed && pulling {
            special_lw::pull::<C>(f, assets);
        }
        return;
    }
    if !f.run_accessory4(pending != Accessory::None) {
        return;
    }
    f.character.get_mut::<C>().specials().accessory = Accessory::None;
    match pending {
        Accessory::SpinSwirl => special_hi::swirl::<C>(f, assets),
        Accessory::None | Accessory::BoomerangThrow | Accessory::PullBomb => unreachable!(),
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
    // The bow's rows keep the charge there (mv.lk.specialn.x0.y).
    if action.0 <= FamilyState::SpecialAirNEnd as u16 {
        return Some(state.get::<C>().specials_ref().bow.charge);
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

/// The grabs' hookshot (the FTKIND_LINK / FTKIND_CLINK arms of the common
/// catch code and the article's accessory callbacks).
pub const fn tether<C: LinkFamily>() -> melee_ft::fighter::tether::Tether {
    melee_ft::fighter::tether::Tether {
        animate: hookshot::grab_animation::<C>,
        departed: hookshot::release::<C>,
        caught: hookshot::caught::<C>,
        pull_done: hookshot::pull_done::<C>,
        released: hookshot::release::<C>,
        accessory: hookshot::accessory::<C>,
    }
}
