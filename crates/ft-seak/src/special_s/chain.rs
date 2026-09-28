//! The chain's links, itseakchain.c and itlinkhookshot.c's link helpers.
//!
//! it_802BAF2C strings `link_count` ItemLink GObjs from the one nearest
//! Sheik's hand (the head, `ip->xDD4.x4`) to the tip (the tail, `x0`).
//! Here `links[0]` is the head and `links[len - 1]` the tip: retail's
//! `link->next` is the link toward the hand (`i - 1`) and `link->prev` the
//! one toward the tip (`i + 1`). Each link carries its own point CollData.
//!
//! Every multiply-add below was checked against the retail asm
//! (asm.py --fused); separately rounded products stay separate.
use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
use gekko_math::msl::{fctiwz, sqrtf};
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::FighterDescError;
use melee_mp::CollMap;
use melee_types::mp::collide;
use melee_types::mp::CollData;

/// itSeakChain_Attrs (it/itCharItems.h), the chain article's special
/// attributes (ftData.x48_items[3]->x4), with the two words ftseakspecials.c
/// reads through its own view (itChainSegment +4C, +50).
#[derive(Clone, Debug, PartialEq)]
pub struct ChainItemAttributes {
    /// +00: links (it_802BAF2C).
    pub link_count: i32,
    /// +04: the length of a link: no link strays further from its
    /// neighbour.
    pub link_length: f32,
    /// +10: a swung link's horizontal drag per frame.
    pub drag: f32,
    /// +14: the horizontal drag of a link lying on the floor.
    pub floor_drag: f32,
    /// +18: gravity.
    pub gravity: f32,
    /// +1C / +20: how strongly the stick's recent motion pulls a link.
    pub pull_x: f32,
    pub pull_y: f32,
    /// +24: the fastest a swung link moves sideways.
    pub max_speed_x: f32,
    /// +28: the vertical speed band inside which gravity is not applied.
    pub sag: f32,
    /// +2C: the fastest a swung link moves vertically.
    pub max_speed_y: f32,
    /// +30: the sideways pull while the chain last touched the floor.
    pub grounded_pull_scale: f32,
    /// +34: each link further from the hand feels this much less.
    pub falloff: f32,
    /// +38 / +3C: the stick's horizontal pull toward and away from the
    /// chain's facing.
    pub forward_gain: f32,
    pub backward_gain: f32,
    /// +40 / +44: its vertical pull up and down.
    pub up_gain: f32,
    pub down_gain: f32,
    /// +48: stick motion (and position) below this is ignored.
    pub stick_deadzone: f32,
    /// +4C: a hitbox point moving further than this in a frame keeps the
    /// hitboxes live (ftSk_SpecialS_80110BCC).
    pub hit_speed: f32,
    /// +50: the tip's speed when thrown (ftSk_SpecialS_CheckInitChain).
    pub throw_speed: f32,
    /// +54: the length reeled in per frame (it_802BC94C).
    pub retract_step: f32,
    /// +58: the tip's rebound off a wall while paying out.
    pub rebound: f32,
    /// +5C: a swung link's bounce off the floor.
    pub bounce: f32,
    /// +60: slower than this, a swung link settles on the floor.
    pub bounce_threshold: f32,
}

/// PlSk.dat ftData.x48_items: the chain article is the fourth.
const CHAIN_ARTICLE: u32 = 3;

impl ChainItemAttributes {
    pub fn read(archive: &Archive, fighter_data: u32) -> Result<Self, FighterDescError> {
        let r = archive.reader();
        let word = |offset| r.u32(offset);
        let items = word(fighter_data + 0x48)?;
        let article = word(items + CHAIN_ARTICLE * 4)?;
        let attributes = word(article + 4)?;
        let r = Reader::new(archive.reader().slice(attributes, 0x64)?);
        let f = |offset| r.f32(offset);
        Ok(Self {
            link_count: r.s32(0x00)?,
            link_length: f(0x04)?,
            drag: f(0x10)?,
            floor_drag: f(0x14)?,
            gravity: f(0x18)?,
            pull_x: f(0x1C)?,
            pull_y: f(0x20)?,
            max_speed_x: f(0x24)?,
            sag: f(0x28)?,
            max_speed_y: f(0x2C)?,
            grounded_pull_scale: f(0x30)?,
            falloff: f(0x34)?,
            forward_gain: f(0x38)?,
            backward_gain: f(0x3C)?,
            up_gain: f(0x40)?,
            down_gain: f(0x44)?,
            stick_deadzone: f(0x48)?,
            hit_speed: f(0x4C)?,
            throw_speed: f(0x50)?,
            retract_step: f(0x54)?,
            rebound: f(0x58)?,
            bounce: f(0x5C)?,
            bounce_threshold: f(0x60)?,
        })
    }
}

