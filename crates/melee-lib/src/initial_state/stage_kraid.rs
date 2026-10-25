//! Brinstar Depths at a saved boundary: the turn (`grKraid_GroundVars` on
//! map 3), Kraid (`grKraid_GroundVars2` on map 4), the effects request
//! (grKr_804D6A0C), the second port's pad buttons, the models, and the
//! stage's six collision joints.
use super::{
    float,
    saved_pose::SavedPose,
    stage_izumi::{
        ground_list, model_joints, restore_animation, restore_vertices, saved_mtx, JOBJ_FLAGS,
        JOBJ_HIDDEN,
    },
    word,
};
use crate::{
    assets::Assets,
    scene_stage::{
        kraid::{load, Animations},
        SceneStage,
    },
};
use anyhow::{ensure, Result};
use hsd_particle::system::ParticleSystem;
use melee_gr::kraid::{
    procs::{KraidStage, EFFECTS_MAP, KRAID_MAP, MAP_ORDER, STAGE_BINDINGS, STAGE_MAP},
    Kraid, KraidPhase, Turn,
};

/// The Ground's `u` union.
const VARS: usize = 0xC4;
/// grKr_804D6A0C: grKraid_801FE2D0's request.
const EFFECT_REQUEST: u32 = 0x804D_6A0C;
/// HSD_PadMasterStatus[1].button (0x804C1FAC + 0x44).
const SECOND_PORT_BUTTONS: u32 = 0x804C_1FAC + 0x44;

fn saved_turn(raw: &[u8]) -> Turn {
    Turn {
        turning: raw[VARS] as i8 != 0,
        shaking: raw[VARS + 1] as i8 != 0,
        direction: float(raw, VARS + 4),
        speed: float(raw, VARS + 8),
        braking: float(raw, VARS + 0xC),
        delay: word(raw, VARS + 0x10) as i32,
    }
}

fn saved_kraid(raw: &[u8]) -> Result<Kraid> {
    let byte = |at: usize| raw[VARS + at] as i8;
    Ok(Kraid {
        phase: KraidPhase::from_saved(byte(0))
            .ok_or_else(|| anyhow::anyhow!("Brinstar Depths Kraid phase {}", byte(0)))?,
        animation: byte(1),
        place: byte(2),
        leftmost: byte(3),
        rightmost: byte(4),
        anywhere: byte(5) != 0,
        side: float(raw, VARS + 8),
        timer: word(raw, VARS + 0xC) as i32,
    })
}

pub(super) fn restore(
    saved: &SavedPose,
    assets: &Assets,
    particles: &ParticleSystem,
) -> Result<(SceneStage, Animations)> {
    ensure!(
        particles
            .generators
            .iter()
            .all(|g| g.attachment_id.is_none()),
        "Brinstar Depths joint-attached particles need attachment restoration"
    );
    let parameters = melee_gr::desc::read_kraid_parameters(&assets.stage)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let grounds = ground_list(saved)?;
    let maps: Vec<u32> = grounds.iter().map(|g| g.map).collect();
    ensure!(
        maps == MAP_ORDER.map(u32::from),
        "Brinstar Depths map order {maps:?}"
    );
    let mut animations = Animations::new();
    let (mut turn, mut kraid) = (None, None);
    for ground in &grounds {
        let key = ground.map as u8;
        let raw = saved.bytes(ground.ground, 0x108);
        let mut animation = load(assets, key)?;
        let joints = model_joints(saved, ground.jobj);
        restore_animation(saved, &mut animation, &joints, ground.map)?;
        let hidden = word(saved.bytes(ground.jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0;
        match key {
            STAGE_MAP => {
                let saved_turn = saved_turn(raw);
                // x10 is uninitialized until a turn draws it.
                ensure!(
                    Turn {
                        delay: 0,
                        ..saved_turn
                    } == Turn::new(),
                    "Brinstar Depths turn {saved_turn:?} at the boundary needs its clocks"
                );
                turn = Some(saved_turn);
            }
            KRAID_MAP => {
                let saved_kraid = saved_kraid(raw)?;
                ensure!(
                    saved_kraid.phase == KraidPhase::Hidden && hidden,
                    "Brinstar Depths Kraid {:?} at the boundary needs his clock",
                    saved_kraid.phase
                );
                animation.set_hidden(true);
                kraid = Some(saved_kraid);
            }
            EFFECTS_MAP => {
                ensure!(hidden, "Brinstar Depths effects shown at the boundary");
                animation.set_hidden(true);
            }
            _ => {}
        }
        animations.insert(key, animation);
    }
    let stage = KraidStage {
        parameters,
        turn: turn.unwrap(),
        kraid: kraid.unwrap(),
        effect_request: word(saved.bytes(EFFECT_REQUEST, 4), 0) as i32,
        second_port_buttons: word(saved.bytes(SECOND_PORT_BUTTONS, 4), 0),
    };
    Ok((SceneStage::Kraid(Box::new(stage)), animations))
}

/// The stage's joints as the last Ground_801C2FE0 left them, from the saved
/// bone 12 matrix. Runs before fighters read the map.
pub(super) fn restore_collision(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let grounds = ground_list(saved)?;
    let stage = grounds
        .iter()
        .find(|g| g.map == u32::from(STAGE_MAP))
        .ok_or_else(|| anyhow::anyhow!("Brinstar Depths map 3 missing"))?;
    let joints = model_joints(saved, stage.jobj);
    for binding in &STAGE_BINDINGS {
        let jobj = joints[binding.extra as usize];
        let state = melee_mp::JobjState {
            mtx: saved_mtx(saved, jobj),
            hidden: word(saved.bytes(jobj + JOBJ_FLAGS, 4), 0) & JOBJ_HIDDEN != 0,
        };
        map.update_joint_transform(i32::from(binding.joint_index), Some(state));
    }
    restore_vertices(saved, map, "the turning stage")
}
