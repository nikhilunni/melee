//! Grounded items pushed out from under fighters and each other: itcoll.c
//! it_802721B8 (802721B8) at the end of each item's animation proc, over the
//! fighters' positions it_802722B0 (802722B0) sampled at the first item's
//! accessory proc (ftCo_80098634). The push is a one-frame nudge (x70) the
//! physics proc adds to the velocity.
use crate::{ItemCore, ItemPool};
use hsd_types::Vec3;
use melee_types::{mp::ItEcb, GroundOrAir};

/// ftCo_DownAttack.c `ecb_offset`: every fighter's push box.
const FIGHTER_PUSH_BOX: ItEcb = ItEcb {
    top: 14.0,
    bottom: 0.0,
    right: -3.0,
    left: 3.0,
};

/// Item_804A0CCC (Item_FtTrack): each fighter's cur_pos in fighter list
/// order, as it_802722B0 last saw them.
#[derive(Clone, Copy, Debug, Default)]
pub struct FighterPushSnapshot {
    positions: [Vec3; 8],
    count: usize,
}
impl FighterPushSnapshot {
    pub fn record(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.count = 0;
        for position in positions {
            self.positions[self.count] = position;
            self.count += 1;
        }
    }
    fn positions(&self) -> &[Vec3] {
        &self.positions[..self.count]
    }
}

/// itColl_chkECBOverlap (itcoll.c, inlined): `target` inside the sum of two boxes
/// placed at `x`, `y` (separate fadds, the boxes' sum first).
fn overlaps(x: f32, y: f32, a: &ItEcb, b: &ItEcb, target: Vec3) -> bool {
    let top = y + (a.top + b.top);
    let bottom = y + (a.bottom + b.bottom);
    let right = x + (a.right + b.right);
    let left = x + (a.left + b.left);
    top >= target.y && bottom <= target.y && right <= target.x && left >= target.x
}

impl ItemCore {
    /// it_80271A58 (80271A58), in the accessory proc's tail: the push box
    /// (xBEC) is the scaled box of the kind (xBDC), mirrored for a
    /// right-facing item whose model is locked to its facing (xDC8 x19),
    /// then turned with the model about its spin axis and boxed again
    /// (it_80271830): each side's point rotated about Z
    /// (lbVector_RotateAboutUnitAxis) and the four points' extremes taken,
    /// by fcmpo in retail's order (0x80271930..0x80271A40). The angle is
    /// first brought into [0, 2 pi] in double precision (0x80271878,
    /// 0x80271890: fadd / fsub, frsp).
    pub(crate) fn refresh_push_box(&mut self) {
        use melee_lb::vector::rotate;
        let mut b = self.push_box_base;
        if self.facing == 1.0 && self.spin_ignores_facing {
            let right = -b.right;
            b.right = -b.left;
            b.left = right;
        }
        let mut angle = self.ecb_angle();
        while angle < 0.0 {
            angle = (f64::from(angle) + core::f64::consts::TAU) as f32;
        }
        while f64::from(angle) > core::f64::consts::TAU {
            angle = (f64::from(angle) - core::f64::consts::TAU) as f32;
        }
        let axis = Vec3::new(0.0, 0.0, 1.0);
        let top = rotate(Vec3::new(0.0, b.top, 0.0), axis, angle);
        let bottom = rotate(Vec3::new(0.0, b.bottom, 0.0), axis, angle);
        let right = rotate(Vec3::new(b.right, 0.0, 0.0), axis, angle);
        let left = rotate(Vec3::new(b.left, 0.0, 0.0), axis, angle);
        self.push_box = ItEcb {
            left: greatest([top.x, bottom.x, right.x, left.x]),
            top: greatest([top.y, bottom.y, right.y, left.y]),
            right: least([top.x, bottom.x, right.x, left.x]),
            bottom: least([top.y, bottom.y, right.y, left.y]),
        };
    }

    /// it_8027518C's flags (xDD1 b0, then it_8026BDB4 clears xDC8 x1A): an
    /// exploding item neither pushes nor is pushed. Its explosion lifetime
    /// and destroy effect are the caller's.
    pub fn mark_exploding(&mut self) {
        self.exploding = true;
        self.pushable = false;
    }
}

/// `(a > b) ? a : b` over the four points, as it_80271830 folds them
/// (fcmpo, ble: an unordered pair takes the later point).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn greatest(values: [f32; 4]) -> f32 {
    let mut result = values[0];
    for value in &values[1..] {
        if !(result > *value) {
            result = *value;
        }
    }
    result
}
/// `(a < b) ? a : b` over the four points (fcmpo, bge).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn least(values: [f32; 4]) -> f32 {
    let mut result = values[0];
    for value in &values[1..] {
        if !(result < *value) {
            result = *value;
        }
    }
    result
}

impl ItemPool {
    /// Item_80269A9C's tail, at the end of an item's accessory proc: the
    /// push box follows the model (it_80271A58), x1B clears (it_80272280),
    /// and the first item in the list samples every fighter's position
    /// (it_802722B0).
    pub fn sample_fighters_for_push(&mut self, id: u32, positions: impl IntoIterator<Item = Vec3>) {
        if let Some(item) = self.get_mut(id) {
            item.refresh_push_box();
            // it_80272280: x1B clears for the next animation proc's item push.
            item.push_settled = false;
        }
        if self.items.first().map(|item| item.id) == Some(id) {
            self.fighter_push.record(positions);
        }
    }

