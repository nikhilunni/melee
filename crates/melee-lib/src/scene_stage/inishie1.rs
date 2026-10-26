//! Mushroom Kingdom composition (grinishie1.c): the backdrop (map 0) and
//! the stage (map 3) in grInishie1_801FA90C's order; the bricks' material
//! items; breaking bricks and weighing the lifts.
use crate::{assets::Assets, initial_state::InitialState, scene_stage::SceneStage};
use anyhow::{ensure, Result};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    inishie1::{
        procs::{
            Inishie1Stage, BREAK_EFFECT, BREAK_SPEED, BRICKS, BRICK_HURT, LIFT_BONES,
            LIFT_JOINTS, MAP_ORDER, STAGE_BINDINGS, STAGE_MAP,
        },
        hatena::{HatenaEngine, HATENA_KEY_BASE, HATENA_MAP},
        Brick, BrickEngine, BrickPhase, LiftEngine, Lifts,
    },
    last::animation::BackgroundAnimation,
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// grLib_801C9EE8's offset around a returning brick.
const RETURN_ROOM: f32 = 15.0;

pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Mushroom Kingdom map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    Ok(animation)
}

/// Ground_801C2FE0's joints on map 3: the archive's, then grI1_803E48C8.
fn update_collision(model: &mut BackgroundAnimation, assets: &Assets, map: &mut melee_mp::CollMap) {
    model.update_collision(map, &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings);
    model.update_collision(map, &STAGE_BINDINGS);
}

/// The controller as grInishie1_801FAADC leaves it, from the models'
/// creation pose.
pub(crate) fn controller(assets: &Assets, model: &BackgroundAnimation) -> Result<Inishie1Stage> {
    let parameters = melee_gr::desc::read_inishie1_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Inishie1Stage {
        parameters,
        bricks: BRICKS.map(|(_, bone)| Brick::new(model.joint_translation(bone).y)),
        items: [None; 19],
        lifts: Lifts::new(model.joint_translation(LIFT_BONES[0]).y),
        hatena_waits: [0; 2],
        all_hatena_cooldown: 0,
        hatena_last: [-1; 2],
        hatena_objects: [None; 19],
        hatena_order: [0; 19],
        hatena_count: 0,
        music_decided: false,
        lights: melee_gr::battle::lights::load_model(
            &assets.stage,
            &assets.stage_desc,
            usize::from(STAGE_MAP),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?,
    })
}

/// grInishie1_801FA90C (0x801FA90C): map 0 (animation 0), then map 3
/// (grInishie1_801FAADC: Ground_801C2ED0, animation 0, the bricks and the
/// lifts). The bricks' items are made once the item pool exists
/// ([`spawn_items`]). `hatena`: grinishie1.c:465's test (items on outside
/// Sudden Death, a common item switched on): two bricks start as `?`
/// blocks, which draws.
pub(crate) fn initialize(
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
    hatena: bool,
    rng: &mut gekko_math::HsdRng,
) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        if map == STAGE_MAP {
            update_collision(&mut animation, assets, collision);
            let bindings = &assets.stage_desc.models[usize::from(map)].joint_mappings;
            for binding in bindings.iter().chain(&STAGE_BINDINGS) {
                collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
        }
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Mushroom Kingdom map {map}: unexpected setup particle key"
        );
        animations.insert(map, animation);
    }
    let mut stage = controller(assets, &animations[&STAGE_MAP])?;
    load_hatena_models(assets, &mut animations)?;
    if hatena {
        let mut engine = Creation {
            rng,
            animations: &mut animations,
            assets,
        };
        stage.place_initial_hatena(&mut engine);
    }
    // mpJointSetCb2 on every brick, mpJointSetCb1 on the lifts.
    for (joint, _) in BRICKS {
        collision.record_ceiling_events(joint, true);
    }
    for joint in LIFT_JOINTS {
        collision.record_joint_events(joint, true);
    }
    Ok((SceneStage::Inishie1(Box::new(stage)), animations))
}

/// How long [`load_hatena_models`] runs the `?` block's animation looking
/// for particle keys.
const HATENA_KEY_SCAN_TICKS: usize = 1200;

