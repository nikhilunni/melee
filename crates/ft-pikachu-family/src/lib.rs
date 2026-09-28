//! Shared ftPk_ specials: Pikachu's code, which Pichu runs as well
//! (ftPichu's motion table points at these callbacks). Character
//! differences are the family trait's constants and typed accessors.
pub mod attributes;
mod common;
pub mod special_hi;
pub mod special_n;
pub mod special_s;

use attributes::PikachuAttributes;
use melee_ft::fighter::{
    assets::FighterAssets, state::callbacks, ActionId, CharacterCallbacks, CharacterState, Fighter,
    MotionRow, SpecialSlot,
};
use melee_types::CommonMotionState;

/// The kind checks inside the shared callbacks (ftLib_GetKind /
/// fp->kind == FTKIND_PICHU).
pub trait PikachuFamily: CharacterCallbacks {
    /// ftPk_SpecialHiStart1_Anim: every kind but Pichu spawns effect 1012
    /// along the zip.
    const QUICK_ATTACK_TRAIL: bool;
    /// ftPk_SpecialN_Anim's ft_PlaySFX id for the jolt.
    const JOLT_SOUND: u32;
    fn attributes(&self) -> &PikachuAttributes;
    fn specials(&mut self) -> &mut Specials;
    fn specials_ref(&self) -> &Specials;
}

/// ftPk_MS_* (forward.h): the family's motion states, contiguous from
/// ftCo_MS_Count.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyState {
    SpecialN = 341,
    SpecialAirN,
    SpecialSStart,
    SpecialSHold,
    SpecialS1,
    SpecialSEnd,
    SpecialS0,
    SpecialAirSStart,
    SpecialAirSHold,
    SpecialAirS1,
    SpecialAirSEnd,
    SpecialAirS0,
    SpecialHiStart0,
    SpecialHiStart1,
    SpecialHiEnd,
    SpecialAirHiStart0,
    SpecialAirHiStart1,
    SpecialAirHiEnd,
    SpecialLwStart,
    SpecialLwLoop0,
    SpecialLwLoop1,
    SpecialLwEnd,
    SpecialAirLwStart,
    SpecialAirLwLoop0,
    SpecialAirLwLoop1,
    SpecialAirLwEnd,
}
impl FamilyState {
    pub const COUNT: usize = 26;
    pub const fn action(self) -> ActionId {
        ActionId(self as u16)
    }
    /// ftPk_Init_MotionStateTable's submotion column: the table order,
    /// except that the aerial Skull Bash launch plays SpecialS1 (ftPk_SM_
    /// SpecialS1) and its hold plays ftPk_SM_SpecialS (ftpikachu.c:127-150).
    pub const fn animation(self) -> i32 {
        const FIRST: i32 = 295; // ftCo_SM_Count
        let submotion = match self {
            Self::SpecialN => 0,
            Self::SpecialAirN => 1,
            Self::SpecialSStart => 2,
            Self::SpecialSHold => 3,
            Self::SpecialS0 => 4,
            Self::SpecialS1 | Self::SpecialAirS1 => 5,
            Self::SpecialSEnd => 6,
            Self::SpecialAirSStart => 7,
            Self::SpecialAirSHold => 8,
            Self::SpecialAirS0 => 9,
            Self::SpecialAirSEnd => 10,
            Self::SpecialHiStart0 => 11,
            Self::SpecialHiStart1 => 12,
            Self::SpecialHiEnd => 13,
            Self::SpecialAirHiStart0 => 14,
            Self::SpecialAirHiStart1 => 15,
            Self::SpecialAirHiEnd => 16,
            Self::SpecialLwStart => 17,
            Self::SpecialLwLoop0 => 18,
            Self::SpecialLwLoop1 => 19,
            Self::SpecialLwEnd => 20,
            Self::SpecialAirLwStart => 21,
            Self::SpecialAirLwLoop0 => 22,
            Self::SpecialAirLwLoop1 => 23,
            Self::SpecialAirLwEnd => 24,
        };
        FIRST + submotion
    }
    fn from_action(action: ActionId) -> Option<Self> {
        let index = action.0.checked_sub(Self::SpecialN as u16)?;
        (usize::from(index) < Self::COUNT).then(|| {
            // SAFETY-free: the discriminants are contiguous from 341.
            ALL[usize::from(index)]
        })
    }
}
const ALL: [FamilyState; FamilyState::COUNT] = {
    use FamilyState as S;
    [
        S::SpecialN,
        S::SpecialAirN,
        S::SpecialSStart,
        S::SpecialSHold,
        S::SpecialS1,
        S::SpecialSEnd,
        S::SpecialS0,
        S::SpecialAirSStart,
        S::SpecialAirSHold,
        S::SpecialAirS1,
        S::SpecialAirSEnd,
        S::SpecialAirS0,
        S::SpecialHiStart0,
        S::SpecialHiStart1,
        S::SpecialHiEnd,
        S::SpecialAirHiStart0,
        S::SpecialAirHiStart1,
        S::SpecialAirHiEnd,
        S::SpecialLwStart,
        S::SpecialLwLoop0,
        S::SpecialLwLoop1,
        S::SpecialLwEnd,
        S::SpecialAirLwStart,
        S::SpecialAirLwLoop0,
        S::SpecialAirLwLoop1,
        S::SpecialAirLwEnd,
    ]
};

