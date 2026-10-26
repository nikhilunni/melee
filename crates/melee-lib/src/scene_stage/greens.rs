//! Green Greens composition (grgreens.c): maps 0, 2, 6, 5, 4 in
//! grGreens_Init's order, the blocks' GObjs, material items and joints,
//! Whispy's animations, fields and apples.
use super::kraid::spawn_keys;
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::Result;
use gekko_math::HsdRng;
use hsd_particle::{rng_sites::DrawLog, system::ParticleSystem};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    greens::{
        blocks::{BlockEngine, Blocks, BLAST_EFFECT, BREAK_EFFECT, CAPSULE_HEIGHT, FIRST_BONE, SLOTS},
        procs::{GreensStage, BLOCKS_MAP, FACE_MAP, MAP_ORDER, WHISPY_MAP},
        whispy::{Whispy, WhispyEngine},
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// The scene's side of Green Greens: the controllers, and each block
/// slot's GObj address (Ground_GetStageGObj of map 1 or 3).
#[derive(Clone)]
pub struct Greens {
    pub stage: GreensStage,
    /// A slot's block GObj; `None` for a block the match began with (its
    /// address is no other object's).
    pub block_gobjs: [Option<hsd_gobj::GObjAddress>; SLOTS],
    /// Each slot's bone in the bind pose: where the initial blocks' items
    /// are made (grMaterial_801C8CFC runs before the bone is placed).
    pub bind_anchors: [melee_it::YakuAnchor; SLOTS],
}

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Green Greens map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

pub(crate) fn read_parameters(assets: &Assets) -> Result<melee_gr::greens::Parameters> {
    melee_gr::desc::read_greens_parameters(&assets.stage).map_err(|e| anyhow::anyhow!("{e}"))
}

fn bindings(assets: &Assets) -> &[melee_gr::desc::JointMapping] {
    &assets.stage_desc.models[usize::from(BLOCKS_MAP)].joint_mappings
}

/// A block slot's bone translated to (x, y, 0) over the map scale
/// (`1.0f / Ground_801C0498()`, then three fmuls).
fn place_bone(model: &mut BackgroundAnimation, slot: usize, x: f32, y: f32, scale: f32) {
    let f = 1.0 / scale;
    model.set_joint_translate(FIRST_BONE + slot, Vec3::new(x * f, y * f, 0.0 * f));
}

/// mpLib_80055E9C then mpLib_80057424 on a slot's joint.
fn snap_joint(model: &mut BackgroundAnimation, collision: &mut melee_mp::CollMap, slot: usize) {
    let bone = FIRST_BONE + slot;
    let state = melee_mp::JobjState {
        mtx: model.joint_matrix(bone),
        hidden: model.joint_hidden(bone),
    };
    collision.update_joint_transform(slot as i32, Some(state));
    collision.joint_snapshot_prev_pos(slot as i32);
}

/// The blocks' engine during creation: the items wait for the pool.
struct Setup<'a> {
    model: &'a mut BackgroundAnimation,
    collision: &'a mut melee_mp::CollMap,
    scale: f32,
    blast_top: f32,
    next_item: u32,
    rng: &'a mut HsdRng,
}

impl BlockEngine for Setup<'_> {
    fn create(&mut self, slot: usize, _bomb: bool) -> u32 {
        self.model.set_joint_hidden_only(FIRST_BONE + slot, false);
        self.next_item += 1;
        u32::MAX - self.next_item
    }
    fn place(&mut self, slot: usize, x: f32, y: f32, snap: bool) {
        place_bone(self.model, slot, x, y, self.scale);
        if snap {
            snap_joint(self.model, self.collision, slot);
        }
    }
    fn script_ended(&self, _item: u32) -> bool {
        unreachable!("no block explodes during creation")
    }
    fn break_block(&mut self, _slot: usize, _item: u32, _bomb: bool) {
        unreachable!("no block breaks during creation")
    }
    fn remove(&mut self, _slot: usize, _item: u32) {
        unreachable!("no block goes during creation")
    }
    fn blast_top(&self) -> f32 {
        self.blast_top
    }
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn stitch(&mut self, _a: i32, _b: i32) {
        unreachable!("no stitching during creation")
    }
}