/// One ItemLink.
#[derive(Clone, Debug, Default)]
pub struct ChainLink {
    pub position: Vec3,
    pub velocity: Vec3,
    /// x2C_b0: paid out (drawn, simulated).
    pub active: bool,
    /// x2C_b1: its point touched the stage last frame.
    pub touching: bool,
    /// x2C_b2: the contact sound played on this contact (a one-bit field
    /// retail sets with 0xF and decrements).
    pub sound_latch: bool,
    pub collision: CollData,
}

/// lbAudioAx_800237A8(0x41F45, 0x7F, 0x40): a link striking the stage.
/// Sound only; no port consumer.
const _LINK_CONTACT_SOUND: u32 = 0x41F45;

/// it_802BB938's return mask: floor bits and wall bits.
const CONTACT_MASK: u32 = collide::FLOOR_MASK | 0xFFF;
/// Wall bits (it_802BBD64 / it_802BBED0 mask the tip's contact with them).
const WALL_BITS: u32 = 0xFFF;

/// it_802A3C98 (802A3C98): the unit vector from `b` to `a` and the
/// distance. The squares round separately; the length is the inline
/// frsqrte sqrtf; its reciprocal divides in double and rounds once.
pub fn direction(a: Vec3, b: Vec3) -> (Vec3, f32) {
    let d = Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z);
    let length = sqrtf(d.z * d.z + (d.x * d.x + d.y * d.y));
    let inverse = if f64::from(length) == 0.0 {
        0.0
    } else {
        (1.0_f64 / f64::from(length)) as f32
    };
    (
        Vec3::new(d.x * inverse, d.y * inverse, d.z * inverse),
        length,
    )
}

/// `anchor + direction * length`, each component one fmadds (retail
/// 802BBDD0..DF8 and the other link placements).
fn along(direction: Vec3, length: f32, anchor: Vec3) -> Vec3 {
    Vec3::new(
        fmadds(direction.x, length, anchor.x),
        fmadds(direction.y, length, anchor.y),
        fmadds(direction.z, length, anchor.z),
    )
}

/// Keep `position` within `length` of `anchor`.
fn keep_within(position: &mut Vec3, anchor: Vec3, length: f32) {
    let (dir, distance) = direction(*position, anchor);
    if distance > length {
        *position = along(dir, length, anchor);
    }
}

impl ChainLink {
    /// it_802A24D0 (802A24D0): the link's CollData at its position, a
    /// point-sized fixed box of `size`.
    pub fn reset(&mut self, map: &CollMap, size: f32) {
        *self = Self::default();
        self.collision.cur_pos = self.position;
        self.collision.last_pos = self.collision.cur_pos;
        map.coll_data_init(&mut self.collision);
        // x34_flags.b1234 = 5.
        self.collision.x34_flags.b1234 = 5;
        melee_mp::set_ecb_source_fixed(&mut self.collision, size, size, size, size);
    }
    /// it_802A4420 (802A4420).
    fn integrate(&mut self) {
        self.position.x += self.velocity.x;
        self.position.y += self.velocity.y;
        self.position.z += self.velocity.z;
    }
    /// it_802A43EC (802A43EC).
    fn track(&mut self) {
        self.collision.last_pos = self.collision.cur_pos;
        self.collision.cur_pos = self.position;
    }
    /// it_802A43B8 (802A43B8).
    fn place(&mut self) {
        self.collision.cur_pos = self.position;
        self.collision.last_pos = self.collision.cur_pos;
    }
}

