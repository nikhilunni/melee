//! Corneria composition (grcorneria.c): the Great Fox (map 3) with its
//! cannons and spawner, the markers riding it (map 0), the sky (map 7),
//! the scenery scrolling past (maps 8, 9 and 4) and the passes of Arwings
//! and Wolfens ([`passes`]), in grCorneria_801DD350's order.
mod great_fox;
mod models;
mod passes;
mod scenery;

pub(crate) use great_fox::material_hit;
pub(crate) use models::load;
pub(crate) use passes::spawn_lasers;

use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Context, Result};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    corneria::{
        arwings::{Spawner, PATH_MAP},
        backgrounds::{Background, CITY_MAP, COAST_MAP, SEA_MAP},
        great_fox::{Cannon, GreatFox, BROKEN_BONE, INTACT_BONE},
        procs::{
            bindings, CorneriaStage, MARKER_MAP, PLATFORM_JOINTS, SHIP_MAP, SKY_MAP,
        },
        Parameters,
    },
    last::animation::BackgroundAnimation,
};
use models::{ARWING_MAP, ARWING_PLATFORM_MAP, WOLFEN_MAP, WOLFEN_PLATFORM_MAP};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The stage's controllers and the limits its markers give this tick.
#[derive(Clone, Debug)]
pub struct Corneria {
    pub stage: CorneriaStage,
    /// Ground_801C39C0 / Ground_801C3BB4 as grCorneria_801E1878 last ran
    /// them: the markers ride the Great Fox.
    pub arena: melee_ft::fighter::life::Arena,
    pub camera: melee_cm::StageCamera,
}

impl Corneria {
    /// stage_info's limits as the stage leaves them: the markers' camera
    /// range and blast zones, the lowest eye height 5 above the sea
    /// (grCorneria_801E2FCC).
    pub(crate) fn limits(&self, map_scale: f32) -> crate::stage_limits::StageLimits {
        let mut camera = self.camera;
        camera.min_eye_height = self.stage.great_fox.lowest_eye_height(map_scale);
        crate::stage_limits::StageLimits {
            camera,
            arena: self.arena,
        }
    }
}

/// The maps a match creates as it goes, reserved at load.
const LATER_MAPS: [u8; 7] = [
    COAST_MAP,
    SEA_MAP,
    PATH_MAP,
    ARWING_MAP,
    WOLFEN_MAP,
    ARWING_PLATFORM_MAP,
    WOLFEN_PLATFORM_MAP,
];