    /// it_802721B8 (802721B8) then it_80272298: this frame's nudge.
    pub fn push_apart(&mut self, id: u32, rng: &core::cell::Cell<gekko_math::HsdRng>) {
        let snapshot = self.fighter_push;
        let push_speed = self.common().push_speed;
        let Some(index) = self.items.iter().position(|item| item.id == id) else {
            return;
        };
        let item = &mut self.items[index];
        item.nudge = Vec3::ZERO;
        let grounded = item.ground_or_air == GroundOrAir::Ground;
        if !item.held && item.pushable && grounded {
            if item.pushed_by_fighters {
                push_from_fighters(item, &snapshot, push_speed, rng);
            }
            if item.pushed_by_items {
                self.push_from_items(index, rng);
            }
        }
        let item = &self.items[index];
        if !item.held && item.ground_or_air == GroundOrAir::Ground && item.heavy {
            self.push_heavy(index, rng);
        }
        self.items[index].push_settled = true;
    }

    /// it_80271F78 (80271F78): a heavy item (a crate or barrel) on the
    /// ground is pushed off every other grounded heavy one it overlaps,
    /// whether or not either is pushable, as it_80271D2C pushes; the last
    /// in the list wins. An exploding one (xDD1 b0) pushes nothing.
    fn push_heavy(&mut self, index: usize, rng: &core::cell::Cell<gekko_math::HsdRng>) {
        let push_speed = self.common().push_speed;
        let target = self.items[index].root_translation;
        let push_box = self.items[index].push_box;
        let mut nudge = None;
        for (other_index, other) in self.items.iter().enumerate() {
            let eligible = other_index != index
                && !other.held
                && other.ground_or_air == GroundOrAir::Ground
                && !other.exploding
                && other.heavy;
            if !eligible
                || !overlaps(
                    other.root_translation.x,
                    other.root_translation.y,
                    &push_box,
                    &other.push_box,
                    target,
                )
            {
                continue;
            }
            let offset = target.x - other.root_translation.x;
            let direction = if gekko_math::msl::fabsf(offset) < 0.001 {
                if !other.push_settled {
                    if other.nudge.x < 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                } else {
                    let mut r = rng.get();
                    let side = r.randi(2);
                    rng.set(r);
                    if side != 0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            } else if offset < 0.0 {
                -1.0
            } else {
                1.0
            };
            // retail: fmuls.
            nudge = Some(push_speed * direction);
        }
        if let Some(nudge) = nudge {
            self.items[index].nudge.x = nudge;
        }
    }

    /// it_80271D2C (80271D2C): every other grounded item this one overlaps
    /// pushes it away from itself at the common push speed (it_804D6D28
    /// +7C); the last in the list wins. One straight above or below takes
    /// the side opposite to the other's own nudge while the other has not
    /// yet been pushed this frame (xDC8 x1B clear), and draws it otherwise
    /// (HSD_Randi(2)).
    fn push_from_items(&mut self, index: usize, rng: &core::cell::Cell<gekko_math::HsdRng>) {
        let push_speed = self.common().push_speed;
        let target = self.items[index].root_translation;
        let push_box = self.items[index].push_box;
        let open_palm = self.items[index].pushed_by_open_palm;
        let mut nudge = None;
        for (other_index, other) in self.items.iter().enumerate() {
            let eligible = other_index != index
                && !other.held
                && other.ground_or_air == GroundOrAir::Ground
                && !other.exploding
                && (other.hold_kind != 3 || open_palm);
            if !eligible
                || !overlaps(
                    other.root_translation.x,
                    other.root_translation.y,
                    &push_box,
                    &other.push_box,
                    target,
                )
            {
                continue;
            }
            let offset = target.x - other.root_translation.x;
            let direction = if gekko_math::msl::fabsf(offset) < 0.001 {
                if !other.push_settled {
                    if other.nudge.x < 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                } else {
                    let mut r = rng.get();
                    let side = r.randi(2);
                    rng.set(r);
                    if side != 0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            } else if offset < 0.0 {
                -1.0
            } else {
                1.0
            };
            // retail: fmuls.
            nudge = Some(push_speed * direction);
        }
        if let Some(nudge) = nudge {
            self.items[index].nudge.x = nudge;
        }
    }
}

/// it_80271B60 (80271B60): a fighter over the item pushes it away from
/// the fighter's position at the common push speed (it_804D6D28 +7C); one
/// straight above draws the side (HSD_Randi(2)). The last overlap wins.
fn push_from_fighters(
    item: &mut ItemCore,
    snapshot: &FighterPushSnapshot,
    push_speed: f32,
    rng: &core::cell::Cell<gekko_math::HsdRng>,
) {
    let target = item.root_translation;
    for position in snapshot.positions() {
        if !overlaps(
            position.x,
            position.y,
            &item.push_box,
            &FIGHTER_PUSH_BOX,
            target,
        ) {
            continue;
        }
        let offset = target.x - position.x;
        let direction = if gekko_math::msl::fabsf(offset) < 0.001 {
            let mut r = rng.get();
            let side = r.randi(2);
            rng.set(r);
            if side != 0 {
                1.0
            } else {
                -1.0
            }
        } else if offset < 0.0 {
            -1.0
        } else {
            1.0
        };
        // retail: fmuls.
        item.nudge.x = push_speed * direction;
    }
}
