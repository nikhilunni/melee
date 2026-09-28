//! The grapple beam's rope: itsamusgrapple.c's link steps and the
//! itlinkhookshot.c helpers it shares (it_802A3C98..it_802A4454).
//!
//! it_802B75FC links the rope from the hand (the head, created first) to
//! the tip (created last). A link's `next` is the neighbour toward the hand,
//! its `prev` the one toward the tip; `links[0]` is the head and the last
//! entry the tip (Item.xDD4 x4 and x0). A link joins the rope (x2C_b0) once
//! the rope has paid out past it; every active link but the tip only follows
//! its neighbour, and only the tip meets the map (it_802A3D90).
//!
//! Every constraint writes `dir * length + anchor` with one fmadds per axis
//! (e.g. retail 0x802B9060..80, 0x802B9858..80); the gravity jitter draws
//! HSD_Randf per active link and frame.
use gekko_math::{fma::fmadds, fma::fnmsubs, msl, HsdRng};
use hsd_types::Vec3;
use melee_types::mp::{collide, CollData};

/// it_802B75FC's link count limit: x34 of the air tether (45 at scale 1).
pub const MAX_LINKS: usize = 48;

/// One ItemLink (itlinkhookshot.h): its position and velocity, and the two
/// positions its CollData tracks (it_802A43EC / it_802A43B8).
#[derive(Clone, Copy, Debug, Default)]
pub struct Link {
    pub pos: Vec3,
    pub vel: Vec3,
    /// coll_data.cur_pos and last_pos.
    pub tracked: Vec3,
    pub tracked_last: Vec3,
    /// x2C_b0: the link is part of the paid-out rope.
    pub active: bool,
    /// x2C_b1: the tip's last pass found a floor.
    pub grounded: bool,
}

/// itSamusGrappleAttributes x38..x58, derived by it_802B75FC from the
/// base words, the fighter's y scale and the tether's coefficient.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RopeAttributes {
    /// x0: what a bounce keeps of the tip's velocity.
    pub bounce: f32,
    /// x38 / x3C: the longest and shortest span between links.
    pub span: f32,
    pub min_span: f32,
    /// x40: the tip's throw speed.
    pub throw_speed: f32,
    /// x44: the hanging rope's gravity (it_802B91C4).
    pub gravity: f32,
    /// x48..x54: reel speeds.
    pub x48: f32,
    pub retract_speed: f32,
    pub x50: f32,
    pub reel_speed: f32,
    /// x58: the loose tip's air friction.
    pub friction: f32,
}

/// The rope. Allocated once with its fighter; `len` links are live.
#[derive(Clone, Debug)]
pub struct Chain {
    pub links: [Link; MAX_LINKS],
    pub len: usize,
    pub attrs: RopeAttributes,
    /// The tip's CollData (only the tip meets the map).
    pub tip_collision: CollData,
    /// x1CC: the tip's wall line, whose motion it rides (it_802A4454).
    pub tip_line: i32,
}

impl Default for Chain {
    fn default() -> Self {
        Self {
            links: [Link::default(); MAX_LINKS],
            len: 0,
            attrs: RopeAttributes::default(),
            tip_collision: CollData::default(),
            tip_line: -1,
        }
    }
}

/// it_802A3C98 (802A3C98): the distance from `to` to `from` and the unit
/// vector along it. The inline sqrtf (three fnmsub steps) takes x²+y² then
/// adds z² (802A3CE8/EC); the reciprocal is a double divide, rounded.
pub fn direction(from: Vec3, to: Vec3) -> (f32, Vec3) {
    let d = Vec3::new(from.x - to.x, from.y - to.y, from.z - to.z);
    let squared = d.z * d.z + (d.x * d.x + d.y * d.y);
    let length = msl::sqrtf(squared);
    let inverse = if f64::from(length) == 0.0 {
        0.0
    } else {
        (1.0 / f64::from(length)) as f32
    };
    (
        length,
        Vec3::new(d.x * inverse, d.y * inverse, d.z * inverse),
    )
}

