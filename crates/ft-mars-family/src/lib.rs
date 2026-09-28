//! Shared ftMs_ specials: Marth's code, which Roy runs as well.
//! ftFe_Init_MotionStateTable (803D2E80) equals ftMs_Init_MotionStateTable
//! (803CF420) row for row; the kind checks inside the shared callbacks
//! (`ftLib_GetKind`, `fp->kind == FTKIND_MARS`) are the family trait's
//! constants, and the attributes are the same `MarsAttributes` layout
//! (ftMs_Init_OnLoadForRoy, 80136474).
pub mod attributes;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

use attributes::MarsAttributes;
use melee_ft::fighter::{
    assets::FighterAssets, state::callbacks, ActionId, CharacterCallbacks, CharacterState, Fighter,
    MotionRow, SpecialSlot,
};

/// The kind-dependent arms of the shared ftMs_ callbacks.
pub trait MarsFamily: CharacterCallbacks {
    /// ftMs_SpecialN_801365A8 / 8013666C: the efSync id of the Shield
    /// Breaker (Flare Blade) release, grounded and aerial.
    const RELEASE_EFFECTS: [u16; 2];
    /// ftMs_SpecialNStart_Anim's ftCo_800BFFD0 color animation when the
    /// charge loop starts (`fp->kind == FTKIND_MARS ? 99 : 100`).
    const CHARGE_COLOR_ANIMATION: u8;
    /// ftMs_SpecialLw_80139140: the efSync id of the Counter's flash.
    const COUNTER_EFFECT: u16;
    /// ftMs_SpecialLwHit_Anim / SpecialAirLwHit_Anim: FTKIND_EMBLEM rewrites
    /// every enabled hitbox's damage with the countered damage each frame.
    const COUNTER_SETS_HIT_DAMAGE: bool;
    fn attributes(&self) -> &MarsAttributes;
    fn specials(&mut self) -> &mut Specials;
    fn specials_ref(&self) -> &Specials;
}

/// Fighter mv.ms and u.ms (ftMars/types.h) for the family's specials,
/// owned by each member's payload.
#[derive(Clone, Debug, Default)]
pub struct Specials {
    pub special_n: special_n::SpecialN,
    pub special_side: special_s::SpecialSide,
    pub special_lw: special_lw::SpecialLw,
    pub special_hi: special_hi::SpecialHi,
    /// Fighter +222C, ftmarsspecials.c:56-60: once-per-airtime vertical boost.
    pub side_special_boost_used: bool,
    /// mv+4 in every family special. ftMars_MotionVars has only an x0 word
    /// (types.h), so no special writes mv+4: it keeps the word the state
    /// before the special left (`None` when the port does not model it), and
    /// a landing from a special inherits it (ftCo_Landing.c:41-50).
    pub retained_scratch_word: Option<f32>,
}
impl Specials {
    /// ftMs_Init_OnDeath / ftFe_Init_OnDeath: the special scratch and +222C.
    pub fn reset(&mut self) {
        let retained_scratch_word = self.retained_scratch_word;
        *self = Self {
            retained_scratch_word,
            ..Self::default()
        };
    }
}

/// Marth's and Roy's special actions, SpecialNStart (341) through
/// SpecialAirLwHit (372).
pub const SPECIAL_COUNT: usize = 32;
const FIRST_SPECIAL: u16 = 341;
const SPECIALS: std::ops::RangeInclusive<u16> = FIRST_SPECIAL..=FIRST_SPECIAL + 31;

/// Called by each special's entry before it changes motion state: capture
/// the mv+4 word the current state leaves behind. From one special into
/// another the word is already the retained one, so it carries through.
pub(crate) fn retain_scratch_word<C: MarsFamily>(f: &mut Fighter) {
    let word = f.inherited_scratch_word();
    f.character.get_mut::<C>().specials().retained_scratch_word = word;
}

/// mv+4 while a family special is current (see `Specials::retained_scratch_word`).
pub fn retained_scratch_word<C: MarsFamily>(
    state: &CharacterState,
    action: ActionId,
) -> Option<f32> {
    if !SPECIALS.contains(&action.0) {
        return None;
    }
    Some(
        state
            .get::<C>()
            .specials_ref()
            .retained_scratch_word
            .unwrap_or_else(|| {
                unimplemented!("ftMs specials: mv+4 inherited from an unmodelled scratch word")
            }),
    )
}