/// it_802BB938 (802BB938): link `i` against the stage, from the link
/// toward the hand when that one is out. A floor contact keeps the link's
/// own x; a wall contact between two paid-out links lifts it by `lift`.
fn collide(links: &mut [ChainLink], i: usize, sound: bool, lift: f32, map: &mut CollMap) -> u32 {
    let hand_side = (i > 0 && links[i - 1].active).then(|| links[i - 1].position);
    let tip_side = i + 1 < links.len();
    let link = &mut links[i];
    link.collision.last_pos = hand_side.unwrap_or(link.collision.cur_pos);
    link.collision.cur_pos = link.position;
    if map.point_collide_pass(&mut link.collision) {
        if !link.touching && sound && !link.sound_latch {
            link.sound_latch = true;
        }
        link.touching = true;
    } else {
        link.touching = false;
        link.sound_latch = false;
    }
    let env = link.collision.env_flags as u32;
    if env & collide::FLOOR_MASK != 0 {
        link.collision.cur_pos.x = link.position.x;
    } else if env & WALL_BITS != 0 && hand_side.is_some() && tip_side {
        link.collision.cur_pos.y += lift;
    }
    link.position = link.collision.cur_pos;
    env & CONTACT_MASK
}

/// it_802BBB0C / it_802BBC38 (802BBB0C / 802BBC38): link `i` at `scale`
/// from `anchor` along its bearing, and every link beyond it toward the
/// tip hangs from its neighbour under gravity.
fn hang(
    links: &mut [ChainLink],
    i: usize,
    anchor: Vec3,
    scale: f32,
    a: &ChainItemAttributes,
    map: &mut CollMap,
) {
    let (dir, _) = direction(links[i].position, anchor);
    links[i].position = along(dir, scale, anchor);
    for next in i + 1..links.len() {
        let link = &mut links[next];
        link.velocity.y -= a.gravity;
        link.integrate();
        link.track();
        collide(links, next, false, a.link_length, map);
        let anchor = links[next - 1].position;
        keep_within(&mut links[next].position, anchor, a.link_length);
    }
}

/// How a paying-out or falling chain fared this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Payout {
    /// Still paying out, the tip clear of walls (0).
    Moving,
    /// Still paying out with the tip against a wall (1).
    Struck,
    /// Every link is out (2).
    Taut,
}

/// The links from the tip toward the hand after the tip moved: those out
/// follow; the first still in comes out once the hand is a link away.
fn pay_out(
    links: &mut [ChainLink],
    hand: Vec3,
    tip_contact: u32,
    a: &ChainItemAttributes,
    map: &mut CollMap,
    mut settle: impl FnMut(&mut ChainLink),
) -> Payout {
    let mut cur = links.len() - 1;
    while cur > 0 {
        let next = cur - 1;
        let anchor = links[cur].position;
        if links[next].active {
            settle(&mut links[next]);
            keep_within(&mut links[next].position, anchor, a.link_length);
            links[next].track();
        } else {
            let (dir, distance) = direction(hand, anchor);
            if distance > a.link_length {
                let link = &mut links[next];
                link.position = along(dir, a.link_length, anchor);
                link.active = true;
                link.place();
            } else if tip_contact != 0 {
                return Payout::Struck;
            } else {
                return Payout::Moving;
            }
        }
        cur = next;
    }
    hang(links, 0, hand, a.link_length, a, map);
    Payout::Taut
}

/// it_802BBD64 (802BBD64): the thrown tip flies on.
pub fn extend(
    links: &mut [ChainLink],
    hand: Vec3,
    a: &ChainItemAttributes,
    map: &mut CollMap,
) -> Payout {
    let tip = links.len() - 1;
    links[tip].integrate();
    let contact = collide(links, tip, true, a.link_length, map) & WALL_BITS;
    pay_out(links, hand, contact, a, map, |_| {})
}

/// it_802BBED0 (802BBED0): after striking a wall the tip falls, and the
/// links out fall with it, less the further from the tip.
pub fn fall(
    links: &mut [ChainLink],
    hand: Vec3,
    a: &ChainItemAttributes,
    map: &mut CollMap,
) -> Payout {
    let tip = links.len() - 1;
    links[tip].velocity.y -= a.gravity;
    links[tip].integrate();
    let contact = collide(links, tip, true, a.link_length, map) & WALL_BITS;
    let mut weight = 1.0_f32;
    pay_out(links, hand, contact, a, map, |link| {
        // retail 802BBF44: fnmsubs, -(gravity * weight - vy).
        link.velocity.y = fnmsubs(a.gravity, weight, link.velocity.y);
        weight *= a.falloff;
        link.integrate();
    })
}