/// One `?` model per brick (map 2, under [`HATENA_KEY_BASE`]), loaded once
/// and hidden until its Ground exists (the tick path allocates nothing).
/// Its animation carries no particle keys, so its Ground only draws.
pub(crate) fn load_hatena_models(assets: &Assets, animations: &mut Animations) -> Result<()> {
    let mut probe = load(assets, HATENA_MAP)?;
    ensure!(
        probe.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "Mushroom Kingdom `?` block: a frame-zero particle key"
    );
    for _ in 0..HATENA_KEY_SCAN_TICKS {
        ensure!(
            probe.tick::<RetailTrig>().is_empty(),
            "Mushroom Kingdom `?` block: an animation particle key"
        );
    }
    for block in 0..BRICKS.len() as u8 {
        let mut model = load(assets, HATENA_MAP)?;
        model.clear_animation();
        model.set_hidden(true);
        animations.insert(HATENA_KEY_BASE + block, model);
    }
    Ok(())
}

/// grInishie1_801FBAA0's model half: the `?` Ground's on_init
/// (grInishie1_801FAC04: animation 0) over the brick's bone.
fn show_hatena_model(assets: &Assets, animations: &mut Animations, block: usize) -> Result<()> {
    let position = {
        let stage = animations.get_mut(&STAGE_MAP).unwrap();
        let matrix = stage.joint_matrix(BRICKS[block].1);
        Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
    };
    let model = animations.get_mut(&(HATENA_KEY_BASE + block as u8)).unwrap();
    model
        .recreate(&assets.stage, &assets.stage_desc.models[usize::from(HATENA_MAP)])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    model.set_hidden(false);
    ensure!(
        model.evaluate_initial_frame::<RetailTrig>().is_empty(),
        "Mushroom Kingdom `?` block: a frame-zero particle key"
    );
    model.set_gobj_translate(position);
    Ok(())
}

/// The `?` blocks during the stage's creation: their Grounds join the
/// scheduler with the stage's own (`Inishie1Stage::proc_table`).
struct Creation<'a> {
    rng: &'a mut gekko_math::HsdRng,
    animations: &'a mut Animations,
    assets: &'a Assets,
}
impl HatenaEngine for Creation<'_> {
    fn rng(&mut self) -> &mut gekko_math::HsdRng {
        self.rng
    }
    fn create(&mut self, block: usize) -> bool {
        show_hatena_model(self.assets, self.animations, block)
            .expect("the `?` block's model");
        false
    }
    fn remove(&mut self, _block: usize, _rank: Option<usize>) {
        unreachable!("no brick breaks during the stage's creation")
    }
    fn drop_item(&mut self, _block: usize) {
        unreachable!("no brick breaks during the stage's creation")
    }
}

/// A brick's material item follows its bone.
fn brick_anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// grInishie1_801FAD84's second loop: grMaterial_801C8CFC(8, 0, gp, jobj,
/// NULL, fn_801FBF6C, NULL) per brick, its capsule (0x801C8DE0), hits on
/// (0x801C8E08) and grounded (0x801C8E68).
pub(crate) fn spawn_items(
    stage: &mut SceneStage,
    animations: &mut Animations,
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
) {
    let SceneStage::Inishie1(stage) = stage else {
        return;
    };
    let model = animations.get_mut(&STAGE_MAP).unwrap();
    for (index, (_, bone)) in BRICKS.into_iter().enumerate() {
        let id = crate::scene_items::spawn_yaku(items, gobjs, resources, map, brick_anchor(model, bone));
        let item = items.get_mut(id).unwrap();
        let ([a, b], radius) = BRICK_HURT;
        it_yaku::set_hurt_capsule(item, [Vec3::new(a[0], a[1], a[2]), Vec3::new(b[0], b[1], b[2])], radius);
        it_yaku::set_hurt_enabled(item, true);
        it_yaku::set_ground_or_air(item, melee_types::GroundOrAir::Ground);
        stage.items[index] = Some(id);
    }
}

