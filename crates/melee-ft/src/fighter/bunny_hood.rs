//! The Bunny Hood (ftcommon.c 8007F8E8..8007FA58, fighter.c:1514-1522 and
//! 2700-2706): worn on the head (x197C) for its frames (x2014), it speeds
//! the fighter's runs and jumps (ftCo_800D105C's Fighter_804D6520) until
//! they run out, a launch knocks it off or the fighter dies.

use super::assets::{FighterAssets, Result};
use super::item_pickup::HeldItem;
use super::FighterCore;

/// Fighter_804D6520 (PlCo `ftLoadCommonData` entry 13): the hood's
/// attribute multipliers (ftCo_800D105C).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HoodMultipliers {
    /// +0: the walk's acceleration multiplier (ftCo_Walk_Enter).
    pub walk_acceleration: f32,
    /// +4 / +8 / +C: dash_accel_mul, dash_accel_base, dash_max_velocity.
    pub dash_accel_mul: f32,
    pub dash_accel_base: f32,
    pub dash_max_velocity: f32,
    /// +10 / +14 / +18 / +1C: jump_h_initial_velocity,
    /// jump_v_initial_velocity (and the multijumps' and animated aerial
    /// jumps' rises), hop_v_initial_velocity, jump_h_max_velocity.
    pub jump_h: f32,
    pub jump_v: f32,
    pub hop_v: f32,
    pub jump_h_max: f32,
    /// +20 / +24 / +28: gravity, terminal_velocity, fast_fall_velocity.
    pub gravity: f32,
    pub terminal_velocity: f32,
    pub fast_fall_velocity: f32,
    /// +2C / +30 / +34 / +38: the ledge and wall jumps' velocities.
    pub ledge_jump_h: f32,
    pub ledge_jump_v: f32,
    pub wall_jump_h: f32,
    pub wall_jump_v: f32,
}
impl HoodMultipliers {
    pub fn read(common: &hsd_archive::Archive, root: u32) -> Result<Self> {
        let table = common.link(root + 13 * 4)?.ok_or("missing hood multipliers")?;
        let r = common.reader();
        let f = |offset: u32| r.f32(table + offset);
        Ok(Self {
            walk_acceleration: f(0x0)?,
            dash_accel_mul: f(0x4)?,
            dash_accel_base: f(0x8)?,
            dash_max_velocity: f(0xC)?,
            jump_h: f(0x10)?,
            jump_v: f(0x14)?,
            hop_v: f(0x18)?,
            jump_h_max: f(0x1C)?,
            gravity: f(0x20)?,
            terminal_velocity: f(0x24)?,
            fast_fall_velocity: f(0x28)?,
            ledge_jump_h: f(0x2C)?,
            ledge_jump_v: f(0x30)?,
            wall_jump_h: f(0x34)?,
            wall_jump_v: f(0x38)?,
        })
    }

    /// ftCo_800D105C's hood arm (ftchangeparam.c:200-214): separate fmuls.
    pub fn apply(&self, a: &mut crate::desc::attributes::FighterAttributes) {
        a.running.dash_accel_mul *= self.dash_accel_mul;
        a.running.dash_accel_base *= self.dash_accel_base;
        a.running.dash_max_velocity *= self.dash_max_velocity;
        a.jumping.jump_h_initial_velocity *= self.jump_h;
        a.jumping.jump_v_initial_velocity *= self.jump_v;
        a.jumping.jump_h_max_velocity *= self.jump_h_max;
        a.jumping.hop_v_initial_velocity *= self.hop_v;
        a.air.gravity *= self.gravity;
        a.air.terminal_velocity *= self.terminal_velocity;
        a.air.fast_fall_velocity *= self.fast_fall_velocity;
        a.ledge.ledge_jump_horizontal_velocity *= self.ledge_jump_h;
        a.ledge.ledge_jump_vertical_velocity *= self.ledge_jump_v;
        a.wall.wall_jump_horizontal_velocity *= self.wall_jump_h;
        a.wall.wall_jump_vertical_velocity *= self.wall_jump_v;
    }
}

/// x197C and x2014: the hood worn and its frames left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BunnyHood {
    pub item: u32,
    pub frames: i32,
    /// Fighter_804D6520 +14, read by ft_800851D0 where no assets reach.
    pub jump_v: f32,
}