/// The stick's recent motion a swung chain remembers (itSeakChain_ItemVars
/// history, x/y of fifteen entries; the swing reads the first ten).
pub const HISTORY_LEN: usize = 15;

/// What it_802BC080 reads of Sheik.
pub struct SwingInput {
    /// fp->u.sk.lstick_delta (ftSk_SpecialS_80110788).
    pub stick_delta: (f32, f32),
    /// fp->input.lstick[0] (ftSk_SpecialS_80110F58 / 80110F64).
    pub stick: (f32, f32),
    /// fp->mv.co.common.x1C read as a word: the hitboxes' live frames.
    pub hit_frames: i32,
    /// The item's facing (its spawn facing).
    pub facing: f32,
}

/// The swing's state kept on the item (itSeakChain_ItemVars).
#[derive(Clone, Debug, Default)]
pub struct SwingState {
    pub history: [(f32, f32); HISTORY_LEN],
    /// x10: the floor bits the last link met last frame.
    pub floor_contact: u32,
    /// x18: the hitboxes' live frames last frame.
    pub hit_frames: i32,
}

/// The sideways drag: toward zero by `limit`, zero within it.
fn drag(velocity: f32, limit: f32, negative_limit: f32) -> f32 {
    if velocity > limit {
        velocity - limit
    } else if velocity < negative_limit {
        velocity + limit
    } else {
        0.0
    }
}

/// Clamp to `maximum` by magnitude; the negative bound is computed apart
/// (`negative`), as retail does.
fn clamp_magnitude(velocity: f32, maximum: f32, negative: f32) -> f32 {
    if velocity.abs() > maximum {
        if velocity > 0.0 {
            maximum
        } else {
            negative
        }
    } else {
        velocity
    }
}

/// it_802BC080 (802BC080): the taut chain swings with the stick.
pub fn swing(
    links: &mut [ChainLink],
    state: &mut SwingState,
    hand: Vec3,
    input: &SwingInput,
    a: &ChainItemAttributes,
    map: &mut CollMap,
) {
    // The first link out from the hand.
    let mut cur = 0;
    while cur + 1 < links.len() && !links[cur].active {
        cur += 1;
    }
    let sound = state.hit_frames != 0;
    // retail 802BC0E0: fmsubs, 0.5 * count - 1.
    let last = fctiwz(fmsubs(0.5, a.link_count as f32, 1.0)) as usize;
    for i in 0..last {
        state.history[last - i] = state.history[last - 1 - i];
    }
    let (dx, dy) = input.stick_delta;
    state.history[0].0 = if dx.abs() > a.stick_deadzone {
        let sign = if dx < 0.0 { -1 } else { 1 };
        if input.facing == sign as f32 {
            dx * a.forward_gain
        } else {
            dx * a.backward_gain
        }
    } else {
        0.0
    };
    state.history[0].1 = if dy.abs() > a.stick_deadzone {
        let sign = if dy < 0.0 { -1 } else { 1 };
        if sign as f32 > 0.0 {
            dy * a.up_gain
        } else {
            dy * a.down_gain
        }
    } else {
        0.0
    };
    state.hit_frames = input.hit_frames;
    swing_first(&mut links[cur], state.history[0], hand, a);

    let grounded_pull = state.floor_contact != 0;
    let (lx, ly) = input.stick;
    let still = if lx.abs() < a.stick_deadzone && ly.abs() < a.stick_deadzone {
        3
    } else if ly < -0.5 {
        1
    } else {
        2
    };
    let mut scale = 1.0_f32 * a.falloff;
    let mut counter = 0;
    let mut contact = 0;
    for next in cur + 1..links.len() {
        let link = &mut links[next];
        // retail 802BC67C..84: fctiwz(0.5 * (counter + 1)).
        let (hx, hy) = state.history[fctiwz(0.5 * (counter + 1) as f32) as usize];
        let mut px = scale * (hx * a.pull_x);
        let py = scale * (hy * a.pull_y);
        if grounded_pull {
            px *= a.grounded_pull_scale;
        }
        link.velocity.x += px;
        link.velocity.y += py;
        link.velocity.x = drag(link.velocity.x, a.drag * scale, -a.drag * scale);
        link.velocity.x = clamp_magnitude(
            link.velocity.x,
            a.max_speed_x * scale,
            -a.max_speed_x * scale,
        );
        let fall = a.gravity * scale;
        if link.velocity.y > fall - a.sag {
            link.velocity.y -= fall;
        // retail 802BC784: fmsubs, -gravity * scale - sag.
        } else if link.velocity.y < fmsubs(-a.gravity, scale, a.sag) {
            link.velocity.y += fall;
        }
        link.velocity.y = clamp_magnitude(
            link.velocity.y,
            a.max_speed_y * scale,
            -a.max_speed_y * scale,
        );
        counter += 1;
        scale *= a.falloff;
        link.integrate();
        contact = if counter > still {
            // it_802BBAEC: every fifth link may sound its contact.
            let sound = counter % 5 == 0 && sound;
            collide(links, next, sound, a.link_length, map)
        } else {
            0
        } & collide::FLOOR_MASK;
        let link = &mut links[next];
        if contact != 0 {
            if link.velocity.y.abs() > a.bounce_threshold {
                link.velocity.y *= -a.bounce;
            } else {
                link.velocity.x = drag(link.velocity.x, a.floor_drag, -a.floor_drag);
                link.velocity.y = 0.0;
            }
        }
        let anchor = links[next - 1].position;
        keep_within(&mut links[next].position, anchor, a.link_length);
    }
    state.floor_contact = contact;
}

