//! Which Ground models the main camera draws, and in which pass. Each stage's
//! map creation, render pass and camera link are ported from its `gr*.c`;
//! liveness reads the simulation's Ground GObjs and never changes them.
use super::{error, ModelSource, Presentation, PresentationError};
use crate::{scene_stage::SceneStage, Match};
use melee_gr::last::animation::BackgroundAnimation;
use melee_types::GrKind;

/// Ground scheduler keys a stage may use (`scene_stage::StageObjects`).
const KEYS: u8 = 16;

/// `Ground.x11_flags.b012 == 2`: the main camera draws these maps first, in
/// its background pass (`fn_800301D0`, 0x800301D0: passes 2, 1, then 0).
/// Each map's on_init sets the flag:
/// - Final Destination: the layered backgrounds 4-9 (grLast_8021AB34 and
///   its siblings, grLast_8021B240, grLast_8021B294);
/// - Battlefield: the backgrounds 1, 2, 4, the transition 3 and the demo
///   backdrop 5 (grBattle_GObj{1,2,4,5}_Callback0, grBattle_BG_Callback0);
/// - Yoshi's Story: map 1 (grStory_801E31C0);
/// - Dream Land: the Bronto Burt flyby 2 and the sky 3 (grOldPupupu_80210BE4,
///   grOldPupupu_80211110).
///
/// - Fountain of Dreams: the star field (grIzumi_801CCB18).
///
/// Pokemon Stadium has none.
pub(super) fn is_background(kind: GrKind, map: u8) -> bool {
    match kind {
        GrKind::Last => map >= 4,
        GrKind::Battle => matches!(map, 1..=5),
        GrKind::Story => map == 1,
        GrKind::OldPupupu => matches!(map, 2 | 3),
        // grIzumi_801CCB18: the star field.
        GrKind::Izumi => map == IZUMI_STAR,
        _ => false,
    }
}

/// Fountain of Dreams' star field (grIzumi_801CCB18): its scheduler key and
/// public joint.
const IZUMI_STAR: u8 = melee_gr::izumi::procs::STAR;
const IZUMI_STAR_JOINT: &str = "GrdIzumiStar_TopN_joint";

/// grIzumi_801CCB90 (0x801CCB90): `HSD_StateSetPointSize(18)`, three pixels
/// (GX point sizes are sixths of a pixel). Presentation draws each point as
/// a quad this many model units across, about three retail pixels at the
/// stage's usual camera distance.
const POINT_QUAD_HALF_SIZE: f32 = 1.0;

/// GX_POINTS as quads in the model's x-y plane.
pub(super) fn point_quads(points: &[super::Vertex]) -> (Vec<super::Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(points.len() * 4);
    let mut indices = Vec::with_capacity(points.len() * 12);
    for point in points {
        let base = vertices.len() as u32;
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let mut corner = point.clone();
            corner.position[0] += dx * POINT_QUAD_HALF_SIZE;
            corner.position[1] += dy * POINT_QUAD_HALF_SIZE;
            vertices.push(corner);
        }
        // Both windings: a point faces every direction.
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    (vertices, indices)
}

/// grIzumi_801CCA64 (0x801CCA64): Fountain of Dreams' map 2 moves to GX
/// link 2, which only the water reflection's camera draws.
fn drawn_by_main_camera(kind: GrKind, map: u8) -> bool {
    !(kind == GrKind::Izumi && map == 2)
}

/// The model behind a Ground scheduler key: Fountain of Dreams' second
/// platform is another instance of map 4 (melee_gr::izumi::procs).
fn model_map(kind: GrKind, key: u8) -> Option<u8> {
    if kind == GrKind::Izumi {
        melee_gr::izumi::procs::model(key)
    } else {
        Some(key)
    }
}

