//! Yoshi's Island composition (gryorster.c): the backdrop (map 0) and the
//! stage (map 1) in grYorster_80201FA4's order; the nine blocks' material
//! items, created when the stage starts; the blocks' hits and bumps.
use crate::{
    assets::Assets,
    initial_state::InitialState,
    scene_stage::SceneStage,
};
use anyhow::{bail, ensure, Result};
use hsd_types::Vec3;
use melee_ft::fighter::RetailTrig;
use melee_gr::{
    last::animation::BackgroundAnimation,
    yorster::{
        block_animation,
        procs::{
            YorsterStage, BLOCK_HURT_RADIUS, LOOP_ANIMATION, LOOP_BONES, MAP_ORDER,
            STAGE_ANIMATION, STAGE_MAP,
        },
        BlockEngine, BLOCK_BONES,
    },
};
use std::collections::BTreeMap;

pub(crate) type Animations = BTreeMap<u8, BackgroundAnimation>;

/// `coll->x34_flags.b1234` of a fighter's and an item's collision.
const FIGHTER_COLLISION: u8 = 1;

/// Ground_GetStageGObj (0x801C14D0): a map's model under its map-scale wrapper.
pub(crate) fn load(assets: &Assets, map: u8) -> Result<BackgroundAnimation> {
    let mut animation =
        BackgroundAnimation::load_model(&assets.stage, &assets.stage_desc.models[usize::from(map)])
            .map_err(|e| anyhow::anyhow!("Yoshi's Island map {map}: {e}"))?;
    animation.set_map_scale(assets.stage_desc.parameters.map_scale);
    if map == STAGE_MAP {
        // Every grAnime_801C7FF8 switch map 1 makes, decoded at load so the
        // blocks replay them without allocating.
        let model = &assets.stage_desc.models[usize::from(map)];
        let blocks = BLOCK_BONES.into_iter().flat_map(|bone| {
            [block_animation::SPIN, block_animation::REST, block_animation::BLOCKED]
                .map(|index| (bone, index))
        });
        for (bone, index) in LOOP_BONES.into_iter().map(|bone| (bone, LOOP_ANIMATION)).chain(blocks) {
            animation
                .prepare_subtree(&assets.stage, model, bone, index)
                .map_err(|e| anyhow::anyhow!("Yoshi's Island bone {bone} animation {index}: {e}"))?;
        }
    }
    Ok(animation)
}

/// Ground_801C32D4(1, bone) for each block: the collision joint GrYt.dat
/// binds to its bone (the last binding that names it).
fn block_joints(assets: &Assets) -> Result<[i32; 9]> {
    let bindings = &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings;
    let mut joints = [-1; 9];
    for (joint, bone) in joints.iter_mut().zip(BLOCK_BONES) {
        for binding in bindings {
            if binding.extra as usize == bone {
                *joint = i32::from(binding.joint_index);
            }
        }
        ensure!(*joint >= 0, "Yoshi's Island block bone {bone} has no collision joint");
    }
    Ok(joints)
}

/// The stage controller as grYorster_802021AC leaves it: waiting for the
/// stage's start, the lights from map 1's environment row.
pub(crate) fn controller(assets: &Assets) -> Result<YorsterStage> {
    let parameters = melee_gr::desc::read_yorster_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut stage = YorsterStage::new(parameters, block_joints(assets)?);
    stage.lights =
        melee_gr::battle::lights::load_model(&assets.stage, &assets.stage_desc, usize::from(STAGE_MAP))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(stage)
}

/// grAnime_801C7FF8(gobj, bone, 7, index, frame, rate) on map 1, from
/// the switches [`load`] prepared.
fn play(animation: &mut BackgroundAnimation, bone: usize, index: usize, frame: f32, rate: f32) {
    animation.play_prepared_subtree_at(bone, index, frame, rate);
}

