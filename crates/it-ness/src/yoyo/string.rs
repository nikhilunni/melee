//! The yo-yo's string: itnessyoyo.c's ItemLink list (it_802BE65C) and the
//! steps its phys callbacks run on it.
//!
//! Index 0 is the link at the hand (the article's `xC`, whose `next` is
//! NULL) and the last index the yo-yo itself (`x8`, whose `prev` is NULL):
//! a link's `next` is the one nearer the hand, its `prev` the one nearer
//! the yo-yo. Every link's joint is the hand's (FtPart_R2ndNa), so the
//! "target" the steps pull toward is the hand's world translation.
//!
//! Every length is it_802A3C98, called out of line (`direction`), and every
//! placement `fmadds(dir, length, base)` per component (retail 802BF1C4..,
//! 802BF364.., 802BF61C..).
use gekko_math::fma::fmadds;
use gekko_math::msl::sqrtf;
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::mp::CollData;

/// itYoyoAttributes (it/itYoyo.h): the article's special attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct YoyoAttributes {
    /// x0: the string's link count, the yo-yo included.
    pub links: i32,
    /// x4 / x8: how many links the up / down smash's charge pays out.
    pub up_charge_links: i32,
    pub down_charge_links: i32,
    /// xC: a link's length (scaled by the owner's model scale).
    pub link_length: f32,
    /// x10: the shortest a tethered link may be.
    pub min_link_length: f32,
    /// x18: the spin rate (u.ns.x223C) once the yo-yo is out. Drawn only.
    pub spin_rate: f32,
    /// x24 / x30: the yo-yo's release velocity for the charge.
    pub release_velocity_x: f32,
    pub release_velocity_y: f32,
    /// x28: the charge's pull, per frame, toward the yo-yo's facing.
    pub pull: f32,
    /// x2C: the charge's horizontal speed limit.
    pub max_speed_x: f32,
    /// x34: the swinging yo-yo's gravity.
    pub gravity: f32,
    /// x38: its fall speed limit.
    pub max_fall_speed: f32,
    /// x3C: the gravity on the string's paid-out links.
    pub string_gravity: f32,
    /// x40 / x44: the up smash's frames for the yo-yo going out (it_802C0010)
    /// and coming back (it_802BFEC4).
    pub up_out_frame: i32,
    pub up_return_frame: i32,
    /// x48 / x4C: the down smash's.
    pub down_out_frame: i32,
    pub down_return_frame: i32,
}

/// One ItemLink (itCharItems.h:314): +8 vel, +14 pos, +2C its flags, +30
/// its CollData.
#[derive(Clone, Debug, Default)]
pub struct YoyoLink {
    pub position: Vec3,
    pub velocity: Vec3,
    /// x2C_b0: paid out (drawn, simulated).
    pub active: bool,
    /// x2C_b1: its last map pass found a floor (written, never read here:
    /// it_802BF030's x2C_b2 latch needs a nonzero second argument, which no
    /// caller passes).
    pub on_floor: bool,
    pub collision: CollData,
}

/// What it_802BF4A0 found (its return value, unread by its caller).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swing {
    /// 0: a link not yet paid out and still within reach.
    Short,
    /// 1: the same, with the yo-yo touching a wall or ceiling.
    ShortTouching,
    /// 2: the whole string stepped.
    Stepped,
}

/// The ECB category of a link's CollData (x34 b1234), it_802A24D0.
const LINK_COLLISION_CATEGORY: u8 = 5;
/// it_802BE65C: every link's fixed ECB is 1 on each side; the yo-yo's is
/// twice the owner's model scale.
const LINK_ECB: f32 = 1.0;
const YOYO_ECB_SCALE: f32 = 2.0;
/// it_802BF28C (@476): a tethered link this near the hand is drawn in.
const DRAWN_IN_DISTANCE: f32 = 0.1;
/// it_802BF4A0 (@533): a yo-yo on a floor is lifted this much.
const FLOOR_LIFT: f32 = 0.001;
/// it_802BF4A0 (@534, a double): the paid-out links' map pass starts this
/// far above the hand.
const STRING_PASS_HEIGHT: f64 = 1.5;
/// it_802BF4A0 (@535): the yo-yo's horizontal speed keeps this share.
const SWING_DRAG: f32 = 0.9;
/// mpColl_800471F8's env_flags: Collide_FloorMask, and the wall and
/// ceiling bits it_802BF030 tests (0xFFF).
const FLOOR_MASK: i32 = 0x18000;
const WALL_CEILING_MASK: i32 = 0xFFF;

