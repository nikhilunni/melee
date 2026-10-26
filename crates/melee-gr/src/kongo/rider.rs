//! grKongo_801D651C / 801D6668: the born path rider, with later GroundAnim guarded.
use gekko_math::HsdRng;
use hsd_archive::desc::spline::Spline;
use hsd_types::Vec3;

/// Actual path progress sampling interval, grkongo.c:775.
const STEP: f32 = 0.001;
// grKongo_801D651C's sentinel before any path sample.
const INITIAL_MAX_HEIGHT: f32 = -99_999.0;
// grKongo_801D6668's audio envelope separation.
const AUDIO_DROP_DISTANCE: f32 = 5.0;
// Ground_801C4B50 promotes this reference-axis threshold to double.
const VERTICAL_REFERENCE_LIMIT: f64 = 0.9;
// Ground_801C4B50's single-precision Euler singularity threshold.
const EULER_SINGULAR_LIMIT: f32 = 0.99999;
#[derive(Clone, Debug)]
pub struct Rider {
    pub spline: usize,
    pub progress: f32,
    pub height: f32,
    pub max_y: f32,
    pub min_z: f32,
}
impl Rider {
    /// grKongo_801D651C: Randi3 selects real pair7/4,8/5,9/6.
    pub fn new(rng: &mut HsdRng) -> Self {
        let choice = rng.randi(3);
        // grKg_803B7FD4's pairs (7, 4), (8, 5), (9, 6): the file's spline.
        let spline = match choice {
            0 => 4,
            1 => 5,
            _ => 6,
        };
        Self {
            spline,
            progress: 0.0,
            height: 0.0,
            max_y: INITIAL_MAX_HEIGHT,
            min_z: f32::MAX,
        }
    }
    /// grKongo_801D6668. The exact sqrt is the same three double fnmsub
    /// MSL refinement sequence, then product/frsp at801D6748..6754.
    pub fn advance(&mut self, spline: &Spline, speed: f32, previous: Vec3) -> (Vec3, Vec3) {
        let mut step = STEP;
        if f64::from(self.progress + step) <= 1.0 {
            let a =
                hsd_anim::spline::arc_length_point(spline, self.progress).expect("rider parameter");
            let b = hsd_anim::spline::arc_length_point(spline, self.progress + step)
                .expect("rider parameter");
            let dx = a.x - b.x;
            let dz = a.z - b.z;
            let dy = a.y - b.y;
            // retail 801D66FC then6700: dx²+dy², then dz²+that sum.
            let distance = gekko_math::msl::sqrtf(dz * dz + (dx * dx + dy * dy));
            if f64::from(distance) > 0.0 {
                step *= speed / distance;
                self.progress += step;
                if f64::from(self.progress) > 1.0 {
                    self.progress = 1.0;
                }
            }
        }
        let (position, rotation) = orientation(spline, self.progress);
        if self.max_y < position.y {
            self.max_y = position.y;
        }
        if self.min_z > position.z {
            self.min_z = position.z;
        }
        // Ground_801C5440(gp, 0, 0x5A550) and Ground_801C5630 once the rider
        // falls AUDIO_DROP_DISTANCE below its peak: a sound and its volume
        // (lbAudioAx_800237A8 draws nothing), no state.
        let _drop_sound = self.max_y - previous.y < AUDIO_DROP_DISTANCE
            && self.max_y - position.y > AUDIO_DROP_DISTANCE;
        (position, rotation)
    }
}
/// Ground_801C4B50 (ground.c:2829): no fused instructions. Trig calls and
/// double pi-minus/frsp sites4C84/4CE0/4D34 preserve the retail evaluation.
pub fn orientation(spline: &Spline, parameter: f32) -> (Vec3, Vec3) {
    use gekko_math::msl::cosf;
    use melee_lb::{trigf::asinf, vector};
    let position =
        hsd_anim::spline::spline_point(spline, parameter).expect("Ground spline parameter");
    let forward = vector::normalize(hsd_anim::spline::bspline_tangent(spline, parameter));
    let y = if forward.y < 0.0 {
        -forward.y
    } else {
        forward.y
    };
    // A steep tangent takes z as its reference instead of y.
    let reference = if f64::from(y) > VERTICAL_REFERENCE_LIMIT {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let side = vector::cross(forward, reference);
    let up = vector::cross(side, forward);
    let yaw = asinf(side.z);
    let z = if side.z < 0.0 { -side.z } else { side.z };
    if z >= EULER_SINGULAR_LIMIT {
        // ground.c:2861-2867: pitch from the tangent alone, no roll.
        let mut pitch = asinf(-forward.y);
        if (forward.x * cosf(pitch)) * gekko_math::msl::sinf(yaw) < 0.0 {
            pitch = pi_minus(pitch);
        }
        return (position, Vec3::new(pitch, yaw, 0.0));
    }
    let cos_yaw = cosf(yaw);
    let mut pitch = asinf(up.z / cos_yaw);
    let cos_yaw = cosf(yaw); // retail4CB4 before cos(pitch)4CC0.
    let cos_pitch = cosf(pitch);
    if (forward.z * cos_pitch) * cos_yaw < 0.0 {
        pitch = pi_minus(pitch);
    }
    let cos_yaw = cosf(yaw);
    let mut roll = asinf(-side.y / cos_yaw);
    let cos_roll = cosf(roll); // retail4D04 before cos(yaw)4D10.
    let cos_yaw = cosf(yaw);
    if (-side.x * cos_yaw) * cos_roll < 0.0 {
        roll = pi_minus(roll);
    }
    (position, Vec3::new(pitch, yaw, roll))
}
/// `M_PI - angle` in double, then frsp (retail 4C84 / 4CE0 / 4D34).
fn pi_minus(angle: f32) -> f32 {
    (core::f64::consts::PI - f64::from(angle)) as f32
}
