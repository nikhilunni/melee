//! Firing a held Fire Flower: ft_0CE3.c (FireFlowerShoot and its air row).
//! The script opens the flower (throw flag b3, it_80292E64) and starts the
//! fire (b0); while firing the animation holds still, a flame leaves the
//! mouth every PlCo +5A8 frames and the holder puffs every +5AC, until at
//! least +5A4 flames have gone, A has been let go and +5B0 frames have
//! passed since its last press. Then the animation goes on, and closes the
//! flower (throw flag b4, it_80292EAC).
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, GroundOrAir, ItemKind};

/// ftCo_800CE650's efSync_Spawn ids: the puff at the holder's root while
/// the flower has flames (a Graphics model), the empty flower's smoke at
/// its mouth (a generator), the first flame's flash at the root (a
/// Graphics model) and its spark at the mouth (a generator).
const FLAME_PUFF: u16 = 0x3FF;
const EMPTY_SMOKE: u16 = 0x405;
const FIRST_FLASH: u16 = 0x3FD;
const FIRST_SPARK: u16 = 0x3F3;
/// ftCo_800CE650's sounds: a puff with flames, and without.
const PUFF_SOUND: u32 = 0xEE;
const EMPTY_SOUND: u32 = 0xEF;

/// mv.co.itemthrow while the flower fires (ftCo_800CE3E0's set-up).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowerShootState {
    /// x0: frames to the next flame.
    pub flame_timer: f32,
    /// x4: frames to the next puff.
    pub puff_timer: f32,
    /// x8: flames fired (or tried, empty).
    pub flames: i32,
    /// xC: A has been let go.
    pub released: bool,
    /// x10: frames since the last A press, counting up to PlCo +5B0.
    pub since_press: f32,
    /// x14: the flower is firing.
    pub firing: bool,
}

impl Fighter {
    /// ftCo_800CE3E0 / ftCo_800CE480 (800CE3E0, 800CE480): throw flags
    /// cleared, the timers set, FireFlowerShoot (or its air row) from frame
    /// 0, the first step, the air drift clamped in the air, then the fire
    /// accessory and the close-on-damage callback (ftCo_800CE620).
    pub(super) fn enter_flower_shoot(
        &mut self,
        ground_or_air: GroundOrAir,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.core.commands.clear_throw_flags();
        let timing = assets.flower;
        self.core.state_data = MotionData::FlowerShoot(FlowerShootState {
            flame_timer: timing.flame_interval,
            puff_timer: timing.puff_interval,
            flames: 0,
            released: false,
            since_press: 0.0,
            firing: false,
        });
        let state = if ground_or_air == GroundOrAir::Air {
            S::FireFlowerShootAir
        } else {
            S::FireFlowerShoot
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        if ground_or_air == GroundOrAir::Air {
            self.clamp_shot_air_drift();
        }
        self.arm_accessory4();
        self.core.flower_close_armed = true;
        Ok(())
    }

    /// ftCo_800CE620 (800CE620), the take_dmg_cb: the held flower closes.
    pub(super) fn flower_close(&mut self) {
        self.flower_control(melee_it::ItemControl::Close);
    }

    fn flower_control(&mut self, control: melee_it::ItemControl) {
        if let Some(held) = self.core.held_item {
            self.core.item_requests.push(melee_it::ItemRequest::ControlInstance {
                item: held.item,
                control,
            });
        }
    }

    /// ftCo_FireFlowerShoot_Anim (800CEB48): with the flower in hand, throw
    /// flag b3 opens it and b4 closes it; at the animation's end, Wait or
    /// Fall (ftCo_800CDE18).
    pub(super) fn flower_shoot_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.held_item.is_none() {
            return Ok(());
        }
        if self.core.commands.take_throw_flag_b3() {
            self.flower_control(melee_it::ItemControl::Open);
        }
        if std::mem::take(&mut self.core.commands.throw_reverse) {
            self.flower_control(melee_it::ItemControl::Close);
        }
        self.item_shoot_animation(assets)
    }

