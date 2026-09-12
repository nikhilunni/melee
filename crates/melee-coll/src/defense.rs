//! Reflect/absorb descriptors and clank priority, lb/types.h / ftcoll.c.
use hsd_types::Vec3;

/// ReflectDesc; ownership transfer is independent of the reflected item's kind.
#[derive(Clone, Copy, Debug)]
pub struct ReflectDescriptor {
    pub bone: usize,
    pub maximum_damage: i32,
    pub offset: Vec3,
    pub radius: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub exclude_master_ball_ownership: bool,
}
/// AbsorbDesc; state callbacks and healing belong to the owner.
#[derive(Clone, Copy, Debug)]
pub struct AbsorbDescriptor {
    pub bone: usize,
    pub offset: Vec3,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClankPriority {
    pub stop_first: bool,
    pub stop_second: bool,
}
/// ftColl_8007699C (8007699C): truncate damage before strict threshold tests.
/// Response application is owned by melee-ft::fighter::clank.
pub fn clank_priority(first: f32, second: f32, threshold: i32) -> ClankPriority {
    let first = gekko_math::msl::fctiwz(first);
    let second = gekko_math::msl::fctiwz(second);
    ClankPriority {
        stop_second: second - threshold < first,
        stop_first: first - threshold < second,
    }
}
