//! Throwing a held item: ftCo_ItemThrow.c (80094E54..80096E68).
//!
//! A throw state plays its animation while an accessory (ftCo_80095EFC)
//! follows the hand. When the script sets the release flag, the item leaves
//! from the hand's position interpolated back to the flag's moment, with a
//! velocity from the throw table (PlCo pData[1]) and the fighter's
//! multipliers; the scene hands it to the item (Item_8026AD20).
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

/// Fighter_804D6550 (PlCo pData[1]), one row per throw state from
/// LightThrowF (94) to HeavyThrowLw4 (119).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItemThrowRow {
    /// Throw speed before item_throw_velocity_multiplier.
    pub speed: f32,
    /// Launch angle in radians.
    pub angle: f32,
    /// The thrown item's hitbox damage scale before
    /// heavy_throw_velocity_multiplier (Item_8026AD20's xC44).
    pub damage_scale: f32,
}

pub const ITEM_THROW_ROWS: usize = 26;

/// ftCo_80095700: the back throws aim behind the fighter.
const BACK_THROWS: [u16; 6] = [95, 101, 105, 109, 113, 117];

/// mv.co.itemthrow / itemthrow4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemThrowState {
    /// mv.co.itemthrow.facing_dir: the throw's direction.
    pub facing: f32,
    /// mv.co.itemthrow4.x8: the hand's position at the previous accessory.
    pub hand: Vec3,
    /// mv.co.itemthrow4.anim_spd: the entry's animation rate, kept by a
    /// ground/air switch.
    pub rate: f32,
    /// facing_dir1: the facing at entry, restored for a ground/air switch.
    pub entry_facing: f32,
}

pub fn read_throw_table(
    common: &hsd_archive::Archive,
    root: u32,
) -> hsd_archive::desc::Result<[ItemThrowRow; ITEM_THROW_ROWS]> {
    let table = common
        .link(root + 4)?
        .ok_or(hsd_archive::desc::DescError::NullPointer {
            field: "ftLoadCommonData[1]",
            at: root + 4,
        })?;
    let r = common.reader();
    let mut rows = [ItemThrowRow::default(); ITEM_THROW_ROWS];
    for (index, row) in rows.iter_mut().enumerate() {
        let at = table + index as u32 * 12;
        *row = ItemThrowRow {
            speed: r.f32(at)?,
            angle: r.f32(at + 4)?,
            damage_scale: r.f32(at + 8)?,
        };
    }
    Ok(rows)
}

impl Fighter {
    /// ftCo_80095A30 (80095A30): a held item leaves toward the stick, smash
    /// directions before tilts, else forward.
    pub(super) fn enter_ground_item_throw(&mut self, assets: &FighterAssets) -> Result<()> {
        let held = self
            .core
            .held_item
            .expect("an item throw needs a held item");
        let input = &self.core.input;
        let common = &assets.input;
        let stick = input.current.stick;
        let forward = stick.x * self.core.physics.facing >= 0.0;
        let horizontal_age = f32::from(input.horizontal.tilt);
        let vertical_age = f32::from(input.vertical.tilt);
        // ftCo_GetLStickAngle (8007D964): atan2f(y, |x|).
        let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
        let state = if fabsf(stick.x) >= common.thresholds.dash_smash_stick_threshold
            && horizontal_age
                < common.thresholds.dash_smash_window as f32 + common.item_smash_window_extension
        {
            if forward {
                S::LightThrowF4
            } else {
                S::LightThrowB4
            }
        } else if stick.y >= common.up_smash_threshold
            && vertical_age
                < common.up_smash_window + self.core.attributes.jumping.jump_startup_time
        {
            S::LightThrowHi4
        } else if stick.y <= common.down_smash_threshold && vertical_age < common.down_smash_window
        {
            S::LightThrowLw4
        } else if fabsf(stick.x) >= common.side_tilt_threshold && fabsf(angle) <= common.tilt_angle
        {
            if forward {
                S::LightThrowF
            } else {
                S::LightThrowB
            }
        } else if stick.y >= common.up_tilt_threshold && angle > common.tilt_angle {
            S::LightThrowHi
        } else if stick.y <= common.down_tilt_threshold && angle < -common.tilt_angle {
            S::LightThrowLw
        } else if held.use_kind == 0 {
            S::LightThrowF
        } else {
            unimplemented!("ftCo_80095A30: swinging a held item (state 99)");
        };
        self.enter_item_throw(state, assets)
    }