    /// ftCo_FireFlowerShoot_Coll (ft_800841B8 with ft_800CE5A0) and the air
    /// row's (ft_80082C74 with ft_800CE528): the other row at the same
    /// frame and rate (flags 0x0C4C5880), the callbacks installed again.
    pub(super) fn flower_shoot_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
    ) -> Result<()> {
        let switched = if self.core.physics.ground_or_air == GroundOrAir::Air {
            self.air_shot_collision(map)
        } else {
            self.ground_shot_collision(map)
        };
        let Some(to_air) = switched else {
            return Ok(());
        };
        let (next, _) = if to_air {
            (S::FireFlowerShootAir, ())
        } else {
            (S::FireFlowerShoot, ())
        };
        let rate = self.core.animation.speed;
        self.change_ground_air_motion_at_rate(next.into(), assets, rate)?;
        if to_air {
            self.clamp_shot_air_drift();
        }
        self.arm_accessory4();
        self.core.flower_close_armed = true;
        Ok(())
    }

    /// ftCo_800CE650 (800CE650), the Fire Flower's accessory4. Returns
    /// whether it owns accessory4 this tick.
    pub fn flower_accessory(&mut self, assets: &FighterAssets) -> bool {
        let action = self.core.motion_state.action.0;
        if !(S::FireFlowerShoot as u16..=S::FireFlowerShootAir as u16).contains(&action)
            || !self.core.accessory4_armed
        {
            return false;
        }
        let Some(held) = self.core.held_item else {
            return true;
        };
        assert_eq!(held.kind, ItemKind::FFlower, "ft_0CE3.c: It_Kind_F_Flower");
        let timing = assets.flower;
        let a_held = self
            .core
            .input
            .current
            .held
            .intersects(crate::input::Buttons::A);
        let a_pressed = self.core.input.pressed.intersects(crate::input::Buttons::A);
        let fire = std::mem::take(&mut self.core.commands.throw_accessory);
        let MotionData::FlowerShoot(state) = &mut self.core.state_data else {
            panic!("Fire Flower scratch missing")
        };
        if !a_held {
            state.released = true;
        }
        if state.since_press < timing.release_frames {
            state.since_press += 1.0;
        }
        if state.since_press < timing.release_frames && a_pressed {
            state.since_press = 0.0;
        }
        if fire {
            state.firing = true;
            self.flower_puff();
            self.flower_flame(assets);
            if self.core.held_item.is_some_and(|held| held.ammo > 0) {
                // The first flame's flash at the root, its spark at the
                // mouth, and the rumble (ftCommon_8007EBAC).
                let root = self.root_position();
                let facing = self.core.physics.facing;
                // After the flame it_80292F14 created (efSync, in order).
                self.core
                    .effects_after_items
                    .push(melee_ef::request::EffectRequest::PositionalGraphics {
                        id: FIRST_FLASH,
                        position: root,
                        facing,
                        angle: 0.0,
                    });
                let mouth = self.shot_origin(it_mouth());
                self.core
                    .effects_after_items
                    .push(melee_ef::request::EffectRequest::PositionalGenerator {
                        id: FIRST_SPARK,
                        position: mouth,
                    });
            }
            self.core
                .animation
                .set_rate(&mut self.core.skeleton, 0.0, false);
        }
        let MotionData::FlowerShoot(state) = &mut self.core.state_data else {
            unreachable!()
        };
        if !state.firing {
            return true;
        }
        state.puff_timer -= 1.0;
        let puff = state.puff_timer <= 0.0;
        if puff {
            state.puff_timer = timing.puff_interval;
        }
        if puff {
            self.flower_puff();
        }
        let MotionData::FlowerShoot(state) = &mut self.core.state_data else {
            unreachable!()
        };
        state.flame_timer -= 1.0;
        let flame = state.flame_timer <= 0.0;
        if flame {
            state.flame_timer = timing.flame_interval;
        }
        if flame {
            self.flower_flame(assets);
        }
        let MotionData::FlowerShoot(state) = &mut self.core.state_data else {
            unreachable!()
        };
        if state.flames >= timing.min_flames
            && state.released
            && state.since_press >= timing.release_frames
        {
            state.firing = false;
            self.core
                .animation
                .set_rate(&mut self.core.skeleton, 1.0, false);
        }
        true
    }

    /// The puff block (ft_0CE3.c:117-131): with flames left, the puff at the
    /// holder's root turned to the facing and its sound; empty, smoke at
    /// the mouth and another sound.
    fn flower_puff(&mut self) {
        let empty = self.core.held_item.is_none_or(|held| held.ammo <= 0);
        let (effect, sound) = if empty {
            let mouth = self.shot_origin(it_mouth());
            (
                melee_ef::request::EffectRequest::PositionalGenerator {
                    id: EMPTY_SMOKE,
                    position: mouth,
                },
                EMPTY_SOUND,
            )
        } else {
            let root = self.root_position();
            (
                melee_ef::request::EffectRequest::PositionalGraphics {
                    id: FLAME_PUFF,
                    position: root,
                    facing: self.core.physics.facing,
                    angle: 0.0,
                },
                PUFF_SOUND,
            )
        };
        self.core.effects.push(effect);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id: sound,
                volume: 127,
                pan: 64,
            });
    }

    /// The flame block (ft_0CE3.c:133-151): with flames left a flame from
    /// the mouth (it_80292F14 -> it_802996D0, toward the holder's ECB
    /// centre); the count, and every PlCo +5B4th flame a new attack
    /// instance (ft_800892A0).
    fn flower_flame(&mut self, assets: &FighterAssets) {
        let Some(mut held) = self.core.held_item else {
            return;
        };
        let mouth = self.shot_origin(it_mouth());
        if held.ammo > 0 {
            let c = &self.core;
            let mut spawn = melee_it::SpawnItem::ray(
                ItemKind::LGunBeam,
                c.player.id,
                Vec3::new(mouth.x, mouth.y, 0.0),
                c.physics.facing,
            );
            // it_8026BB68 -> ftLib_80086990: the ECB's midpoint.
            let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
            spawn.position = Vec3::new(
                c.physics.position.x + 0.0,
                c.physics.position.y + midpoint,
                c.physics.position.z + 0.0,
            );
            self.core.item_requests.push(melee_it::ItemRequest::Shot {
                source: held.item,
                spawn,
            });
            held.ammo -= 1;
            self.core.held_item = Some(held);
        }
        let MotionData::FlowerShoot(state) = &mut self.core.state_data else {
            unreachable!()
        };
        state.flames += 1;
        let interval = assets.flower.flames_per_instance;
        if interval - 1 == state.flames % interval {
            self.core.combat.stale.new_instance();
        }
    }

    /// lb_8000B1CC(fp->parts[0].joint, NULL): the root part's world position.
    fn root_position(&mut self) -> Vec3 {
        super::caches::part_position(&mut self.core.skeleton, &self.core.animation, 0, Vec3::ZERO)
    }
}

/// it_80292EF8: the flower's mouth in the hand joint's frame.
fn it_mouth() -> Vec3 {
    Vec3::new(0.0, 5.5, 2.5)
}
