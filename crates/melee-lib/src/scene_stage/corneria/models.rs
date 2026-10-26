//! Corneria's models as the loader prepares them: every animation switch a
//! creation or proc makes is decoded at load, so the tick replays it
//! without allocating. The ships share Venom's switches (the Star Fox
//! inlines of gr/inlines.h and the same maneuver table columns).
use crate::{
    assets::Assets,
    scene_stage::venom::models::{
        ARWING_MANEUVERS, ARWING_SWITCHES, WOLFEN_MANEUVERS, WOLFEN_SWITCHES,
    },
};
use anyhow::Result;
use melee_gr::{
    corneria::arwings::{PATH_MAP, SHIP_BONE},
    last::animation::BackgroundAnimation,
};

/// The Arwing's ship model (group 1..3) and the Wolfen's.
pub(crate) const ARWING_MAP: u8 = 2;
pub(crate) const WOLFEN_MAP: u8 = 10;
/// The platforms' identity roots: the Arwing's (collision joint 0) and the
/// Wolfen's (joint 5).
pub(crate) const ARWING_PLATFORM_MAP: u8 = 13;
pub(crate) const WOLFEN_PLATFORM_MAP: u8 = 16;

fn error(map: u8) -> impl Fn(Box<dyn std::error::Error>) -> anyhow::Error {
    move |e| anyhow::anyhow!("Corneria map {map}: {e}")
}

/// A map's model (an identity root past the archive's eleven) with the
/// switches its creation and proc make prepared.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    let Some(model) = assets.stage_desc.models.get(usize::from(map)) else {
        return BackgroundAnimation::identity_root(&assets.stage, scale).map_err(error(map));
    };
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model).map_err(error(map))?;
    animation.set_map_scale(scale);
    let switches: &[(usize, usize, bool)] = match map {
        ARWING_MAP => &ARWING_SWITCHES,
        WOLFEN_MAP => &WOLFEN_SWITCHES,
        _ => &[],
    };
    for &(bone, index, subtree) in switches {
        if subtree {
            animation.prepare_subtree(&assets.stage, model, bone, index)
        } else {
            animation.prepare_joint_switch(&assets.stage, model, bone, index)
        }
        .map_err(error(map))?;
    }
    let maneuvers: &[usize] = match map {
        ARWING_MAP => &ARWING_MANEUVERS,
        WOLFEN_MAP => &WOLFEN_MANEUVERS,
        _ => &[],
    };
    for &index in maneuvers {
        animation
            .prepare_joint_switch(&assets.stage, model, SHIP_BONE, index)
            .map_err(error(map))?;
    }
    if matches!(map, ARWING_MAP | WOLFEN_MAP) {
        // lb_8000C2F8's RObjs on the ship's root, for a far pass.
        animation.reserve_constraint(0);
    }
    if map == PATH_MAP {
        // Map 1 starts its pass's animation itself (grCorneria_801DE024).
        animation.clear_animation();
    }
    Ok(animation)
}

/// Play `switches` (grAnime_801C7FF8 / 801C8098 with every type, from
/// frame zero at rate 1), as prepared by [`load`].
pub(crate) fn play(animation: &mut BackgroundAnimation, switches: &[(usize, usize, bool)]) {
    crate::scene_stage::venom::models::play(animation, switches);
}
