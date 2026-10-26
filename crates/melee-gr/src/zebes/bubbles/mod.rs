//! The organic bubbles between Brinstar's upper parts (`grZe_8049F170`,
//! twenty `grZe_BubbleEntry`), with the four points of the two parts they
//! hang from (`grZe_8049F158`, `grZe_8049F140`) and their spawn cooldown
//! (`grZe_804D6994`).
//!
//! Slots 0 and 6 are the chain's anchors and have no model; 1..5 are the
//! chain, which must keep both neighbours; 7..19 are loose bubbles. Every
//! bubble but the anchors has a map 3 model (grZebes_801D8558(3)).
pub mod capsule;
pub mod columns;
pub mod physics;
pub mod spawn;

use gekko_math::HsdRng;
use hsd_gobj::GObjAddress;
use hsd_types::Vec3;

pub const BUBBLES: usize = 20;
/// The chain: anchors 0 and 6 and the links between them.
pub const CHAIN: usize = 7;
pub const LEFT_ANCHOR: usize = 0;
pub const RIGHT_ANCHOR: usize = 6;

pub fn is_anchor(index: usize) -> bool {
    index == LEFT_ANCHOR || index == RIGHT_ANCHOR
}

/// A bubble's state (`x00_active`, u8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleState {
    Free = 0,
    Held = 1,
    /// Bursts on its next update (a hit, fn_801DAC90).
    Bursting = 2,
    /// Bursts when its timer runs out (a chain link that lost a
    /// neighbour).
    Dropping = 3,
    /// Grown back with the top platform; held once it is up.
    Growing = 4,
}

impl BubbleState {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Free,
            1 => Self::Held,
            2 => Self::Bursting,
            3 => Self::Dropping,
            4 => Self::Growing,
            _ => return None,
        })
    }
}

/// A bubble's map 3 GObj.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleObject {
    /// Made before the match's first tick: its address is one no object of
    /// the match has had.
    Initial,
    Live(GObjAddress),
}

/// One `grZe_BubbleEntry` (0x24 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bubble {
    pub state: BubbleState,
    /// x02 (s16): a dropping bubble's wait.
    pub timer: i16,
    /// x04: the pulse model (map 2) grZebes_801DAA08 hung on the bubble,
    /// as the ticks its animation has run.
    pub pulse: Option<u32>,
    /// x08 / x0C: the position; x10 / x14 this tick's push.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// x18: the size; x1C the size it grows toward.
    pub size: f32,
    pub target_size: f32,
    /// x20.
    pub object: Option<BubbleObject>,
}

impl Bubble {
    pub const FREE: Self = Self {
        state: BubbleState::Free,
        timer: 0,
        pulse: None,
        x: 0.0,
        y: 0.0,
        vx: 0.0,
        vy: 0.0,
        size: 0.0,
        target_size: 0.0,
        object: None,
    };

    pub fn is_held(&self) -> bool {
        self.state == BubbleState::Held
    }
}

/// The two parts' points: `grZe_8049F140` (the bounds the loose bubbles
/// stay inside) and `grZe_8049F158` (the anchors), each `[left, right]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Points {
    pub bounds: [Vec3; 2],
    pub anchors: [Vec3; 2],
}

/// grZe_803B8044's four points, in the parts' own coordinates:
/// `grZebes_801DC9DC` starts from them before either part has been placed.
pub const INITIAL_POINTS: Points = Points {
    bounds: [Vec3::new(7.59, 2.5, 0.0), Vec3::new(24.05, 2.2, 0.0)],
    anchors: [Vec3::new(8.2, -4.55, 0.0), Vec3::new(24.1, -4.6, 0.0)],
};

/// grZe_803B7FF0's points on map 6's bone 14: the left anchor and bound,
/// and the chain line's first vertex.
pub const LEFT_ANCHOR_POINT: Vec3 = Vec3::new(8.2, -4.55, 0.0);
pub const LEFT_BOUND_POINT: Vec3 = Vec3::new(7.59, 2.5, 0.0);
pub const UPPER_VERTEX_POINT: Vec3 = Vec3::new(8.589, 1.215, 0.0);
/// grZebes_801D925C's points on map 7's bone 1, and grZe_803B7FF0's last
/// vertex.
pub const RIGHT_ANCHOR_POINT: Vec3 = Vec3::new(24.1, -4.6, 0.0);
pub const RIGHT_BOUND_POINT: Vec3 = Vec3::new(24.05, 2.2, 0.0);
pub const LOWER_VERTEX_POINT: Vec3 = Vec3::new(23.151, 1.207, 0.0);

/// The bubbles and the points they hang from.
#[derive(Clone, Debug, PartialEq)]
pub struct Bubbles {
    pub entries: [Bubble; BUBBLES],
    pub points: Points,
    /// grZe_804D6994: ticks before the next loose bubble may grow (set by a
    /// hit's burst, cleared when the top platform comes back).
    pub cooldown: i32,
}

/// What the bubbles ask of their models, in C order.
pub trait BubbleEngine {
    fn rng(&mut self) -> &mut HsdRng;
    /// grZebes_801D8558(3): a bubble's model.
    fn create(&mut self) -> BubbleObject;
    /// Ground_801C4A08 on a bubble's model, then grLib_801C96F8(0x7530,
    /// 0x1E) at its centre, 4 in front.
    fn burst(&mut self, object: BubbleObject, position: Vec3);
}

impl Bubbles {
    pub fn held_links(&self) -> impl Iterator<Item = (usize, &Bubble)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(index, bubble)| bubble.is_held() && !is_anchor(*index))
    }
}
