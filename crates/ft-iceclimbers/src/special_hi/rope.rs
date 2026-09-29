//! The Belay's rope, itclimbersstring.c (802C248C..802C3AA4): a row of
//! ItemLinks (it_802C248C) between Nana's hand and Popo's. The rope item
//! (It_Kind_IceClimber_GumStrings) is only its handle in Popo's hand; the
//! links and their steps belong to Popo, whose accessory work the item's
//! on_accessory is (`it_climbersice::string`).
//!
//! Index 0 is the item's `x8` (the head: `next` NULL), the last index its
//! `x4` (the tail: `prev` NULL): a link's `next` is the one before it, its
//! `prev` the one after. The tail hangs from Nana's hand (Popo's
//! u.pp.x2240) and the rope pays out from Popo's hand toward the head.
//!
//! Only the rope's shape decides when it is reeled in (the item's state 3
//! ends in state 0); the drawing (it_802C3520, it_802C33B8's matrix) has
//! no port owner.
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_types::Vec3;

/// itClimbersStringAttributes x0: the links it_802C248C creates.
pub const MAX_LINKS: usize = 40;

/// itClimbersStringAttributes (it/types.h), Popo's third article's
/// special attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RopeAttributes {
    /// x0: how many links the rope has.
    pub links: usize,
    /// x4: while hanging, links past this many from the tail that are not
    /// yet out sit at Popo's hand.
    pub hanging_links: i32,
    /// x8: one link's length.
    pub link_length: f32,
    /// xC: paying out, a link nearer than this to the one before goes in.
    pub minimum_link_length: f32,
    /// x14: gravity on a hanging link.
    pub gravity: f32,
    /// x18 / x1C / x20: the Belay frames (u.pp's counter) at which the rope
    /// pays out, hangs and is reeled in.
    pub pay_out_frame: i32,
    pub hang_frame: i32,
    pub reel_frame: i32,
}

/// One ItemLink (it/types.h): +8 vel, +14 pos, +2C bit 0 (out).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RopeLink {
    pub position: Vec3,
    pub velocity: Vec3,
    /// x2C_b0: the link is out of Popo's hand.
    pub out: bool,
}

/// The item's motion states, each naming the step its on_accessory runs.
pub mod phase {
    /// In the hand (it_2725_Logic70_PickedUp: fn_802C28B8).
    pub const HELD: u16 = 0;
    /// Paying out between the climbers (it_802C3950: fn_802C28DC).
    pub const PAYING_OUT: u16 = 1;
    /// Hanging (it_802C3810: fn_802C29E8).
    pub const HANGING: u16 = 2;
    /// Reeled in (it_802C3864: fn_802C2AF4).
    pub const REELING: u16 = 3;
}

/// it_802C3864: the reel-in speed (xDD4 x0).
const REEL_SPEED: f32 = 2.25;
/// it_802C2EC4: a link this near Popo's hand goes back in.
const IN_HAND: f32 = 0.1;
/// it_802C30E8: the tail's share of gravity, and of its sideways speed
/// kept each frame (retail @417).
const TAIL_DAMPING: f32 = 0.9;

/// The rope in Popo's hands, with the item's motion state.
#[derive(Clone, Debug, PartialEq)]
pub struct Rope {
    pub attributes: RopeAttributes,
    pub phase: u16,
    pub links: [RopeLink; MAX_LINKS],
}

impl Default for Rope {
    fn default() -> Self {
        Self {
            attributes: RopeAttributes::default(),
            phase: phase::HELD,
            links: [RopeLink::default(); MAX_LINKS],
        }
    }
}

/// it_802A3C98 (802A3C98), out of line: `from - to` normalised, and its
/// length. Separate fmuls and fadds (y² + x², then z² + that), MSL's
/// sqrtf, a double reciprocal rounded once, then fmuls.
fn direction(from: Vec3, to: Vec3) -> (Vec3, f32) {
    let d = Vec3::new(from.x - to.x, from.y - to.y, from.z - to.z);
    let length = sqrtf(d.z * d.z + (d.x * d.x + d.y * d.y));
    let inverse = if f64::from(length) == 0.0 {
        0.0
    } else {
        (1.0 / f64::from(length)) as f32
    };
    (
        Vec3::new(d.x * inverse, d.y * inverse, d.z * inverse),
        length,
    )
}

