//! The hookshot's chain (itlinkhookshot.c's ItemLink list): a row of links
//! from the thrower's thumb to the claw, each a point with a velocity and
//! its own map collision. The claw leads; every other link is pulled to
//! within one link length of its neighbour toward the claw, or toward the
//! hand while the chain pays out. The thrower's accessory proc runs one of
//! the article's per-state steps each frame (`Hookshot::mode`).
//!
//! Index 0 is the link at the hand (the article's `x4`, whose `next` is
//! NULL) and the last index the claw (`x0`, whose `prev` is NULL): a link's
//! `next` is the one below it, its `prev` the one above.
//!
//! Every length here is `it_802A3C98`'s shape, which the compiler inlined
//! at each site with the sum of squares contracted:
//! `fmadds(dz, dz, fmadds(dx, dx, dy * dy))`, MSL's `sqrtf`, a double
//! reciprocal, and the placements `fmadds(dir, length, base)`.
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::mp::{collide, CollData};

/// The most links a chain has: xC (15 for Link) times the aerial factor
/// x50 (1.5).
pub const MAX_LINKS: usize = 24;

/// itLinkHookshotAttributes (itCharItems.h:363), the article's special
/// attributes, as `it_link_attr_math` scales them for one throw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChainAttributes {
    /// x0: the share of its speed the claw keeps when it rebounds off a wall.
    pub rebound: f32,
    /// x2C: the link count.
    pub links: i32,
    /// x30: one link's length (x10 scaled).
    pub link_length: f32,
    /// x38: the throw speed (x18 scaled, by x4C or x50).
    pub throw_speed: f32,
    /// x3C: gravity on a loose link (x1C scaled).
    pub gravity: f32,
    /// x40: the reel-in speed (x20 scaled, by x4C or x50).
    pub reel_speed: f32,
    /// x44: the reel-in speed with a catch (x24 scaled, by x4C or x50).
    pub catch_reel_speed: f32,
    /// x48: the claw's horizontal slowdown while it hangs (x28 scaled).
    pub friction: f32,
}

/// Indices into the article's special attribute words.
mod word {
    pub const REBOUND: usize = 0;
    pub const OVERSCALE_MIX: usize = 2;
    pub const LINKS: usize = 3;
    pub const LINK_LENGTH: usize = 4;
    pub const UNKNOWN_14: usize = 5;
    pub const THROW_SPEED: usize = 6;
    pub const GRAVITY: usize = 7;
    pub const REEL_SPEED: usize = 8;
    pub const CATCH_REEL_SPEED: usize = 9;
    pub const FRICTION: usize = 10;
    pub const GROUND_FACTOR: usize = 19;
    pub const AIR_FACTOR: usize = 20;
}

impl ChainAttributes {
    /// it_link_attr_math (inlined in it_802A2568): the throw's lengths and
    /// speeds for the thrower's y scale; `aerial` (ftLk_MS_AirCatch) takes
    /// x50's factor where the grabs take x4C's. Separate fmuls throughout;
    /// the link count's only fused site is the over-scaled mix.
    pub fn for_throw(words: &[f32], aerial: bool, scale: f32) -> Self {
        let factor = words[if aerial {
            word::AIR_FACTOR
        } else {
            word::GROUND_FACTOR
        }];
        let link_length = words[word::LINK_LENGTH] * scale;
        let _x34 = words[word::UNKNOWN_14] * scale;
        let throw_speed = factor * (words[word::THROW_SPEED] * scale);
        let gravity = words[word::GRAVITY] * scale;
        let reel_speed = factor * (words[word::REEL_SPEED] * scale);
        let catch_reel_speed = factor * (words[word::CATCH_REEL_SPEED] * scale);
        let friction = words[word::FRICTION] * scale;
        // `scale > 1.0` in double: retail 802A2680 mixes the scaled and
        // unscaled lengths (fmadds) by x8.
        assert!(
            f64::from(scale) <= 1.0,
            "it_802A2568: a hookshot thrown at a y scale above 1 ({scale})"
        );
        let count = words[word::LINKS].to_bits() as i32;
        let _ = word::OVERSCALE_MIX;
        // retail 802A26A4..26D4: (f32)xC * x30, times the factor, over x30,
        // truncated (fctiwz).
        let span = (count as f32 * link_length) * factor;
        let links = gekko_math::msl::fctiwz(span / link_length);
        Self {
            rebound: words[word::REBOUND],
            links,
            link_length,
            throw_speed,
            gravity,
            reel_speed,
            catch_reel_speed,
            friction,
        }
    }
}

