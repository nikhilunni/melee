//! Brinstar's models as the loader prepares them: every animation switch
//! a proc makes is decoded at load, so the tick replays it without
//! allocating.
use crate::assets::Assets;
use anyhow::Result;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    zebes::{
        island::{RISE_ANIMATION, SINK_ANIMATION},
        plates::{CRACK_ANIMATION, GROWTH_ANIMATION},
        procs::{
            LEFT_CRACK_BONE, LEFT_MAP, LEFT_PLATE_BONE, LEFT_PLATFORM_BONE, PULSE_MAP,
            REACTION_MAP, RIGHT_CRACK_BONE, RIGHT_MAP, RIGHT_PART_BONE, RIGHT_PLATE_BONE,
        },
        scenery::{LARGE_SHAKE, REACTION_BONE, SMALL_SHAKE, STILL},
    },
};

/// A map's model with the switches its proc makes prepared.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let model = &assets.stage_desc.models[usize::from(map)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
        .map_err(|e| anyhow::anyhow!("Brinstar map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    let error = |e| anyhow::anyhow!("Brinstar map {map}: {e}");
    match map {
        REACTION_MAP => {
            for index in [STILL, SMALL_SHAKE, LARGE_SHAKE] {
                animation
                    .prepare_subtree(&assets.stage, model, REACTION_BONE, index)
                    .map_err(error)?;
            }
        }
        LEFT_MAP | RIGHT_MAP => {
            let (plate, crack, platform) = if map == LEFT_MAP {
                (LEFT_PLATE_BONE, LEFT_CRACK_BONE, LEFT_PLATFORM_BONE)
            } else {
                (RIGHT_PLATE_BONE, RIGHT_CRACK_BONE, RIGHT_PART_BONE)
            };
            for bone in [plate, crack] {
                for index in [CRACK_ANIMATION, GROWTH_ANIMATION] {
                    animation
                        .prepare_subtree(&assets.stage, model, bone, index)
                        .map_err(error)?;
                }
            }
            for index in [SINK_ANIMATION, RISE_ANIMATION] {
                animation
                    .prepare_joint_switch(&assets.stage, model, platform, index)
                    .map_err(error)?;
            }
        }
        _ => {}
    }
    Ok(animation)
}

/// grZebes_801DAA08's pulse: map 2's model hung under a bubble's
/// (Ground_801C13D0), its animations added (no joint animation, its
/// material one), requested at 0 and evaluated (HSD_JObjAnimAll), looping
/// cleared (grAnime_801C78FC(gobj, 0, 7)); then each tick the bubble's
/// Ground_801C1CD0 advances it. Returns the ticks after which the
/// bubble's first AObj (this one) reads as ended (grAnime_801C83D0).
pub(crate) fn pulse_ticks(assets: &Assets) -> Result<u32> {
    let model = &assets.stage_desc.models[usize::from(PULSE_MAP)];
    let mut animation = BackgroundAnimation::load_model(&assets.stage, model)
        .map_err(|e| anyhow::anyhow!("Brinstar map 2: {e}"))?;
    animation.clear_loops();
    animation.clear_material_loops();
    animation.evaluate_initial_frame::<RetailTrig>();
    for ticks in 0..u32::from(u16::MAX) {
        if animation.first_animation_ended() {
            return Ok(ticks);
        }
        animation.tick::<RetailTrig>();
    }
    anyhow::bail!("Brinstar map 2: the pulse never ends")
}
