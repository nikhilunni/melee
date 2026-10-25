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
        Brick, BrickEngine, LiftEngine, Lifts,
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
/// ([`spawn_items`]). `items_on`: gm_8016AE80() != -1.
pub(crate) fn initialize(
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
    items_on: bool,
) -> Result<(SceneStage, Animations)> {
    ensure!(
        !items_on,
        "grinishie1.c:465: Mushroom Kingdom's `?` blocks with items on are not ported"
    );
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
    let stage = controller(assets, &animations[&STAGE_MAP])?;
    // mpJointSetCb2 on every brick, mpJointSetCb1 on the lifts.
    for (joint, _) in BRICKS {
        collision.record_ceiling_events(joint, true);
    }
    for joint in LIFT_JOINTS {
        collision.record_joint_events(joint, true);
    }
    Ok((SceneStage::Inishie1(Box::new(stage)), animations))
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
        let lifts: Vec<_> = self
            .bindings
            .iter()
            .filter(|b| LIFT_JOINTS.contains(&i32::from(b.joint_index)))
            .cloned()
            .collect();
        self.model.update_collision(self.map, &lifts);
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
pub(crate) fn run_proc(state: &mut InitialState, map: u8) -> Result<()> {
    if map != STAGE_MAP {
        return Ok(());
    }
    let assets = std::sync::Arc::clone(&state.assets);
    // fn_801FC9AC's calls since the last step.
    let mut weighed = Vec::new();
    for joint in LIFT_JOINTS {
        for event in state.map.take_joint_events(joint).iter() {
            weighed.push(*event);
        }
    }
    let SceneStage::Inishie1(stage) = &mut state.stage else {
        unreachable!()
    };
    ensure!(
        stage.hatena_waits == [0; 2] && stage.all_hatena_cooldown == 0,
        "grinishie1.c:623: `?` blocks are not ported"
    );
    let parameters = stage.parameters;
    for event in &weighed {
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

/// grInishie1_801FB0AC and 801FBCEC (0x801FBCEC): the brick breaks.
fn break_brick(state: &mut InitialState, brick: usize) -> Result<()> {
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
pub(crate) fn apply_bumps(state: &mut InitialState) -> Result<()> {
    let events = state.map.take_ceiling_events();
    for event in events.iter() {
        if f64::from(if event.delta_y < 0.0 { -event.delta_y } else { event.delta_y }) > BREAK_SPEED {
            let brick = Inishie1Stage::brick_of_joint(event.joint)
                .ok_or_else(|| anyhow::anyhow!("grinishie1.c:496: joint {}", event.joint))?;
            break_brick(state, brick)?;
        }
    }
    Ok(())
}

/// fn_801FBF6C (0x801FBF6C), the bricks' hit-taken callback: any hit
/// breaks the brick.
pub(crate) fn material_hit(state: &mut InitialState, item: u32) -> Result<()> {
    let SceneStage::Inishie1(stage) = &state.stage else {
        return Ok(());
    };
    if let Some(brick) = stage.brick_of_item(item) {
        break_brick(state, brick)?;
    }
    Ok(())
}
