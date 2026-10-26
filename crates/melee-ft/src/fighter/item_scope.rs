//! Firing a held Super Scope: ftCo_ItemScopeStart.c (the charge),
//! ftCo_ItemScopeRapid.c (rapid fire), ftCo_ItemScopeFire.c (a charged
//! shot and the shot itself) and ftCo_ItemScope.c (the end). Every row has
//! an air twin four rows on and an empty twin eight rows on
//! (fn_800D769C).
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, GroundOrAir, ItemKind};

/// fn_800D84D4's efSync_Spawn ids: the shot's muzzle flash and the empty
/// scope's puff.
const MUZZLE_FLASH: u16 = 0x430;
const EMPTY_PUFF: u16 = 0x405;
/// fn_800D84D4's sounds: a rapid shot, a weak charged shot (levels 1..4),
/// a strong one (5..9) and the empty scope.
const RAPID_SOUND: u32 = 0xFC;
const WEAK_SOUND: u32 = 0xFF;
const STRONG_SOUND: u32 = 0x100;
const EMPTY_SOUND: u32 = 0x101;

/// The scope's scratch (mv at fp+2340), as each phase reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScopeState {
    /// ItemScopeStart: x2340 the frames charged (a float), x2344 A let go.
    Charge { frames: f32, released: bool },
    /// ItemScopeRapid: mv.co.common.x0, the rapid shots left; ItemScopeFire:
    /// the charge level fired (-1 empty). Neither writes mv+4, which keeps
    /// the charge's x2344 (`retained_word`).
    Shots { count: i32, retained_word: f32 },
}

/// The scope's rows are 158..=173.
fn is_scope_state(action: u16) -> bool {
    (S::ItemScopeStart as u16..=S::ItemScopeAirEndEmpty as u16).contains(&action)
}

/// fn_800D769C (800D769C): `row`, or its empty twin once the fighter is in
/// an empty row.
fn with_empty(current: S, row: S) -> S {
    if current as u16 >= S::ItemScopeStartEmpty as u16 {
        S::try_from(i32::from(row as u16 + 8)).expect("an empty scope row")
    } else {
        row
    }
}

/// The air twin of a ground scope row, or the ground twin of an air one.
fn other_ground(row: S) -> S {
    let index = row as u16 - S::ItemScopeStart as u16;
    let twin = if index % 8 < 4 { row as u16 + 4 } else { row as u16 - 4 };
    S::try_from(i32::from(twin)).expect("a scope row's twin")
}

impl Fighter {
    /// ft_800D76B8 / ft_800D7770 (800D76B8, 800D7770): ItemScopeStart (or
    /// its empty row) from frame 0, the first step, the air drift clamped
    /// in the air, the charge cleared, the hand let go of the scope
    /// (ftCommon_8007E79C) and the charge-spending take_dmg_cb
    /// (fn_800D7938).
    pub(super) fn enter_scope_start(
        &mut self,
        ground_or_air: GroundOrAir,
        assets: &FighterAssets,
    ) -> Result<()> {
        let held = self.core.held_item.expect("ftCo_ItemScopeStart.c:33: fp->item_gobj");
        let empty = held.ammo <= 0;
        let row = match (ground_or_air, empty) {
            (GroundOrAir::Ground, false) => S::ItemScopeStart,
            (GroundOrAir::Ground, true) => S::ItemScopeStartEmpty,
            (GroundOrAir::Air, false) => S::ItemScopeAirStart,
            (GroundOrAir::Air, true) => S::ItemScopeAirStartEmpty,
        };
        self.change_motion_state(row.into(), assets)?;
        self.step_animation(assets);
        if ground_or_air == GroundOrAir::Air {
            self.clamp_shot_air_drift();
        }
        self.core.state_data = MotionData::Scope(ScopeState::Charge {
            frames: 0.0,
            released: false,
        });
        self.core.release_hand_pose(assets);
        self.core.scope_charge_armed = true;
        Ok(())
    }