/// ftData_SpecialN/S/Hi/Lw[kind] and the aerial tables.
pub fn enter_special<C: MarsFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    air: bool,
    assets: &FighterAssets,
) {
    match slot {
        SpecialSlot::Neutral => special_n::enter::<C>(f, air, assets),
        SpecialSlot::Side => special_s::enter::<C>(f, air, assets),
        SpecialSlot::Up => special_hi::enter::<C>(f, air, assets),
        SpecialSlot::Down => special_lw::enter::<C>(f, air, assets),
    }
}

/// A family motion row: rows are contiguous from ftCo_MS_Count, and each
/// plays the submotion of the same index (ftMs_SM_*).
const fn row(
    index: usize,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    const FIRST_SUBMOTION: i32 = 295; // ftCo_SM_Count
    MotionRow {
        action: ActionId(FIRST_SPECIAL + index as u16),
        id: melee_types::CommonMotionState::None,
        animation: FIRST_SUBMOTION + index as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ftMs_Init_MotionStateTable / ftFe_Init_MotionStateTable.
pub const fn rows<C: MarsFamily>() -> [MotionRow; SPECIAL_COUNT] {
    let mut rows = [melee_ft::fighter::state::unimplemented_row(); SPECIAL_COUNT];
    let mut i = 0;
    // Shield Breaker: Start, Loop, End0, End1, then the aerial four.
    while i < 8 {
        rows[i] = row(
            i,
            match i % 4 {
                0 => special_n::start::<C>,
                1 => special_n::hold::<C>,
                _ => special_n::end::<C>,
            },
            if i % 4 == 1 {
                special_n::input::<C>
            } else {
                no_input
            },
            if i >= 4 {
                special_n::air_physics::<C>
            } else if i == 0 {
                special_n::startup_physics::<C>
            } else {
                callbacks::physics::guard_on
            },
            if i >= 4 {
                special_n::air_collision
            } else {
                special_n::collision
            },
        );
        i += 1;
    }
    // Dancing Blade: S1, S2Hi/Lw, S3Hi/S/Lw, S4Hi/S/Lw, then the aerial nine.
    while i < 26 {
        rows[i] = row(
            i,
            special_s::anim,
            if (i - 8) % 9 < 6 {
                special_s::input
            } else {
                no_input
            },
            if i >= 17 {
                special_s::air_physics::<C>
            } else if i < 11 {
                callbacks::physics::guard_on
            } else {
                callbacks::physics::jab
            },
            if i >= 17 {
                special_s::air_collision::<C>
            } else {
                special_s::collision
            },
        );
        i += 1;
    }
    // Dolphin Slash (Blazer), grounded and aerial.
    while i < 28 {
        rows[i] = row(
            i,
            special_hi::anim::<C>,
            special_hi::input::<C>,
            special_hi::physics::<C>,
            special_hi::collision::<C>,
        );
        i += 1;
    }
    // Counter: stance and retaliation, grounded then aerial.
    while i < 32 {
        rows[i] = row(
            i,
            if i % 2 == 0 {
                special_lw::anim::<C>
            } else {
                special_lw::hit_anim::<C>
            },
            no_input,
            if i >= 30 {
                special_lw::air_physics::<C>
            } else if i == 28 {
                special_lw::physics
            } else {
                callbacks::physics::guard_on
            },
            if i >= 30 {
                special_lw::air_collision::<C>
            } else {
                special_lw::collision::<C>
            },
        );
        i += 1;
    }
    rows
}

/// ftColl_8007ABD0 (8007ABD0) on every enabled hitbox: the integer
/// knockback damage, then the damage staled by ft_80089228.
pub(crate) fn set_enabled_hit_damage(f: &mut Fighter, damage: u32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: charged family special"
    );
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    for hit in f.commands.hitboxes.iter_mut().flatten() {
        if hit.phase == melee_coll::hitbox::CapsulePhase::Enabled {
            hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
            hit.descriptor.damage = staled;
        }
    }
}

fn no_input(_: &mut Fighter, _: melee_ft::fighter::state::InputPhase<'_>) {}

/// ftMs_Init_MotionStateTable's FtMoveId values, including aerial counterparts.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; SPECIAL_COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_COUNT];
    let mut i = 0;
    while i < SPECIAL_COUNT {
        moves[i] = Some(if i < 8 {
            M::SpecialNeutral
        } else if i < 26 {
            M::SpecialSide
        } else if i < 28 {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
}
