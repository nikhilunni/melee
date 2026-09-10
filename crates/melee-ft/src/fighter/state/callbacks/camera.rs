//! ftCamera_UpdateCameraBox and ftCo_Cliff_Cam, s_link 18.
use crate::fighter::state::CameraPhase;
use crate::fighter::*;

/// ftCamera_UpdateCameraBox: ordinary rows track the fighter's camera box.
pub fn follow_fighter(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    update(&mut fighter.core, phase, false);
}

/// ftCo_Cliff_Cam (80081644): the scheduler notifies the ledge after updating.
pub fn cliff(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    update(&mut fighter.core, phase, true);
}

fn update(fighter: &mut FighterCore, phase: CameraPhase<'_>, on_ledge: bool) {
    let CameraPhase {
        assets,
        zoom: fixed_zoom,
    } = phase;
    fighter.camera.on_ledge =
        on_ledge && fighter.physics.ground_or_air == melee_types::GroundOrAir::Air;
    // ftCamera_80076018 / ftCamera_UpdateCameraBox: separate fmuls/fadds,
    // no contraction (retail asm.py --fused).
    let [h, v] = assets.camera_extents;
    let scale = fighter.player.scale;
    let h = Vec3::new(h.x * scale, h.y * scale, h.z * scale);
    fighter.camera.vertical = Vec3::new(v.x * scale, v.y * scale, v.z * scale);
    fighter.camera.horizontal = if fighter.physics.facing == 1.0 {
        Vec2::new(h.z, h.y * fixed_zoom)
    } else {
        Vec2::new(-h.y * fixed_zoom, -h.z)
    };
    fighter.camera.facing = fighter.physics.facing;
    let position = fighter.physics.position;
    fighter.camera.position = Vec3::new(position.x, position.y + h.x, position.z);
    fighter.camera.bone_position = caches::bone_position(
        &mut fighter.skeleton,
        fighter.animation.root,
        fighter.attributes.camera.camera_zoom_target_bone as usize,
        fighter.attributes.camera.zoom_offset,
    );
}
