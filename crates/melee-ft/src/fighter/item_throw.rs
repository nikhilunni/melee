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
use gekko_math::fma::fmadds;
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
    /// ftCo_800957F4 (800957F4): enter a throw state; the accessory runs
    /// once at once to record the hand.
    pub(super) fn enter_item_throw(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        let held = self.core.held_item.expect("a throw needs a held item");
        self.core.commands.variables[0] = 0;
        self.core.commands.variables[1] = 0;
        // throw_flags = 0.
        self.core.commands.take_throw_flag_b3();
        self.core.commands.throw_reverse = false;
        // getAnimSpeed: 1 / itGetDamageMultiplier, with PlCo x400 from
        // LightThrowF4 on.
        assert!(
            (state as u16) < S::LightThrowF4 as u16 && held.damage_multiplier == 1.0,
            "ftCo_800957F4: a throw animation speed other than 1"
        );
        let facing = if BACK_THROWS.contains(&(state as u16)) {
            -self.core.physics.facing
        } else {
            self.core.physics.facing
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::ItemThrow(ItemThrowState {
            facing,
            hand: Vec3::ZERO,
        });
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
            if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
                unimplemented!("ftCo_Fall_Enter after an item throw");
            }
            self.change_motion_state(S::Wait.into(), assets)?;
        }
        Ok(())
    }

    /// ftCo_LightThrow_Coll (80096228): ft_800841B8; leaving the ground
    /// switches to the air throw (ftCo_80096250), which is unported.
    pub(super) fn item_throw_collision(&mut self, map: &mut melee_mp::CollMap) {
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
            unimplemented!("ftCo_80096250: an item throw leaving the ground");
        }
    }

    /// ftCo_80095EFC (80095EFC), accessory4 in the throw states. Returns
    /// whether it owns accessory4 this tick.
    pub fn item_throw_accessory(&mut self, assets: &FighterAssets) -> bool {
        let MotionData::ItemThrow(throw) = self.core.state_data else {
            return false;
        };
        let Some(held) = self.core.held_item else {
            return true;
        };
        // lb_8000B1CC(it_80272C90(item), NULL): the item's hold joint sits on
        // the hand it is constrained to.
        let hand = self.core.held_part_position();
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
            .item_holder(self.core.bones.model.animation_translation);
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
    /// The world translation of the held part (ftData x8 +0x10).
    pub fn held_part_position(&mut self) -> Vec3 {
        let part = self.bones.model.animation_translation;
        self.item_holder(part).part_position()
    }
}
