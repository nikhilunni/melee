//! Hyrule Temple (`gr/grshrine.c`, GrSh.dat, StKind 0x0E). The stage has no
//! hazard and draws nothing: three static maps, and a permanent
//! directional dynamics field (lb_80011A50) that keeps the stage windy for
//! dynamic bones from creation on.
pub mod procs;
use hsd_types::Vec3;
use melee_lb::radial_force::DirectionalGust;

/// The stage controller. Hyrule Temple keeps no state of its own
/// (grShrine_80201C64 only creates the maps and the dynamics field).
#[derive(Clone, Debug, Default)]
pub struct Shrine {
    pub lights: Vec<crate::battle::lights::Light>,
}

/// grShrine_80201C64 (0x80201C64): `lb_80011A50(&{0.5, 0, 0}, -1, 0.5, 0.0,
/// M_PI / 3, -10000, 10000, 10000, -10000)`, a field that never expires,
/// never decays and covers the whole stage.
pub fn wind() -> DirectionalGust {
    DirectionalGust {
        direction: Vec3::new(0.5, 0.0, 0.0),
        frames: -1,
        strength: 0.5,
        decay: 0.0,
        // M_PI / 3 is a double constant passed as the float argument.
        phase_step: (core::f64::consts::PI / 3.0) as f32,
        rectangle: [-10000.0, 10000.0, 10000.0, -10000.0],
    }
}