/// grGreens_Init (0x80213460): maps 0, 2, 6, 5, 4 and their draws (the
/// block layout's eighteen, the first block wait, Whispy's wait and side).
pub(crate) fn initialize(
    assets: &Assets,
    rng: &mut HsdRng,
    particles: &mut ParticleSystem,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let parameters = read_parameters(assets)?;
    let draws = &mut DrawLog::default();
    let scale = assets.stage_desc.parameters.map_scale;
    let mut animations = Animations::new();
    let (mut blocks, mut whispy) = (None, None);
    let mut bind_anchors = [melee_it::YakuAnchor::default(); SLOTS];
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        match map {
            BLOCKS_MAP => {
                // grGreens_80214674: Ground_801C2ED0 (no animation).
                animation.clear_animation();
                animation.update_collision(collision, bindings(assets));
                for binding in bindings(assets) {
                    collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
                }
                for (slot, bind) in bind_anchors.iter_mut().enumerate() {
                    *bind = anchor(&mut animation, FIRST_BONE + slot);
                }
                let mut positions = [(0.0, 0.0); SLOTS];
                for (slot, position) in positions.iter_mut().enumerate() {
                    let matrix = animation.joint_matrix(FIRST_BONE + slot);
                    *position = (matrix.0[0][3], matrix.0[1][3]);
                }
                // grGreens_80214FA8: every slot's ceiling callback, every bone hidden.
                for slot in 0..SLOTS {
                    collision.record_ceiling_events(slot as i32, true);
                    animation.set_joint_hidden_only(FIRST_BONE + slot, true);
                }
                let mut engine = Setup {
                    model: &mut animation,
                    collision,
                    scale,
                    blast_top: assets.stage_camera.blast_top(),
                    next_item: 0,
                    rng,
                };
                blocks = Some(Blocks::new(&positions, &parameters, &mut engine));
            }
            WHISPY_MAP => {
                // grGreens_802139C4: the device, the wait and the side, animation 0.
                whispy = Some(Whispy::new(&parameters, rng));
                spawn_keys(&mut animation, map, assets, particles, None, rng, draws)?;
            }
            _ => spawn_keys(&mut animation, map, assets, particles, None, rng, draws)?,
        }
        animations.insert(map, animation);
    }
    let stage = GreensStage {
        parameters,
        blocks: blocks.unwrap(),
        whispy: whispy.unwrap(),
        waiting: true,
    };
    Ok((
        SceneStage::Greens(Box::new(Greens {
            stage,
            block_gobjs: [None; SLOTS],
            bind_anchors,
        })),
        animations,
    ))
}

/// The material item grGreens_80215358 gives a block: state 0 on its bone,
/// the capsule at (0, 5 * scale, 0), its radius x24 or x28 times the
/// scale, hurt on, grounded.
#[allow(clippy::too_many_arguments)]
fn block_item(
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    collision: &mut melee_mp::CollMap,
    live: Option<(&mut hsd_gobj::TaggedWorld, &mut crate::scene_items::Objects)>,
    anchor: melee_it::YakuAnchor,
    bomb: bool,
    parameters: &melee_gr::greens::Parameters,
    scale: f32,
) -> u32 {
    let id = match live {
        Some((world, objects)) => crate::scene_items::spawn_yaku_live(
            items, gobjs, resources, collision, world, objects, anchor,
        ),
        None => crate::scene_items::spawn_yaku(items, gobjs, resources, collision, anchor),
    };
    let item = items.get_mut(id).unwrap();
    let radius = if bomb { parameters.bomb_radius } else { parameters.block_radius };
    let height = CAPSULE_HEIGHT * scale;
    let end = Vec3::new(0.0, height, 0.0);
    it_yaku::set_hurt_capsule(item, [end, end], radius as f32 * scale);
    it_yaku::set_hurt_enabled(item, true);
    it_yaku::set_ground_or_air(item, melee_types::GroundOrAir::Ground);
    id
}

fn anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// The initial blocks' material items, once the pool exists, in the order
/// grGreens_80214FA8 made them.
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
    scale: f32,
) {
    let SceneStage::Greens(greens) = stage else {
        return;
    };
    let parameters = greens.stage.parameters;
    let model = animations.get_mut(&BLOCKS_MAP).unwrap();
    for block in greens.stage.blocks.cells.iter_mut().flatten() {
        if block.status == melee_gr::greens::blocks::Status::None {
            continue;
        }
        let bind = greens.bind_anchors[block.slot];
        let id = block_item(items, gobjs, resources, map, None, bind, block.bomb, &parameters, scale);
        it_yaku::set_anchor(items.get_mut(id).unwrap(), anchor(model, FIRST_BONE + block.slot));
        block.item = Some(id);
    }
}

/// fn_80214658, deferred to the stage's start: the blocks begin and Whispy
/// wakes.
pub(crate) fn start(state: &mut InitialState) {
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    greens.stage.blocks.waiting = false;
    greens.stage.waiting = false;
}