    /// fn_800D79B4 (800D79B4), the start rows' IASA: the charge grows while
    /// A stays held from the start; at PlCo +5B8 frames and the
    /// animation's end the charged shot fires; once A is let go and the
    /// animation ends, an uncharged start goes to rapid fire and a charged
    /// one fires.
    pub(super) fn scope_start_input(&mut self, assets: &FighterAssets) -> Result<()> {
        let Some(held) = self.core.held_item else {
            return Ok(());
        };
        let a_held = self.core.input.current.held.intersects(crate::input::Buttons::A);
        let MotionData::Scope(ScopeState::Charge { frames, released }) = &mut self.core.state_data
        else {
            panic!("Super Scope charge scratch missing")
        };
        if !*released && a_held {
            *frames += 1.0;
        }
        if !a_held {
            *released = true;
        }
        let (frames, released) = (*frames, *released);
        let charges = held.scope.expect("the Super Scope's charge table");
        let level = charges.affordable(held.ammo, gekko_math::msl::fctiwz(frames));
        let ended = !self.core.animation.frames_remaining(&self.core.skeleton);
        if frames >= assets.scope.full_charge && ended {
            return self.enter_scope_fire(level, assets);
        }
        if !released || !ended {
            return Ok(());
        }
        if charges.level(gekko_math::msl::fctiwz(frames)) == 0 {
            self.enter_scope_rapid(assets)
        } else {
            self.enter_scope_fire(level, assets)
        }
    }

    /// fn_800D7BDC / fn_800D7C60 (800D7BDC, 800D7C60): rapid fire from
    /// frame 0 (Ft_MF_SkipAttackCount, statistics only), PlCo +5BC shots,
    /// the shot accessory and the hand-restoring take_dmg_cb.
    fn enter_scope_rapid(&mut self, assets: &FighterAssets) -> Result<()> {
        let air = self.core.physics.ground_or_air == GroundOrAir::Air;
        let row = with_empty(
            self.core.motion_state.id,
            if air {
                S::ItemScopeAirRapid
            } else {
                S::ItemScopeRapid
            },
        );
        self.change_motion_state(row.into(), assets)?;
        if air {
            self.clamp_shot_air_drift();
        }
        self.core.state_data = MotionData::Scope(ScopeState::Shots {
            count: assets.scope.rapid_shots,
            retained_word: self.scope_retained_word(),
        });
        self.arm_accessory4();
        self.core.swing_hand_armed = true;
        Ok(())
    }

    /// fn_800D8140 / fn_800D81D0 (800D8140, 800D81D0): throw flags cleared,
    /// the fire row from frame 0 with the level to fire, the shot accessory
    /// and the hand-restoring take_dmg_cb.
    fn enter_scope_fire(&mut self, level: i32, assets: &FighterAssets) -> Result<()> {
        let air = self.core.physics.ground_or_air == GroundOrAir::Air;
        self.core.commands.clear_throw_flags();
        let row = with_empty(
            self.core.motion_state.id,
            if air {
                S::ItemScopeAirFire
            } else {
                S::ItemScopeFire
            },
        );
        self.change_motion_state(row.into(), assets)?;
        if air {
            self.clamp_shot_air_drift();
        }
        self.core.state_data = MotionData::Scope(ScopeState::Shots {
            count: level,
            retained_word: self.scope_retained_word(),
        });
        self.arm_accessory4();
        self.core.swing_hand_armed = true;
        Ok(())
    }

    /// fn_800D86E0 / fn_800D874C (800D86E0, 800D874C): the end row from
    /// frame 0 and the hand-restoring take_dmg_cb.
    fn enter_scope_end(&mut self, assets: &FighterAssets) -> Result<()> {
        let air = self.core.physics.ground_or_air == GroundOrAir::Air;
        let row = with_empty(
            self.core.motion_state.id,
            if air {
                S::ItemScopeAirEnd
            } else {
                S::ItemScopeEnd
            },
        );
        self.change_motion_state(row.into(), assets)?;
        if air {
            self.clamp_shot_air_drift();
        }
        self.core.swing_hand_armed = true;
        Ok(())
    }

