//! Shared ftCa_ specials: Captain Falcon's code, which Ganondorf runs as
//! well (ftGn_Init_MotionStateTable points at the ftCa_ callbacks and
//! ftdata.c's special entries are ftCa_*_Enter for both kinds). The kind
//! switches inside the shared callbacks (ftLib_GetKind) become the family
//! trait's effect table; everything else is common.
pub mod attributes;
pub mod special_hi;
pub mod special_hi_catch;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use attributes::CaptainAttributes;
use melee_ft::fighter::{
    assets::FighterAssets,
    state::{self, callbacks},
    ActionId, CharacterCallbacks, CharacterState, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FtPart;

/// A member of the Captain Falcon family: its attributes (the shared
/// ftCaptain_DatAttrs layout), its scratch and its effect ids.
pub trait CaptainFamily: CharacterCallbacks {
    /// The `switch (ftLib_GetKind(gobj))` arms of the shared specials.
    const EFFECTS: FamilyEffects;
    fn attributes(&self) -> &CaptainAttributes;
    fn specials(&mut self) -> &mut Specials;
    fn specials_ref(&self) -> &Specials;
}

/// The kind-dependent effects of the shared specials.
#[derive(Clone, Copy, Debug)]
pub struct FamilyEffects {
    /// doPhys (ftcaptainspecialn.c:152-163): the punch's efSync id and the
    /// two fighter joints (`fp->parts[FtPart_TopN]`, `fp->parts[n]`) its
    /// pair of models attach to.
    pub punch: u16,
    pub punch_bones: [usize; 2],
    /// ftCaptain_SpecialN_CreateWindEffect: only Ganondorf's punch pushes
    /// wind (lb_800119DC) while it winds up.
    pub punch_wind: bool,
    /// ftCa_SpecialS_Enter / setupAirStart: the startup's efSync id and the
    /// part it attaches to.
    pub boost_start: u16,
    pub boost_start_part: FtPart,
    /// ftCa_SpecialS_Anim / ftCa_SpecialAirS_Anim: the lunge's efSync ids.
    pub ground_lunge: u16,
    pub air_lunge: u16,
    /// ftCa_SpecialHi_800E3EAC: the kick's efAsync id (kind 3).
    pub kick_flame: u16,
}

/// The family's per-fighter state: the FighterVars words and the mv.ca
/// scratch the specials own.
#[derive(Clone, Debug, Default)]
pub struct Specials {
    /// Fighter +222C, u.ca.during_specials_start: the Raptor Boost / Gerudo
    /// Dragon startup effect is active.
    pub start_effect_active: bool,
    /// Fighter +2230, u.ca.during_specials: the lunge effect is active.
    pub lunge_effect_active: bool,
    /// mv.ca.speciallw: the grounded kick's hit slowdown.
    pub kick: special_lw::FalconKick,
    /// mv.ca.specials: the aerial boost's vertical velocity.
    pub boost: special_s::RaptorBoost,
    /// mv.ca.specialhi: the dive's carried velocity and flags.
    pub dive: special_hi::FalconDive,
}
impl Specials {
    /// ftCaptain_FighterVars (types.h): the saved startup and lunge flags.
    pub fn restore_saved(&mut self, raw: &[u8]) {
        self.start_effect_active = u32::from_be_bytes(raw[0x222C..0x2230].try_into().unwrap()) != 0;
        self.lunge_effect_active = u32::from_be_bytes(raw[0x2230..0x2234].try_into().unwrap()) != 0;
    }
    /// ftCa_Init_OnDeath / ftGn_Init_OnDeath: +2230, then +222C.
    pub fn reset(&mut self) {
        self.lunge_effect_active = false;
        self.start_effect_active = false;
    }
}

pub(crate) fn family<C: CaptainFamily>(f: &mut Fighter) -> &mut C {
    f.character.get_mut::<C>()
}
pub(crate) fn attributes<C: CaptainFamily>(f: &Fighter) -> &CaptainAttributes {
    f.character.get::<C>().attributes()
}

/// ftCa_Init_MotionStateTable / ftGn_Init_MotionStateTable rows 341..363
/// (ftCa_MS_SwordSwing4 through ftCa_MS_SpecialHiThrow1), animations from
/// ftCa_SM_SwordSwing4 (295).
pub const ROW_COUNT: usize = 23;
const FIRST_ACTION: u16 = 341;
const FIRST_ANIMATION: i32 = 295;

/// ftCa_SM_* for a motion: the table's order, except that the animation
/// enum lists SpecialLwEndAir before SpecialAirLwEndAir while the motion
/// enum lists them the other way round (ftCaptain/forward.h:59-60, 87-88).
const fn animation(action: ActionId) -> i32 {
    let index = match action.0 - FIRST_ACTION {
        20 => 21,
        21 => 20,
        other => other,
    };
    FIRST_ANIMATION + index as i32
}

/// A ported special row.
const fn row(
    action: ActionId,
    anim: state::AnimFn,
    iasa: state::InputFn,
    physics: state::PhysicsFn,
    collision: state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: melee_types::CommonMotionState::None,
        animation: animation(action),
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

const fn place(rows: &mut [MotionRow; ROW_COUNT], row: MotionRow) {
    rows[(row.action.0 - FIRST_ACTION) as usize] = row;
}

/// Raptor Boost's IASA callbacks are empty; the Falcon Kick rows have none
/// (ftcaptain.c:199-273).
fn no_input(_: &mut Fighter, _: state::InputPhase<'_>) {}

/// Character rows are contiguous from ftCo_MS_Count. The item swings stay
/// explicit boundaries.
pub const fn special_rows<C: CaptainFamily>() -> [MotionRow; ROW_COUNT] {
    let mut rows = [state::unimplemented_row(); ROW_COUNT];
    let mut i = 0;
    while i < ROW_COUNT {
        rows[i].action = ActionId(FIRST_ACTION + i as u16);
        rows[i].animation = animation(rows[i].action);
        i += 1;
    }
    use special_hi as hi;
    use special_lw as lw;
    place(
        &mut rows,
        row(
            hi::GROUND,
            hi::anim::<C>,
            hi::input::<C>,
            hi::physics::<C>,
            hi::collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            hi::AIR,
            hi::anim::<C>,
            hi::input::<C>,
            hi::physics::<C>,
            hi::collision::<C>,
        ),
    );
    use special_hi_catch as catch;
    place(
        &mut rows,
        row(
            hi::CATCH,
            catch::catch_anim::<C>,
            catch::no_input,
            catch::catch_physics,
            catch::catch_collision,
        ),
    );
    place(
        &mut rows,
        row(
            hi::THROW,
            catch::throw_anim::<C>,
            catch::no_input,
            catch::throw_physics::<C>,
            catch::throw_collision::<C>,
        ),
    );
    use special_s as s;
    let no_iasa = no_input as state::InputFn;
    place(
        &mut rows,
        row(
            s::GROUND_START,
            s::ground_start_anim,
            no_iasa,
            s::ground_physics,
            s::ground_start_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            s::GROUND,
            s::ground_anim::<C>,
            no_iasa,
            s::ground_physics,
            s::ground_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            s::AIR_START,
            s::air_start_anim::<C>,
            no_iasa,
            s::air_start_physics::<C>,
            s::air_start_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            s::AIR,
            s::air_anim::<C>,
            no_iasa,
            s::air_physics::<C>,
            s::air_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            special_n::GROUND,
            special_n::ground_anim::<C>,
            special_n::ground_input,
            special_n::ground_physics::<C>,
            special_n::ground_collision,
        ),
    );
    place(
        &mut rows,
        row(
            special_n::AIR,
            special_n::air_anim::<C>,
            special_n::air_input::<C>,
            special_n::air_physics::<C>,
            special_n::air_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND,
            lw::ground_anim::<C>,
            no_input,
            lw::ground_physics::<C>,
            lw::ground_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND_END,
            lw::end_anim,
            no_input,
            lw::ground_end_physics::<C>,
            lw::end_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR,
            lw::air_anim,
            no_input,
            lw::air_physics::<C>,
            lw::air_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR_LANDING,
            lw::landing_anim,
            no_input,
            lw::landing_physics::<C>,
            lw::landing_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::AIR_END,
            lw::fall_anim,
            no_input,
            lw::air_end_physics,
            lw::air_collision::<C>,
        ),
    );
    place(
        &mut rows,
        row(
            lw::GROUND_END_AIR,
            lw::end_anim,
            no_input,
            lw::ground_end_air_physics,
            lw::end_collision,
        ),
    );
    place(
        &mut rows,
        row(
            lw::REBOUND,
            lw::fall_anim,
            no_input,
            lw::rebound_physics,
            callbacks::collision::air_catch_hit,
        ),
    );
    rows
}

/// The motion tables' FtMoveId values. The item swings' move ids
/// (FtMoveId_SwordSwing4..LipstickSwing4) have no StaleMove yet.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; ROW_COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; ROW_COUNT];
    let mut i = 6;
    while i < ROW_COUNT {
        moves[i] = Some(match i {
            6..=7 => M::SpecialNeutral,
            8..=11 => M::SpecialSide,
            12..=15 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
}

/// ftData_SpecialN/S/Hi/Lw[kind] and the aerial tables: ftCa_*_Enter for
/// both kinds.
pub fn enter_special<C: CaptainFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    air: bool,
    a: &FighterAssets,
) {
    match slot {
        SpecialSlot::Neutral => special_n::enter(f, air, a),
        SpecialSlot::Side => special_s::enter::<C>(f, air, a),
        SpecialSlot::Up => special_hi::enter::<C>(f, air, a),
        SpecialSlot::Down => special_lw::enter::<C>(f, air, a),
    }
}

/// mv+4 while `action` is current, for the family's rows that retain it.
pub fn retained_scratch_word<C: CaptainFamily>(
    state: &CharacterState,
    action: ActionId,
) -> Option<f32> {
    let specials = state.get::<C>().specials_ref();
    special_s::retained_scratch_word(specials, action)
        .or_else(|| special_hi::retained_scratch_word(specials, action))
}
