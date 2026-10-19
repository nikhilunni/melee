//! The copied Falcon Punch and Warlock Punch, ftkirbyspecialcaptain.c:
//! Captain Falcon's states (`ft_captain_family::special_n`) over Kirby's
//! rows 433..434 (Captain Falcon's ability) and 528..529 (Ganondorf's),
//! with Kirby's attributes and effects. No articles, no scratch and no
//! put-away arm (ftKb_SpecialN_800F19AC / 800F1A8C have no case for
//! either kind).
use crate::init::Kirby;
use ft_captain_family::{
    attributes::FalconPunchAttributes,
    special_n::{self, FalconPunch, PunchEffect},
};
use melee_ft::fighter::{assets::FighterAssets, state::callbacks, ActionId, Fighter, MotionRow};
use melee_types::{CommonMotionState, FighterKind};

/// Kirby with Captain Falcon's or Ganondorf's ability: one marker, since
/// ftkirbyspecialcaptain.c's callbacks pick the rows and the effect by the
/// hat's kind themselves.
pub struct KirbyPunch;

/// ftKb_MS_CaSpecialN / CaSpecialAirN (433, 434), ftKb_SM_CaSpecialN (368).
const CAPTAIN_ROWS: [ActionId; 2] = [ActionId(433), ActionId(434)];
const CAPTAIN_ANIMATION: i32 = 368;
/// ftKb_MS_GnSpecialN / GnSpecialAirN (528, 529), ftKb_SM_GnSpecialN (463).
const GANON_ROWS: [ActionId; 2] = [ActionId(528), ActionId(529)];
const GANON_ANIMATION: i32 = 463;

/// `fp->parts[0]` and `fp->parts[44]` (ftKb_CaSpecialN_Phys, 800F9C58).
const PUNCH_BONES: [usize; 2] = [0, 44];

impl FalconPunch for KirbyPunch {
    /// ftKb_CaSpecialN_Anim / ftKb_CaSpecialAirN_Anim push no wind.
    const WIND: bool = false;
    /// ftKb_SpecialNCa_800F99BC and the two Coll callbacks: Ganondorf's
    /// rows for any hat but Captain Falcon's.
    fn rows(f: &Fighter) -> [ActionId; 2] {
        if f.character.get::<Kirby>().copied == FighterKind::Captain {
            CAPTAIN_ROWS
        } else {
            GANON_ROWS
        }
    }
    /// ftKb_DatAttrs +1F8 (`specialn_ca_*`) for both kinds.
    fn attributes(f: &Fighter) -> &FalconPunchAttributes {
        &f.character.get::<Kirby>().attributes.falcon_punch
    }
    /// ftKb_CaSpecialN_Phys's switch on the hat's kind: efSync 1195 (0x4AB,
    /// EfKbCa.dat's models 0x9471 then 0x9470) or 1203 (0x4B3, EfKbGn.dat's
    /// 0xB799 then 0xB798).
    fn effect(f: &Fighter) -> Option<PunchEffect> {
        let id = match f.character.get::<Kirby>().copied {
            FighterKind::Captain => 0x4AB,
            FighterKind::Ganon => 0x4B3,
            _ => return None,
        };
        Some(PunchEffect {
            id,
            bones: PUNCH_BONES,
        })
    }
}

/// ftKb_SpecialNCa_800F99BC / 800F9A54 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    special_n::enter::<KirbyPunch>(f, airborne, assets);
}

const fn row(action: ActionId, animation: i32, airborne: bool) -> MotionRow {
    use special_n as n;
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation,
        anim: if airborne {
            n::air_anim::<KirbyPunch>
        } else {
            n::ground_anim::<KirbyPunch>
        },
        iasa: if airborne {
            n::air_input::<KirbyPunch>
        } else {
            n::ground_input
        },
        physics: if airborne {
            n::air_physics::<KirbyPunch>
        } else {
            n::ground_physics::<KirbyPunch>
        },
        collision: if airborne {
            n::air_collision::<KirbyPunch>
        } else {
            n::ground_collision::<KirbyPunch>
        },
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// Rows 433, 434 (ftKb_MS_CaSpecial*N) and 528, 529 (ftKb_MS_GnSpecial*N):
/// the same callbacks, ftKb_CaSpecial*N_*.
pub const fn rows() -> [MotionRow; 4] {
    [
        row(CAPTAIN_ROWS[0], CAPTAIN_ANIMATION, false),
        row(CAPTAIN_ROWS[1], CAPTAIN_ANIMATION + 1, true),
        row(GANON_ROWS[0], GANON_ANIMATION, false),
        row(GANON_ROWS[1], GANON_ANIMATION + 1, true),
    ]
}
