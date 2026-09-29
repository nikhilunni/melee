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
    /// it_8027518C's flags (xDD1 b0, then it_8026BDB4 clears xDC8 x1A): an
    /// exploding item neither pushes nor is pushed. Its explosion lifetime
    /// and destroy effect are the caller's.
    pub fn mark_exploding(&mut self) {
        self.exploding = true;
        self.pushable = false;
    }
}

impl ItemPool {
    /// it_802722B0 (802722B0), at the end of an item's accessory proc: the
    /// first item in the list samples every fighter's position.
    pub fn sample_fighters_for_push(&mut self, id: u32, positions: impl IntoIterator<Item = Vec3>) {
        if self.items.first().map(|item| item.id) == Some(id) {
            self.fighter_push.record(positions);
        }
        // it_80272280: x1B clears for the next animation proc's item push.
        if let Some(item) = self.get_mut(id) {
            item.push_settled = false;
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
                self.push_from_items(index);
            }
        }
        // it_80271F78 pushes heavy items (crates, barrels) off each other;
        // none is a supported kind.
        self.items[index].push_settled = true;
    }

    /// it_80271D2C (80271D2C): another grounded item under this one pushes
    /// it. Not ported: no supported scene leaves two such items touching.
    fn push_from_items(&self, index: usize) {
        let item = &self.items[index];
        for (other_index, other) in self.items.iter().enumerate() {
            let eligible = other_index != index
                && !other.held
                && other.ground_or_air == GroundOrAir::Ground
                && !other.exploding
                && (other.hold_kind != 3 || item.pushed_by_open_palm);
            if eligible
                && overlaps(
                    other.root_translation.x,
                    other.root_translation.y,
                    &item.push_box,
                    &other.push_box,
                    item.root_translation,
                )
            {
                unimplemented!("it_80271D2C: an item pushed by another item");
            }
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