/// it_802BC080's first link: the newest stick motion at full strength.
fn swing_first(link: &mut ChainLink, pull: (f32, f32), hand: Vec3, a: &ChainItemAttributes) {
    // retail 802BC43C / 802BC450: fmadds.
    link.velocity.x = fmadds(pull.0, a.pull_x, link.velocity.x);
    link.velocity.y = fmadds(pull.1, a.pull_y, link.velocity.y);
    link.velocity.x = drag(link.velocity.x, a.drag, -a.drag);
    link.velocity.x = clamp_magnitude(link.velocity.x, a.max_speed_x, -a.max_speed_x);
    if link.velocity.y > a.gravity - a.sag {
        link.velocity.y -= a.gravity;
    } else if link.velocity.y < -a.gravity - a.sag {
        link.velocity.y += a.gravity;
    }
    link.velocity.y = clamp_magnitude(link.velocity.y, a.max_speed_y, -a.max_speed_y);
    link.integrate();
    keep_within(&mut link.position, hand, a.link_length);
}

/// it_802BC94C (802BC94C) with Item_RetractChain: reel in a step. Links
/// closer to the hand than the step go back in; the rest hang from the
/// hand. True once only the tip is left.
pub fn retract(
    links: &mut [ChainLink],
    hand: Vec3,
    a: &ChainItemAttributes,
    map: &mut CollMap,
) -> bool {
    let mut cur = 0;
    while cur + 1 < links.len() && !links[cur].active {
        cur += 1;
    }
    let (_, mut distance) = direction(links[cur].position, hand);
    while cur + 1 < links.len() && a.retract_step > distance {
        links[cur].active = false;
        distance = direction(links[cur + 1].position, hand).1;
        cur += 1;
    }
    let mut remaining = distance - a.retract_step;
    if remaining > a.link_length {
        remaining = a.link_length;
    }
    hang(links, cur, hand, remaining, a, map);
    cur + 1 == links.len()
}

/// it_802BCB88 (802BCB88): the hitbox points along the links out, from
/// the hand: hitboxes 0..2 at every `count / 3`-th link (the ones past the
/// last reached stay on it), hitbox 3 on the tip. Link orientation is
/// render state.
pub fn hitbox_points(links: &[ChainLink], count: i32, mut place: impl FnMut(usize, Vec3)) {
    let stride = (count / 3) as usize;
    let mut first = 0;
    while !links[first].active {
        first += 1;
    }
    for (count, (i, link)) in links.iter().enumerate().skip(first).enumerate() {
        if count % stride == 0 {
            for hitbox in (count / stride + 1..=3).rev() {
                place(hitbox - 1, link.position);
            }
        }
        if i + 1 == links.len() {
            place(3, link.position);
        }
    }
}