struct Stage<'a> {
    model: &'a mut BackgroundAnimation,
    items: &'a mut melee_it::ItemPool,
    item_ids: &'a [Option<u32>; 19],
    fighters: &'a [crate::scene_fighter::SceneFighter],
    blast_zone_bottom: f32,
    bindings: &'a [melee_gr::desc::JointMapping],
    map: &'a mut melee_mp::CollMap,
    error: Option<anyhow::Error>,
}
impl BrickEngine for Stage<'_> {
    /// grLib_801C9EE8(lb_8000B1CC(brick), 15.0).
    fn occupied(&mut self, brick: usize) -> bool {
        let matrix = self.model.joint_matrix(BRICKS[brick].1);
        let point = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        let inside = |cd: &melee_types::mp::CollData| {
            melee_gr::yorster::point_inside_ecb(cd.cur_pos, &cd.ecb, point, RETURN_ROOM)
        };
        if self
            .fighters
            .iter()
            .any(|f| crate::scene_fighter::with_fighter!(f, |f| inside(&f.collision.data)))
        {
            return true;
        }
        for item in self.items.iter() {
            if item.kind == melee_types::ItemKind::PokemonRandom || item.destroyed {
                continue;
            }
            match &item.collision {
                Some(cd) if inside(cd) => return true,
                Some(_) => {}
                None => {
                    self.error.get_or_insert_with(|| {
                        anyhow::anyhow!("grLib_801C9EE8: {:?} has no collision data", item.kind)
                    });
                }
            }
        }
        false
    }
    /// HSD_JObjClearFlagsAll(brick, JOBJ_HIDDEN) and grMaterial_801C8E08.
    fn show(&mut self, brick: usize) {
        self.model.set_joint_hidden(BRICKS[brick].1, false);
        let id = self.item_ids[brick].expect("a brick's item");
        it_yaku::set_hurt_enabled(self.items.get_mut(id).unwrap(), true);
    }
}
impl LiftEngine for Stage<'_> {
    fn set_lift_y(&mut self, lift: usize, y: f32) {
        self.model.set_joint_translate_y(LIFT_BONES[lift], y);
    }
    fn drop_children(&mut self, dy: f32) {
        for bone in LIFT_BONES {
            let child = bone + 1;
            let y = self.model.joint_translation(child).y;
            self.model.set_joint_translate_y(child, y + dy);
        }
    }
    fn child_heights(&mut self) -> [f32; 2] {
        LIFT_BONES.map(|bone| self.model.joint_matrix(bone + 1).0[1][3])
    }
    fn reset_children(&mut self) {
        for bone in LIFT_BONES {
            self.model.set_joint_translate_y(bone + 1, 0.0);
        }
        // mpLib_80055E9C(0x14), (0x15), then mpLib_80057424 on each.
        for binding in self.bindings {
            if LIFT_JOINTS.contains(&i32::from(binding.joint_index)) {
                self.model
                    .update_collision(self.map, std::slice::from_ref(binding));
            }
        }
        for joint in LIFT_JOINTS {
            self.map.joint_snapshot_prev_pos(joint);
        }
    }
    fn blast_zone_bottom(&self) -> f32 {
        self.blast_zone_bottom
    }
}