    /// ftCo_80095328 (80095328): a held item thrown in the air, by a C-stick
    /// flick (ftCo_800DF50C) or A (ftCo_80094E54). Returns whether it threw.
    pub fn try_air_item_throw(&mut self, assets: &FighterAssets) -> Result<bool> {
        use crate::input::Buttons;
        let Some(held) = self.core.held_item else {
            return Ok(false);
        };
        let input = &self.core.input;
        let common = &assets.input;
        let throwable = held.use_kind == 0;
        let lr = input.current.held.intersects(Buttons::SHIELD);
        // ftCo_800DF478: a C-stick axis crossing its aerial threshold.
        let (cstick, previous) = (input.current.cstick, input.previous.cstick);
        let flicked = (fabsf(previous.x) < common.aerial_horizontal_threshold
            && fabsf(cstick.x) >= common.aerial_horizontal_threshold)
            || (fabsf(previous.y) < common.aerial_vertical_threshold
                && fabsf(cstick.y) >= common.aerial_vertical_threshold);
        let (stick, horizontal_age, vertical_age) = if flicked {
            assert!(
                throwable,
                "ftCo_800DF50C: gm_8016B0FC for a non-throwable item"
            );
            (cstick, 0.0, 0.0)
        } else if input.pressed.intersects(Buttons::A) && (lr || throwable) {
            let stick = input.current.stick;
            (
                stick,
                f32::from(input.horizontal.tilt),
                f32::from(input.vertical.tilt),
            )
        } else {
            return Ok(false);
        };
        let state = if fabsf(stick.x) < common.aerial_horizontal_threshold
            && fabsf(stick.y) < common.aerial_vertical_threshold
        {
            if !throwable || lr {
                // The state is unchanged; the aerial catch stays shut until
                // the next grounded motion entry (x2224_b1).
                self.core.drop_held_item(held, assets);
                self.core.item_catch_locked = true;
                return Ok(true);
            }
            S::LightThrowAirF
        } else {
            let smash = |age: f32| age < assets.air_smash_throw_window as f32;
            let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
            if angle > common.tilt_angle {
                if smash(vertical_age) {
                    S::LightThrowAirHi4
                } else {
                    S::LightThrowAirHi
                }
            } else if angle < -common.tilt_angle {
                if smash(vertical_age) {
                    S::LightThrowAirLw4
                } else {
                    S::LightThrowAirLw
                }
            } else if stick.x * self.core.physics.facing >= 0.0 {
                if smash(horizontal_age) {
                    S::LightThrowAirF4
                } else {
                    S::LightThrowAirF
                }
            } else if smash(horizontal_age) {
                S::LightThrowAirB4
            } else {
                S::LightThrowAirB
            }
        };
        self.enter_item_throw(state, assets)?;
        Ok(true)
    }

    /// ftCo_800D8A38 / ftCo_800D8AE0's item half (ftCo_80095254): A with a
    /// held item (ftCo_80094E54) while dashing or running is a dash throw.
    pub(super) fn try_dash_item_throw(&mut self, assets: &FighterAssets) -> Result<bool> {
        if !self.core.item_throw_pressed() {
            return Ok(false);
        }
        self.enter_item_throw(S::LightThrowDash, assets)?;
        Ok(true)
    }

    /// ftCo_800957F4 (800957F4): enter a throw state; the accessory runs
    /// once at once to record the hand.
    pub(super) fn enter_item_throw(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        let held = self.core.held_item.expect("a throw needs a held item");
        self.core.commands.variables[0] = 0;
        self.core.commands.variables[1] = 0;
        // throw_flags = 0.
        self.core.commands.take_throw_flag_b3();
        self.core.commands.throw_reverse = false;
        // getAnimSpeed (ftCo_800957F4): PlCo +400 from LightThrowF4 on, then
        // times 1 / itGetDamageMultiplier; separate fmuls, no fusion.
        let mut rate = 1.0;
        if state as u16 >= S::LightThrowF4 as u16 {
            rate *= assets.smash_throw_rate;
        }
        rate *= 1.0 / held.damage_multiplier;
        let facing = if BACK_THROWS.contains(&(state as u16)) {
            -self.core.physics.facing
        } else {
            self.core.physics.facing
        };
        self.change_motion_state_with_rate(state.into(), assets, 0.0, rate)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::ItemThrow(ItemThrowState {
            facing,
            hand: Vec3::ZERO,
            rate,
            entry_facing: self.core.physics.facing,
        });
        self.arm_accessory4();
        self.item_throw_accessory(assets);
        Ok(())
    }

