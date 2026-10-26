//! The flower a Lip's Stick spore grows on a fighter's head
//! (ftcommon.c 8007FC7C..800801E4, Fighter_8006A360 and
//! Fighter_Spaghetti_8006AD10's flower steps).
//!
//! The Lipstick-element damage a fighter takes in a frame (ftColl_8007BBCC)
//! plants a flower item (It_Kind_F_Flower_Flame) on its held-item part
//! (ftData x8 +0x12) or feeds the one it has: its timer (x2018) grows by
//! the damage times PlCo +704 up to +700, and the flower's size follows the
//! damage taken (x2024). Every PlCo +6F8 frames it deals +6F4 damage;
//! mashing or turning the stick takes +6FC off the timer, and when the
//! timer runs out the flower drops and wilts.
use super::{assets::FighterAssets, Fighter, FighterCore};
use crate::input::Buttons;
use hsd_types::Vec3;
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FighterKind, ItemKind};

/// PlCo +6F4..+710.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +6F4: the damage each tick of the flower deals.
    pub damage: i32,
    /// +6F8: frames between those ticks.
    pub interval: i32,
    /// +6FC: what a mash takes off the timer.
    pub mash: i32,
    /// +700: the longest the timer runs.
    pub max_timer: i32,
    /// +704: timer frames per point of damage.
    pub timer_per_damage: f32,
    /// +708 / +70C / +710: the size's base, its cap, its growth per point of
    /// damage taken.
    pub size_base: f32,
    pub size_max: f32,
    pub size_per_damage: f32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> super::assets::Result<Self> {
        let r = archive.reader();
        Ok(Self {
            damage: r.s32(base + 0x6F4)?,
            interval: r.s32(base + 0x6F8)?,
            mash: r.s32(base + 0x6FC)?,
            max_timer: r.s32(base + 0x700)?,
            timer_per_damage: r.f32(base + 0x704)?,
            size_base: r.f32(base + 0x708)?,
            size_max: r.f32(base + 0x70C)?,
            size_per_damage: r.f32(base + 0x710)?,
        })
    }
}

/// x1980 and x2018..x2024.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlowerHead {
    /// x1980: the flower item, once the scene made it.
    pub item: Option<u32>,
    /// x2018: frames until it wilts.
    pub timer: i32,
    /// x201C: frames until its next damage tick.
    pub tick: i32,
    /// x2024: the damage that grew it.
    pub damage: i32,
    /// x2020 / x2021: the stick's last direction on each axis.
    pub stick: [i8; 2],
}

/// ftCommon_8007FC7C's sound (ft_PlaySFX 0x11F).
const GROW_SOUND: u32 = 0x11F;

