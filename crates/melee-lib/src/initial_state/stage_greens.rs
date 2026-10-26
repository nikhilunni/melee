//! Green Greens at a saved boundary: maps 0, 2, 6, 5, 4 (and the blocks'
//! own GObjs), the block grid and Whispy as creation leaves them (only a
//! boundary from before the stage starts is imported).
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS, JOBJ_HIDDEN},
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        greens::{load, read_parameters, Animations, Greens},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::greens::{
    blocks::{spots, Block, Blocks, Status, BLOCK_MAP, BOMB_MAP, COLUMNS, FIRST_BONE, ROWS, SLOTS},
    procs::{GreensStage, BLOCKS_MAP, MAP_ORDER, WHISPY_MAP},
    whispy::Whispy,
};

/// Map 6: x0 flags (b0 set until the start), the grid and blocks pointers,
/// the timer.
const BLOCK_FLAGS: usize = 0xC4;
const BLOCK_CELLS: usize = 0xCC;
const BLOCK_TIMER: usize = 0xD0;
const BLOCK_SIZE: u32 = 0x20;
/// Map 5: `greens2`'s words from +0xC4.
const WHISPY: usize = 0xC4;

fn block(raw: &[u8]) -> Result<Block> {
    let bits = word(raw, 0);
    let status = match bits >> 28 {
        0 => Status::None,
        1 => Status::Falling,
        2 => Status::Stacked,
        3 => Status::Landed,
        other => anyhow::bail!("Green Greens block status {other}"),
    };
    let flag = |bit: u32| bits & (1 << bit) != 0;
    Ok(Block {
        status,
        slot: ((bits >> 23) & 0x1F) as usize,
        bomb: flag(22),
        exploding: flag(21),
        breaking: flag(20),
        bumped: flag(19),
        stitch: flag(18),
        emptied: flag(17),
        hit: flag(16),
        speed: float(raw, 4),
        y: float(raw, 8),
        item: None,
    })
}

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    ensure!(
        particles.generators.iter().all(|g| g.attachment_id.is_none()),
        "Green Greens joint-attached particles need attachment restoration"
    );
    let parameters = read_parameters(assets)?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds
        .iter()
        .map(|g| g.map)
        .filter(|&m| m != u32::from(BLOCK_MAP) && m != u32::from(BOMB_MAP))
        .collect();
    ensure!(maps == MAP_ORDER.map(u32::from), "Green Greens maps {maps:?}");
    let mut animations = Animations::new();
    let (mut blocks, mut whispy, mut bind_anchors) = (None, None, None);
    for ground in grounds.iter().filter(|g| MAP_ORDER.contains(&(g.map as u8))) {
        let key = ground.map as u8;
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        let raw = saved.bytes(ground.ground, 0x130);
        match key {
            BLOCKS_MAP => {
                ensure!(
                    raw[BLOCK_FLAGS] & 0x80 != 0,
                    "Green Greens' blocks at the boundary have started"
                );
                let mut anchors = [melee_it::YakuAnchor::default(); SLOTS];
                let mut positions = [(0.0, 0.0); SLOTS];
                let mut bind = load(assets, key)?;
                for slot in 0..SLOTS {
                    anchors[slot] = crate::scene_stage::greens::bind_anchor(&mut bind, slot);
                    positions[slot] = (anchors[slot].position.x, anchors[slot].position.y);
                }
                let mut grid = Blocks {
                    spots: spots(&positions),
                    cells: [[block(&[0; 0x20])?; COLUMNS]; ROWS],
                    timer: word(raw, BLOCK_TIMER) as i32,
                    waiting: true,
                };
                let base = word(raw, BLOCK_CELLS);
                for row in 0..ROWS {
                    for column in 0..COLUMNS {
                        let at = base + (row * COLUMNS + column) as u32 * BLOCK_SIZE;
                        grid.cells[row][column] = block(saved.bytes(at, BLOCK_SIZE as usize))?;
                    }
                }
                // The slots' bones as placed, their visibility as saved.
                for (slot, &jobj) in joints[FIRST_BONE..FIRST_BONE + SLOTS].iter().enumerate() {
                    let translate = saved.bytes(jobj + super::stage_izumi::JOBJ_TRANSLATE, 12);
                    animation.set_joint_translate(
                        FIRST_BONE + slot,
                        hsd_types::Vec3::new(float(translate, 0), float(translate, 4), float(translate, 8)),
                    );
                    let hidden = word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
                    animation.set_joint_hidden_only(FIRST_BONE + slot, hidden);
                }
                blocks = Some(grid);
                bind_anchors = Some(anchors);
            }
            WHISPY_MAP => {
                let w = |i: usize| word(raw, WHISPY + i * 4) as i32;
                whispy = Some(Whispy {
                    turn: w(0),
                    mode: w(1),
                    phase: w(2),
                    timer: w(3),
                    entering: w(4) != 0,
                    side: w(5),
                    wind: w(6),
                    apples: w(8),
                    apple_side: w(9),
                });
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let greens = Greens {
        stage: GreensStage {
            parameters,
            blocks: blocks.unwrap(),
            whispy: whispy.unwrap(),
            waiting: true,
        },
        block_gobjs: [None; SLOTS],
        bind_anchors: bind_anchors.unwrap(),
    };
    Ok((SceneStage::Greens(Box::new(greens)), animations))
}

/// Map 6's thirty joints from the saved matrices, their ceiling callbacks
/// recorded, then the saved vertices checked.
pub(super) fn restore_collision(
    saved: &SavedPose,
    assets: &Assets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let grounds = ground_list(saved)?;
    let ground = grounds
        .iter()
        .find(|g| g.map == u32::from(BLOCKS_MAP))
        .ok_or_else(|| anyhow::anyhow!("Green Greens map 6 missing"))?;
    let joints = model_joints(saved, ground.jobj);
    for binding in &assets.stage_desc.models[usize::from(BLOCKS_MAP)].joint_mappings {
        let jobj = joints[binding.extra as usize];
        let hidden = word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
        // Ground_801C2ED0 moved every joint before grGreens_80214FA8 hid
        // the unused slots' bones; a hidden joint keeps those vertices.
        let mtx = saved_mtx(saved, jobj);
        map.update_joint_transform(i32::from(binding.joint_index), Some(melee_mp::JobjState { mtx, hidden: false }));
        map.update_joint_transform(i32::from(binding.joint_index), Some(melee_mp::JobjState { mtx, hidden }));
        map.record_ceiling_events(i32::from(binding.joint_index), true);
    }
    restore_vertices(saved, map, "a block")
}