pub(crate) fn read_parameters(assets: &Assets) -> Result<Parameters> {
    melee_gr::desc::read_corneria_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

/// grCorneria_801DD350's lb_80011A50: a constant field over the whole
/// stage (M_PI / 3 as the float 1.0471976).
pub(crate) fn wind() -> melee_lb::radial_force::DirectionalGust {
    melee_lb::radial_force::DirectionalGust {
        direction: Vec3::new(1.0, 0.0, 0.0),
        frames: -1,
        strength: 1.0,
        decay: 0.0,
        phase_step: core::f32::consts::FRAC_PI_3,
        rectangle: [-100000.0, 100000.0, 100000.0, -100000.0],
    }
}

/// Ground_801C2ED0 (0x801C2ED0) for `map`: its joints from its current
/// pose, that pose taken as the previous one too.
fn bind(animation: &mut BackgroundAnimation, map: u8, collision: &mut melee_mp::CollMap) {
    for binding in bindings(map) {
        animation.update_collision(collision, std::slice::from_ref(binding));
        collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
    }
}

/// Ground_801C2FE0 (0x801C2FE0) for `map`: its joints follow their bones.
fn follow(state: &mut InitialState, map: u8) {
    let model = state.stage_animations.get_mut(&map).unwrap();
    for binding in bindings(map) {
        model.update_collision(&mut state.map, std::slice::from_ref(binding));
    }
}

/// A map's frame-zero evaluation (grAnime_801C8138's HSD_JObjAnimAll);
/// Corneria's maps have no particle keys.
fn evaluate_frame_zero(animation: &mut BackgroundAnimation, map: u8) -> Result<()> {
    ensure!(
        animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "Corneria map {map}: frame-zero particle keys"
    );
    Ok(())
}

/// The markers Ground_801C39C0 and Ground_801C3BB4 read (the camera
/// range 0x94..0x96, the blast zones 0x97 and 0x98) and the revival
/// markers 4..7.
const LIMIT_MARKERS: [i16; 9] = [0x94, 0x95, 0x96, 0x97, 0x98, 4, 5, 6, 7];

/// grCorneria_801E1878 (0x801E1878): map 0 takes the Great Fox's
/// translation, then Ground_801C39C0 and Ground_801C3BB4 read the markers
/// from its live joints (Ground_801C2D24). The blast zones are kept as
/// Stage_GetBlastZone*Offset gives them: the relative bounds plus the
/// camera's centre.
pub(crate) fn carry_markers(
    assets: &Assets,
    animations: &mut Animations,
    translation: Vec3,
) -> Result<(melee_ft::fighter::life::Arena, melee_cm::StageCamera)> {
    let markers = animations.get_mut(&MARKER_MAP).unwrap();
    markers.set_gobj_translate(translation);
    let mut found = [None; LIMIT_MARKERS.len()];
    for (slot, index) in found.iter_mut().zip(LIMIT_MARKERS) {
        // Ground_801C34AC: the marker's last pair registers last (x280).
        if let Some(binding) = assets
            .stage_desc
            .position_bindings
            .iter()
            .rfind(|b| b.stage_position == index)
        {
            ensure!(
                binding.model_id == usize::from(MARKER_MAP),
                "Corneria marker {index} on map {}",
                binding.model_id
            );
            let matrix = markers.joint_matrix(binding.joint_index as usize);
            *slot = Some(Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]));
        }
    }
    // GrCn.dat binds revival marker 4 alone; 5..7 fall back to it (as
    // stage_limits would after a failed lookup, without building an error
    // on the tick path).
    let first_revival = LIMIT_MARKERS.iter().position(|&m| m == 4).unwrap();
    for slot in first_revival + 1..LIMIT_MARKERS.len() {
        if found[slot].is_none() {
            found[slot] = found[first_revival];
        }
    }
    let (mut arena, camera) = crate::assets::stage_limits(&assets.stage_desc, |index| {
        LIMIT_MARKERS
            .iter()
            .position(|&m| m == index)
            .and_then(|slot| found[slot])
            .context("stage marker")
    })?;
    arena.left = camera.blast_left();
    arena.right = camera.blast_right();
    arena.top = camera.blast_top();
    arena.bottom = camera.blast_bottom();
    Ok((arena, camera))
}

/// grCorneria_801DD350 (0x801DD350) and its on_load, grCorneria_801DD478:
/// maps 7 and 3 (whose init binds joints 3 and 4, plays its loop, stands
/// at y 250 and makes map 0), the first wait (no draw: x3C and x40 are
/// both 600), the platforms' joints off, and the city (map 8) that on_load
/// makes anew. The scenery creation makes and on_load removes again
/// leaves only freed GObjs, which the HUD's creation (ifAll_802F390C,
/// after on_load) takes again; the cannons' material items follow once
/// the pool exists ([`spawn_items`]).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let mut animations = Animations::new();
    // grCorneria_801E0C3C: grAnime_801C8138 on a model without animations;
    // Ground_801C4E70's points only set the fog's range.
    let mut sky = load(assets, SKY_MAP)?;
    evaluate_frame_zero(&mut sky, SKY_MAP)?;
    animations.insert(SKY_MAP, sky);
    // grCorneria_801DD674: Ground_801C2ED0, then its loop from frame zero,
    // at the Great Fox's rest position.
    let great_fox = GreatFox::default();
    let mut ship = load(assets, SHIP_MAP)?;
    bind(&mut ship, SHIP_MAP, collision);
    evaluate_frame_zero(&mut ship, SHIP_MAP)?;
    // The intact cannon shown, the broken one hidden.
    ship.set_joint_hidden(INTACT_BONE, false);
    ship.set_joint_hidden(BROKEN_BONE, true);
    animations.insert(SHIP_MAP, ship);
    // grCorneria_801DD620 (map 0, made by map 3's init): no animation.
    let mut markers = load(assets, MARKER_MAP)?;
    evaluate_frame_zero(&mut markers, MARKER_MAP)?;
    animations.insert(MARKER_MAP, markers);
    let translation = great_fox.translation();
    animations
        .get_mut(&SHIP_MAP)
        .unwrap()
        .set_gobj_translate(translation);
    let (arena, camera) = carry_markers(assets, &mut animations, translation)?;
    let spawner = Spawner::create(&parameters, rng);
    for joint in PLATFORM_JOINTS {
        collision.joint_list_remove(joint);
    }
    // grCorneria_801DD478 -> grCorneria_801DFC2C: the city anew, its
    // scroll starting at x 0 (no animation).
    let mut city = load(assets, CITY_MAP)?;
    city.clear_animation();
    animations.insert(CITY_MAP, city);
    reserve_later_maps(assets, &mut animations)?;
    let stage = CorneriaStage {
        parameters,
        waiting: true,
        great_fox,
        cannon: Cannon::default(),
        cannon_items: [None; 2],
        spawner,
        backgrounds: [Some(Background::default()), None, None],
        flight: None,
        pending_shot: None,
    };
    Ok((
        SceneStage::Corneria(Box::new(Corneria {
            stage,
            arena,
            camera,
        })),
        animations,
    ))
}