/// grYorster_80201FA4 (0x80201FA4): maps 0 and 1. Map 0 has no animation
/// (grYorster_80202124's grAnime_801C8138 finds none); map 1
/// (grYorster_802021AC) binds its collision joints from the bind pose
/// (Ground_801C2ED0), plays animation 4 from frame zero and animation 3 on
/// bones 23 and 26, and defers the blocks to the stage's start
/// (Ground_801C10B8). Nothing draws: GrYt.dat has no particle keys.
pub(crate) fn initialize(
    assets: &Assets,
    collision: &mut melee_mp::CollMap,
) -> Result<(SceneStage, Animations)> {
    let mut animations = Animations::new();
    for map in MAP_ORDER {
        let mut animation = load(assets, map)?;
        if map == STAGE_MAP {
            let bindings = &assets.stage_desc.models[usize::from(map)].joint_mappings;
            animation.update_collision(collision, bindings);
            for binding in bindings {
                collision.joint_snapshot_prev_pos(i32::from(binding.joint_index));
            }
            animation
                .select_animation(
                    &assets.stage,
                    &assets.stage_desc.models[usize::from(map)],
                    STAGE_ANIMATION,
                )
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        ensure!(
            animation.evaluate_initial_frame::<RetailTrig>().is_empty(),
            "Yoshi's Island map {map}: unexpected setup particle key"
        );
        if map == STAGE_MAP {
            for bone in LOOP_BONES {
                play(&mut animation, bone, LOOP_ANIMATION, 0.0, 1.0);
            }
        }
        animations.insert(map, animation);
    }
    Ok((SceneStage::Yorster(Box::new(controller(assets)?)), animations))
}

/// it_2E6A_UnkMotion19_Phys's source for a block: its bone's world
/// translation, local scale and rotation.
fn block_anchor(model: &mut BackgroundAnimation, bone: usize) -> melee_it::YakuAnchor {
    let matrix = model.joint_matrix(bone);
    melee_it::YakuAnchor {
        position: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        scale: model.joint_scale(bone),
        rotation: model.joint_rotation(bone),
    }
}

/// grYorster_80202160 (0x80202160), deferred by Ground_801C10B8 and run by
/// Ground_801C0FB8 when the countdown releases the stage: the blocks
/// (grYorster_802022A4), Ground_801C2FE0, mpLib_80058560.
pub(crate) fn start(
    state: &mut InitialState,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_items::Objects,
) -> Result<()> {
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Yorster(stage) = &mut state.stage else {
        unreachable!()
    };
    ensure!(stage.waiting, "Yoshi's Island blocks created twice");
    let model = state.stage_animations.get_mut(&STAGE_MAP).unwrap();
    for (index, bone) in BLOCK_BONES.into_iter().enumerate() {
        // grMaterial_801C8CFC(0, 0, gp, jobj, NULL, grYorster_80202428, NULL).
        let anchor = block_anchor(model, bone);
        let id = crate::scene_items::spawn_yaku_live(
            &mut state.items,
            &mut state.effects.gobjs,
            &assets.items,
            &mut state.map,
            world,
            objects,
            anchor,
        );
        let item = state.items.get_mut(id).unwrap();
        it_yaku::set_ground_or_air(item, melee_types::GroundOrAir::Ground);
        // mpJointSetCb2(joint, gp, grYorster_802024F0).
        state.map.record_ceiling_events(stage.joints[index], true);
        it_yaku::set_hurt_capsule(item, [Vec3::ZERO; 2], BLOCK_HURT_RADIUS);
        it_yaku::set_hurt_enabled(item, true);
        play(model, bone, block_animation::REST, 0.0, 0.0);
        stage.items[index] = Some(id);
    }
    let bindings = &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings;
    model.update_collision(&mut state.map, bindings);
    state.map.stitch_all_joints();
    stage.waiting = false;
    Ok(())
}

struct Blocks<'a> {
    stage: &'a YorsterStage,
    model: &'a mut BackgroundAnimation,
    items: &'a mut melee_it::ItemPool,
    map: &'a mut melee_mp::CollMap,
    fighters: &'a [crate::scene_fighter::SceneFighter],
    error: Option<anyhow::Error>,
}
impl Blocks<'_> {
    fn item(&mut self, block: usize) -> &mut melee_it::ItemCore {
        let id = self.stage.items[block].expect("a block's material item");
        self.items.get_mut(id).expect("a block's material item")
    }
}
impl BlockEngine for Blocks<'_> {
    fn disable_hurt(&mut self, block: usize) {
        it_yaku::set_hurt_enabled(self.item(block), false);
    }
    fn enable_hurt(&mut self, block: usize) {
        it_yaku::set_hurt_enabled(self.item(block), true);
    }
    fn end_frame(&mut self, block: usize) -> f32 {
        self.model.subtree_joint_end_frame(BLOCK_BONES[block])
    }
    fn play(&mut self, block: usize, index: usize, frame: f32, rate: f32) {
        play(self.model, BLOCK_BONES[block], index, frame, rate);
    }
    fn rewound(&mut self, block: usize) -> bool {
        self.model.subtree_animation_rewound(BLOCK_BONES[block])
    }
    fn set_solid(&mut self, block: usize, solid: bool) {
        let joint = self.stage.joints[block];
        if solid {
            self.map.joint_list_add(joint);
        } else {
            self.map.joint_list_remove(joint);
        }
    }
    /// grLib_801C9EE8 (0x801C9EE8): any fighter, then any item but the
    /// stage material items, whose ECB box holds the block's local
    /// translation (HSD_JObjGetTranslation).
    fn occupied(&mut self, block: usize, offset: f32) -> bool {
        let point = self.model.joint_translation(BLOCK_BONES[block]);
        let inside = |cd: &melee_types::mp::CollData| {
            melee_gr::yorster::point_inside_ecb(cd.cur_pos, &cd.ecb, point, offset)
        };
        if self
            .fighters
            .iter()
            .any(|f| crate::scene_fighter::with_fighter!(f, |f| inside(&f.collision.data)))
        {
            return true;
        }
        let mut found = false;
        for item in self.items.iter() {
            if item.kind == melee_types::ItemKind::PokemonRandom || item.destroyed {
                continue;
            }
            match &item.collision {
                Some(cd) if inside(cd) => {
                    found = true;
                    break;
                }
                Some(_) => {}
                None => {
                    self.error.get_or_insert_with(|| {
                        anyhow::anyhow!("grLib_801C9EE8: {:?} has no collision data", item.kind)
                    });
                }
            }
        }
        found
    }
}