impl FighterCore {
    /// ftColl_8007BBCC (8007BBCC) then ftCommon_8007FC7C (8007FC7C) in
    /// Fighter_8006CB94: this frame's Lipstick-element damage grows or
    /// plants the flower.
    pub(super) fn grow_flower(&mut self, damage: f32, assets: &FighterAssets) {
        if damage <= 0.0 {
            return;
        }
        let p = &assets.head_flower;
        // retail 8007FC9C: fmuls, fctiwz.
        let added = gekko_math::msl::fctiwz(damage * p.timer_per_damage);
        match &mut self.flower {
            Some(flower) => {
                flower.timer += added;
                if flower.timer > p.max_timer {
                    flower.timer = p.max_timer;
                }
                // x2024 = (s32) ((float) x2024 + damage).
                flower.damage = gekko_math::msl::fctiwz(flower.damage as f32 + damage);
                self.size_flower(assets);
            }
            None => self.plant_flower(added, damage, assets),
        }
        self.commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: GROW_SOUND,
                volume: 0x7F,
                pan: 0x40,
            });
    }

    /// it_8029A748 (8029A748) and ftCommon_8007FE84 (8007FE84): the flower
    /// made at cur_pos (on the stage plane) from the ECB's centre
    /// (it_8026BB68), held at ftData x8 +0x12; the timers start
    /// (x201C at PlCo +6F8). The scene sizes it once made
    /// (`flower_made`).
    fn plant_flower(&mut self, timer: i32, damage: f32, assets: &FighterAssets) {
        if self.kind == FighterKind::Pikachu {
            unimplemented!("ftPk_Init_UnkMotionStates1 (ftcommon.c:1485): a flower on Pikachu");
        }
        let position = self.physics.position;
        let mut spawn = SpawnItem::ray(
            ItemKind::FFlowerFlame,
            self.player.id,
            Vec3::new(position.x, position.y, 0.0),
            self.physics.facing,
        );
        let ecb = &self.collision.data.ecb;
        // ftLib_80086990: fadds, fmuls, fadds.
        let midpoint = 0.5 * (ecb.top.y + ecb.bottom.y);
        spawn.position = Vec3::new(position.x + 0.0, position.y + midpoint, position.z + 0.0);
        self.item_requests.push(ItemRequest::SpawnInHand {
            spawn,
            part: self.bones.model.held_item,
            hold: false,
            catch_item: false,
            scale_by_owner: false,
        });
        self.flower = Some(FlowerHead {
            item: None,
            timer,
            tick: assets.head_flower.interval,
            damage: gekko_math::msl::fctiwz(damage),
            stick: [0; 2],
        });
    }

    /// The flower `item` the planting asked for exists: ftCommon_8007FE84's
    /// x1980, then ftCommon_8007FDA0's size.
    pub fn flower_made(&mut self, item: u32, assets: &FighterAssets) {
        let flower = self.flower.as_mut().expect("a flower was planted");
        flower.item = Some(item);
        self.size_flower(assets);
    }

    /// ftCommon_8007FDA0 (8007FDA0): the flower's size,
    /// min(+710 * x2024 + +708, +70C) (retail 8007FDF0: fmadds), moves
    /// joint 2 by co_attrs x130 over that size (fdivs, three fmuls) and
    /// scales joint 1 by size * x13C * the fighter's y scale (two fmuls).
    fn size_flower(&mut self, assets: &FighterAssets) {
        let flower = self.flower.expect("a flower to size");
        let Some(item) = flower.item else {
            return;
        };
        let p = &assets.head_flower;
        let mut size = gekko_math::fma::fmadds(p.size_per_damage, flower.damage as f32, p.size_base);
        if size > p.size_max {
            size = p.size_max;
        }
        let reciprocal = 1.0 / size;
        let items = &self.attributes.items;
        let offset = Vec3::new(
            items.flower_flame_offset.x * reciprocal,
            items.flower_flame_offset.y * reciprocal,
            items.flower_flame_offset.z * reciprocal,
        );
        let scale = self.player.scale * (size * items.flower_flame_scale);
        self.item_requests.push(ItemRequest::ControlInstance {
            item,
            control: ItemControl::FlowerSize { scale, offset },
        });
    }

    /// Fighter_8006A360's flower step (fighter.c:1543-1559): a damage tick
    /// every PlCo +6F8 frames (Fighter_TakeDamage_8006CC7C), and once the
    /// timer runs out the flower leaves the head with no push
    /// (Item_8026ABD8(x1980, &zero, 0)) and is forgotten (ftCommon_8007FF74).
    pub(super) fn tick_flower(&mut self, assets: &FighterAssets) {
        let Some(flower) = &mut self.flower else {
            return;
        };
        flower.timer -= 1;
        flower.tick -= 1;
        let tick = flower.tick == 0;
        if tick {
            flower.tick = assets.head_flower.interval;
        }
        let wilted = flower.timer <= 0;
        let item = flower.item;
        if tick {
            self.take_percent_damage(assets.head_flower.damage as f32, assets);
        }
        if !wilted {
            return;
        }
        // ftCommon_8007F8E8: no kind hook for the ported characters.
        let item = item.expect("a wilting flower was made");
        let part = self.bones.model.held_item;
        let mut holder = self.item_holder(part, assets);
        let position = holder.part_position();
        let hand = holder.part_matrix();
        let (center, attack) = (holder.center, holder.attack);
        self.item_requests.push(ItemRequest::Drop {
            item,
            position,
            hand,
            speed: 0.0,
            center,
            attack,
        });
        self.flower = None;
    }

    /// ft_0D31.c:121-125, a death: the flower goes (Item_8026A8EC) and is
    /// forgotten.
    pub(super) fn lose_flower(&mut self) {
        if let Some(flower) = self.flower.take() {
            let item = flower.item.expect("a flower was made");
            self.item_requests.push(ItemRequest::Destroy { item });
        }
    }
}

impl Fighter {
    /// ftCommon_8007FFD8 (8007FFD8), before the IASA (fighter.c:2128): any
    /// of A, B, X, Y or L/R pressed takes PlCo +6FC off the timer, and so
    /// does the stick turning past PlCo +308 on either axis.
    pub(super) fn mash_flower(&mut self, assets: &FighterAssets) {
        let threshold = assets.grab_escape.stick_threshold;
        let mash = assets.head_flower.mash as f32;
        let input = &self.core.input;
        let pressed = input
            .pressed
            .intersects(Buttons::A | Buttons::B | Buttons::XY | Buttons::SHIELD);
        let stick = [input.current.stick.x, input.current.stick.y];
        let Some(flower) = &mut self.core.flower else {
            return;
        };
        // x2018 -= (float) x6FC: an int through a float subtract.
        if pressed {
            flower.timer = gekko_math::msl::fctiwz(flower.timer as f32 - mash);
        }
        let previous = flower.stick;
        for (direction, value) in flower.stick.iter_mut().zip(stick) {
            if value < -threshold {
                *direction = -1;
            }
            if value > threshold {
                *direction = 1;
            }
        }
        if previous != flower.stick {
            flower.timer = gekko_math::msl::fctiwz(flower.timer as f32 - mash);
        }
    }
}