/// it_802A3C98 (802A3C98), out of line: `from - to` normalised, and its
/// length. Separate fmuls and fadds (x² + y², then z² + that), MSL's
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

/// `|v|` as retail branches it (fcmpo against 0, fneg): -0 stays -0.
fn magnitude(v: f32) -> f32 {
    if v < 0.0 {
        -v
    } else {
        v
    }
}

impl YoyoLink {
    /// it_802A4420 (802A4420): three fadds.
    fn integrate(&mut self) {
        self.position.x += self.velocity.x;
        self.position.y += self.velocity.y;
        self.position.z += self.velocity.z;
    }
    /// it_802A43EC (802A43EC): the map pass resumes from its last point.
    fn track(&mut self) {
        self.collision.last_pos = self.collision.cur_pos;
        self.collision.cur_pos = self.position;
    }
    /// it_802A43B8 (802A43B8): the map pass starts afresh here.
    fn reset_tracking(&mut self) {
        self.collision.cur_pos = self.position;
        self.collision.last_pos = self.collision.cur_pos;
    }
    /// it_802A42F4 (802A42F4): an airborne pass from `height` straight
    /// above its last point.
    fn collide_from_height(&mut self, height: f32, map: &mut CollMap) {
        let cd = &mut self.collision;
        cd.last_pos = cd.cur_pos;
        cd.last_pos.y = height;
        cd.cur_pos = self.position;
        self.on_floor = map.air_collide_pass(cd, None);
        self.position = cd.cur_pos;
    }
}

/// The yo-yo's links.
#[derive(Clone, Debug, Default)]
pub struct YoyoString {
    links: Vec<YoyoLink>,
    /// ip->facing_dir: the owner's, turned for the down smash.
    pub facing: f32,
    /// ip->xDD4.x0: the owner's motion at the spawn (ftNs_MS_AttackHi4 or
    /// ftNs_MS_AttackLw4), which picks the charge's string length.
    pub owner_motion: u16,
    /// The item's motion state (`super::motion`), mirrored.
    pub motion: u16,
}

/// ftNs_MS_AttackHi4 (0x156): the up smash's string.
const UP_SMASH: u16 = 342;

impl YoyoString {
    /// Room for `attributes.links` links, allocated once with the owner.
    pub fn with_capacity(attributes: &YoyoAttributes) -> Self {
        let count = usize::try_from(attributes.links).expect("yo-yo link count");
        assert!(count >= 2, "it_802BE65C: {count} yo-yo links");
        Self {
            links: vec![YoyoLink::default(); count],
            ..Default::default()
        }
    }

    pub fn links(&self) -> &[YoyoLink] {
        &self.links
    }

    fn yoyo_index(&self) -> usize {
        self.links.len() - 1
    }

    /// The yo-yo itself (the article's `x8`).
    pub fn yoyo(&self) -> &YoyoLink {
        &self.links[self.yoyo_index()]
    }

    /// it_802BE65C (802BE65C): every link at the item's position and still,
    /// only the yo-yo paid out; each CollData starts there (it_802A24D0:
    /// mpColl_80041EE4, category 5, a fixed ECB of 1, the yo-yo's twice the
    /// owner's scale). `facing` is the item's (it_802BE9D8 turns it for the
    /// down smash).
    pub fn lay_out(
        &mut self,
        position: Vec3,
        owner_motion: u16,
        facing: f32,
        scale: f32,
        map: &CollMap,
    ) {
        let yoyo = self.yoyo_index();
        for (i, link) in self.links.iter_mut().enumerate() {
            let ecb = if i == yoyo {
                YOYO_ECB_SCALE * scale
            } else {
                LINK_ECB
            };
            let mut collision = CollData::default();
            collision.cur_pos = position;
            collision.last_pos = collision.cur_pos;
            map.coll_data_init(&mut collision);
            collision.x34_flags.b1234 = LINK_COLLISION_CATEGORY;
            melee_mp::set_ecb_source_fixed(&mut collision, ecb, ecb, ecb, ecb);
            *link = YoyoLink {
                position,
                velocity: Vec3::ZERO,
                active: i == yoyo,
                on_floor: false,
                collision,
            };
        }
        self.facing = facing;
        self.owner_motion = owner_motion;
        self.motion = super::motion::HELD;
    }

