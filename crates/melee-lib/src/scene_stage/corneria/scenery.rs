//! The sky (map 7) and the scenery scrolling past (maps 8, 9 and 4): each
//! rises with the Great Fox's descent; each background moves right, makes
//! the next one behind it and goes once past the stage.
use super::{corneria, evaluate_frame_zero, passes};
use crate::initial_state::InitialState;
use anyhow::Result;
use hsd_gobj::TaggedWorld;
use melee_gr::corneria::{
    backgrounds::{next, Background, Scroll},
    procs::SKY_MAP,
};

/// grCorneria_801E0D30 (0x801E0D30): the sky at the sea's height.
pub(super) fn sky(state: &mut InitialState) {
    let descent = corneria(state).stage.great_fox.descent;
    let model = state.stage_animations.get_mut(&SKY_MAP).unwrap();
    let mut translation = model.gobj_translation();
    translation.y = -descent;
    model.set_gobj_translate(translation);
}

/// grCorneria_801DFC98 / 801DFF20 / 801E01A8: the background moves right
/// by x88 and to the sea's height, then makes the next background
/// (grCorneria_801E03C8) or goes.
pub(super) fn scroll(
    state: &mut InitialState,
    map: u8,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let scale = state.assets.stage_desc.parameters.map_scale;
    let stage = &mut corneria(state).stage;
    let parameters = stage.parameters;
    let descent = stage.great_fox.descent;
    let slot = stage.background_mut(map);
    let mut background = slot.expect("a background's proc without its controller");
    let step = background.scroll(map, &parameters, scale);
    *slot = Some(background);
    let model = state.stage_animations.get_mut(&map).unwrap();
    let mut translation = model.gobj_translation();
    // HSD_JObjAddTranslationX (fadds), then HSD_JObjSetTranslateY.
    translation.x = background.x;
    translation.y = -descent;
    model.set_gobj_translate(translation);
    match step {
        Scroll::Nothing => {}
        Scroll::MakeNext { x } => make(state, next(map), x, world, objects)?,
        Scroll::Leave => {
            *corneria(state).stage.background_mut(map) = None;
            passes::remove_object(state, map, world, objects);
        }
    }
    Ok(())
}

/// grCorneria_801E03C8's Ground_GetStageGObj: the background's init
/// (grCorneria_801DFEB8 / 801E0140: grAnime_801C8138 on a model without
/// animations, scrolling) and proc, then its place behind its maker.
fn make(
    state: &mut InitialState,
    map: u8,
    x: f32,
    world: &mut TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
) -> Result<()> {
    let object = passes::create_object(state, map, world, objects)?;
    let model = state.stage_animations.get_mut(&map).unwrap();
    evaluate_frame_zero(model, map)?;
    passes::add_map_proc(world, object, map);
    let model = state.stage_animations.get_mut(&map).unwrap();
    let mut translation = model.gobj_translation();
    translation.x = x;
    model.set_gobj_translate(translation);
    let slot = corneria(state).stage.background_mut(map);
    anyhow::ensure!(slot.is_none(), "grCorneria_801E03C8: map {map} made twice");
    *slot = Some(Background { x, passed: false });
    Ok(())
}
