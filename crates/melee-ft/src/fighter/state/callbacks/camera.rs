//! The fighter's camera subject callbacks, s_link 18 (ftcamera.c and the
//! dead / revival states' `_Cam` rows).
use crate::fighter::assets::FighterAssets;
use crate::fighter::state::CameraPhase;
use crate::fighter::*;
use melee_cm::{StageCamera, Subject, SubjectState};

/// ftCamera_UpdateCameraBox: ordinary rows track the fighter's camera box.
pub fn follow_fighter(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    update(&mut fighter.core, phase, false);
}

/// ftCo_Cliff_Cam (80081644): the scheduler notifies the ledge after updating.
pub fn cliff(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    update(&mut fighter.core, phase, true);
}

/// ftCo_DeadDown_Cam / DeadLeft_Cam / DeadRight_Cam: the subject stays where
/// the fighter left the stage until the revival resets it.
pub fn hold(_fighter: &mut Fighter, _phase: CameraPhase<'_>) {}

/// ftCo_DeadUpStar_Cam / ftCo_DeadUpFall_Cam -> ftCamera_80076320
/// (0x80076320): track as usual, then pin the subject to the top blast
/// line, its x scaled from the camera top toward the blast top.
pub fn above_blast_zone(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    let stage = phase.stage;
    update(&mut fighter.core, phase, false);
    let centre_y = stage.offset_y;
    let blast_height = stage.blast_top() - centre_y;
    assert!(
        blast_height != 0.0,
        "stGetPlyDeadUp() - center_pos.y != 0.0F"
    );
    let camera_height = stage.top() - centre_y;
    let subject = &mut fighter.core.camera;
    subject.position.x = (subject.position.x * camera_height) / blast_height;
    subject.position.y = stage.blast_top();
}

/// ftCo_Rebirth_Cam (0x800D5A50 area): follow the platform's destination
/// (mv.co.common.x4) rather than the fighter; the extents stay as reset.
pub fn revival(fighter: &mut Fighter, phase: CameraPhase<'_>) {
    let core = &mut fighter.core;
    let MotionData::Life(life::LifeState::Revival { target, .. }) = core.state_data else {
        panic!("revival camera scratch")
    };
    let [h, _] = scaled_extents(phase.assets.camera_extents, core.player.scale);
    let height = h.x;
    core.camera.position = Vec3::new(target.x, target.y + height, 0.0);
    core.camera.bone_position = camera_bone(core);
}

/// ftLib_800866DC: the camera bone's world position.
fn camera_bone(fighter: &mut FighterCore) -> Vec3 {
    caches::bone_position(
        &mut fighter.skeleton,
        fighter.animation.root,
        fighter.attributes.camera.camera_zoom_target_bone as usize,
        fighter.attributes.camera.zoom_offset,
    )
}

/// ftCamera_80076018: the ftData camera box scaled by the fighter's y
/// scale, as `[horizontal, vertical]`. Separate fmuls (retail asm.py --fused).
fn scaled_extents(extents: [Vec3; 2], scale: f32) -> [Vec3; 2] {
    extents.map(|v| Vec3::new(v.x * scale, v.y * scale, v.z * scale))
}

/// The facing-dependent horizontal reach (ftCamera_UpdateCameraBox).
fn set_horizontal(subject: &mut Subject, horizontal: Vec3, facing: f32, fixed_zoom: f32) {
    if facing == 1.0 {
        subject.target_extents.left = horizontal.z;
        subject.target_extents.right = horizontal.y * fixed_zoom;
        subject.facing = 1.0;
    } else {
        subject.target_extents.left = -horizontal.y * fixed_zoom;
        subject.target_extents.right = -horizontal.z;
        subject.facing = -1.0;
    }
}

fn update(fighter: &mut FighterCore, phase: CameraPhase<'_>, on_ledge: bool) {
    let CameraPhase { assets, stage } = phase;
    let [h, _] = scaled_extents(assets.camera_extents, fighter.player.scale);
    set_horizontal(
        &mut fighter.camera,
        h,
        fighter.physics.facing,
        stage.fixed_zoom,
    );
    let position = fighter.physics.position;
    fighter.camera.position = Vec3::new(position.x, position.y + h.x, position.z);
    // ftCamera_UpdateCameraBox clears on_ledge; ftCo_Cliff_Cam sets it again
    // while the fighter hangs in the air.
    fighter.camera.on_ledge =
        on_ledge && fighter.physics.ground_or_air == melee_types::GroundOrAir::Air;
    fighter.camera.bone_position = camera_bone(fighter);
}

impl FighterCore {
    /// ftCamera_80076064 (0x80076064), from Fighter_UnkProcessDeath: make the
    /// subject active and snap its extents to the fighter's.
    pub(crate) fn reset_camera_subject(&mut self, assets: &FighterAssets, stage: &StageCamera) {
        let [h, v] = scaled_extents(assets.camera_extents, self.player.scale);
        let subject = &mut self.camera;
        subject.state = SubjectState::Active;
        set_horizontal(subject, h, self.physics.facing, stage.fixed_zoom);
        subject.target_extents.top = v.x;
        subject.target_extents.bottom = v.y;
        subject.target_extents.radius = v.z;
        subject.extents = subject.target_extents;
        let position = self.physics.position;
        subject.position = Vec3::new(position.x, position.y + h.x, position.z);
        subject.bone_position = subject.position;
    }
}