    /// ftCo_ItemScopeRapid_Anim (800D7DF0): each pass through frame 0 of
    /// the loop is a new attack instance and a shot fewer: at none the end
    /// row; an emptied scope goes on in the empty row.
    pub(super) fn scope_rapid_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let animation = &self.core.animation;
        if !(animation.frame >= 0.0 && animation.frame < animation.speed) {
            return Ok(());
        }
        self.core.combat.stale.new_instance();
        let MotionData::Scope(ScopeState::Shots { count: shots, .. }) = &mut self.core.state_data
        else {
            panic!("Super Scope rapid scratch missing")
        };
        *shots -= 1;
        if *shots == 0 {
            return self.enter_scope_end(assets);
        }
        if self.core.held_item.is_some_and(|held| held.ammo <= 0) {
            // ftCo_ItemScopeRapid.c:70-81: the empty row by the ground
            // state, at the same frame and rate (flags 0x0C4C5080), even
            // from the empty row itself.
            let row = if self.core.physics.ground_or_air == GroundOrAir::Ground {
                S::ItemScopeRapidEmpty
            } else {
                S::ItemScopeAirRapidEmpty
            };
            let rate = self.core.animation.speed;
            self.change_ground_air_motion_at_rate(row.into(), assets, rate)?;
            self.arm_accessory4();
            self.core.swing_hand_armed = true;
        }
        Ok(())
    }

    /// ftCo_ItemScopeRapid_IASA (800D80B0): A again restores the shots.
    pub(super) fn scope_rapid_input(&mut self, assets: &FighterAssets) {
        if self.core.input.pressed.intersects(crate::input::Buttons::A) {
            if let MotionData::Scope(ScopeState::Shots { count: shots, .. }) =
                &mut self.core.state_data
            {
                *shots = assets.scope.rapid_shots;
            }
        }
    }

    /// ftCo_ItemScopeFire_Anim (800D83B4), the fire and end rows: at the
    /// animation's end, Wait or Fall (ftCommon_8007D92C) and the hand takes
    /// the scope again (ftCommon_8007E7E4).
    pub(super) fn scope_fire_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.item_shoot_animation(assets)?;
            self.show_held_item_in_hand(assets);
        }
        Ok(())
    }

    /// The scope rows' collisions: the other row at the same frame and rate
    /// (flags 0x0C4C5080, the start rows' 0x0C4C5280), the start rows'
    /// charge callback or the others' accessory and hand callback again.
    pub(super) fn scope_collision(
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
        let current = self.core.motion_state.id;
        let next = other_ground(current);
        let rate = self.core.animation.speed;
        self.change_ground_air_motion_at_rate(next.into(), assets, rate)?;
        if to_air {
            self.clamp_shot_air_drift();
        }
        let index = (current as u16 - S::ItemScopeStart as u16) % 4;
        match index {
            // Start: fn_800D7938 again.
            0 => self.core.scope_charge_armed = true,
            // End: the hand callback alone.
            3 => self.core.swing_hand_armed = true,
            _ => {
                self.arm_accessory4();
                self.core.swing_hand_armed = true;
            }
        }
        Ok(())
    }

    /// fn_800D7938 (800D7938), the start rows' take_dmg_cb: the charge so
    /// far is spent (it_80291F14 at it_80291DAC's level) and the hand takes
    /// the scope again.
    pub(super) fn scope_charge_damaged(&mut self, assets: &FighterAssets) {
        let Some(mut held) = self.core.held_item else {
            return;
        };
        let MotionData::Scope(ScopeState::Charge { frames, .. }) = self.core.state_data else {
            panic!("Super Scope charge scratch missing")
        };
        let charges = held.scope.expect("the Super Scope's charge table");
        let level = charges.affordable(held.ammo, gekko_math::msl::fctiwz(frames));
        if level != -1 {
            self.core.item_requests.push(melee_it::ItemRequest::ControlInstance {
                item: held.item,
                control: melee_it::ItemControl::Spend(level),
            });
            held.ammo = charges.spend(held.ammo, level);
            self.core.held_item = Some(held);
        }
        self.show_held_item_in_hand(assets);
    }

    /// fn_800D80F4 / fn_800D86B8 (800D80F4, 800D86B8), the rapid and fire
    /// rows' accessory4: fn_800D84D4 with an uncharged shot's level, or the
    /// level the fire row holds. Returns whether it owns accessory4.
    pub fn scope_accessory(&mut self) -> bool {
        let action = self.core.motion_state.action.0;
        if !is_scope_state(action) || !self.core.accessory4_armed {
            return false;
        }
        let Some(mut held) = self.core.held_item else {
            return true;
        };
        // fn_800D84D4: ftCheckThrowB0.
        if !std::mem::take(&mut self.core.commands.throw_accessory) {
            return true;
        }
        assert_eq!(held.kind, ItemKind::SScope, "ftCo_ItemScope*.c: It_Kind_S_Scope");
        let charges = held.scope.expect("the Super Scope's charge table");
        let rapid = matches!(
            self.core.motion_state.id,
            S::ItemScopeRapid | S::ItemScopeAirRapid | S::ItemScopeRapidEmpty | S::ItemScopeAirRapidEmpty
        );
        let level = if rapid {
            charges.affordable(held.ammo, 0)
        } else {
            let MotionData::Scope(ScopeState::Shots { count: level, .. }) = self.core.state_data
            else {
                panic!("Super Scope fire scratch missing")
            };
            level
        };
        let muzzle = self.shot_origin(held.shot_offset);
        if !(0..=melee_it::scope::MAX_LEVEL).contains(&level) {
            self.core
                .effects
                .push(melee_ef::request::EffectRequest::PositionalGenerator {
                    id: EMPTY_PUFF,
                    position: muzzle,
                });
            self.push_scope_sound(EMPTY_SOUND);
            return true;
        }
        // it_80291FA8: the level's shots spent, then it_80298DEC's beam
        // from the muzzle (z 0) toward the holder's ECB centre.
        self.core.item_requests.push(melee_it::ItemRequest::ControlInstance {
            item: held.item,
            control: melee_it::ItemControl::Spend(level),
        });
        held.ammo = charges.spend(held.ammo, level);
        self.core.held_item = Some(held);
        let c = &self.core;
        let mut spawn = melee_it::SpawnItem::ray(
            ItemKind::SScopeBeam,
            c.player.id,
            Vec3::new(muzzle.x, muzzle.y, 0.0),
            c.physics.facing,
        );
        let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
        spawn.position = Vec3::new(
            c.physics.position.x + 0.0,
            c.physics.position.y + midpoint,
            c.physics.position.z + 0.0,
        );
        spawn.spawn_argument = level;
        self.core.item_requests.push(melee_it::ItemRequest::Spawn(spawn));
        // efSync after the beam was made.
        self.core
            .effects_after_items
            .push(melee_ef::request::EffectRequest::PositionalGenerator {
                id: MUZZLE_FLASH,
                position: Vec3::new(muzzle.x, muzzle.y, 0.0),
            });
        self.push_scope_sound(match level {
            0 => RAPID_SOUND,
            1..=4 => WEAK_SOUND,
            _ => STRONG_SOUND,
        });
        true
    }

    /// mv+4 as the next scope row keeps it: the charge's x2344, or what the
    /// shots kept of it.
    fn scope_retained_word(&self) -> f32 {
        match self.core.state_data {
            MotionData::Scope(ScopeState::Charge { released, .. }) => {
                f32::from_bits(u32::from(released))
            }
            MotionData::Scope(ScopeState::Shots { retained_word, .. }) => retained_word,
            _ => unreachable!("a scope shot outside the scope's rows"),
        }
    }

    fn push_scope_sound(&mut self, id: u32) {
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id,
                volume: 127,
                pan: 64,
            });
    }
}