    /// ftCo_ItemThrow_Anim (80095E80): the reverse flag turns the fighter;
    /// the end returns to Wait or Fall (ftCommon_8007D92C).
    pub(super) fn item_throw_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if std::mem::take(&mut self.core.commands.throw_reverse) {
            self.core.physics.facing = -self.core.physics.facing;
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            assert!(
                self.core.held_item.is_none(),
                "ft_8008A2BC: a throw ending with the item still held"
            );
            // ftCommon_8007D92C: Fall in the air, Wait on the ground.
            let next = if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
                S::Fall
            } else {
                S::Wait
            };
            self.change_motion_state(next.into(), assets)?;
        }
        Ok(())
    }

    /// ftCo_LightThrow_Coll (80096228): ft_800841B8; leaving the ground
    /// switches to the air throw (ftCo_80096250).
    pub(super) fn item_throw_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        use crate::collision::ground::{map_escape, WaitGroundResult};
        if map_escape(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            self.leave_ground();
            self.switch_item_throw(true, assets)?;
        }
        Ok(())
    }

    /// ftCo_LightThrowAir_Coll (800962D4): ft_80082C74; landing switches to
    /// the ground throw (ftCo_80096374).
    pub(super) fn air_item_throw_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        crate::collision::air::begin_map(
            &self.core.physics,
            &mut self.core.collision,
            &mut self.core.skeleton,
            self.core.animation.root,
        );
        if crate::collision::air::collide_air_dodge(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
        ) {
            self.land();
            self.switch_item_throw(false, assets)?;
        }
        Ok(())
    }

    /// ftCo_80096250 / ftCo_80096374 inlineA0: the same throw in the other
    /// ground/air table (air throws sit 6 rows after the ground ones, 4
    /// among the smash throws), at the entry facing, frame and rate; the
    /// accessory is reinstalled and runs at once.
    fn switch_item_throw(&mut self, to_air: bool, assets: &FighterAssets) -> Result<()> {
        let MotionData::ItemThrow(throw) = self.core.state_data else {
            panic!("item throw scratch missing")
        };
        let id = self.core.motion_state.action.0;
        let step = if id >= S::LightThrowF4 as u16 { 4 } else { 6 };
        let next = if to_air { id + step } else { id - step };
        let facing = self.core.physics.facing;
        self.core.physics.facing = throw.entry_facing;
        self.change_ground_air_motion_at_rate(super::ActionId(next), assets, throw.rate)?;
        self.core.physics.facing = facing;
        self.arm_accessory4();
        self.item_throw_accessory(assets);
        Ok(())
    }

    /// ftCo_80095EFC (80095EFC), accessory4 in the throw states. Returns
    /// whether it owns accessory4 this tick.
    pub fn item_throw_accessory(&mut self, assets: &FighterAssets) -> bool {
        let MotionData::ItemThrow(throw) = self.core.state_data else {
            return false;
        };
        if !self.core.accessory4_armed {
            return false;
        }
        let Some(held) = self.core.held_item else {
            return true;
        };
        // lb_8000B1CC(it_80272C90(item), NULL): the item's hold joint sits on
        // the hand it is constrained to.
        let hand = self.core.held_part_position(assets);
        if !self.core.commands.take_throw_flag_b3() {
            if let MotionData::ItemThrow(throw) = &mut self.core.state_data {
                throw.hand = hand;
            }
            return true;
        }
        let row = self.throw_row(assets);
        let velocity = self.throw_velocity(&throw, row);
        let mut scale = 1.0;
        let variable = self.core.commands.variables[1];
        if variable != 0 {
            scale = 0.01 * (variable & 0x3F_FFFF) as f32;
            self.core.commands.variables[1] = 0;
        }
        // retail 80095FE8/FF0: fmuls, fmuls.
        let speed =
            scale * (self.core.attributes.items.heavy_throw_velocity_multiplier * row.damage_scale);
        // retail 80095FC8: fneg, fdivs; 80095FEC/80096004: fmadds.
        let moment = -self.core.commands.release_timer / self.core.animation.speed;
        let position = Vec3::new(
            fmadds(moment, throw.hand.x - hand.x, hand.x),
            fmadds(moment, throw.hand.y - hand.y, hand.y),
            0.0,
        );
        assert_ne!(
            self.core.motion_state.id,
            S::LightThrowDrop,
            "Item_8026AC74: dropping an item"
        );
        let holder = self
            .core
            .item_holder(self.core.bones.model.animation_translation, assets);
        let (center, attack) = (holder.center, holder.attack);
        self.core.item_requests.push(melee_it::ItemRequest::Throw {
            item: held.item,
            position,
            velocity,
            speed,
            center,
            attack,
        });
        // it_80273F34 -> Item_8026A848: the hand lets go at once.
        self.core.release_held_item(held.item, assets);
        true
    }

    fn throw_row(&self, assets: &FighterAssets) -> ItemThrowRow {
        let index = usize::from(self.core.motion_state.action.0) - S::LightThrowF as usize;
        assets.item_throws[index]
    }

    /// ftCo_80095D5C (80095D5C): the release velocity from the throw row, or
    /// from a script-set speed and angle (cmd_vars[0]).
    fn throw_velocity(&mut self, throw: &ItemThrowState, row: ItemThrowRow) -> Vec3 {
        let variable = self.core.commands.variables[0];
        let mut speed = 1.0;
        if variable != 0 {
            speed = 0.01 * ((variable >> 12) & 0x3FF) as f32;
        }
        // retail 80095DD4/DD8: fmuls, fmuls.
        speed *= self.core.attributes.items.item_throw_velocity_multiplier * row.speed;
        let angle = if variable != 0 {
            let degrees = ((variable as i32) << 20) >> 20;
            self.core.commands.variables[0] = 0;
            if degrees == 361 {
                row.angle
            } else {
                // retail 80095E1C: fmuls by pi/180.
                0.017_453_292 * degrees as f32
            }
        } else {
            row.angle
        };
        Vec3::new(
            throw.facing * (speed * gekko_math::msl::cosf(angle)),
            speed * gekko_math::msl::sinf(angle),
            0.0,
        )
    }
}

