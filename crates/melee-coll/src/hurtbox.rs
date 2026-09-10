//! Hurt capsules and table-order metadata, lb/types.h.
use hsd_types::Vec3;
#[derive(Clone, Copy, Debug)]
pub enum HurtHeight {
    Low,
    Middle,
    High,
}
impl HurtHeight {
    pub fn from_retail(value: u32) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Middle,
            2 => Self::High,
            _ => panic!("hurtbox height {value}"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct HurtCapsule {
    pub grabbable: bool,
    pub height: HurtHeight,
    pub bone: usize,
    pub offsets: [Vec3; 2],
    pub radius: f32,
    pub positions: [Vec3; 2],
    pub cached: bool,
}