/// The blocks' engine during the match.
struct Live<'a, 'b> {
    state: &'a mut InitialState,
    scene: &'a mut super::castle::Scene<'b>,
}

impl Live<'_, '_> {
    fn greens(&mut self) -> &mut Greens {
        let SceneStage::Greens(greens) = &mut self.state.stage else {
            unreachable!()
        };
        greens
    }
    fn model(&mut self) -> &mut BackgroundAnimation {
        self.state.stage_animations.get_mut(&BLOCKS_MAP).unwrap()
    }
}

impl BlockEngine for Live<'_, '_> {
    fn create(&mut self, slot: usize, bomb: bool) -> u32 {
        // grGreens_80213524(1 or 3): a GObj with no animation and empty procs.
        let address = self.state.effects.gobjs.create(hsd_gobj::GObjClass::Stage);
        self.greens().block_gobjs[slot] = Some(address);
        let model = self.model();
        model.set_joint_hidden_only(FIRST_BONE + slot, false);
        let anchor = anchor(model, FIRST_BONE + slot);
        let parameters = self.greens().stage.parameters;
        let scale = self.state.assets.stage_desc.parameters.map_scale;
        let assets = std::sync::Arc::clone(&self.state.assets);
        let state = &mut *self.state;
        block_item(
            &mut state.items,
            &mut state.effects.gobjs,
            &assets.items,
            &mut state.map,
            Some((&mut *self.scene.world, &mut *self.scene.item_objects)),
            anchor,
            bomb,
            &parameters,
            scale,
        )
    }
    fn place(&mut self, slot: usize, x: f32, y: f32, snap: bool) {
        let scale = self.state.assets.stage_desc.parameters.map_scale;
        let state = &mut *self.state;
        let model = state.stage_animations.get_mut(&BLOCKS_MAP).unwrap();
        place_bone(model, slot, x, y, scale);
        if snap {
            snap_joint(model, &mut state.map, slot);
        }
    }
    fn script_ended(&self, item: u32) -> bool {
        self.state
            .items
            .iter()
            .find(|i| i.id == item)
            .is_some_and(|i| i.script.instruction.is_none())
    }
    fn break_block(&mut self, slot: usize, item: u32, bomb: bool) {
        break_block(self.state, slot, item, bomb, false);
    }
    fn remove(&mut self, slot: usize, item: u32) {
        let address = self.greens().block_gobjs[slot].take();
        let gobjs = &mut self.state.effects.gobjs;
        let address = address.unwrap_or_else(|| gobjs.outside());
        gobjs.destroy(hsd_gobj::GObjClass::Stage, address);
        crate::scene_items::destroy_yaku(self.state, self.scene.world, self.scene.item_objects, item);
        self.model().set_joint_hidden_only(FIRST_BONE + slot, true);
    }
    fn blast_top(&self) -> f32 {
        self.state.assets.stage_camera.blast_top()
    }
    fn rng(&mut self) -> &mut HsdRng {
        &mut self.state.rng
    }
    fn stitch(&mut self, a: i32, b: i32) {
        self.state.map.stitch_joints(a, b);
    }
}

/// grGreens_802159B8's effects on a struck block: its hurt capsule off; a
/// bomb's GObj hidden, its item in state 1 (grMaterial_801C8D98) reaching
/// its kin (it_80275414) and its blast (efSync 1039); a block's Small quake
/// and its break (efSync 1032 with rotation 0). `late`: the strike came
/// from the item's hit proc (item link 14), past link 11's capsule
/// refresh, so the blast's hitbox is placed at once (it_802790C0) and
/// its efAsync dispatches at once.
fn break_block(state: &mut InitialState, _slot: usize, item: u32, bomb: bool, late: bool) {
    let assets = std::sync::Arc::clone(&state.assets);
    let yaku = assets.items.get(melee_types::ItemKind::PokemonRandom);
    let scale = assets.stage_desc.parameters.map_scale;
    let Some(core) = state.items.get_mut(item) else {
        return;
    };
    it_yaku::set_hurt_enabled(core, false);
    if bomb {
        let previous = (core.past_hitbox_refresh, core.efasync_immediate);
        (core.past_hitbox_refresh, core.efasync_immediate) = (late, late);
        it_yaku::change_state(core, 1, yaku);
        (core.past_hitbox_refresh, core.efasync_immediate) = previous;
        core.strikes_kindred_items = true;
    }
    let mut position = core.position;
    // retail 0x80215AC8 / 0x80215B4C: fmadds.
    position.y = gekko_math::fma::fmadds(CAPSULE_HEIGHT, scale, position.y);
    let result = if bomb {
        state.effects.spawn_positional::<RetailTrig>(
            BLAST_EFFECT,
            position,
            &assets.common_particle_bank,
            &mut state.particles,
            &mut state.rng,
        )
    } else {
        state.quakes.request(
            &mut state.camera,
            melee_cm::QuakeKind::Small,
            &mut state.effects.gobjs,
        );
        state.effects.spawn_block_break::<RetailTrig>(
            BREAK_EFFECT,
            position,
            0.0,
            &assets.common_particle_bank,
            &mut state.particles,
            &mut state.rng,
        )
    };
    if let Err(error) = result {
        panic!("grGreens_802159B8: {error}");
    }
}