/// `Ground_801C466C` (0x801C466C): the light set of the first map whose
/// StageCallbacks row has `flags_b0` (bit 31) set. Every other stage's fog
/// row is also its light row; Final Destination splits them (lights 3, fog 4).
pub(super) fn light_map(kind: GrKind) -> Result<usize, PresentationError> {
    Ok(match kind {
        GrKind::Last => 3,
        GrKind::Battle => 6,
        GrKind::Story => 3,
        GrKind::OldPupupu => 5,
        GrKind::Izumi => 3,
        GrKind::PStadium => 2,
        _ => return Err(error("stage light set unknown")),
    })
}

/// The main camera's fog (`HSD_Fog` set by Ground_801C1E2C): GX
/// perspective-linear fog between eye depths `start` and `end`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub color: [u8; 3],
    pub start: f32,
    pub end: f32,
}
/// `GX_FOG_PERSP_LIN`, the only fog type these stages use.
const GX_FOG_PERSP_LIN: u32 = 2;

/// `Ground_801C1E94` (0x801C1E94): the fog of the first map whose row has
/// `flags_b1` (bit 30) set, its depths scaled by the map scale. Of the
/// supported stages only Final Destination's row (map 4) carries one.
pub(super) fn fog_desc(
    assets: &crate::assets::Assets,
) -> Result<Option<melee_gr::desc::FogDesc>, PresentationError> {
    let kind = assets.stage_desc.kind;
    let map = if kind == GrKind::Last {
        4
    } else {
        light_map(kind)?
    };
    let Some(offset) = assets.stage_desc.models.get(map).and_then(|m| m.fog_offset) else {
        return Ok(None);
    };
    let mut fog = melee_gr::desc::read_fog(&assets.stage, offset).map_err(error)?;
    if fog.kind != GX_FOG_PERSP_LIN {
        return Err(error("stage fog type unsupported"));
    }
    let scale = assets.stage_desc.parameters.map_scale;
    fog.start *= scale;
    fog.end *= scale;
    Ok(Some(fog))
}

/// The fog this tick: Final Destination switches it off while its stars
/// show (Ground_801C1E00 from grLast_8021B920) and animates its color.
pub(super) fn fog(game: &Match, desc: &Option<melee_gr::desc::FogDesc>) -> Option<Fog> {
    let desc = desc.as_ref()?;
    let color = match &game.engine.state().stage {
        SceneStage::FinalDestination(stage) if !stage.ground.fog_enabled => return None,
        SceneStage::FinalDestination(stage) => stage.ground.fog,
        _ => [desc.color[0], desc.color[1], desc.color[2]],
    };
    Some(Fog {
        color,
        start: desc.start,
        end: desc.end,
    })
}

/// Whether the Ground GObj behind `key` exists. Final Destination tracks its
/// own maps (`Ground::live_maps`); every other stage reads the scheduler.
pub(super) fn live(game: &Match, key: u8) -> bool {
    match &game.engine.state().stage {
        SceneStage::FinalDestination(stage) => stage
            .ground
            .live_maps
            .get(usize::from(key))
            .copied()
            .unwrap_or(false),
        _ => game.engine.stage_object_live(key),
    }
}

/// Pokemon Stadium's screen (map 1).
const STADIUM_SCREEN: u8 = 1;
/// `grStadium_801D1EF8` (0x801D1EF8): each bit of the screen's layer mask
/// (`grStadium_Display::xE6`) shows one Ground_801C3FA4 descendant.
const SCREEN_LAYERS: [(u8, usize); 8] = [
    (0x01, 6),
    (0x02, 3),
    (0x04, 4),
    (0x08, 7),
    (0x10, 9),
    (0x20, 8),
    (0x40, 2),
    (0x80, 5),
];
/// The layer that carries the live text and camera feed (`xC8`'s TObj, its
/// image replaced every frame by grStadium_801D1EF8). Presentation does not
/// render that feed; the layer shows a dark screen instead of its
/// placeholder image.
const SCREEN_FEED: usize = 2;
/// Overlay standing in for the unrendered feed.
pub(super) const SCREEN_OFF: [f32; 4] = [0.02, 0.03, 0.04, 1.0];