/// ft_800851D0 (800851D0): an animation-driven aerial jump's rise, times
/// the hood's +14 while one is worn (one fmuls).
pub(super) fn animated_rise(rise: f32, hood_jump_v: Option<f32>) -> f32 {
    match hood_jump_v {
        Some(scale) => rise * scale,
        None => rise,
    }
}

impl FighterCore {
    /// ftCommon_8007FA58 (8007FA58), the Bunny Hood's pickup. A second
    /// hood only renews the frames (it_8026B54C) and is gone. Otherwise it
    /// leaves the hand (Item_8026ABD8 with no push and xC44 0, muted by
    /// xDCD b2) and hangs from the head part (Item_8026AB54 at ftData x8
    /// +0x12), and the attributes are recomputed. The sound and rumble are
    /// not simulated; the hood model's joint offsets and scale
    /// (ftCommon_8007FA00) are presentation.
    pub(super) fn wear_bunny_hood(&mut self, held: HeldItem, assets: &FighterAssets) {
        let frames = held.bunny_hood.expect("a Bunny Hood's frames");
        if let Some(hood) = &mut self.status.bunny_hood {
            hood.frames = frames;
            self.item_requests
                .push(melee_it::ItemRequest::Destroy { item: held.item });
            self.release_held_item(held.item, assets);
            return;
        }
        self.drop_held_item(held, 0.0, assets);
        self.item_requests.push(melee_it::ItemRequest::PickUp {
            item: held.item,
            part: self.bones.model.held_item,
        });
        self.status.bunny_hood = Some(BunnyHood {
            item: held.item,
            frames,
            jump_v: assets.bunny_hood.jump_v,
        });
        self.commands.wearing_bunny_hood = true;
        self.recompute_attributes(assets);
    }

    /// ftCommon_8007F9B4 (8007F9B4): the hood is no longer worn; the
    /// attributes are recomputed. ftCommon_8007F8E8's kind hook (Pikachu's
    /// ears) is presentation.
    fn take_off_bunny_hood(&mut self, assets: &FighterAssets) {
        self.status.bunny_hood = None;
        self.commands.wearing_bunny_hood = false;
        self.recompute_attributes(assets);
    }

    /// Fighter_8006A360 (fighter.c:1514-1522): the hood's countdown; at 0
    /// it is gone (Item_8026A8EC).
    pub(super) fn tick_bunny_hood(&mut self, assets: &FighterAssets) {
        let Some(hood) = &mut self.status.bunny_hood else {
            return;
        };
        if hood.frames == 0 {
            return;
        }
        hood.frames -= 1;
        if hood.frames == 0 {
            let item = hood.item;
            self.item_requests
                .push(melee_it::ItemRequest::Destroy { item });
            self.take_off_bunny_hood(assets);
        }
    }

    /// Fighter_8006CDA4 (fighter.c:2700-2706): a launch draws whether it
    /// knocks the hood off (Randi(PlCo +418) < the damage); it leaves the
    /// head as a dropped item would (Item_8026ABD8, xC44 1).
    pub(super) fn maybe_knock_off_bunny_hood(
        &mut self,
        damage: i32,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        let Some(hood) = self.status.bunny_hood else {
            return;
        };
        if rng.randi(assets.damage.item_drop_range) >= damage {
            return;
        }
        let mut holder = self.item_holder(self.bones.model.held_item, assets);
        let position = holder.part_position();
        let hand = holder.part_matrix();
        let (center, attack) = (holder.center, holder.attack);
        self.item_requests.push(melee_it::ItemRequest::Drop {
            item: hood.item,
            position,
            hand,
            speed: 1.0,
            center,
            attack,
        });
        self.take_off_bunny_hood(assets);
    }

    /// ft_0D31.c:116-120: a death takes the hood with it (Item_8026A8EC).
    pub(super) fn lose_bunny_hood(&mut self, assets: &FighterAssets) {
        if let Some(hood) = self.status.bunny_hood {
            self.item_requests
                .push(melee_it::ItemRequest::Destroy { item: hood.item });
            self.take_off_bunny_hood(assets);
        }
    }

    /// The hood's +14 while worn, for the rises the animation drives
    /// (ft_800851D0).
    pub fn hood_jump_v(&self) -> Option<f32> {
        self.status.bunny_hood.map(|hood| hood.jump_v)
    }
}