/// `dir * length + anchor`, one fmadds per axis.
fn along(dir: Vec3, length: f32, anchor: Vec3) -> Vec3 {
    Vec3::new(
        fmadds(dir.x, length, anchor.x),
        fmadds(dir.y, length, anchor.y),
        fmadds(dir.z, length, anchor.z),
    )
}

/// samus_grapple_calc_grav / it_802B9328_grav: one link's jitter. A draw
/// above 0.9 (double) takes a second draw times `factor` away from the
/// link's fall (802B90B4 fnmsubs / 802B90C0 fmadds); otherwise none.
fn jitter(rng: &mut HsdRng, vel_y: f32, factor: f32) -> f32 {
    if f64::from(rng.randf()) > 0.9 {
        if f64::from(vel_y) < 0.0 {
            fnmsubs(factor, rng.randf(), vel_y)
        } else {
            fmadds(factor, rng.randf(), vel_y)
        }
    } else {
        0.0
    }
}

/// samus_grapple_calc_grav's factor.
const JITTER: f32 = 0.6;
/// it_802B9328_grav's factor.
const THROW_JITTER: f32 = 1.0;

impl Link {
    /// it_802A4420 (802A4420): three fadds.
    fn step(&mut self) {
        self.pos = Vec3::new(
            self.pos.x + self.vel.x,
            self.pos.y + self.vel.y,
            self.pos.z + self.vel.z,
        );
    }
    /// it_802A43EC (802A43EC).
    fn track(&mut self) {
        self.tracked_last = self.tracked;
        self.tracked = self.pos;
    }
    /// it_802A43B8 (802A43B8).
    fn reset_track(&mut self) {
        self.tracked = self.pos;
        self.tracked_last = self.tracked;
    }
    /// Clamp this link's span from `anchor` into [min, max] (the shared
    /// else-if of every rope walk).
    fn hold_span(&mut self, anchor: Vec3, max: f32, min: Option<f32>) {
        let (distance, dir) = direction(self.pos, anchor);
        if distance > max {
            self.pos = along(dir, max, anchor);
        } else if let Some(min) = min {
            if distance < min {
                self.pos = along(dir, min, anchor);
            }
        }
    }
}

impl Chain {
    pub fn tip(&self) -> usize {
        self.len - 1
    }
    pub fn head(&self) -> usize {
        0
    }
    /// The neighbour toward the hand.
    fn next(&self, i: usize) -> Option<usize> {
        i.checked_sub(1)
    }
    /// The neighbour toward the tip.
    fn prev(&self, i: usize) -> Option<usize> {
        (i + 1 < self.len).then_some(i + 1)
    }

    /// it_802B75FC's links: `count` inactive links at the origin whose
    /// trackers start there (samus_grapple_init_link); the tip's CollData
    /// has the 1.5 box, the others 1 (mpColl_SetECBSource_Fixed).
    pub fn build(&mut self, count: usize, attrs: RopeAttributes, map: &melee_mp::CollMap) {
        assert!(count <= MAX_LINKS, "it_802B75FC: {count} grapple links");
        self.len = count;
        self.attrs = attrs;
        for link in &mut self.links[..count] {
            *link = Link::default();
        }
        let mut collision = CollData::default();
        map.coll_data_init(&mut collision);
        melee_mp::set_ecb_source_fixed(&mut collision, 1.5, 1.5, 1.5, 1.5);
        self.tip_collision = collision;
        self.tip_line = -1;
    }