/// `grStadium_801D2528` (0x801D2528): the layer mask each mode selects.
fn screen_layers(mode: melee_gr::stadium::ScreenMode) -> u8 {
    use melee_gr::stadium::ScreenMode::*;
    match mode {
        AnnounceBase => 0x01,
        AnnounceFire => 0x02,
        AnnounceGrass => 0x04,
        AnnounceWater => 0x10,
        AnnounceRock => 0x08,
        Picture15 => 0x20,
        Picture16 => 0x80,
        _ => 0x40,
    }
}

/// Joints the stage's own display code hides beyond `JOBJ_HIDDEN`.
pub(super) fn hides_joint(game: &Match, key: u8, joint: hsd_anim::jobj::JObjId) -> bool {
    let SceneStage::Stadium(stage) = &game.engine.state().stage else {
        return false;
    };
    if key != STADIUM_SCREEN {
        return false;
    }
    let Some(screen) = game.engine.state().stage_animations.get(&key) else {
        return false;
    };
    let mask = screen_layers(stage.screen.mode);
    SCREEN_LAYERS
        .iter()
        .any(|&(bit, bone)| screen.joint_id(bone) == Some(joint) && mask & bit == 0)
}

/// Whether `joint` is Pokemon Stadium's screen feed layer.
pub(super) fn is_screen_feed(game: &Match, key: u8, joint: hsd_anim::jobj::JObjId) -> bool {
    matches!(game.engine.state().stage, SceneStage::Stadium(_))
        && key == STADIUM_SCREEN
        && game
            .engine
            .state()
            .stage_animations
            .get(&key)
            .and_then(|screen| screen.joint_id(SCREEN_FEED))
            == Some(joint)
}

/// The live model behind `key`, or the template of one not created yet.
pub(super) fn animation<'a>(
    game: &'a Match,
    key: u8,
    template: &'a Option<Box<BackgroundAnimation>>,
) -> &'a BackgroundAnimation {
    game.engine
        .state()
        .stage_animations
        .get(&key)
        .or(template.as_deref())
        .expect("stage model without live animation or template")
}

/// The stage model a key draws, from the stage archive or, for Pokemon
/// Stadium's forms, the form archive that holds it (grDatFiles_801C6330).
fn model_source(
    assets: &crate::assets::Assets,
    map: u8,
) -> Option<(Option<usize>, &melee_gr::desc::ModelDesc)> {
    let own = assets.stage_desc.models.get(usize::from(map))?;
    if own.present {
        return Some((None, own));
    }
    assets
        .stage_forms
        .iter()
        .enumerate()
        .find_map(|(index, form)| {
            let model = form.models.get(usize::from(map))?;
            model.present.then_some((Some(index), model))
        })
}

impl Presentation {
    /// Every Ground model the match may draw: maps live now or held in
    /// reserve by the simulation, and Pokemon Stadium's forms, whose Ground
    /// is created mid-match. A form's pose comes from its live model; a
    /// template built as `grStadium_801D10F8` builds it stands in until then.
    pub(super) fn add_stage_models(&mut self, game: &Match) -> Result<(), PresentationError> {
        let assets = std::sync::Arc::clone(&self.assets);
        let kind = assets.stage_desc.kind;
        let animations = &game.engine.state().stage_animations;
        for key in 0..KEYS {
            let Some(map) = model_map(kind, key) else {
                continue;
            };
            if !drawn_by_main_camera(kind, map) {
                continue;
            }
            let Some((form, desc)) = model_source(&assets, map) else {
                continue;
            };
            let archive = match form {
                Some(index) => &assets.stage_forms[index].archive,
                None => &assets.stage,
            };
            // Pokemon Stadium creates and destroys its forms (and the base
            // arena) mid-match: grStadium_801D10F8 loads each as
            // Ground_GetStageGObj does, under the map-scale wrapper.
            let template = if kind == GrKind::PStadium {
                let mut animation =
                    BackgroundAnimation::load_model(archive, desc).map_err(error)?;
                animation.set_map_scale(assets.stage_desc.parameters.map_scale);
                Some(Box::new(animation))
            } else if animations.contains_key(&key) {
                None
            } else {
                // Never created by this stage's code (Dream Land's flyby).
                continue;
            };
            let source = ModelSource::Stage {
                key,
                map,
                form,
                template,
            };
            let tree = source.tree(game).clone();
            self.add_model(archive, &desc.joint, &tree, source)?;
        }
        if kind == GrKind::Izumi {
            self.add_izumi_star(game)?;
        }
        Ok(())
    }

