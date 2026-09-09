//! Map 7's two damped angular oscillators. Limits do not depend on visibility.
use gekko_math::{
    fma::{fmadds, fmsubs},
    HsdRng,
};

// Constants loaded from retail .sdata2, 0x804DBB7C..0x804DBBA0.
const MIN_ACCELERATION: f32 = 0.000_017_453_292;
const ACCELERATION_RANGE: f32 = 0.000_069_813_17;
const DAMPING: f32 = 0.99;
const SPEED_LIMIT: f32 = 0.000_872_664_6;
pub const PITCH_LIMIT: f32 = 0.261_799_4; // 15 degrees
pub const YAW_LIMIT: f32 = 0.174_532_92; // 10 degrees

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BackgroundMotion {
    pub pitch: f32,
    pub yaw: f32,
    pub pitch_speed: f32,
    pub yaw_speed: f32,
    pub pitch_acceleration: f32,
    pub yaw_acceleration: f32,
    pub amplitude: f32,
    pub generator_present: bool,
    /// Plain data outputs for the root JObj's rotation.
    pub applied_pitch: f32,
    pub applied_yaw: f32,
}

impl BackgroundMotion {
    /// `grLast_8021AC30` (grlast.c), retail 0x8021AC30. Four draws, X then Y.
    pub fn initialize(rng: &mut HsdRng) -> Self {
        // retail 0x8021AC78 / 0x8021ACB4: fmadds
        let pitch = fmadds(ACCELERATION_RANGE, rng.randf(), MIN_ACCELERATION);
        let pitch = pitch * if rng.randi(2) != 0 { 1.0 } else { -1.0 };
        let yaw = fmadds(ACCELERATION_RANGE, rng.randf(), MIN_ACCELERATION);
        let yaw = yaw * if rng.randi(2) != 0 { 1.0 } else { -1.0 };
        Self {
            pitch_acceleration: pitch,
            yaw_acceleration: yaw,
            ..Self::default()
        }
    }

    /// `grLast_8021ADD0` (grlast.c), retail 0x8021ADD0.
    /// Returns the number of direct LCG advances; generator evaluation is separate.
    pub fn tick(&mut self, rng: &mut HsdRng) -> u32 {
        // retail 0x8021AE00..0x8021AE58: separate fadds, store, fmuls.
        self.pitch_speed = ((self.pitch_speed + self.pitch_acceleration) * DAMPING)
            .clamp(-SPEED_LIMIT, SPEED_LIMIT);
        self.yaw_speed =
            ((self.yaw_speed + self.yaw_acceleration) * DAMPING).clamp(-SPEED_LIMIT, SPEED_LIMIT);
        let mut draws = 0;
        self.pitch += self.pitch_speed;
        if self.pitch > PITCH_LIMIT {
            self.pitch = PITCH_LIMIT;
            self.pitch_speed = -self.pitch_speed.abs();
            // retail 0x8021AED8: fmsubs, with fneg of the random operand.
            self.pitch_acceleration = fmsubs(ACCELERATION_RANGE, -rng.randf(), MIN_ACCELERATION);
            draws += 1;
        } else if self.pitch < -PITCH_LIMIT {
            self.pitch = -PITCH_LIMIT;
            self.pitch_speed = self.pitch_speed.abs();
            // retail 0x8021AF18: fmadds
            self.pitch_acceleration = fmadds(ACCELERATION_RANGE, rng.randf(), MIN_ACCELERATION);
            draws += 1;
        }
        self.applied_pitch = self.pitch * self.amplitude;
        self.yaw += self.yaw_speed;
        if self.yaw > YAW_LIMIT {
            self.yaw = YAW_LIMIT;
            self.yaw_speed = -self.yaw_speed.abs();
            // retail 0x8021B00C: fmsubs
            self.yaw_acceleration = fmsubs(ACCELERATION_RANGE, -rng.randf(), MIN_ACCELERATION);
            draws += 1;
        } else if self.yaw < -YAW_LIMIT {
            self.yaw = -YAW_LIMIT;
            self.yaw_speed = self.yaw_speed.abs();
            // retail 0x8021B04C: fmadds
            self.yaw_acceleration = fmadds(ACCELERATION_RANGE, rng.randf(), MIN_ACCELERATION);
            draws += 1;
        }
        self.applied_yaw = self.yaw * self.amplitude;
        draws
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invisible_background_still_draws_at_both_limits_in_pitch_yaw_order() {
        let mut motion = BackgroundMotion {
            pitch: PITCH_LIMIT,
            yaw: -YAW_LIMIT,
            pitch_speed: SPEED_LIMIT,
            yaw_speed: -SPEED_LIMIT,
            ..Default::default()
        };
        let mut rng = HsdRng::new(1);
        assert_eq!(motion.tick(&mut rng), 2);
        assert_eq!(rng.seed, 3_357_800_067); // two hand-evaluated LCG steps
        assert_eq!(motion.pitch, PITCH_LIMIT);
        assert_eq!(motion.yaw, -YAW_LIMIT);
        assert!(motion.pitch_speed < 0.0 && motion.yaw_speed > 0.0);
        assert_eq!(motion.applied_pitch.to_bits(), 0);
        assert_eq!(motion.applied_yaw.to_bits(), (-0.0_f32).to_bits());
    }
    #[test]
    fn zero_velocity_at_exact_limit_does_not_draw() {
        let mut motion = BackgroundMotion {
            pitch: PITCH_LIMIT,
            ..Default::default()
        };
        let mut rng = HsdRng::new(1);
        assert_eq!(motion.tick(&mut rng), 0);
        assert_eq!(rng.seed, 1);
    }
}