    /// it_802A3D90 (802A3D90): the tip's airborne pass from its tracked
    /// position; returns the floor and wall bits it touched.
    fn tip_pass(&mut self, map: &mut melee_mp::CollMap) -> u32 {
        let tip = self.tip();
        let link = &mut self.links[tip];
        let cd = &mut self.tip_collision;
        link.tracked_last = link.tracked;
        link.tracked = link.pos;
        cd.last_pos = link.tracked_last;
        cd.cur_pos = link.tracked;
        link.grounded = map.air_collide_pass(cd, None);
        link.tracked = cd.cur_pos;
        link.pos = cd.cur_pos;
        cd.env_flags as u32 & (collide::FLOOR_MASK | collide::WALL_MASK)
    }

    /// it_802A4454 (802A4454): a tip on a moving wall line rides it.
    pub fn ride_line(&mut self, i: usize, map: &melee_mp::CollMap) {
        if !map.line_is_active(self.tip_line) {
            return;
        }
        let link = &mut self.links[i];
        if let Some(speed) = map.line_speed(self.tip_line, &link.pos) {
            link.pos = Vec3::new(
                link.pos.x + speed.x,
                link.pos.y + speed.y,
                link.pos.z + speed.z,
            );
        }
    }

    /// it_802B9328_attach: the tip leaves the hand at `hand` and joins.
    pub fn attach_tip(&mut self, hand: Vec3) {
        let tip = self.tip();
        let link = &mut self.links[tip];
        link.pos = hand;
        link.reset_track();
        link.active = true;
    }

    /// The rope from `from` toward the hand: each active link follows
    /// with `step` and holds its span; the first inactive one joins once
    /// the hand is a span away from its neighbour. Returns whether every
    /// link joined (the walk reached the head).
    fn pay_out(
        &mut self,
        from: usize,
        hand: Vec3,
        mut step: impl FnMut(&mut Link),
        min_span: bool,
        track: bool,
    ) -> Option<usize> {
        let a = self.attrs;
        let mut cur = from;
        while let Some(next) = self.next(cur) {
            let anchor = self.links[cur].pos;
            if self.links[next].active {
                let link = &mut self.links[next];
                step(link);
                link.hold_span(anchor, a.span, min_span.then_some(a.min_span));
                if track {
                    link.track();
                }
            } else {
                let (distance, dir) = direction(hand, anchor);
                if distance > a.span {
                    let link = &mut self.links[next];
                    link.pos = along(dir, a.span, anchor);
                    link.active = true;
                    link.reset_track();
                } else {
                    return None;
                }
            }
            cur = next;
        }
        Some(cur)
    }

    /// it_802B900C (802B900C): `from` pinned `distance` from `hand` along
    /// its current direction, then every link toward the tip jitters
    /// (samus_grapple_calc_grav), moves, tracks and holds its span.
    pub fn hang(&mut self, from: usize, hand: Vec3, distance: f32, rng: &mut HsdRng) {
        let a = self.attrs;
        let (_, dir) = direction(self.links[from].pos, hand);
        self.links[from].pos = along(dir, distance, hand);
        let mut link = from;
        while let Some(prev) = self.prev(link) {
            let anchor = self.links[link].pos;
            let l = &mut self.links[prev];
            l.vel.y -= jitter(rng, l.vel.y, JITTER);
            l.step();
            l.track();
            l.hold_span(anchor, a.span, Some(a.min_span));
            link = prev;
        }
    }

