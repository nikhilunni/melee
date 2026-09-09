//! FD runtime Ground views (gr/types.h: Map_GroundVars, Last_GroundVars).
use super::{float, saved_pose::SavedPose, word};
use crate::assets::Assets;
use anyhow::{ensure, Result};
use gekko_math::HsdRng;
use melee_gr::{
    ground::Phase,
    last::{background::BackgroundMotion, FinalDestination},
};

pub(super) fn restore(saved: &SavedPose, assets: &Assets) -> Result<FinalDestination> {
    // Cold initialization builds owned defaults only. Its four draws are private
    // and never enter the restored match's shared stream.
    let mut stage = FinalDestination::initialize(&assets.stage_desc, &mut HsdRng::new(0));
    stage.actions.clear();
    stage.ground.live_maps.fill(false);
    let entities = word(saved.bytes(0x804D_782C, 4), 0);
    let mut gobj = word(saved.bytes(entities + 5 * 4, 4), 0);
    let mut maps = Vec::new();
    while gobj != 0 {
        ensure!(maps.len() < 10, "cyclic or unsupported Ground list");
        let object = saved.bytes(gobj, 0x30);
        let address = word(object, 0x2C);
        // The stage-query GObj has no Ground user data.
        if address != 0 {
            let raw = saved.bytes(address, 0x108);
            let map = word(raw, 0x14) as usize;
            ensure!(
                map < 10 && !stage.ground.live_maps[map],
                "invalid/duplicate map {map}"
            );
            maps.push(map);
            stage.ground.live_maps[map] = true;
            match map {
                3 => {
                    let flags = word(raw, 0xC4);
                    stage.ground.waiting_for_start = flags & (1 << 31) != 0;
                    stage.ground.demo_frozen = flags & (1 << 30) != 0;
                    stage.ground.phase = Phase::try_from(((flags >> 14) & 0xFFFF) as u16)
                        .map_err(anyhow::Error::msg)?;
                    stage.ground.transition_enabled = flags & (1 << 13) != 0;
                    stage.ground.fade.complete = flags & (1 << 12) != 0;
                    stage.ground.elapsed = float(raw, 0xC8);
                }
                7 => {
                    stage.ground.background = Some(BackgroundMotion {
                        pitch: float(raw, 0xC4),
                        yaw: float(raw, 0xC8),
                        pitch_speed: float(raw, 0xCC),
                        yaw_speed: float(raw, 0xD0),
                        pitch_acceleration: float(raw, 0xD4),
                        yaw_acceleration: float(raw, 0xD8),
                        amplitude: float(raw, 0xDC),
                        generator_present: word(raw, 0xE0) != 0,
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
        gobj = word(object, 8);
    }
    ensure!(
        maps == (0..9).collect::<Vec<_>>(),
        "unsupported FD creation order: {maps:?}"
    );
    ensure!(
        stage.ground.phase == Phase::LayeredStart
            && !stage.ground.transition_enabled
            && stage.ground.fade.complete
            && !stage.ground.waiting_for_start
            && !stage.ground.demo_frozen,
        "unsupported FD controller boundary"
    );
    let bg = stage.ground.background.as_ref().unwrap();
    ensure!(
        bg.amplitude == 0.0 && !bg.generator_present,
        "tilt generator unsupported"
    );
    Ok(stage)
}