impl super::FighterCore {
    /// ftCo_80094E54 (80094E54): an item in hand and A, with a shoulder
    /// held or the item throwable.
    pub(super) fn item_throw_pressed(&self) -> bool {
        use crate::input::Buttons;
        self.held_item.is_some_and(|held| {
            self.input.pressed.intersects(Buttons::A)
                && (self.input.current.held.intersects(Buttons::SHIELD) || held.use_kind == 0)
        })
    }

    /// ftCo_LightThrowDash_Phys (80096144): PlCo +404 of the ground friction,
    /// scaled by +40C through frame +408; separate fmuls.
    pub(super) fn dash_throw_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Vec3,
    ) {
        let [multiplier, frames, scale] = assets.dash_throw_friction;
        let friction = multiplier * self.attributes.ground.ground_friction;
        let friction = if self.animation.frame <= frames {
            friction * scale
        } else {
            friction
        };
        self.root_motion_or_friction(friction, assets, map, wind);
    }

    /// The world translation of the held part (ftData x8 +0x10).
    pub fn held_part_position(&mut self, assets: &FighterAssets) -> Vec3 {
        let part = self.bones.model.animation_translation;
        self.item_holder(part, assets).part_position()
    }
}

impl super::FighterCore {
    /// Item_8026ABD8 from ftCo_80095744 (80095744) or Fighter_8006CDA4: let
    /// go of the held item at the hand with no push (vec 0, xC44 = 1).
    /// Parasol states are not in scope.
    pub(super) fn drop_held_item(
        &mut self,
        held: super::item_pickup::HeldItem,
        assets: &FighterAssets,
    ) {
        let mut holder = self.item_holder(self.bones.model.animation_translation, assets);
        let position = holder.part_position();
        let (center, attack) = (holder.center, holder.attack);
        self.item_requests.push(melee_it::ItemRequest::Drop {
            item: held.item,
            position,
            speed: 1.0,
            center,
            attack,
        });
        self.release_held_item(held.item, assets);
    }
}