/// `base + dir * length`, each component fmadds(dir, length, base).
fn place(dir: Vec3, length: f32, base: Vec3) -> Vec3 {
    Vec3::new(
        fmadds(dir.x, length, base.x),
        fmadds(dir.y, length, base.y),
        fmadds(dir.z, length, base.z),
    )
}

/// it_802A4420 (802A4420): a link moves by its velocity.
fn integrate(link: &mut RopeLink) {
    link.position.x += link.velocity.x;
    link.position.y += link.velocity.y;
    link.position.z += link.velocity.z;
}

impl Rope {
    fn tail(&self) -> usize {
        self.attributes.links - 1
    }

    /// it_802C248C (802C248C): every link at the item's position and still;
    /// only the tail is out.
    pub fn lay_out(&mut self, attributes: RopeAttributes, position: Vec3) {
        assert!(
            (1..=MAX_LINKS).contains(&attributes.links),
            "it_802C248C: {} rope links",
            attributes.links
        );
        self.attributes = attributes;
        self.phase = phase::HELD;
        let tail = self.tail();
        for (i, link) in self.links.iter_mut().enumerate() {
            *link = RopeLink {
                position,
                velocity: Vec3::ZERO,
                out: i == tail,
            };
        }
    }

    /// it_802C3950 (802C3950): the tail leaves Popo's hand, which is
    /// `hand` (its coll_data positions follow; no port owner reads them).
    pub fn start_paying_out(&mut self, hand: Vec3) {
        let tail = self.tail();
        self.links[tail].position = hand;
        self.links[tail].out = true;
    }

    /// fn_802C28DC / fn_802C29E8: the tail is out before each step.
    pub fn release_tail(&mut self) {
        let tail = self.tail();
        self.links[tail].out = true;
    }

    /// it_802C33B8 (802C33B8), the held rope's step: the tail is in.
    pub fn hold(&mut self) {
        let tail = self.tail();
        self.links[tail].out = false;
    }

    /// it_802C2EC4 (802C2EC4): the tail at Nana's hand (`anchor`, unless
    /// zero), and each link after it out to within one link length of
    /// the one before it, toward Popo's `hand`; a link nearer than the
    /// minimum goes back in, and the first link still in comes out once
    /// the hand is a link length away.
    pub fn pay_out(&mut self, anchor: Vec3, hand: Vec3) {
        let a = self.attributes;
        let tail = self.tail();
        if anchor.x != 0.0 || anchor.y != 0.0 {
            self.links[tail].position = anchor;
        }
        self.links[tail].velocity = Vec3::ZERO;
        let mut cur = tail;
        while cur > 0 {
            let i = cur - 1;
            let base = self.links[cur].position;
            if self.links[i].out {
                self.links[i].position = hand;
                let (dir, length) = direction(self.links[i].position, base);
                if length > a.link_length {
                    self.links[i].position = place(dir, a.link_length, base);
                } else if length < a.minimum_link_length {
                    if direction(self.links[i].position, hand).1 <= IN_HAND {
                        self.links[i].out = false;
                    } else {
                        self.links[i].position = place(dir, a.minimum_link_length, base);
                    }
                }
            } else {
                let (dir, length) = direction(hand, base);
                if length > a.link_length {
                    self.links[i].position = place(dir, a.link_length, base);
                    self.links[i].out = true;
                } else {
                    return;
                }
            }
            cur = i;
        }
    }