    /// it_802C0010 (802C0010): the yo-yo out at the hand with `velocity`,
    /// its map pass starting there; the item goes to motion 1.
    pub fn send_out(&mut self, hand: Vec3, velocity: Vec3) {
        let yoyo = self.yoyo_index();
        let link = &mut self.links[yoyo];
        link.position = hand;
        link.collision.cur_pos = link.position;
        link.collision.last_pos = link.collision.cur_pos;
        link.active = true;
        link.velocity = velocity;
        self.motion = super::motion::TETHERED;
    }

    /// it_802BFE5C (802BFE5C): the yo-yo thrown for the charge, its
    /// horizontal speed turned to the item's facing; the item goes to
    /// motion 2.
    pub fn throw(&mut self, velocity: Vec3) {
        let facing = self.facing;
        let yoyo = self.yoyo_index();
        let link = &mut self.links[yoyo];
        link.velocity = velocity;
        link.velocity.x = facing * magnitude(link.velocity.x);
        self.motion = super::motion::SWINGING;
    }

    /// it_802BF28C (802BF28C), motion 1: the yo-yo pinned at the smash's
    /// hitbox point, each paid-out link pulled from the hand to within
    /// [x10, xC] of the one before it, and links paid out while the hand is
    /// out of reach. `scale` is the owner's model scale (ftLib_80086A0C).
    pub fn tether(&mut self, attributes: &YoyoAttributes, point: Vec3, hand: Vec3, scale: f32) {
        // retail 802BF2D4 / 802BF2EC: fmuls.
        let shortest = attributes.min_link_length * scale;
        let longest = attributes.link_length * scale;
        let mut cur = self.yoyo_index();
        self.links[cur].position = point;
        self.links[cur].velocity = Vec3::ZERO;
        for next in (0..cur).rev() {
            if self.links[next].active {
                self.links[next].position = hand;
                let base = self.links[cur].position;
                let (dir, distance) = direction(self.links[next].position, base);
                if distance > longest {
                    self.links[next].position = place(dir, longest, base);
                } else if distance < shortest {
                    let (_, from_hand) = direction(self.links[next].position, hand);
                    if from_hand <= DRAWN_IN_DISTANCE {
                        self.links[next].active = false;
                    } else {
                        self.links[next].position = place(dir, shortest, base);
                    }
                }
            } else {
                let base = self.links[cur].position;
                let (dir, distance) = direction(hand, base);
                if distance > longest {
                    self.links[next].position = place(dir, longest, base);
                    self.links[next].active = true;
                } else {
                    return;
                }
            }
            cur = next;
        }
    }

    /// it_802BF030 (802BF030) with a zero second argument: link `index`'s
    /// airborne pass from its last point (mpColl_800471F8). On a floor its
    /// x stays; against a wall or ceiling, a link between a paid-out one
    /// and the yo-yo's side is lifted by `lift`. Returns its floor, wall
    /// and ceiling bits.
    fn collide(&mut self, index: usize, lift: f32, map: &mut CollMap) -> i32 {
        let raise = index > 0 && self.links[index - 1].active && index < self.yoyo_index();
        let link = &mut self.links[index];
        let cd = &mut link.collision;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = link.position;
        link.on_floor = map.air_collide_pass(cd, None);
        let env = cd.env_flags;
        if env & FLOOR_MASK != 0 {
            cd.cur_pos.x = link.position.x;
        } else if env & WALL_CEILING_MASK != 0 && raise {
            // retail 802BF130: fadds.
            cd.cur_pos.y += lift;
        }
        link.position = cd.cur_pos;
        env & (FLOOR_MASK | WALL_CEILING_MASK)
    }