/// grInishie1_801FAB68 (0x801FAB68), map 3's proc: the bricks
/// (grInishie1_801FB3F0), the lifts (801FC664), the music (801FCB10, sound
/// only), then Ground_801C2FE0 (lb_800115F4 runs in the scene dispatch).
pub(crate) fn run_proc(state: &mut InitialState, map: u8, scheduler: Scheduler<'_>) -> Result<()> {
    if map >= HATENA_KEY_BASE {
        step_hatena_object(state, usize::from(map - HATENA_KEY_BASE));
        return Ok(());
    }
    if map != STAGE_MAP {
        return Ok(());
    }
    let assets = std::sync::Arc::clone(&state.assets);
    // fn_801FC9AC's calls since the last step.
    // Both joints' calls come from one fixed event list, so they fit in it.
    let mut weighed = state.map.take_joint_events(LIFT_JOINTS[0]);
    for event in state.map.take_joint_events(LIFT_JOINTS[1]).iter() {
        weighed.push(*event);
    }
    // grInishie1_801FB3F0's prologue: the `?` blocks' waits.
    with_live(state, scheduler, |stage, live| stage.step_hatena(live))?;
    let SceneStage::Inishie1(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    for event in weighed.iter() {
        let lift = if event.joint == LIFT_JOINTS[0] { 0 } else { 1 };
        stage.lifts.weigh(lift, event.ground_kind, event.x50, &parameters);
    }
    let model = state.stage_animations.get_mut(&STAGE_MAP).unwrap();
    let mut engine = Stage {
        model,
        items: &mut state.items,
        item_ids: &stage.items,
        fighters: &state.fighters,
        blast_zone_bottom: assets.arena.bottom,
        bindings: &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings,
        map: &mut state.map,
        error: None,
    };
    for (index, brick) in stage.bricks.iter_mut().enumerate() {
        // grinishie1.c:704 / 723: a `?` model over a broken brick hides,
        // over a returning one blinks with it (drawing only).
        if let Some(object) = stage.hatena_objects[index].as_mut() {
            match brick.phase {
                BrickPhase::Broken => object.hidden = true,
                BrickPhase::Returning if brick.flicker > 0 => {
                    object.hidden = brick.flicker & 1 == 0;
                }
                _ => {}
            }
        }
        brick.step(index, &parameters, &mut engine);
    }
    stage.lifts.step(&parameters, &mut engine);
    if let Some(error) = engine.error {
        return Err(error);
    }
    let Stage { model, map, items, .. } = engine;
    update_collision(model, &assets, map);
    for (index, (_, bone)) in BRICKS.into_iter().enumerate() {
        let id = stage.items[index].expect("a brick's item");
        it_yaku::set_anchor(items.get_mut(id).unwrap(), brick_anchor(model, bone));
    }
    Ok(())
}

/// grInishie1_801FAC58 (0x801FAC58), a `?` Ground's proc: its blink, and
/// its model over the brick (drawing only).
fn step_hatena_object(state: &mut InitialState, block: usize) {
    let SceneStage::Inishie1(stage) = &mut state.stage else {
        unreachable!()
    };
    let Some(object) = stage.hatena_objects[block].as_mut() else {
        return;
    };
    object.step();
    let hidden = object.hidden;
    let matrix = state
        .stage_animations
        .get_mut(&STAGE_MAP)
        .unwrap()
        .joint_matrix(BRICKS[block].1);
    let model = state
        .stage_animations
        .get_mut(&(HATENA_KEY_BASE + block as u8))
        .unwrap();
    model.set_gobj_translate(Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]));
    model.set_hidden(hidden);
}

/// The scheduler a `?` Ground and a dropped item join.
pub(crate) struct Scheduler<'a> {
    pub(crate) world: &'a mut hsd_gobj::TaggedWorld,
    pub(crate) item_objects: &'a mut crate::scene_items::Objects,
    pub(crate) stage_objects: &'a mut crate::scene_stage::StageObjects,
}

/// The `?` blocks during the match.
struct Live<'a> {
    rng: &'a mut gekko_math::HsdRng,
    animations: &'a mut Animations,
    assets: &'a Assets,
    items: &'a mut melee_it::ItemPool,
    gobjs: &'a mut hsd_gobj::GObjAddresses,
    stage_gobjs: &'a mut crate::scene_stage::StageGObjs,
    map: &'a mut melee_mp::CollMap,
    fighters: &'a mut [crate::scene_fighter::SceneFighter],
    spawner: Option<melee_it::random_spawn::RandomItemSpawner>,
    scheduler: Scheduler<'a>,
    error: Option<anyhow::Error>,
}

