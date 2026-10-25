//! Hyrule Temple composition (grshrine.c): three static maps created in
//! grShrine_80201C64's order, and the stage-wide dynamics field it adds.
use crate::{assets::Assets, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    shrine::{procs::MAP_ORDER, Shrine},
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// Ground_GetStageGObj (0x801C14D0): a map's model under its map-scale wrapper.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Hyrule Temple map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// The stage controller: only the lights, from map 2's environment row.
pub(crate) fn controller(assets: &Assets) -> Result<Shrine> {
    Ok(Shrine {
        lights: melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, 2)
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    })
}

/// grShrine_80201C64 (0x80201C64): maps 0, 1, 2, each evaluating animation
/// 0's frame zero (grAnime_801C8138 in grShrine_80201E08, 80201E44 and
/// 80201E9C). Map 2 binds collision joints (Ground_801C2ED0) but GrSh.dat
/// has none. Nothing draws: the archive has no particle keys.
pub(crate) fn initialize(assets: &Assets) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Hyrule Temple map {map}: unexpected setup particle key"
        );
        animations.insert(map, animation);
    }
    Ok((SceneStage::Shrine(controller(assets)?), animations))
}