/// A block's material item was hit (fn_80215B84, yaku.x18).
pub(crate) fn material_hit(state: &mut InitialState, item: u32) {
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    let Some((row, column)) = greens.stage.blocks.cell_of_item(item) else {
        return;
    };
    let block = greens.stage.blocks.cells[row][column];
    if block.hit || block.item.is_none() {
        return;
    }
    greens.stage.blocks.cells[row][column].hit = true;
    if block.bomb {
        greens.stage.blocks.cells[row][column].exploding = true;
    } else {
        greens.stage.blocks.cells[row][column].breaking = true;
    }
    break_block(state, block.slot, item, block.bomb, true);
}

/// fn_80216DE4 (mpJointSetCb2) after a collider's proc: a head met a block.
pub(crate) fn apply_bumps(state: &mut InitialState) {
    let events = state.map.take_ceiling_events();
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    for event in events.iter() {
        greens.stage.blocks.bump(event.joint);
    }
}

/// A Green Greens gobj proc (s_link 4). Map 6's lb_800115F4 runs in the
/// scene dispatch (it ends grGreens_8021479C; nothing before it reads the
/// field pool).
pub(crate) fn run_proc(
    state: &mut InitialState,
    map: u8,
    scene: &mut super::castle::Scene<'_>,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    match map {
        BLOCKS_MAP => blocks(state, scene),
        WHISPY_MAP => {
            whispy(state, scene, forces)?;
            // Generators on Whispy's JObjs read their world matrices in the
            // particle proc, after a new animation's frame zero.
            for map in [WHISPY_MAP, FACE_MAP] {
                let animation = state.stage_animations.get_mut(&map).unwrap();
                super::publish_joint_matrices(animation, map, &mut state.particles, &mut state.effects.events);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// grGreens_8021479C (0x8021479C).
fn blocks(state: &mut InitialState, scene: &mut super::castle::Scene<'_>) -> Result<()> {
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    let waiting = greens.stage.blocks.waiting;
    let parameters = greens.stage.parameters;
    let mut blocks = greens.stage.blocks.clone();
    if !waiting {
        let mut engine = Live { state, scene };
        blocks.step(&parameters, &mut engine);
    }
    let assets = std::sync::Arc::clone(&state.assets);
    let model = state.stage_animations.get_mut(&BLOCKS_MAP).unwrap();
    model.update_collision(&mut state.map, bindings(&assets));
    if !waiting {
        let mut engine = Live { state, scene };
        blocks.stitch(&mut engine);
    }
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    greens.stage.blocks = blocks;
    for block in greens.stage.blocks.cells.iter().flatten() {
        if let Some(id) = block.item {
            if block.status != melee_gr::greens::blocks::Status::None {
                let model = state.stage_animations.get_mut(&BLOCKS_MAP).unwrap();
                let anchor = anchor(model, FIRST_BONE + block.slot);
                if let Some(item) = state.items.get_mut(id) {
                    it_yaku::set_anchor(item, anchor);
                }
            }
        }
    }
    let model = state.stage_animations.get_mut(&BLOCKS_MAP).unwrap();
    super::publish_joint_matrices(model, BLOCKS_MAP, &mut state.particles, &mut state.effects.events);
    Ok(())
}

struct WhispyModel<'a, 'b> {
    state: &'a mut InitialState,
    scene: &'a mut super::castle::Scene<'b>,
    forces: &'a mut melee_lb::radial_force::RadialForces,
    error: Option<anyhow::Error>,
}

impl WhispyEngine for WhispyModel<'_, '_> {
    fn play(&mut self, animation: usize) {
        for map in [WHISPY_MAP, FACE_MAP] {
            if let Err(error) = select(self.state, map, animation, self.scene.draws) {
                self.error.get_or_insert(error);
            }
        }
    }
    fn ended(&self) -> bool {
        self.state.stage_animations[&WHISPY_MAP].first_animation_ended()
    }
    fn rewound(&self) -> bool {
        self.state.stage_animations[&WHISPY_MAP].first_animation_rewound()
    }
    fn rng(&mut self) -> &mut HsdRng {
        &mut self.state.rng
    }
    fn fighters_to_the_right(&mut self) -> bool {
        let position = self.state.stage_animations[&WHISPY_MAP].gobj_translation();
        crate::scene_items::facing_toward_fighters(position, &mut self.state.fighters, &mut self.state.rng) == 1.0
    }
    fn quake(&mut self) {
        let state = &mut *self.state;
        state.quakes.request(
            &mut state.camera,
            melee_cm::QuakeKind::Loop,
            &mut state.effects.gobjs,
        );
    }
    fn field(&mut self, gust: melee_lb::radial_force::DirectionalGust) {
        self.forces.insert_directional(gust);
    }
    /// it_802EE200 (802EE200): a draw for its kind and fate, its facing
    /// (HSD_Randi(2)), then the item. With items off (it_8026D3CC false)
    /// no healing apple comes and the draw loses x78; below x74 it stays,
    /// above it vanishes where it lands.
    fn apple(&mut self, position: Vec3) {
        let SceneStage::Greens(greens) = &self.state.stage else {
            unreachable!()
        };
        let p = greens.stage.parameters;
        let state = &mut *self.state;
        let mut random = state.rng.randf();
        let facing = if state.rng.randi(2) != 0 { 1.0 } else { -1.0 };
        if state.item_spawner.is_some() {
            unimplemented!("itwhispyapple.c:110: it_8026D3CC with items on (a healing apple)");
        }
        random -= p.apple_heal;
        if random < 0.0 {
            random = 0.0;
        }
        // Unordered counts as not below (a NaN draw vanishes).
        let vanishes = random.partial_cmp(&p.apple_kept) != Some(std::cmp::Ordering::Less);
        let assets = std::sync::Arc::clone(&state.assets);
        let bounds = crate::frame::item_bounds(&state.stage_limits);
        let mut rng = state.rng;
        crate::scene_items::request(
            &mut state.items,
            &mut state.effects.gobjs,
            &assets.items,
            &mut state.map,
            self.scene.world,
            self.scene.item_objects,
            melee_it::ItemRequest::Spawn(it_whispyapple::spawn(position, facing, vanishes)),
            crate::scene_items::RequestOwner {
                slot: None,
                secondary: false,
                held_item: None,
                after_hitbox_refresh: false,
                efasync_immediate: false,
                stale_multiplier: 1.0,
                bounds: Some(&bounds),
            },
            &mut rng,
        );
        state.rng = rng;
    }
}

/// grAnime_801C8138 on one of Whispy's maps, with its frame-zero keys.
fn select(state: &mut InitialState, map: u8, index: usize, draws: &mut DrawLog) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let animation = state.stage_animations.get_mut(&map).unwrap();
    // lb_8000B804: the rest pose first.
    animation.reset_pose();
    animation
        .select_animation(&assets.stage, &assets.stage_desc.models[usize::from(map)], index)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    spawn_keys(
        animation,
        map,
        &assets,
        &mut state.particles,
        Some(&mut state.effects.events),
        &mut state.rng,
        draws,
    )
}

/// grGreens_80213C10 (0x80213C10): the device's direction cleared, then
/// Whispy's step once the stage started.
fn whispy(
    state: &mut InitialState,
    scene: &mut super::castle::Scene<'_>,
    forces: &mut melee_lb::radial_force::RadialForces,
) -> Result<()> {
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    greens.stage.whispy.wind = 0;
    if greens.stage.waiting {
        return Ok(());
    }
    let parameters = greens.stage.parameters;
    let mut whispy = greens.stage.whispy;
    let mut engine = WhispyModel {
        state,
        scene,
        forces,
        error: None,
    };
    whispy.step(&parameters, &mut engine);
    if let Some(error) = engine.error {
        return Err(error);
    }
    let SceneStage::Greens(greens) = &mut state.stage else {
        unreachable!()
    };
    greens.stage.whispy = whispy;
    Ok(())
}

/// A slot's bone as a material item anchor (its bind pose before any
/// placement).
pub(crate) fn bind_anchor(model: &mut BackgroundAnimation, slot: usize) -> melee_it::YakuAnchor {
    anchor(model, FIRST_BONE + slot)
}