    /// it_802B9328 (802B9328), the throw. `attach` is the hand once the
    /// counter reaches the timeline's throw (it_802B9328_attach). The tip
    /// flies (it_802A4420) and meets the map: 1 a wall (its line kept for
    /// a left wall only as retail orders the tests), 2 a floor. Paid-out
    /// links fall with the throw jitter and hold their spans; once the
    /// hand is past the last link the rope hangs (it_802B900C) and the
    /// result is 3.
    pub fn throw(
        &mut self,
        hand: Vec3,
        attach: bool,
        map: &mut melee_mp::CollMap,
        rng: &mut HsdRng,
    ) -> i32 {
        if attach {
            self.attach_tip(hand);
        }
        let tip = self.tip();
        self.links[tip].step();
        let mut result = self.tip_pass(map) as i32;
        if result != 0 {
            if result & collide::LEFT_WALL_MASK as i32 != 0 {
                result = 1;
                self.tip_line = self.tip_collision.left_facing_wall.index;
            } else if result & collide::RIGHT_WALL_MASK as i32 != 0 {
                result = 1;
                self.tip_line = self.tip_collision.right_facing_wall.index;
            } else {
                result = 2;
            }
        }
        let a = self.attrs;
        let mut cur = tip;
        while let Some(next) = self.next(cur) {
            let anchor = self.links[cur].pos;
            if self.links[next].active {
                let l = &mut self.links[next];
                l.vel.y -= jitter(rng, l.vel.y, THROW_JITTER);
                l.pos.y += l.vel.y;
                l.hold_span(anchor, a.span, Some(a.min_span));
                l.track();
            } else {
                let (distance, dir) = direction(hand, anchor);
                if distance > a.span {
                    let l = &mut self.links[next];
                    l.pos = along(dir, a.span, anchor);
                    l.active = true;
                    l.reset_track();
                } else {
                    return result;
                }
            }
            cur = next;
        }
        self.hang(cur, hand, a.span, rng);
        3
    }

    /// it_802B99A0 (802B99A0), the bounce: the tip jitters and loses x58
    /// of its x speed, flies and meets the map (1 a wall, 2 a floor);
    /// paid-out links jitter, move and hold their spans.
    pub fn bounce(&mut self, hand: Vec3, map: &mut melee_mp::CollMap, rng: &mut HsdRng) -> i32 {
        let a = self.attrs;
        let tip = self.tip();
        {
            let l = &mut self.links[tip];
            l.vel.y -= jitter(rng, l.vel.y, JITTER);
            apply_friction(&mut l.vel.x, a.friction);
            l.step();
        }
        let mut result = self.tip_pass(map) as i32;
        if result != 0 {
            result = if result & 0xFFF != 0 { 1 } else { 2 };
        }
        let done = self.pay_out(
            tip,
            hand,
            |l| {
                l.vel.y -= jitter(rng, l.vel.y, JITTER);
                l.step();
            },
            true,
            true,
        );
        match done {
            Some(head) => {
                self.hang(head, hand, a.span, rng);
                3
            }
            None => result,
        }
    }

    /// it_802B9CE8 (802B9CE8), the slack rope: from the last paid-out link
    /// toward the hand, each jitters, moves and holds its span (the first
    /// from the hand only its longest); the tip then loses x58 of its x
    /// speed.
    pub fn sag(&mut self, hand: Vec3, rng: &mut HsdRng) {
        let a = self.attrs;
        let mut link = self.head();
        let mut prev = self.prev(link);
        while let Some(p) = prev {
            if self.links[link].active {
                break;
            }
            link = p;
            prev = self.prev(p);
        }
        {
            let l = &mut self.links[link];
            l.vel.y -= jitter(rng, l.vel.y, JITTER);
            l.step();
            l.hold_span(hand, a.span, None);
        }
        while let Some(p) = prev {
            let anchor = self.links[link].pos;
            let l = &mut self.links[p];
            l.vel.y -= jitter(rng, l.vel.y, JITTER);
            l.step();
            l.hold_span(anchor, a.span, Some(a.min_span));
            link = p;
            prev = self.prev(p);
        }
        apply_friction(&mut self.links[link].vel.x, a.friction);
    }