/// One ItemLink (it/types.h): +8 vel, +14 pos, +30 its CollData,
/// +1CC the wall line, +2C its flags.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChainLink {
    pub position: Vec3,
    pub velocity: Vec3,
    pub collision: CollData,
    /// x2C_b0: paid out (part of the chain in the world).
    pub active: bool,
    /// x2C_b1: resting on a floor (the grounded collision pass).
    pub grounded: bool,
    /// x2C_b2: its landing sound played (it_802A3E50).
    pub landed_sound: bool,
    /// x1CC: the wall line the claw struck, or -1.
    pub wall_line: i32,
}

/// What `it_802A4BFC` found when the claw stopped short.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extension {
    /// 0: still paying out.
    Extending,
    /// 1: the claw met a wall.
    Wall,
    /// 2: every link is out.
    Extended,
    /// 3 / 4: the claw met a ceiling or a floor.
    Ceiling,
    Floor,
}

/// The thrower's motion as the chain steps read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Throw {
    /// fp->motion_id is the throwing motion (Catch, CatchDash or AirCatch)
    /// and mv's counter equals its launch frame (x88 / x98 / xA8).
    pub launching: bool,
    /// ...or is below it (it_802A6A78's early return).
    pub before_launch: bool,
}

/// The article's chain of links.
#[derive(Clone, Debug, PartialEq)]
pub struct Chain {
    links: Vec<ChainLink>,
    count: usize,
    pub attributes: ChainAttributes,
}

