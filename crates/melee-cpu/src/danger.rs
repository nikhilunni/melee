//! Escaping the stage's dangers and the screen's edge: ftCo_800A229C and
//! ftCo_800AE7AC, the head of every Versus CPU's decision.
use crate::{select, targets, world::Scene};
use gekko_math::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::Fighter;
use melee_types::{GrKind, GroundOrAir};

/// What ftCo_800A229C returns besides its point: 1 for a stage's own
/// hazard (Hyrule Castle's tornado, Brinstar's acid), 2 for a stage moving
/// away (Great Bay's turtle, Rainbow Cruise, Big Blue, Icicle Mountain),
/// -1 outside the camera bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Escape {
    pub kind: i32,
    /// Where to escape toward (the stage's point, or the fighter itself).
    pub point: Vec3,
}

/// The escape kind of a fighter outside the camera bounds.
const OFF_CAMERA: i32 = -1;

/// ftCo_800A229C (0x800A229C): whether the CPU must escape, and how.
pub fn escape_needed(fp: &Fighter, scene: &Scene) -> Option<Escape> {
    let stage = scene.stage();
    if matches!(
        stage,
        GrKind::GreatBay | GrKind::Castle | GrKind::Zebes | GrKind::RCruise | GrKind::BigBlue
            | GrKind::Icemt
    ) {
        unimplemented!("ftCo_800A229C: {stage:?}'s dangers");
    }
    // block_43: Camera_8003118C(&cur_pos, 0.0f).
    let position = fp.core.physics.position;
    if !scene.camera.contains(position) {
        return Some(Escape {
            kind: OFF_CAMERA,
            point: position,
        });
    }
    None
}

/// ftCo_800AE7AC (0x800AE7AC): escape. A stage hazard (kind 1) keeps the
/// attack, recovery and defence modes (xF9_b2, b4, b6); a moving stage or
/// the screen's edge drops them. Off camera, a grounded CPU heads for the
/// stage's centre (ftCo_800A6FC4) or its target.
pub fn escape(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng, escape: Escape) {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b0 = true;
    cpu.xf9_b3 = false;
    cpu.xf9_b5 = false;
    cpu.xf9_b7 = false;
    cpu.xf9_b1 = false;
    let keep = escape.kind <= 1;
    cpu.xf9_b2 = keep;
    cpu.xf9_b4 = keep;
    cpu.xf9_b6 = keep;
    fp.core.cpu.target = targets::nearest_opponent(fp, scene, rng);
    targets::update_common_item(fp, scene);
    targets::update_special_item(fp, scene);
    if crate::should_act(&mut fp.core.cpu) {
        if escape.kind >= 0 {
            unimplemented!("ftCo_800A8210: escaping toward a stage's point");
        }
        if fp.core.physics.ground_or_air != GroundOrAir::Air {
            unimplemented!("ftCo_800A6FC4: a grounded CPU off camera heads for the stage's centre");
        }
    }
    // ftCo_800AE7AC's frame under the think's.
    scene
        .stale
        .caller_frame
        .set(
            scene
                .stale
                .think_frame
                .get()
                .map(|frame| frame - crate::world::ESCAPE_FRAME_SIZE),
        );
    select::choose_behavior(fp, scene, rng);
}