/// The maps the match makes later, loaded once and kept hidden until made
/// (the tick path allocates nothing).
pub(crate) fn reserve_later_maps(assets: &Assets, animations: &mut Animations) -> Result<()> {
    for map in LATER_MAPS {
        let mut model = load(assets, map)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(map, model);
    }
    Ok(())
}

/// grCorneria_801DD674's material items, in creation order: the left
/// cannon's on bone 2 (state 0), the right's on bone 3 (state 2), each
/// taking hits on a capsule from x 4 to 16, radius 4 (grMaterial_801C8DE0,
/// 801C8E08).
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) {
    let SceneStage::Corneria(corneria) = stage else {
        return;
    };
    corneria.stage.cannon_items = great_fox::spawn_cannon_items(animations, items, gobjs, resources, map);
}

/// The deferred call Ground_801C0FB8 runs when the countdown releases the
/// stage: grCorneria_801DD65C, the Great Fox's controller starts.
pub(crate) fn start(state: &mut InitialState) {
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!()
    };
    corneria.stage.waiting = false;
}

/// The display pass (HSD_JObjDispAll on the ship): a far ship's root
/// resolves its RObjs against the path's bone 4 as the tick left it.
pub(crate) fn display(state: &mut InitialState) {
    let SceneStage::Corneria(corneria) = &state.stage else {
        return;
    };
    if let Some(flight) = corneria.stage.flight {
        if flight.path && flight.ship.is_some() {
            passes::display_far_ship(state, flight);
        }
    }
}

/// A Corneria gobj proc (s_link 4).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_stage::StageObjects,
    item_objects: &mut crate::scene_items::Objects,
    draws: &mut hsd_particle::rng_sites::DrawLog,
) -> Result<()> {
    match map {
        SHIP_MAP => great_fox::tick(state, world, objects, item_objects, draws)?,
        SKY_MAP => scenery::sky(state),
        CITY_MAP | COAST_MAP | SEA_MAP => scenery::scroll(state, map, world, objects)?,
        PATH_MAP => passes::path(state, world, objects)?,
        ARWING_MAP | WOLFEN_MAP => passes::ship(state, map, world, objects)?,
        ARWING_PLATFORM_MAP | WOLFEN_PLATFORM_MAP => passes::platform(state, map, world, objects),
        _ => {}
    }
    // A generator on a bone reads its JObj when the particles update
    // (hsd_8039D214), after the proc moved the model.
    let moved: &[u8] = if map == SHIP_MAP {
        &[SHIP_MAP, MARKER_MAP]
    } else {
        std::slice::from_ref(&map)
    };
    for &map in moved {
        if let Some(model) = state.stage_animations.get_mut(&map) {
            crate::scene_stage::publish_joint_matrices(
                model,
                map,
                &mut state.particles,
                &mut state.effects.events,
            );
        }
    }
    Ok(())
}

fn corneria(state: &mut InitialState) -> &mut Corneria {
    let SceneStage::Corneria(corneria) = &mut state.stage else {
        unreachable!("a Corneria proc on another stage")
    };
    corneria
}
