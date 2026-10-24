//! The event objects grKongo_801D7BBC creates (grKongo_801D5340): the logs
//! that float down the waterfall (maps 1 and 2, grKongo_801D6074 /
//! 801D6198) and the background passers-by (maps 7 to 9,
//! grKongo_801D637C / 801D64BC).
use super::Parameters;
use gekko_math::HsdRng;
use hsd_types::Vec3;

/// The logs' maps and the collision joint each binds (grKg_803E16E0).
pub const LOG_MAPS: [u8; 2] = [1, 2];
pub fn log_joint(map: u8) -> i32 {
    match map {
        1 => 0,
        2 => 1,
        _ => unreachable!("Kongo Jungle log map {map}"),
    }
}
/// grKongo_801D7E78: the descendant whose world translation places a log
/// or a passer-by.
pub const PLACE_JOINT: usize = 2;

/// A log (`GroundVars_kongo3` on maps 1 and 2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Log {
    pub map: u8,
    /// gp+C4: 0 floating in, 1 floating, 2 sinking.
    pub phase: i16,
    /// gp+C6: frames in a row someone stood on it, as the last landing set.
    pub stood: i16,
    /// gp+C8: `stood` as the last step saw it.
    pub last_stood: i16,
    /// gp+CA: frames afloat.
    pub age: i16,
    /// gp+CC: its x offset.
    pub offset: f32,
}

/// What a log step asks of the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogStep {
    Nothing,
    /// Ground_801C2ED0 + Ground_801C2FE0 on the map, mpJointListAdd and the
    /// landing callback.
    Land,
    /// grAnime_801C8138(gobj, map, 1), mpLib_80057BC0, mpJointClearCb1.
    Sink,
    /// Ground_801C4A08.
    Remove,
}

impl Log {
    /// grKongo_801D6074 (0x801D6074) after grAnime_801C8138: its x offset.
    /// retail 0x801D60DC: fmadd 45 * r - 15 in double for map 1; 0x801D60F4
    /// fmul 9 * r in double for map 2.
    pub fn new(map: u8, rng: &mut HsdRng) -> Self {
        let r = f64::from(rng.randf());
        let offset = if map == 1 {
            gekko_math::fma::fmadd(45.0, r, -15.0) as f32
        } else {
            (9.0 * r) as f32
        };
        Self {
            map,
            phase: 0,
            stood: 0,
            last_stood: 0,
            age: 0,
            offset,
        }
    }

    /// fn_801D7E60 (0x801D7E60): someone stands on it (ground kind 2).
    pub fn stood_on(&mut self) {
        self.stood = self.last_stood.wrapping_add(1);
    }

    /// grKongo_801D6198 (0x801D6198). `ended`: the joint animation stopped
    /// (grAnime_801C83D0(gobj, 0, 1)); `crowded`: grKongo_801D7F78.
    pub fn step(
        &mut self,
        p: &Parameters,
        ended: bool,
        crowded: impl FnOnce() -> bool,
    ) -> LogStep {
        match self.phase {
            0 => {
                if ended {
                    self.phase = 1;
                    LogStep::Land
                } else {
                    LogStep::Nothing
                }
            }
            1 => {
                self.age = self.age.wrapping_add(1);
                if self.last_stood == self.stood {
                    self.stood = 0;
                    self.last_stood = 0;
                } else {
                    self.last_stood = self.stood;
                }
                // 801D62B4/801D62D8: the s16s to float, fcmpo.
                if f32::from(self.stood) > p.log_landings
                    || f32::from(self.age) > p.log_frames
                    || crowded()
                {
                    self.phase = 2;
                    LogStep::Sink
                } else {
                    LogStep::Nothing
                }
            }
            2 if ended => LogStep::Remove,
            _ => LogStep::Nothing,
        }
    }
}

/// grKongo_801D637C (0x801D637C): a passer-by's x offset (maps 7, 8, 9).
/// Map 7: 45 * r - 15; map 8: 20 * r - 10; map 9: 9 * r, in double.
pub fn passerby_offset(map: u8, rng: &mut HsdRng) -> f32 {
    let r = f64::from(rng.randf());
    match map {
        7 => gekko_math::fma::fmadd(45.0, r, -15.0) as f32,
        8 => gekko_math::fma::fmadd(20.0, r, -10.0) as f32,
        _ => (9.0 * r) as f32,
    }
}

/// grKongo_801D7F78 (0x801D7F78): another event object passes 100 to 150
/// units in front of `position` within 80 units across.
pub fn crowded(position: Vec3, others: impl IntoIterator<Item = Vec3>) -> bool {
    others.into_iter().any(|other| {
        let dz = other.z - position.z;
        if dz < -100.0 && dz > -150.0 {
            let dx = other.x - position.x;
            dx * dx < 6400.0
        } else {
            false
        }
    })
}
