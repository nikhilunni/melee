//! Venom's models as the loader prepares them: every animation switch a
//! creation or proc makes is decoded at load, so the tick replays it
//! without allocating.
use crate::assets::Assets;
use anyhow::Result;
use melee_gr::{
    last::animation::BackgroundAnimation,
    venom::{
        arwings::{PATH_MAP, SHIP_BONE},
        procs::{SHIP_MAP, TUNNEL_MAP},
    },
};

/// The Arwing's ship model (group 1..3) and the Wolfen's.
pub(crate) const ARWING_MAP: u8 = 3;
pub(crate) const WOLFEN_MAP: u8 = 6;
/// The platforms' identity roots: the Arwing's (collision joint 4) and the
/// Wolfen's (joint 3).
pub(crate) const ARWING_PLATFORM_MAP: u8 = 10;
pub(crate) const WOLFEN_PLATFORM_MAP: u8 = 13;

/// grVenom_802040F0's switches on map 5, in its order (bone, animation,
/// whole subtree): grAnime_801C7FF8 on 0 with animation 0, 801C8098 on 0
/// with 1, 801C7FF8 on 11, 13 and 15 with 2 and on 4 and 7 with 3.
pub(crate) const SHIP_SWITCHES: [(usize, usize, bool); 7] = [
    (0, 0, true),
    (0, 1, false),
    (11, 2, true),
    (13, 2, true),
    (15, 2, true),
    (4, 3, true),
    (7, 3, true),
];
/// Ground_AnimateStarFoxArwingWithBackground (an Arwing, map 3):
/// grAnime_801C7FF8 on 7 and 8 with animation 0, 801C8098 on 2 with 3.
pub(crate) const ARWING_SWITCHES: [(usize, usize, bool); 3] = [(7, 0, true), (8, 0, true), (2, 3, false)];
/// Ground_AnimateStarFoxArwing (a Wolfen, map 6): grAnime_801C7FF8 on 0
/// with animation 1, 801C8098 on 2 with 3.
pub(crate) const WOLFEN_SWITCHES: [(usize, usize, bool); 2] = [(0, 1, true), (2, 3, false)];
/// grVe_803E5644's maneuvers on bone 1: an Arwing's and a Wolfen's.
pub(crate) const ARWING_MANEUVERS: [usize; 4] = [1, 2, 4, 5];
pub(crate) const WOLFEN_MANEUVERS: [usize; 4] = [0, 2, 4, 5];

fn error(map: u8) -> impl Fn(Box<dyn std::error::Error>) -> anyhow::Error {
    move |e| anyhow::anyhow!("Venom map {map}: {e}")
}

/// A map's model (an identity root for maps 9, 10 and 13) with the
/// switches its creation and proc make prepared.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let scale = assets.stage_desc.parameters.map_scale;
    if matches!(map, TUNNEL_MAP | ARWING_PLATFORM_MAP | WOLFEN_PLATFORM_MAP) {
        return BackgroundAnimation::identity_root(&assets.stage, scale).map_err(error(map));
    }
    let model = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model).map_err(error(map))?;
    animation.set_map_scale(scale);
    let switches: &[(usize, usize, bool)] = match map {
        SHIP_MAP => &SHIP_SWITCHES,
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
        // Map 2 starts its pass's animation itself (grVenom_802053B0).
        animation.clear_animation();
    }
    Ok(animation)
}

/// Play `switches` (grAnime_801C7FF8 / 801C8098 with every type, from
/// frame zero at rate 1), as prepared by [`load`].
pub(crate) fn play(animation: &mut BackgroundAnimation, switches: &[(usize, usize, bool)]) {
    for &(bone, index, subtree) in switches {
        if subtree {
            animation.play_subtree_switch(bone, index, 0.0, 1.0);
        } else {
            animation.play_joint_switch(bone, index, 0.0, 1.0);
        }
    }
}
