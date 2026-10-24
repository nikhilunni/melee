//! The copied bow, ftkirbyspeciallink.c (800FB394..800FBF0C): Link's
//! states (`ft_link_family::special_n`) over Kirby's rows 401..406 (Link's
//! ability) and 514..519 (Young Link's), with his attributes (+268 /
//! +27C, Link's layout) and It_Kind_Kirby_LinkArrow / LinkBow and their
//! Young Link kinds.
//!
//! Kirby's code reads `fp->parts` with the part enum as the index
//! (FtPart_R3rdNa for the bow, FtPart_RHandN for the arrow; the shot's
//! parts[44] and parts[39]), and draws the arrow at the zero vectors of
//! ftKb_Init_803B7548 (800FBE00..B8: all nine words are zero in the DOL)
//! instead of the hands.
use crate::init::Kirby;
use ft_link_family::{
    attributes::BowAttributes,
    special_n::{self, Bow, BowUser},
};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, FtPart, ItemKind};

/// Kirby with Link's ability.
pub struct KirbyLink;
/// Kirby with Young Link's ability.
pub struct KirbyYoungLink;

fn state(f: &Fighter) -> &Bow {
    &f.character.get::<Kirby>().bow
}
fn scratch(f: &mut Fighter) -> &mut Bow {
    &mut f.character.get_mut::<Kirby>().bow
}

/// it_802AF1A4(..., FtPart_R3rdNa, ...) at lb_8000B1CC(fp->parts[FtPart_R3rdNa])
/// (800FB5F4).
const BOW_PART: u8 = FtPart::R3rdNa as u8;
/// it_802A83E0(..., FtPart_RHandN, ...) at fp->parts[FtPart_RHandN]
/// (800FB500).
const ARROW_PART: u8 = FtPart::RHandN as u8;
/// ftKb_SpecialNLk800FB6DC: lb_8000B1CC(fp->parts[44]) and (fp->parts[39]).
const SHOT_TIP_PART: u8 = 44;
const SHOT_TAIL_PART: u8 = 39;

macro_rules! kirby_bow {
    ($type:ty, $start:expr, $animation:expr, $attributes:ident) => {
        impl BowUser for $type {
            const START: ActionId = ActionId($start);
            const START_ANIMATION: i32 = $animation;
            fn attributes(f: &Fighter) -> BowAttributes {
                f.character.get::<Kirby>().attributes.$attributes
            }
            fn scratch(f: &mut Fighter) -> &mut Bow {
                scratch(f)
            }
            fn state(f: &Fighter) -> &Bow {
                state(f)
            }
            /// ftKb_SpecialN_set_cbs.
            fn arm_callbacks(f: &mut Fighter) {
                f.character.get_mut::<Kirby>().damage_callbacks = true;
            }
            fn bow_part(_: &FighterAssets) -> u8 {
                BOW_PART
            }
            fn arrow_part(_: &FighterAssets) -> u8 {
                ARROW_PART
            }
            /// it_802A8398(x74, &vg->b, &vg->a): ftKb_Init_803B7548's zeros.
            fn aim_points(_: &mut Fighter, _: &FighterAssets) -> (Vec3, Vec3) {
                (Vec3::ZERO, Vec3::ZERO)
            }
            fn shot_points(f: &mut Fighter, _: &FighterAssets) -> (Vec3, Vec3) {
                let tip = special_n::joint(f, SHOT_TIP_PART);
                let tail = special_n::joint(f, SHOT_TAIL_PART);
                (tip, tail)
            }
        }
    };
}
// ftKb_MS_LkSpecialNStart (401), ftKb_SM_LkSpecialNStart (340); +268.
kirby_bow!(KirbyLink, 401, 340, link_bow);
// ftKb_MS_ClSpecialNStart (514), ftKb_SM_ClSpecialNStart (449); +27C.
kirby_bow!(KirbyYoungLink, 514, 449, young_link_bow);

/// ftKb_SpecialNLk800FB880 / 800FBA00 (the grounded and aerial entries;
/// the hat's kind picks the rows and the attributes).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    match f.character.get::<Kirby>().copied {
        FighterKind::CLink => special_n::enter::<KirbyYoungLink>(f, airborne, assets),
        _ => special_n::enter::<KirbyLink>(f, airborne, assets),
    }
}

/// Rows 401..406 (ftKb_LkSpecialN*) and 514..519 (the same callbacks).
pub const fn rows() -> [MotionRow; 12] {
    let link = special_n::rows::<KirbyLink>();
    let young = special_n::rows::<KirbyYoungLink>();
    [
        link[0], link[1], link[2], link[3], link[4], link[5], young[0], young[1], young[2],
        young[3], young[4], young[5],
    ]
}

/// ftKb_SpecialNLk800FB394 (800FB394): the row the bow follows, for the
/// hat's kind.
pub fn stage(f: &Fighter) -> Option<u8> {
    match f.character.get::<Kirby>().copied {
        FighterKind::Link => special_n::stage::<KirbyLink>(f),
        FighterKind::CLink => special_n::stage::<KirbyYoungLink>(f),
        _ => None,
    }
}

/// ftKb_SpecialNLk800FB800 / 800FB840 (800FB800, 800FB840), the Link and
/// Young Link arm of 800F19AC and 800F1A8C: the arrow (it_802A8A7C) and the
/// bow (it_802AF304) go.
pub fn put_away(f: &mut Fighter, kind: FighterKind) {
    match kind {
        FighterKind::Link => special_n::remove_articles::<KirbyLink>(f),
        FighterKind::CLink => special_n::remove_articles::<KirbyYoungLink>(f),
        _ => {}
    }
}

/// The articles' Destroyed callbacks: ftKb_SpecialNLk800FB418 (the arrow,
/// before its release) and 800FB444 (the bow).
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    match kind {
        ItemKind::KirbyLinkArrow | ItemKind::KirbyLinkBow => {
            special_n::article_destroyed::<KirbyLink>(f, kind)
        }
        ItemKind::KirbyCLinkArrow | ItemKind::KirbyCLinkBow => {
            special_n::article_destroyed::<KirbyYoungLink>(f, kind)
        }
        _ => {}
    }
}