/// Run `f` on the controller with the rest of the scene as its engine.
fn with_live<R>(
    state: &mut InitialState,
    scheduler: Scheduler<'_>,
    f: impl FnOnce(&mut Inishie1Stage, &mut Live<'_>) -> R,
) -> Result<R> {
    let InitialState {
        stage,
        stage_animations,
        assets,
        items,
        effects,
        stage_gobjs,
        map,
        fighters,
        item_spawner,
        rng,
        ..
    } = state;
    let SceneStage::Inishie1(stage) = stage else {
        unreachable!()
    };
    let mut live = Live {
        rng,
        animations: stage_animations,
        assets,
        items,
        gobjs: &mut effects.gobjs,
        stage_gobjs,
        map,
        fighters,
        spawner: *item_spawner,
        scheduler,
        error: None,
    };
    let result = f(stage, &mut live);
    match live.error {
        Some(error) => Err(error),
        None => Ok(result),
    }
}

impl HatenaEngine for Live<'_> {
    fn rng(&mut self) -> &mut gekko_math::HsdRng {
        self.rng
    }
    /// Ground_GetStageGObj(2): the GObj, Ground_801C1CD0 and 801C1D38, the
    /// on_init, then grInishie1_801FAC58.
    fn create(&mut self, block: usize) -> bool {
        let key = HATENA_KEY_BASE + block as u8;
        let object = self.scheduler.world.create(3, 5, 0);
        self.stage_gobjs.create(self.gobjs, HATENA_MAP);
        for (index, link) in [1, 4, 4].into_iter().enumerate() {
            self.scheduler.world.add_tagged_proc(
                object,
                link,
                super::last::TAG_BASE + usize::from(key) * 3 + index,
            );
        }
        self.scheduler.stage_objects[usize::from(key)] = Some(object);
        if let Err(error) = show_hatena_model(self.assets, self.animations, block) {
            self.error.get_or_insert(error);
        }
        true
    }
    /// Ground_801C4A08 (0x801C4A08): the GObj freed (map 2 binds no
    /// collision joints).
    fn remove(&mut self, block: usize, rank: Option<usize>) {
        let key = HATENA_KEY_BASE + block as u8;
        self.stage_gobjs
            .destroy_nth(self.gobjs, HATENA_MAP, rank.unwrap_or(usize::MAX));
        if let Some(object) = self.scheduler.stage_objects[usize::from(key)].take() {
            self.scheduler.world.destroy(object);
        }
        let model = self.animations.get_mut(&key).unwrap();
        model.clear_animation();
        model.set_hidden(true);
    }
    /// grInishie1_801FBA34 (0x801FBA34) -> it_8026F7C8 (0x8026F7C8): a kind
    /// from the contents table (it_8026C75C on it_804A0E50), created five
    /// above the brick (it_8026F5C8 with no container), launched.
    fn drop_item(&mut self, block: usize) {
        // gm_8016B238 (Sudden Death) and an empty table make no spawner.
        let Some(spawner) = self.spawner else {
            return;
        };
        let poke_balls = self
            .items
            .iter()
            .filter(|item| item.kind == melee_types::ItemKind::MBall)
            .count();
        let limit = self.items.common().hold_limits[9].unwrap_or(0);
        let barred = !spawner.switched_on(melee_types::ItemKind::MBall) || poke_balls >= limit;
        if barred && poke_balls > 0 {
            unimplemented!("it_8026C75C: Item_804A0C64.x1C counts Poke Balls (item.c:408)");
        }
        let Some(kind) = spawner.contents.pick(barred, self.rng) else {
            return;
        };
        if kind == melee_types::ItemKind::MBall {
            unimplemented!("grInishie1_801FBA34: a Poke Ball from a `?` block needs a witness");
        }
        let matrix = self
            .animations
            .get_mut(&STAGE_MAP)
            .unwrap()
            .joint_matrix(BRICKS[block].1);
        // retail 0x801FBA60: fadds, five up; it_8026F5C8 zeroes z.
        let position = Vec3::new(matrix.0[0][3], matrix.0[1][3] + HATENA_DROP_HEIGHT, 0.0);
        let facing = crate::scene_items::facing_toward_fighters(position, self.fighters, self.rng);
        let spawned = crate::scene_items::spawn_released_item(
            self.items,
            self.gobjs,
            &self.assets.items,
            self.map,
            self.scheduler.world,
            self.scheduler.item_objects,
            crate::scene_items::Released {
                kind,
                position,
                previous_position: position,
                facing,
                attack: None,
            },
            self.rng,
        );
        if let Some(id) = spawned {
            // retail 0x8026F84C..5C: fsubs then two separate fmuls; the
            // kind's x18 upward.
            let spread = self.items.common().random_items.release_spread;
            let random = self.rng.randf();
            let rise = self.assets.items.get(kind).launch_vertical_velocity;
            self.items.get_mut(id).expect("the dropped item").velocity =
                Vec3::new(spread * (2.0 * (random - 0.5)), rise, 0.0);
            // it_80274ED8: the spawn count nothing in a match reads.
        }
    }
}