/// Fighter mv.pk (ftPikachu/types.h) and the callbacks the specials
/// install, owned by each family member's payload.
#[derive(Clone, Debug, Default)]
pub struct Specials {
    /// mv.pk.unk3.x0: Skull Bash's charge frames.
    pub skull_bash_charge: i32,
    pub quick_attack: special_hi::QuickAttack,
    /// The one-shot accessory4 a special armed.
    pub accessory: Accessory,
    /// mv+4 as the state before the special left it; Thunder Jolt and
    /// Skull Bash never write it, Quick Attack not before its first zip.
    pub retained_word: Option<f32>,
}

/// accessory4_cb while a family special owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftPk_SpecialN_SpawnEffect0: Skull Bash's charge sparks (1214).
    ChargeSparks,
    /// ftPk_SpecialN_SpawnEffect1: Skull Bash's launch sparks (1215).
    LaunchSparks,
}

/// A family motion row.
pub(crate) const fn row(
    state: FamilyState,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
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

/// ftPk_Init_MotionStateTable (ftpikachu.c:19-282). Thunder stays
/// unported rows.
pub const fn rows<C: PikachuFamily>() -> [MotionRow; FamilyState::COUNT] {
    let mut rows = [melee_ft::fighter::state::unimplemented_row(); FamilyState::COUNT];
    let mut i = 0;
    while i < FamilyState::COUNT {
        rows[i].action = ALL[i].action();
        rows[i].animation = ALL[i].animation();
        i += 1;
    }
    let neutral = special_n::rows::<C>();
    i = 0;
    while i < neutral.len() {
        rows[(neutral[i].action.0 - FamilyState::SpecialN as u16) as usize] = neutral[i];
        i += 1;
    }
    let side = special_s::rows::<C>();
    i = 0;
    while i < side.len() {
        rows[(side[i].action.0 - FamilyState::SpecialN as u16) as usize] = side[i];
        i += 1;
    }
    let up = special_hi::rows::<C>();
    i = 0;
    while i < up.len() {
        rows[(up[i].action.0 - FamilyState::SpecialN as u16) as usize] = up[i];
        i += 1;
    }
    rows
}

/// ftPk_Init_MotionStateTable's move ids: each row its special's.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; FamilyState::COUNT] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; FamilyState::COUNT];
    let mut i = 0;
    while i < FamilyState::COUNT {
        moves[i] = Some(match i {
            0..=1 => M::SpecialNeutral,
            2..=11 => M::SpecialSide,
            12..=17 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
}

/// ftData_SpecialN/S/Hi/Lw[kind] and the aerial tables.
pub fn enter_special<C: PikachuFamily>(
    f: &mut Fighter,
    slot: SpecialSlot,
    airborne: bool,
    assets: &FighterAssets,
) {
    // Each entry leaves mv+4 as the state before it had it.
    let retained = f.inherited_scratch_word();
    f.character.get_mut::<C>().specials().retained_word = retained;
    match slot {
        SpecialSlot::Side => special_s::enter::<C>(f, airborne, assets),
        SpecialSlot::Up => special_hi::enter::<C>(f, airborne, assets),
        SpecialSlot::Neutral => special_n::enter::<C>(f, airborne, assets),
        SpecialSlot::Down => unimplemented!(
            "ftPk_SpecialLw_Enter / ftPk_SpecialAirLw_Enter (airborne: {airborne}): Thunder"
        ),
    }
}

/// Fighter_8006C80C: the special's one-shot accessory4.
pub fn accessory<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let pending = f.character.get::<C>().specials_ref().accessory;
    if !f.run_accessory4(pending != Accessory::None) {
        return;
    }
    f.character.get_mut::<C>().specials().accessory = Accessory::None;
    match pending {
        Accessory::ChargeSparks | Accessory::LaunchSparks => {
            special_s::sparks(f, assets, pending == Accessory::LaunchSparks)
        }
        Accessory::None => unreachable!(),
    }
}

/// mv+4 while `action`, one of the family's rows, is current.
pub fn retained_scratch_word<C: PikachuFamily>(
    state: &CharacterState,
    action: ActionId,
) -> Option<f32> {
    let family = FamilyState::from_action(action)?;
    let specials = state.get::<C>().specials_ref();
    use FamilyState as S;
    let zip = match family {
        S::SpecialHiStart0
        | S::SpecialHiStart1
        | S::SpecialHiEnd
        | S::SpecialAirHiStart0
        | S::SpecialAirHiStart1
        | S::SpecialAirHiEnd => specials.quick_attack.zip_frames,
        S::SpecialLwStart
        | S::SpecialLwLoop0
        | S::SpecialLwLoop1
        | S::SpecialLwEnd
        | S::SpecialAirLwStart
        | S::SpecialAirLwLoop0
        | S::SpecialAirLwLoop1
        | S::SpecialAirLwEnd => {
            unimplemented!("ftPk_SpecialLw: mv.pk.speciallw.x4 as the retained scratch word")
        }
        _ => None,
    };
    if let Some(frames) = zip {
        // mv.pk.specialhi.x4, an s32 read back as the float word.
        return Some(f32::from_bits(frames as u32));
    }
    Some(specials.retained_word.unwrap_or_else(|| {
        unimplemented!("ftPk specials: mv+4 inherited from an unmodelled scratch word")
    }))
}

/// Motion flags shared by the family's ground/air counterparts.
pub(crate) mod flags {
    /// ftCommon_GroundAirColl_MF (ftCommon/forward.h:9-12).
    pub const GROUND_AIR: u32 = 0x0C4C_5080;
    pub const KEEP_GFX: u32 = 1 << 1;
    pub const KEEP_COL_ANIM_HIT_STATUS: u32 = 1 << 2;
    pub const SKIP_HIT: u32 = 1 << 3;
    pub const KEEP_SFX: u32 = 1 << 9;
}

pub use common::no_input;