    /// grIzumi_801CCB18 (0x801CCB18): Fountain of Dreams' star field, a
    /// public joint Ground_801C1A20 loads under the map-scale wrapper with
    /// no animation, drawn as GX points in the background pass.
    fn add_izumi_star(&mut self, game: &Match) -> Result<(), PresentationError> {
        let assets = std::sync::Arc::clone(&self.assets);
        let Some(offset) = assets.stage.public(IZUMI_STAR_JOINT) else {
            return Err(error("Fountain of Dreams star joint missing"));
        };
        let joint = hsd_archive::desc::JObjDesc::read(&assets.stage, offset).map_err(error)?;
        let desc = melee_gr::desc::ModelDesc {
            present: true,
            joint,
            animations: Vec::new(),
            animation_loops: Vec::new(),
            joint_mappings: Vec::new(),
            material_animation_offsets: Vec::new(),
            shape_animation_offsets: Vec::new(),
            light_list_offset: None,
            fog_offset: None,
        };
        let mut animation = BackgroundAnimation::load_model(&assets.stage, &desc).map_err(error)?;
        animation.set_map_scale(assets.stage_desc.parameters.map_scale);
        let source = ModelSource::Stage {
            key: IZUMI_STAR,
            map: IZUMI_STAR,
            form: None,
            template: Some(Box::new(animation)),
        };
        let tree = source.tree(game).clone();
        self.add_model(&assets.stage, &desc.joint, &tree, source)
    }

    /// Items whose model lives in the stage archive: Yoshi's Story's Shy
    /// Guys (Ground_801C0800 -> it_8026B40C), one animated model per slot.
    pub(super) fn add_stage_items(&mut self) -> Result<(), PresentationError> {
        let assets = std::sync::Arc::clone(&self.assets);
        let kind = melee_types::ItemKind::Heiho;
        let Some(heiho) = assets.items.try_get(kind) else {
            return Ok(());
        };
        for slot in 0..SHY_GUY_SLOTS {
            let held =
                super::items::ArticleModel::new(&assets.stage, &heiho.visual, slot, kind, 0)?;
            let tree = held.tree().clone();
            self.add_model(
                &assets.stage,
                &heiho.visual.model,
                &tree,
                ModelSource::StageItem(held),
            )?;
        }
        Ok(())
    }
}

/// grStory_801E3418 spawns at most six Shy Guys at once (one, or three to
/// six), and waits until none remain.
const SHY_GUY_SLOTS: usize = 6;

/// A stage item's pose: the slot's live item of the kind, in pool order,
/// at its animation frame, under the item's world SRT (retail items
/// overwrite their root SRT with ItemCore's).
pub(super) fn capture_item(
    game: &Match,
    held: &mut super::items::ArticleModel,
) -> Result<(), PresentationError> {
    let item = game
        .engine
        .state()
        .items
        .iter()
        .filter(|item| item.kind == held.kind && !item.destroyed)
        .nth(held.owner);
    held.capture(item, hsd_types::Mtx::default())?;
    if let Some(item) = item {
        held.set_root_srt(&item.position, &item.rotation, &item.model_scale);
    }
    Ok(())
}