    /// it_802C30E8 (802C30E8): the rope hangs from Nana's hand (or the
    /// tail falls on its own), each out link falls and stays within a
    /// link length of the one before; links past the hanging count wait at
    /// Popo's `hand`, the first link still in comes out a link length
    /// from it. Then the rope is drawn taut to the hand from the head
    /// (it_802C2CA8) and the tail's sideways speed decays.
    pub fn hang(&mut self, anchor: Vec3, hand: Vec3) {
        let a = self.attributes;
        let tail = self.tail();
        // retail 802C311C: fnmsubs.
        self.links[tail].velocity.y =
            gekko_math::fma::fnmsubs(TAIL_DAMPING, a.gravity, self.links[tail].velocity.y);
        if anchor.x != 0.0 || anchor.y != 0.0 {
            self.links[tail].position = anchor;
        } else {
            integrate(&mut self.links[tail]);
        }
        let mut counter = 0;
        let mut cur = tail;
        while cur > 0 {
            let i = cur - 1;
            counter += 1;
            let base = self.links[cur].position;
            if self.links[i].out {
                self.links[i].velocity.y -= a.gravity;
                integrate(&mut self.links[i]);
                let (dir, length) = direction(self.links[i].position, base);
                if length > a.link_length {
                    self.links[i].position = place(dir, a.link_length, base);
                }
            } else if counter > a.hanging_links {
                self.links[i].position = hand;
            } else {
                let (dir, length) = direction(hand, base);
                if length > a.link_length {
                    self.links[i].position = place(dir, a.link_length, base);
                    self.links[i].out = true;
                } else {
                    return;
                }
            }
            cur = i;
        }
        self.draw_taut(cur, hand, a.link_length);
        self.links[tail].velocity.x *= TAIL_DAMPING;
    }

    /// it_802C2CA8 (802C2CA8): link `from` at `length` from `target` along
    /// its bearing, then each out link after it to within a link length
    /// of the one before.
    fn draw_taut(&mut self, from: usize, target: Vec3, length: f32) {
        let link_length = self.attributes.link_length;
        let (dir, _) = direction(self.links[from].position, target);
        self.links[from].position = place(dir, length, target);
        for i in from + 1..self.attributes.links {
            let base = self.links[i - 1].position;
            if self.links[i].out {
                let (dir, length) = direction(self.links[i].position, base);
                if length > link_length {
                    self.links[i].position = place(dir, link_length, base);
                }
            }
        }
    }

    /// it_802C32D4 (802C32D4) with Item_RetractChain: from the head, the
    /// links within the reel speed of Popo's `hand` go in; the first one
    /// still out is drawn that much closer, and the rest fall after it
    /// (it_802C2DB0). True once the walk has reached the tail: the rope
    /// is in (it_2725_Logic70_PickedUp).
    pub fn reel(&mut self, hand: Vec3) -> bool {
        let a = self.attributes;
        let last = self.tail();
        let mut cur = 0;
        while cur < last && !self.links[cur].out {
            cur += 1;
        }
        let mut distance = direction(self.links[cur].position, hand).1;
        while cur < last && REEL_SPEED > distance {
            self.links[cur].out = false;
            distance = direction(self.links[cur + 1].position, hand).1;
            cur += 1;
        }
        let mut remaining = distance - REEL_SPEED;
        if remaining > a.link_length {
            remaining = a.link_length;
        }
        self.fall_after(cur, hand, remaining);
        cur == last
    }

    /// it_802C2DB0 (802C2DB0): link `from` at `length` from `target`, and
    /// every link after it falls (x14, it_802A4420) to within a link
    /// length of the one before.
    fn fall_after(&mut self, from: usize, target: Vec3, length: f32) {
        let a = self.attributes;
        let (dir, _) = direction(self.links[from].position, target);
        self.links[from].position = place(dir, length, target);
        for i in from + 1..a.links {
            let base = self.links[i - 1].position;
            self.links[i].velocity.y -= a.gravity;
            integrate(&mut self.links[i]);
            let (dir, length) = direction(self.links[i].position, base);
            if length > a.link_length {
                self.links[i].position = place(dir, a.link_length, base);
            }
        }
    }
}