/// grYorster_80202254 (0x80202254), map 1's proc: the blocks once they
/// exist (grYorster_8020266C), then Ground_801C2FE0 (lb_800115F4 runs in
/// the scene dispatch). Each block's material item then follows its bone.
/// Map 0's grYorster_80202158 is empty.
pub(crate) fn run_proc(state: &mut InitialState, map: u8) -> Result<()> {
    if map != STAGE_MAP {
        return Ok(());
    }
    let assets = std::sync::Arc::clone(&state.assets);
    let SceneStage::Yorster(stage) = &mut state.stage else {
        unreachable!()
    };
    let model = state.stage_animations.get_mut(&STAGE_MAP).unwrap();
    if !stage.waiting {
        let parameters = stage.parameters;
        let map_scale = assets.stage_desc.parameters.map_scale;
        let mut blocks = std::mem::replace(&mut stage.blocks, BLOCK_BONES.map(melee_gr::yorster::Block::new));
        let mut engine = Blocks {
            stage,
            model,
            items: &mut state.items,
            map: &mut state.map,
            fighters: &state.fighters,
            error: None,
        };
        for (index, block) in blocks.iter_mut().enumerate() {
            block.step(index, &parameters, map_scale, &mut engine);
        }
        if let Some(error) = engine.error {
            return Err(error);
        }
        stage.blocks = blocks;
    }
    let bindings = &assets.stage_desc.models[usize::from(STAGE_MAP)].joint_mappings;
    model.update_collision(&mut state.map, bindings);
    if !stage.waiting {
        for (index, bone) in BLOCK_BONES.into_iter().enumerate() {
            let anchor = block_anchor(model, bone);
            let id = stage.items[index].expect("a block's material item");
            it_yaku::set_anchor(state.items.get_mut(id).expect("a block's item"), anchor);
        }
    }
    Ok(())
}

/// Who ran the collision whose ceiling calls are being applied.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Collider {
    Fighter(usize),
    Item,
    Other,
}

/// grYorster_802024F0 (0x802024F0), the blocks' ceiling callback, for
/// every call `collider`'s proc made: a fighter (not in Entry) bumping
/// with `pos_delta.y` at least `bump_speed` is frozen `bump_freeze`
/// frames (ftLib_80086A4C), and the block gathers the bump.
pub(crate) fn apply_bumps(state: &mut InitialState, collider: Collider) -> Result<()> {
    let events = state.map.take_ceiling_events();
    if events.is_empty() {
        return Ok(());
    }
    let SceneStage::Yorster(stage) = &mut state.stage else {
        unreachable!()
    };
    let parameters = stage.parameters;
    for event in events.iter() {
        let delta_y = match (event.collision_type, collider) {
            (FIGHTER_COLLISION, Collider::Fighter(index)) => {
                use melee_types::CommonMotionState as S;
                let fighter = &mut state.fighters[index];
                let (entering, delta) = crate::scene_fighter::with_fighter!(fighter, |f| (
                    matches!(f.motion_state.id, S::Entry | S::EntryStart | S::EntryEnd),
                    f.physics.position_delta.y
                ));
                if entering {
                    continue;
                }
                if delta >= parameters.bump_speed {
                    crate::scene_fighter::with_fighter!(fighter, |f| f
                        .combat
                        .freeze_request = parameters.bump_freeze as f32);
                }
                delta
            }
            (2 | 3, Collider::Item) => {
                if event.delta_y >= parameters.bump_speed {
                    bail!("grYorster_802024F0: an item's bump freezes it (it_8026B718)");
                }
                event.delta_y
            }
            (1..=3, collider) => bail!(
                "grYorster_802024F0: collision type {} from {collider:?}",
                event.collision_type
            ),
            _ => continue,
        };
        if let Some(block) = stage.block_of_joint(event.joint) {
            stage.blocks[block].bump += delta_y;
        }
    }
    Ok(())
}

/// grYorster_80202428 (0x80202428), the material items' hit-taken
/// callback: a fighter's hit freezes it `hit_freeze` frames, and the
/// block takes the damage.
pub(crate) fn material_hit(
    state: &mut InitialState,
    item: u32,
    fighter: Option<usize>,
    damage: i32,
) {
    let SceneStage::Yorster(stage) = &mut state.stage else {
        unreachable!()
    };
    if let Some(index) = fighter {
        let freeze = stage.parameters.hit_freeze as f32;
        crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
            .combat
            .freeze_request = freeze);
    }
    if let Some(block) = stage.block_of_item(item) {
        stage.blocks[block].damage += damage as f32;
    }
}