    /// it_802BA194 (802BA194), the reel: links within `target` of the hand
    /// leave the rope from the hand's end, the last kept one hangs the rest
    /// of the way (capped at a span); true once none is left. Every link's
    /// x and z velocity then eases toward its tracked motion by 0.12
    /// (802BA294 / 802BA2A8: double fmadd, rounded).
    pub fn retract(&mut self, hand: Vec3, target: f32, rng: &mut HsdRng) -> bool {
        let (cur, next, distance) = self.reel_in(hand, target);
        let mut remaining = distance - target;
        if remaining > self.attrs.span {
            remaining = self.attrs.span;
        }
        self.hang(cur, hand, remaining, rng);
        let done = next.is_none();
        // From the head toward the tip, every link.
        for l in &mut self.links[..self.len] {
            let dx = l.tracked.x - l.tracked_last.x;
            let dz = l.tracked.z - l.tracked_last.z;
            let remaining_z = dz - l.vel.z;
            l.vel.x =
                gekko_math::fma::fmadd(0.12, f64::from(dx - l.vel.x), f64::from(l.vel.x)) as f32;
            l.vel.z =
                gekko_math::fma::fmadd(0.12, f64::from(remaining_z), f64::from(l.vel.z)) as f32;
        }
        done
    }

    /// it_802BA2D8 (802BA2D8), a caught fighter's reel: Item_RetractChain,
    /// then the rope hangs from the link kept; true once none is left.
    pub fn reel(&mut self, hand: Vec3, target: f32, rng: &mut HsdRng) -> bool {
        let (cur, next, distance) = self.reel_in(hand, target);
        let mut remaining = distance - target;
        if remaining > self.attrs.span {
            remaining = self.attrs.span;
        }
        self.hang(cur, hand, remaining, rng);
        next.is_none()
    }

    /// Item_RetractChain (itkinds/inlines.h): from the hand's end, the
    /// first paid-out link and those within `target` of the hand leave the
    /// rope. Returns the link kept, the next toward the tip and its distance.
    fn reel_in(&mut self, hand: Vec3, target: f32) -> (usize, Option<usize>, f32) {
        let mut cur = self.head();
        let mut next = self.prev(cur);
        while let Some(n) = next {
            if self.links[cur].active {
                break;
            }
            cur = n;
            next = self.prev(n);
        }
        let (mut distance, _) = direction(self.links[cur].pos, hand);
        while let Some(n) = next {
            if target <= distance {
                break;
            }
            self.links[cur].active = false;
            distance = direction(self.links[n].pos, hand).0;
            cur = n;
            next = self.prev(n);
        }
        (cur, next, distance)
    }

    /// it_802BA760 (802BA760), in hitlag (accessory3): before the throw's
    /// frame true (the tip stays in the hand); after, paid-out links hold
    /// their spans without moving and the rope hangs once all are out.
    pub fn hold(&mut self, hand: Vec3, thrown: bool, rng: &mut HsdRng) -> bool {
        if !thrown {
            return true;
        }
        let tip = self.tip();
        if let Some(head) = self.pay_out(tip, hand, |_| {}, false, true) {
            self.hang(head, hand, self.attrs.span, rng);
        }
        false
    }

    /// it_802A7168 (802A7168): the links' models follow their positions;
    /// only the tip's world translation (fp->parts[0x8B]) is read back.
    pub fn tip_position(&self) -> Vec3 {
        self.links[self.tip()].pos
    }
}

/// The loose tip's x friction (it_802B99A0 / it_802B9CE8).
fn apply_friction(vel_x: &mut f32, friction: f32) {
    if *vel_x > friction {
        *vel_x -= friction;
    } else if *vel_x < -friction {
        *vel_x += friction;
    } else {
        *vel_x = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_of_a_unit_step_is_exact() {
        let (length, dir) = direction(Vec3::new(3.0, 4.0, 0.0), Vec3::ZERO);
        assert_eq!(length, 5.0);
        assert_eq!(dir, Vec3::new(0.6, 0.8, 0.0));
    }

    #[test]
    fn direction_of_nothing_is_zero() {
        let (length, dir) = direction(Vec3::ZERO, Vec3::ZERO);
        assert_eq!(length, 0.0);
        assert_eq!(dir, Vec3::ZERO);
    }
}