/// `(to - from)` normalised and its length (the inlined it_802A3C98).
fn direction(to: Vec3, from: Vec3) -> (Vec3, f32) {
    let d = Vec3::new(to.x - from.x, to.y - from.y, to.z - from.z);
    let length = gekko_math::msl::sqrtf(fmadds(d.z, d.z, fmadds(d.x, d.x, d.y * d.y)));
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

/// `base + dir * length`, each component fmadds (dir, length, base).
fn place(dir: Vec3, length: f32, base: Vec3) -> Vec3 {
    Vec3::new(
        fmadds(dir.x, length, base.x),
        fmadds(dir.y, length, base.y),
        fmadds(dir.z, length, base.z),
    )
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

/// The ECB category of a chain link's CollData (x34 b1234).
const LINK_COLLISION_CATEGORY: u8 = 5;

impl Default for Chain {
    fn default() -> Self {
        Self::with_capacity()
    }
}

impl Chain {
    /// A chain with room for [`MAX_LINKS`], allocated once with its owner.
    pub fn with_capacity() -> Self {
        Self {
            links: vec![ChainLink::default(); MAX_LINKS],
            count: 0,
            attributes: ChainAttributes::default(),
        }
    }

    /// it_802A2568 (802A2568): `attributes.links` links at the origin, each
    /// with a fresh CollData (mpColl_80041EE4, category 5, a fixed ECB of
    /// `scale` on every side) and no wall line.
    pub fn lay_out(&mut self, attributes: ChainAttributes, scale: f32, map: &CollMap) {
        let count = usize::try_from(attributes.links).expect("hookshot link count");
        assert!(
            (2..=MAX_LINKS).contains(&count),
            "it_802A2568: {count} hookshot links"
        );
        self.attributes = attributes;
        self.count = count;
        for link in &mut self.links[..count] {
            let mut collision = CollData::default();
            map.coll_data_init(&mut collision);
            collision.x34_flags.b1234 = LINK_COLLISION_CATEGORY;
            melee_mp::set_ecb_source_fixed(&mut collision, scale, scale, scale, scale);
            *link = ChainLink {
                collision,
                wall_line: -1,
                ..Default::default()
            };
        }
    }

    fn head(&self) -> usize {
        self.count - 1
    }

    pub fn links(&self) -> &[ChainLink] {
        &self.links[..self.count]
    }

    /// The claw.
    pub fn claw(&self) -> &ChainLink {
        &self.links[self.head()]
    }

    /// it_802A78B8: the claw's throw velocity.
    pub fn throw(&mut self, velocity: Vec3) {
        let head = self.head();
        self.links[head].velocity = velocity;
    }

    /// it_802A2EE4's wall case: the claw rebounds with x0 of its speed
    /// (separate fneg and fmuls: `vel.x *= -x0`).
    pub fn rebound(&mut self) {
        let head = self.head();
        self.links[head].velocity.x *= -self.attributes.rebound;
    }

    /// `item_link->next->x2C_b0` of the claw: the link below it is out.
    pub fn below_claw_out(&self) -> bool {
        self.links[self.head() - 1].active
    }

    /// it_802A7168 / it_802A7384 set every paid-out link's model at its
    /// position from the hand up; the claw's joint (fp->parts[139]) ends at
    /// the claw's position. Retail walks down from the hand to the first
    /// paid-out link and faults when there is none.
    pub fn claw_joint_position(&self) -> Vec3 {
        assert!(
            self.links[..self.count].iter().any(|l| l.active),
            "it_802A7168: no hookshot link is out"
        );
        self.claw().position
    }

    fn restart_collision(link: &mut ChainLink) {
        link.collision.cur_pos = link.position;
        link.collision.last_pos = link.collision.cur_pos;
    }

    fn advance_collision(link: &mut ChainLink) {
        link.collision.last_pos = link.collision.cur_pos;
        link.collision.cur_pos = link.position;
    }

    /// it_802A40D0 (802A40D0) and it_802A3E50 (802A3E50): one link's map
    /// pass. The sweep starts from the link below when that one is out. A
    /// link hanging over the end of the floor island its lower neighbour
    /// rests on is lifted to the island's right end height instead (both
    /// ends' tests set it_802A3E50's x14.y). Returns the floor and wall
    /// bits of `env_flags`, which the island case reads unrefreshed.
    fn collide(&mut self, index: usize, width: f32, map: &mut CollMap) -> u32 {
        const MASK: u32 = collide::FLOOR_MASK | collide::WALL_MASK;
        let below_out = index > 0 && self.links[index - 1].active;
        let below = if index > 0 {
            Some(self.links[index - 1])
        } else {
            None
        };
        let link = &mut self.links[index];
        link.collision.last_pos = match below {
            Some(below) if below_out => below.collision.cur_pos,
            _ => link.collision.cur_pos,
        };
        link.collision.cur_pos = link.position;
        if let (true, Some(below)) = (below_out, below) {
            if below.grounded && !link.grounded {
                if let Some(island) = map
                    .island_of_line(below.collision.floor.index)
                    .map(|node| *map.island(node))
                {
                    let p = link.position;
                    if (island.right.x < p.x && island.right.y > p.y)
                        || (island.left.x > p.x && island.left.y > p.y)
                    {
                        link.position.y = island.right.y;
                        return link.collision.env_flags as u32 & MASK;
                    }
                }
            }
        }
        link.grounded = if link.grounded {
            map.ground_collide_pass(&mut link.collision, None)
        } else {
            map.air_collide_pass(&mut link.collision, None)
        };
        let env = link.collision.env_flags as u32;
        let above = index + 1 < self.count;
        if env & collide::FLOOR_MASK != 0 {
            link.collision.cur_pos.x = link.position.x;
        } else if env & collide::WALL_MASK != 0
            && below_out
            && above
            && below.is_some_and(|b| b.position.y < link.position.y)
        {
            // retail 802A42A4: fadds.
            link.collision.cur_pos.y += width;
        }
        link.position = link.collision.cur_pos;
        env & MASK
    }

    /// it_802A3E50's landing sound bookkeeping around the same pass.
    fn collide_middle(&mut self, index: usize, width: f32, map: &mut CollMap) -> u32 {
        let was_grounded = self.links[index].grounded;
        let result = self.collide(index, width, map);
        let link = &mut self.links[index];
        if link.grounded && !was_grounded && !link.landed_sound {
            // lbAudioAx_800237A8(0x2714F / 0x111BF).
            link.landed_sound = true;
        }
        result
    }

    /// Gravity on `index`'s velocity, then its step (separate fsubs, fadds).
    fn fall_step(&mut self, index: usize) {
        let gravity = self.attributes.gravity;
        let link = &mut self.links[index];
        link.velocity.y -= gravity;
        link.position = add(link.position, link.velocity);
    }

    /// it_802A44CC (802A44CC): `from` sits `distance` from `anchor` along
    /// the line to it; each link above falls, collides and is pulled to
    /// within a link of the one below.
    fn hang_above(&mut self, from: usize, anchor: Vec3, distance: f32, map: &mut CollMap) {
        let (dir, _) = direction(self.links[from].position, anchor);
        self.links[from].position = place(dir, distance, anchor);
        let length = self.attributes.link_length;
        for index in from + 1..self.count {
            self.fall_step(index);
            self.collide(index, length, map);
            let (dir, gap) = direction(self.links[index].position, self.links[index - 1].position);
            if gap > length {
                self.links[index].position = place(dir, length, self.links[index - 1].position);
            }
        }
    }

    /// it_802A49B0 (802A49B0): it_802A44CC without the fall.
    fn settle_above(&mut self, from: usize, anchor: Vec3, distance: f32, map: &mut CollMap) {
        let (dir, _) = direction(self.links[from].position, anchor);
        self.links[from].position = place(dir, distance, anchor);
        let length = self.attributes.link_length;
        for index in from + 1..self.count {
            self.collide(index, length, map);
            let (dir, gap) = direction(self.links[index].position, self.links[index - 1].position);
            if gap > length {
                self.links[index].position = place(dir, length, self.links[index - 1].position);
            }
        }
    }

    /// The pay-out walk shared by it_802A4BFC, it_802A5320 and
    /// it_802A6A78: from the claw down, a paid-out link keeps within a
    /// link of the one above (`step` runs on it first), and the first link
    /// still in the hand comes out a link from the one above toward the
    /// hand, or at the hand itself, where the walk stops (false). True:
    /// every link is out, for the caller to settle from the hand up.
    fn pay_out(&mut self, anchor: Vec3, mut step: impl FnMut(&mut Self, usize, usize)) -> bool {
        let length = self.attributes.link_length;
        let mut above = self.head();
        let mut walked = 0;
        while above > 0 {
            let index = above - 1;
            if self.links[index].active {
                step(self, index, walked);
                let (dir, gap) = direction(self.links[index].position, self.links[above].position);
                if gap > length {
                    self.links[index].position = place(dir, length, self.links[above].position);
                }
                Self::advance_collision(&mut self.links[index]);
            } else {
                let base = self.links[above].position;
                let (dir, gap) = direction(anchor, base);
                let link = &mut self.links[index];
                let out = gap > length;
                link.position = if out {
                    place(dir, length, base)
                } else {
                    anchor
                };
                link.active = true;
                Self::restart_collision(link);
                if !out {
                    return false;
                }
            }
            above = index;
            walked += 1;
        }
        true
    }

    /// it_802A4BFC (802A4BFC), the claw flying out: at the launch frame it
    /// starts at the hand; it moves, collides (remembering a wall's line)
    /// and the links follow it out.
    pub fn extend(&mut self, anchor: Vec3, throw: Throw, map: &mut CollMap) -> Extension {
        let head = self.head();
        if throw.launching {
            let claw = &mut self.links[head];
            claw.position = anchor;
            Self::restart_collision(claw);
            claw.active = true;
        }
        let claw = &mut self.links[head];
        claw.position = add(claw.position, claw.velocity);
        let length = self.attributes.link_length;
        let flags = self.collide(head, length, map);
        let claw = &mut self.links[head];
        if flags & collide::LEFT_WALL_MASK != 0 {
            claw.wall_line = claw.collision.left_facing_wall.index;
        } else if flags & collide::RIGHT_WALL_MASK != 0 {
            claw.wall_line = claw.collision.right_facing_wall.index;
        }
        if self.pay_out(anchor, |_, _, _| {}) {
            self.hang_above(0, anchor, length, map);
            Extension::Extended
        } else if flags & collide::WALL_MASK != 0 {
            Extension::Wall
        } else if flags & collide::CEILING_MASK != 0 {
            Extension::Ceiling
        } else if flags & collide::FLOOR_MASK != 0 {
            Extension::Floor
        } else {
            Extension::Extending
        }
    }

    /// it_802A5320 (802A5320), the claw falling back after a wall: it
    /// falls with its horizontal speed worn down by x48, the paid-out
    /// links fall too (the middle one with it_802A3E50) and the chain keeps
    /// paying out. Returns it_802A4BFC's codes 0, 1 (a wall) or 2 (all out).
    pub fn fall_back(&mut self, anchor: Vec3, map: &mut CollMap) -> Extension {
        let head = self.head();
        let friction = self.attributes.friction;
        let claw = &mut self.links[head];
        claw.velocity.y -= self.attributes.gravity;
        claw.velocity.x = if claw.velocity.x > friction {
            claw.velocity.x - friction
        } else if claw.velocity.x < -friction {
            claw.velocity.x + friction
        } else {
            0.0
        };
        claw.position = add(claw.position, claw.velocity);
        let length = self.attributes.link_length;
        let wall = self.collide(head, length, map) & collide::WALL_MASK;
        let middle = (self.attributes.links / 2) as usize;
        let paid_out = self.pay_out(anchor, |chain, index, walked| {
            chain.fall_step(index);
            if walked == middle {
                chain.collide_middle(index, length, map);
            } else {
                chain.collide(index, length, map);
            }
        });
        if paid_out {
            self.hang_above(0, anchor, length, map);
            Extension::Extended
        } else if wall != 0 {
            Extension::Wall
        } else {
            Extension::Extending
        }
    }

    /// it_802A5770 (802A5770), the chain hanging fully out: from the
    /// lowest paid-out link up, each falls, collides (the middle one with
    /// it_802A3E50) and hangs within a link of the one below (the lowest
    /// within a link of the hand); the top link's horizontal speed wears
    /// down by x48.
    pub fn hang(&mut self, anchor: Vec3, map: &mut CollMap) {
        let length = self.attributes.link_length;
        let mut lowest = 0;
        while lowest + 1 < self.count && !self.links[lowest].active {
            lowest += 1;
        }
        self.fall_step(lowest);
        self.collide(lowest, length, map);
        let (dir, gap) = direction(self.links[lowest].position, anchor);
        if gap > length {
            self.links[lowest].position = place(dir, length, anchor);
        }
        let middle = (self.attributes.links / 2) as usize;
        let mut top = lowest;
        for (walked, index) in (lowest + 1..self.count).enumerate() {
            self.fall_step(index);
            if walked == middle {
                self.collide_middle(index, length, map);
            } else {
                self.collide(index, length, map);
            }
            let (dir, gap) = direction(self.links[index].position, self.links[index - 1].position);
            if gap > length {
                self.links[index].position = place(dir, length, self.links[index - 1].position);
            }
            top = index;
        }
        let friction = self.attributes.friction;
        let velocity = &mut self.links[top].velocity;
        velocity.x = if velocity.x > friction {
            velocity.x - friction
        } else if velocity.x < -friction {
            velocity.x + friction
        } else {
            0.0
        };
    }

    /// it_802A5E28 (802A5E28) and it_802A678C (802A678C), reeling in by
    /// `speed`: from the lowest paid-out link, links nearer the hand than
    /// `speed` go back in; the next is drawn `speed` nearer (at most a
    /// link) and the rest hang above it. True once every link is in.
    pub fn reel(&mut self, anchor: Vec3, speed: f32, map: &mut CollMap) -> bool {
        let mut lowest = 0;
        while lowest + 1 < self.count && !self.links[lowest].active {
            lowest += 1;
        }
        let (_, mut gap) = direction(self.links[lowest].position, anchor);
        while lowest + 1 < self.count && speed > gap {
            self.links[lowest].active = false;
            let (_, next) = direction(self.links[lowest + 1].position, anchor);
            gap = next;
            lowest += 1;
        }
        let remaining = gap - speed;
        let length = self.attributes.link_length;
        let distance = if remaining > length {
            length
        } else {
            remaining
        };
        self.hang_above(lowest, anchor, distance, map);
        lowest + 1 >= self.count
    }

    /// it_802A4454's inline copies at the top of it_802A5AE0, it_802A5FE0
    /// and it_802A6474: a claw in a moving wall line rides it.
    fn ride_wall(&mut self, map: &CollMap) {
        let head = self.head();
        let claw = &mut self.links[head];
        if let Some(speed) = map.line_speed(claw.wall_line, &claw.position) {
            claw.position = add(claw.position, speed);
        }
    }

    /// The lowest paid-out link, counting up from the hand.
    fn lowest_out(&self) -> usize {
        let mut lowest = 0;
        while lowest + 1 < self.count && !self.links[lowest].active {
            lowest += 1;
        }
        lowest
    }

    /// From the claw down, each paid-out link is kept within a link of the
    /// one above (`stop_at_slack`: until the first already within one);
    /// then the hand within a link of the last. Returns how many links the
    /// walk passed.
    fn pull_from_claw(&mut self, anchor: &mut Vec3, stop_at_slack: bool) -> usize {
        let length = self.attributes.link_length;
        let mut count = 0;
        let mut slack = false;
        let mut above = self.head();
        while above > 0 && self.links[above - 1].active {
            let index = above - 1;
            count += 1;
            if !slack {
                let base = self.links[above].position;
                let (dir, gap) = direction(self.links[index].position, base);
                if gap > length {
                    self.links[index].position = place(dir, length, base);
                } else if stop_at_slack {
                    slack = true;
                }
            }
            above = index;
        }
        let base = self.links[above].position;
        let (dir, gap) = direction(*anchor, base);
        if gap > length {
            *anchor = place(dir, length, base);
        }
        count
    }

    /// it_802A5AE0 (802A5AE0), the claw in a wall while the thrower falls
    /// away: it rides its wall line; from it down, paid-out links fall,
    /// collide (it_802A40D0) and keep within a link of the one above, and
    /// the next comes out once the hand is a link past the last. True once
    /// every link is out.
    pub fn pay_out_from_wall(&mut self, anchor: Vec3, map: &mut CollMap) -> bool {
        self.ride_wall(map);
        let length = self.attributes.link_length;
        let mut above = self.head();
        while above > 0 {
            let index = above - 1;
            let base = self.links[above].position;
            if self.links[index].active {
                self.fall_step(index);
                self.collide(index, length, map);
                let base = self.links[above].position;
                let (dir, gap) = direction(self.links[index].position, base);
                if gap > length {
                    self.links[index].position = place(dir, length, base);
                }
            } else {
                let (dir, gap) = direction(anchor, base);
                if gap <= length {
                    return false;
                }
                let link = &mut self.links[index];
                link.position = place(dir, length, base);
                link.active = true;
                Self::restart_collision(link);
            }
            above = index;
        }
        true
    }

    /// it_802A5FE0 (802A5FE0), the climb: the claw rides its wall line and
    /// stays put while the chain hangs from the hand (it_802A44CC); links
    /// within `speed` of the hand go back in and the rest settle the
    /// remaining way, at most a link (it_802A49B0); from the claw the links
    /// and then the hand are pulled within a link. True once only the claw
    /// is out.
    pub fn climb(&mut self, anchor: &mut Vec3, speed: f32, map: &mut CollMap) -> bool {
        self.ride_wall(map);
        let head = self.head();
        let saved = self.links[head].position;
        let length = self.attributes.link_length;
        let mut lowest = self.lowest_out();
        self.hang_above(lowest, *anchor, length, map);
        self.links[head].position = saved;
        let (_, mut gap) = direction(self.links[lowest].position, *anchor);
        while lowest + 1 < self.count && speed > gap {
            self.links[lowest].active = false;
            gap = direction(self.links[lowest + 1].position, *anchor).1;
            lowest += 1;
        }
        let remaining = gap - speed;
        let distance = if remaining > length {
            length
        } else {
            remaining
        };
        self.settle_above(lowest, *anchor, distance, map);
        self.links[head].position = saved;
        self.pull_from_claw(anchor, false) == 0
    }

    /// it_802A4758 (802A4758): `from` sits `distance` from `anchor`; each
    /// link above falls, moves, tracks (it_802A43EC) and keeps within a
    /// link of the one below, without meeting the map.
    fn hang_loose(&mut self, from: usize, anchor: Vec3, distance: f32) {
        let (dir, _) = direction(self.links[from].position, anchor);
        self.links[from].position = place(dir, distance, anchor);
        let length = self.attributes.link_length;
        for index in from + 1..self.count {
            self.fall_step(index);
            Self::advance_collision(&mut self.links[index]);
            let base = self.links[index - 1].position;
            let (dir, gap) = direction(self.links[index].position, base);
            if gap > length {
                self.links[index].position = place(dir, length, base);
            }
        }
    }

    /// it_802A6474 (802A6474), the swing: the claw rides its wall line and
    /// stays put while the chain hangs from the hand (it_802A4758); from the
    /// claw the links are pulled taut up to the first slack one, and the
    /// hand within a link of the last.
    pub fn swing(&mut self, anchor: &mut Vec3, map: &CollMap) {
        self.ride_wall(map);
        let head = self.head();
        let saved = self.links[head].position;
        let lowest = self.lowest_out();
        let length = self.attributes.link_length;
        self.hang_loose(lowest, *anchor, length);
        self.links[head].position = saved;
        self.pull_from_claw(anchor, true);
    }

    /// it_802A6A78 (802A6A78), the hitlag step: before the launch nothing
    /// moves (true: the claw stays at the hand); otherwise the chain pays
    /// out behind the claw as it_802A4BFC's walk does and the rest settles
    /// (it_802A49B0). False either way once launched.
    pub fn hold_in_hitlag(&mut self, anchor: Vec3, throw: Throw, map: &mut CollMap) -> bool {
        if throw.before_launch {
            return true;
        }
        if self.pay_out(anchor, |_, _, _| {}) {
            let length = self.attributes.link_length;
            self.settle_above(0, anchor, length, map);
        }
        false
    }
}