    /// it_802BF4A0 (802BF4A0), motion 2: the thrown yo-yo pulled toward its
    /// facing and falling, meeting the map; the paid-out links fall after
    /// it, links pay out while the hand is out of reach up to the smash's
    /// charge length, and the string is drawn back within reach of the
    /// hand, the yo-yo last.
    pub fn swing(
        &mut self,
        attributes: &YoyoAttributes,
        hand: Vec3,
        scale: f32,
        map: &mut CollMap,
    ) -> Swing {
        // retail 802BF4F0: fmuls.
        let size = attributes.link_length * scale;
        let facing = self.facing;
        let yoyo = self.yoyo_index();
        {
            let v = &mut self.links[yoyo].velocity;
            if magnitude(v.x) < attributes.max_speed_x {
                // retail 802BF514: fmadds(x28, facing, vel.x).
                v.x = fmadds(attributes.pull, facing, v.x);
            } else {
                v.x = attributes.max_speed_x * facing;
            }
            if magnitude(v.y) < attributes.max_fall_speed {
                v.y -= attributes.gravity;
            } else {
                v.y = -attributes.max_fall_speed;
            }
        }
        self.links[yoyo].integrate();
        let mut touching = self.collide(yoyo, size, map);
        if touching & FLOOR_MASK != 0 {
            let link = &mut self.links[yoyo];
            link.collision.cur_pos.y += FLOOR_LIFT;
            link.position.y = link.collision.cur_pos.y;
            if link.velocity.y < 0.0 {
                link.velocity.y = 0.0;
            }
        }
        touching &= WALL_CEILING_MASK;
        let charge_links = if self.owner_motion == UP_SMASH {
            attributes.up_charge_links
        } else {
            attributes.down_charge_links
        };
        let mut cur = yoyo;
        let mut count = 0;
        for next in (0..yoyo).rev() {
            count += 1;
            if self.links[next].active {
                self.links[next].velocity.y -= attributes.string_gravity;
                self.links[next].integrate();
                // Retail passes the link before it, not this one.
                self.collide(cur, size, map);
                let base = self.links[cur].position;
                let (dir, distance) = direction(self.links[next].position, base);
                if distance > size {
                    self.links[next].position = place(dir, size, base);
                }
                self.links[next].track();
                // retail 802BF654: fadd in double, then frsp.
                let height = (STRING_PASS_HEIGHT + f64::from(hand.y)) as f32;
                self.links[next].collide_from_height(height, map);
            } else if count > charge_links {
                self.links[next].position = hand;
            } else {
                let base = self.links[cur].position;
                let (dir, distance) = direction(hand, base);
                if distance > size {
                    let link = &mut self.links[next];
                    link.position = place(dir, size, base);
                    link.active = true;
                    link.reset_tracking();
                } else if touching != 0 {
                    return Swing::ShortTouching;
                } else {
                    return Swing::Short;
                }
            }
            cur = next;
        }
        // The inlined tail pass: the last link stepped is put `size` from
        // the hand, and each link back to the yo-yo within `size` of it.
        let (dir, _) = direction(self.links[cur].position, hand);
        self.links[cur].position = place(dir, size, hand);
        for prev in cur + 1..=yoyo {
            let base = self.links[prev - 1].position;
            let (dir, distance) = direction(self.links[prev].position, base);
            if distance > size {
                self.links[prev].position = place(dir, size, base);
            }
        }
        // retail 802BF7DC: fmuls.
        self.links[yoyo].velocity.x *= SWING_DRAG;
        Swing::Stepped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(links: i32) -> (YoyoString, YoyoAttributes) {
        let attributes = YoyoAttributes {
            links,
            link_length: 1.0,
            min_link_length: 0.7,
            ..Default::default()
        };
        (YoyoString::with_capacity(&attributes), attributes)
    }

    #[test]
    fn a_throw_turns_the_release_speed_to_the_items_facing() {
        let (mut s, _) = string(4);
        s.facing = -1.0;
        s.throw(Vec3::new(0.7, 0.8, 0.0));
        assert_eq!(s.yoyo().velocity, Vec3::new(-0.7, 0.8, 0.0));
        assert_eq!(s.motion, super::super::motion::SWINGING);
    }

    #[test]
    fn a_tethered_yoyo_pays_out_links_toward_a_hand_out_of_reach() {
        let (mut s, a) = string(4);
        s.send_out(Vec3::ZERO, Vec3::ZERO);
        s.tether(&a, Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), 1.0);
        let out: Vec<_> = s.links().iter().map(|l| (l.active, l.position.x)).collect();
        assert_eq!(out, [(true, 3.0), (true, 2.0), (true, 1.0), (true, 0.0)]);
    }

    #[test]
    fn a_tethered_yoyo_pays_out_nothing_with_the_hand_in_reach() {
        let (mut s, a) = string(4);
        s.send_out(Vec3::ZERO, Vec3::ZERO);
        s.tether(&a, Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0), 1.0);
        assert_eq!(s.links().iter().filter(|l| l.active).count(), 1);
    }
}