/// grInishie1_801FBA34: an item drops this far above its brick.
const HATENA_DROP_HEIGHT: f32 = 5.0;

/// grInishie1_801FB0AC and 801FBCEC (0x801FBCEC): the brick breaks.
fn break_brick(state: &mut InitialState, brick: usize, scheduler: Scheduler<'_>) -> Result<()> {
    with_live(state, scheduler, |stage, live| stage.break_hatena(brick, live))?;
    let SceneStage::Inishie1(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    stage.bricks[brick].break_apart(&parameters, &mut state.rng);
    let id = stage.items[brick].expect("a brick's item");
    let item = state.items.get_mut(id).unwrap();
    it_yaku::set_hurt_enabled(item, false);
    let position = item.position;
    let model = state.stage_animations.get_mut(&STAGE_MAP).unwrap();
    model.set_joint_hidden(BRICKS[brick].1, true);
    state.effects.spawn_positional::<RetailTrig>(
        BREAK_EFFECT,
        position,
        &state.assets.common_particle_bank,
        &mut state.particles,
        &mut state.rng,
    )?;
    // Ground_801C5414(0x136, 0xBA): sound. Camera_RequestQuake(Small).
    state.quakes.request(
        &mut state.camera,
        melee_cm::QuakeKind::Small,
        &mut state.effects.gobjs,
    );
    Ok(())
}

/// fn_801FBEB8 (0x801FBEB8), the bricks' ceiling callback: a contact
/// faster than 0.7 breaks the brick, whoever made it.
pub(crate) fn apply_bumps(
    state: &mut InitialState,
    collider: super::yorster::Collider,
    drew: bool,
    scheduler: Scheduler<'_>,
) -> Result<()> {
    let events = state.map.take_ceiling_events();
    let Scheduler {
        world,
        item_objects,
        stage_objects,
    } = scheduler;
    for event in events.iter() {
        if f64::from(if event.delta_y < 0.0 { -event.delta_y } else { event.delta_y }) > BREAK_SPEED {
            // Retail breaks the brick inside the collider's own map pass
            // (mpCollEnd's callback), ahead of what its collision does next;
            // the scene applies it after the proc. That order is the same
            // unless the item's proc drew too (a bounce spark's draws).
            if matches!(collider, super::yorster::Collider::Item(_)) && drew {
                unimplemented!(
                    "fn_801FBEB8 from an item's own map pass: the break precedes the rest of its collision"
                );
            }
            let brick = Inishie1Stage::brick_of_joint(event.joint)
                .ok_or_else(|| anyhow::anyhow!("grinishie1.c:496: joint {}", event.joint))?;
            break_brick(
                state,
                brick,
                Scheduler {
                    world: &mut *world,
                    item_objects: &mut *item_objects,
                    stage_objects: &mut *stage_objects,
                },
            )?;
        }
    }
    Ok(())
}

/// fn_801FBF6C (0x801FBF6C), the bricks' hit-taken callback: any hit
/// breaks the brick.
pub(crate) fn material_hit(
    state: &mut InitialState,
    item: u32,
    scheduler: Scheduler<'_>,
) -> Result<()> {
    let SceneStage::Inishie1(stage) = &state.stage else {
        return Ok(());
    };
    if let Some(brick) = stage.brick_of_item(item) {
        break_brick(state, brick, scheduler)?;
    }
    Ok(())
}
