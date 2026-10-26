//! The Super and Poison Mushrooms' size change: ftCo_KinokoGiantStart.c ..
//! ftCo_KinokoSmallEnd.c and ft_0D27.c.
//!
//! A Mushroom's touch counts in x200C (Super) or x2010 (Poison)
//! (ftColl_80077C60); the status proc applies each (Fighter_8006A1BC,
//! fighter.c:1429-1437). Growing or shrinking, the fighter stops in a
//! Kinoko state (motions 314..321) with its movement saved (fn_800D2B30)
//! while its scale (x34_scale.y) follows the Mushroom's joint animation
//! from where it stands to the new size (fn_800D299C, fn_800D2A3C:
//! ftLib_SetScale). At the animation's end the movement returns and the new
//! size takes hold (ftCo_800D2770: the attributes, the camera box and the
//! ECB); the size lasts x2008 frames (PlCo +688, a giant's longer with its
//! percent), counted in Fighter_8006A360, then shrinks or grows back.
use super::{
    assets::{FighterAssets, Result},
    Fighter, FighterCore, MotionData, MotionEntryFlags,
};
use crate::anim::WaitChoice;
use crate::collision::{air, ground};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, GroundOrAir};

/// PlCo +678..+690.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MushroomParameters {
    /// +678 / +67C: a Super Mushroom's scale ratio, from a normal size or
    /// from one already above 1 (x34_scale.x).
    pub giant_ratio: f32,
    pub giant_ratio_from_large: f32,
    /// +680 / +684: a Poison Mushroom's, from a normal size or from one
    /// already below 1.
    pub small_ratio: f32,
    pub small_ratio_from_small: f32,
    /// +688: the frames a size lasts.
    pub frames: i32,
    /// +68C / +690: a giant's percent above +68C adds to its frames, at
    /// most +690.
    pub percent_threshold: i32,
    pub percent_cap: i32,
}
impl MushroomParameters {
    pub fn read(r: &hsd_archive::Reader<'_>, common: u32) -> Result<Self> {
        Ok(Self {
            giant_ratio: r.f32(common + 0x678)?,
            giant_ratio_from_large: r.f32(common + 0x67C)?,
            small_ratio: r.f32(common + 0x680)?,
            small_ratio_from_small: r.f32(common + 0x684)?,
            frames: r.s32(common + 0x688)?,
            percent_threshold: r.s32(common + 0x68C)?,
            percent_cap: r.s32(common + 0x690)?,
        })
    }
}

/// The Super Mushroom's joint animations (it_80293660: KinokoAnim x8 and
/// xC) as fn_800D2A3C samples them: the x scale after each HSD_JObjAnim
/// step past the entry's, the last one the step whose AObj ended
/// (HSD_AObjGetFlags & 0x40000000).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MushroomSizes {
    pub curves: [Vec<f32>; 2],
}

/// x2008 / x200C / x2010 / x2220_b5 / x2220_b6.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MushroomStatus {
    /// x2220_b5: grown by a Super Mushroom.
    pub giant: bool,
    /// x2220_b6: shrunk by a Poison Mushroom.
    pub small: bool,
    /// x2008: the frames the size has left.
    pub frames: i32,
    /// x200C / x2010: Super and Poison Mushroom touches this frame.
    pub super_touches: u8,
    pub poison_touches: u8,
}

/// fn_800D2890 (mv.co.mushroom.x0): the fall an airborne fighter takes at
/// the change's end, by the state it changed size from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MushroomExit {
    /// ftCo_Fall_Enter.
    Fall,
    /// ftCo_FallAerial_Enter, out of FallAerial.
    FallAerial,
    /// ftCo_800968C8, out of FallSpecial, ItemParasolFallSpecial or Peach's
    /// 0x172.
    FallSpecial,
}

/// mv.co.mushroom.x4: the end the change finishes with (also the
/// take_dmg_cb).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MushroomFinish {
    /// ftCo_800D15D0: grown.
    Grown,
    /// ftCo_800D1B04: back from giant.
    GiantEnded,
    /// ftCo_800D1F6C: shrunk.
    Shrunk,
    /// ftCo_800D23F4: back from small.
    SmallEnded,
}

/// The movement fn_800D2B30 saves (mv.co.common x14..x58).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SavedMovement {
    pub ground_acceleration: f32,
    pub secondary_ground_acceleration: f32,
    pub animation_velocity: Vec3,
    pub ground_velocity: f32,
    pub self_velocity: Vec3,
    pub ground_knockback_velocity: f32,
    pub knockback_velocity: Vec3,
    pub ground_shield_knockback_velocity: f32,
    pub shield_knockback_velocity: Vec3,
}

/// The Kinoko states' scratch.
#[derive(Clone, Debug)]
pub struct MushroomState {
    pub exit: MushroomExit,
    pub finish: MushroomFinish,
    /// mv.co.mushroom.x8: an end that cuts a size short (one Mushroom
    /// over the other) clears the movement once more (ftCommon_8007E2FC).
    pub clears_movement: bool,
    /// Which of [`MushroomSizes::curves`] plays (fn_800D299C's idx) and
    /// the HSD_JObjAnim steps taken.
    pub curve: usize,
    pub steps: usize,
    /// mv.co.walk.middle_anim_frame / fast_anim_frame: the scale the
    /// change starts from and the one it ends at.
    pub from: f32,
    pub to: f32,
    pub saved: SavedMovement,
}

/// Ft_MF_SkipModel | SkipMatAnim (0x90), and the ends' KeepFastFall (0x91).
const START: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_MODEL.0 | MotionEntryFlags::SKIP_MAT_ANIM.0);
/// 0xC4C5080: a ground/air switch keeps the change running.
const SWITCH: MotionEntryFlags = MotionEntryFlags(0x0C4C_5080);

impl Fighter {
    /// Fighter.x2070 as ft_800895E0 stored it.
    fn current_motion_flags(&self) -> u32 {
        self.motion_flags().unwrap_or_else(|| {
            unimplemented!(
                "ftCo_800D27C4 / fn_800D2818: x2071 of action {:?} (its row's flags are not in the port)",
                self.core.motion_state.action
            )
        })
    }

    /// ftCo_800D27C4 (800D27C4): a size change may start unless frozen
    /// (DamageIce) or in a state of class 12 or 13 (x2071_b0_3).
    fn can_change_size(&self) -> bool {
        if self.core.motion_state.id == S::DamageIce {
            return false;
        }
        let class = super::state::class_nibble(self.current_motion_flags());
        !matches!(class, 12 | 13)
    }

    /// fn_800D2818 (800D2818): a size may end unless frozen, already
    /// changing, or in a state whose class is neither 0 nor 5 without
    /// x2071_b4 (bit 19 of the flags word).
    fn can_end_size(&self) -> bool {
        if self.core.motion_state.id == S::DamageIce || self.core.in_mushroom_state() {
            return false;
        }
        let flags = self.current_motion_flags();
        let class = super::state::class_nibble(flags);
        class == 5 || class == 0 || flags & (1 << 19) != 0
    }
}

impl FighterCore {
    /// The Kinoko states (ftCo_MS_KinokoGiantStart..KinokoSmallEndAir).
    pub fn in_mushroom_state(&self) -> bool {
        (S::KinokoGiantStart as u16..=S::KinokoSmallEndAir as u16)
            .contains(&(self.motion_state.id as u16))
    }

    /// fn_800D2B30 (800D2B30): the movement, saved, then cleared
    /// (ftCommon_8007E2FC).
    fn save_movement(&mut self) -> SavedMovement {
        let p = &self.physics;
        let saved = SavedMovement {
            ground_acceleration: p.ground_acceleration,
            secondary_ground_acceleration: p.secondary_ground_acceleration,
            animation_velocity: p.animation_velocity,
            ground_velocity: p.ground_velocity,
            self_velocity: p.self_velocity,
            ground_knockback_velocity: p.ground_knockback_velocity,
            knockback_velocity: p.knockback_velocity,
            ground_shield_knockback_velocity: p.ground_shield_knockback_velocity,
            shield_knockback_velocity: p.shield_knockback_velocity,
        };
        self.clear_movement();
        saved
    }

    /// fn_800D2BF8 (800D2BF8): the saved movement back.
    fn restore_movement(&mut self, saved: &SavedMovement) {
        let p = &mut self.physics;
        p.ground_acceleration = saved.ground_acceleration;
        p.secondary_ground_acceleration = saved.secondary_ground_acceleration;
        p.animation_velocity = saved.animation_velocity;
        p.ground_velocity = saved.ground_velocity;
        p.self_velocity = saved.self_velocity;
        p.ground_knockback_velocity = saved.ground_knockback_velocity;
        p.knockback_velocity = saved.knockback_velocity;
        p.ground_shield_knockback_velocity = saved.ground_shield_knockback_velocity;
        p.shield_knockback_velocity = saved.shield_knockback_velocity;
    }

    /// ftLib_SetScale (800866B8): x34_scale.y, the model root's scale
    /// (Fighter_UpdateModelScale) and what follows it: the root motion's
    /// scale and the hitbox damage's (ftColl_8007ABD0). ftCommon_80080174's
    /// worn Bunny Hood scale is the hood model's only.
    pub fn set_scale(&mut self, scale: f32) {
        self.player.scale = scale;
        self.update_model_scale();
        if let Some(root_motion) = self.animation.root_motion.as_mut() {
            // ftCommon_GetModelScale: retail 0x8007F69C fmuls.
            root_motion.effective_scale = scale * self.attributes.size.model_scaling;
        }
        self.commands.model_scale = (scale != 1.0).then_some(scale);
    }

    /// ftCo_800D2770 (800D2770): the new size, its attributes
    /// (ftCo_800D105C), the camera box (ftCamera_80076064, applied by the
    /// next camera proc) and the ECB and ledge snaps (ft_80081C88).
    fn take_size(&mut self, scale: f32, assets: &FighterAssets) {
        self.set_scale(scale);
        self.recompute_attributes(assets);
        self.camera_reset_pending = true;
        crate::collision::ecb::rescale(
            &mut self.collision.data,
            &assets.bones.ecb,
            self.player.scale,
            self.attributes.size.weight,
        );
    }

    /// ftColl_80077C60 (ftcoll.c:1127-1136): a Mushroom's touch counts.
    pub(super) fn touch_mushroom(&mut self, poison: bool) {
        let status = &mut self.status.mushroom;
        if poison {
            status.poison_touches = status.poison_touches.wrapping_add(1);
        } else {
            status.super_touches = status.super_touches.wrapping_add(1);
        }
    }

    /// The camera box ftCamera_80076064 resets from inside a proc without
    /// the stage's camera: the camera proc applies it.
    pub(super) fn reset_pending_camera(&mut self, assets: &FighterAssets, stage: &melee_cm::StageCamera) {
        if std::mem::take(&mut self.camera_reset_pending) {
            self.reset_camera_subject(assets, stage);
        }
    }
}

impl Fighter {
    /// Fighter_8006A1BC (fighter.c:1429-1437): each touch this frame
    /// applies its Mushroom, Super ones first.
    pub(super) fn apply_mushrooms(&mut self, assets: &FighterAssets) -> Result<()> {
        while self.core.status.mushroom.super_touches != 0 {
            self.apply_super_mushroom(assets)?;
            self.core.status.mushroom.super_touches -= 1;
        }
        while self.core.status.mushroom.poison_touches != 0 {
            self.apply_poison_mushroom(assets)?;
            self.core.status.mushroom.poison_touches -= 1;
        }
        Ok(())
    }

    /// Whether this status proc's first Mushroom starts a size change
    /// (fn_800D290C) while the fighter is in a grab pair, which the scene
    /// then parts.
    pub fn mushroom_parts_grab(&self, assets: &FighterAssets) -> bool {
        let _ = assets;
        let status = self.core.status.mushroom;
        if self.core.combat.grab.is_none()
            || self.core.status.disabled
            || (status.super_touches == 0 && status.poison_touches == 0)
            || !self.can_change_size()
        {
            return false;
        }
        if self.core.in_mushroom_state() {
            unimplemented!("fn_800D290C: a grab pair in a Kinoko state");
        }
        // Super first: a giant's touch only renews the frames.
        if status.super_touches != 0 {
            return !status.giant;
        }
        !status.small
    }

    /// Fighter_SuperMushroomApply (800D14B4): a change under way finishes
    /// first; a giant's frames renew; a small fighter grows back
    /// (ftCo_800D2490 / ftCo_800D2600 with x8 set); else it grows.
    fn apply_super_mushroom(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.can_change_size() {
            return Ok(());
        }
        self.finish_mushroom_now(assets)?;
        let status = self.core.status.mushroom;
        if status.giant {
            self.core.status.mushroom.frames = assets.mushroom.frames;
            return Ok(());
        }
        if status.small {
            return self.start_size_change(assets, MushroomFinish::SmallEnded, true);
        }
        self.start_size_change(assets, MushroomFinish::Grown, false)
    }

    /// Fighter_PoisonMushroomApply (800D1E80): as the Super's, mirrored.
    fn apply_poison_mushroom(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.can_change_size() {
            return Ok(());
        }
        self.finish_mushroom_now(assets)?;
        let status = self.core.status.mushroom;
        if status.small {
            self.core.status.mushroom.frames = assets.mushroom.frames;
            return Ok(());
        }
        if status.giant {
            return self.start_size_change(assets, MushroomFinish::GiantEnded, true);
        }
        self.start_size_change(assets, MushroomFinish::Shrunk, false)
    }

    /// Fighter_8006A360 (fighter.c:1500-1512): the size's frames count
    /// down; at none left it ends, if the state allows (else it tries
    /// again next frame).
    pub(super) fn tick_mushroom(&mut self, assets: &FighterAssets) -> Result<()> {
        let status = self.core.status.mushroom;
        if !status.giant && !status.small {
            return Ok(());
        }
        if status.frames != 0 {
            self.core.status.mushroom.frames -= 1;
        }
        if self.core.status.mushroom.frames == 0 && self.can_end_size() {
            // Fighter_SuperMushroomEnd / Fighter_PoisonMushroomEnd.
            let finish = if status.giant {
                MushroomFinish::GiantEnded
            } else {
                MushroomFinish::SmallEnded
            };
            self.start_size_change(assets, finish, false)?;
        }
        Ok(())
    }

    /// The mv.co.mushroom.x4 of a change under way, run at once (a second
    /// Mushroom, or a hit: the take_dmg_cb).
    pub(super) fn finish_mushroom_now(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.in_mushroom_state() {
            return Ok(());
        }
        self.finish_size_change(assets)
    }

    /// ftCo_800D170C / ftCo_800D18CC (grow), ftCo_800D1BA0 / ftCo_800D1D10
    /// (end a giant), ftCo_800D1FFC / ftCo_800D21BC (shrink),
    /// ftCo_800D2490 / ftCo_800D2600 (end a small one), on the ground or
    /// in the air: fn_800D290C, the Kinoko state, the movement saved, the
    /// exit by the state left (fn_800D2890) and the scale's animation
    /// (fn_800D299C). The sounds and rumble are not simulated.
    fn start_size_change(
        &mut self,
        assets: &FighterAssets,
        finish: MushroomFinish,
        clears_movement: bool,
    ) -> Result<()> {
        let previous = (self.core.motion_state.id, self.core.motion_state.action);
        self.prepare_size_change(assets);
        let air = self.core.physics.ground_or_air == GroundOrAir::Air;
        let (ground, airborne, flags) = match finish {
            MushroomFinish::Grown => (S::KinokoGiantStart, S::KinokoGiantStartAir, START),
            MushroomFinish::GiantEnded => (
                S::KinokoGiantEnd,
                S::KinokoGiantEndAir,
                MotionEntryFlags(START.0 | MotionEntryFlags::KEEP_FAST_FALL.0),
            ),
            MushroomFinish::Shrunk => (S::KinokoSmallStart, S::KinokoSmallStartAir, START),
            MushroomFinish::SmallEnded => (
                S::KinokoSmallEnd,
                S::KinokoSmallEndAir,
                MotionEntryFlags(START.0 | MotionEntryFlags::KEEP_FAST_FALL.0),
            ),
        };
        let state = if air { airborne } else { ground };
        self.change_motion_state_with_flags(state.into(), assets, flags, 0.0, 1.0)?;
        let saved = self.core.save_movement();
        let exit = self.mushroom_exit(previous);
        let from = self.core.player.scale;
        let base = self.core.base_scale;
        let m = &assets.mushroom;
        let (curve, to) = match finish {
            // retail 800D1784..: fmuls.
            MushroomFinish::Grown => (
                0,
                from * if base > 1.0 {
                    m.giant_ratio_from_large
                } else {
                    m.giant_ratio
                },
            ),
            MushroomFinish::Shrunk => (
                0,
                from * if base < 1.0 {
                    m.small_ratio_from_small
                } else {
                    m.small_ratio
                },
            ),
            MushroomFinish::GiantEnded => (1, base),
            MushroomFinish::SmallEnded => (0, base),
        };
        self.core.state_data = MotionData::Mushroom(MushroomState {
            exit,
            finish,
            clears_movement,
            curve,
            // fn_800D299C's own HSD_JObjAnim.
            steps: 0,
            from,
            to,
            saved,
        });
        Ok(())
    }

    /// fn_800D290C (800D290C): the actions stop (ftCommon_8007DB58), a
    /// heavy item drops (ftCo_8009750C); a grab pair parts.
    fn prepare_size_change(&mut self, assets: &FighterAssets) {
        self.interrupt_actions(assets);
        if self.core.held_item.is_some_and(|held| held.heavy) {
            unimplemented!("fn_800D290C (ft_0D27.c:74): a heavy item dropped by a size change");
        }
        // The grab pair was parted ahead of the status proc
        // (`mushroom_parts_grab`).
        assert!(
            self.core.combat.grab.is_none(),
            "fn_800D290C (ft_0D27.c:76-85): a size change parting a grab outside the status proc"
        );
    }

    /// fn_800D2890 (800D2890): by the state the change began in.
    fn mushroom_exit(&self, previous: (S, super::ActionId)) -> MushroomExit {
        let (id, action) = previous;
        if usize::from(action.0) >= super::COMMON_COUNT {
            if self.core.kind == melee_types::FighterKind::Peach && action.0 == 0x172 {
                return MushroomExit::FallSpecial;
            }
            // The switch on ms_id sees no common id: the default.
            return MushroomExit::Fall;
        }
        match id {
            S::FallAerial => MushroomExit::FallAerial,
            S::FallSpecial | S::ItemParasolFallSpecial => MushroomExit::FallSpecial,
            _ => MushroomExit::Fall,
        }
    }

    /// ftCo_800D15D0, ftCo_800D1B04, ftCo_800D1F6C, ftCo_800D23F4: the
    /// movement back, the size flag and frames (a giant's grow with its
    /// percent), the new size (ftCo_800D2770), the movement cleared, then
    /// Wait (ft_8008A324) or the exit fall.
    fn finish_size_change(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Mushroom(state) = self.core.state_data.clone() else {
            unreachable!("a Kinoko state's scratch")
        };
        self.core.restore_movement(&state.saved);
        let m = &assets.mushroom;
        match state.finish {
            MushroomFinish::Grown => {
                let status = &mut self.core.status.mushroom;
                status.giant = true;
                status.frames = m.frames;
                // retail 800D1600..800D16B4: the ints as floats, fsubs,
                // the cap by fcmpo, then fadds and fctiwz.
                let percent = self.core.physics.percent;
                let threshold = m.percent_threshold as f32;
                if percent > threshold {
                    let mut extra = percent - threshold;
                    if extra >= m.percent_cap as f32 {
                        extra = m.percent_cap as f32;
                    }
                    let status = &mut self.core.status.mushroom;
                    status.frames = gekko_math::msl::fctiwz(status.frames as f32 + extra);
                }
                self.core.take_size(state.to, assets);
                self.core.clear_movement();
            }
            MushroomFinish::Shrunk => {
                let status = &mut self.core.status.mushroom;
                status.small = true;
                status.frames = m.frames;
                self.core.take_size(state.to, assets);
                self.core.clear_movement();
            }
            MushroomFinish::GiantEnded | MushroomFinish::SmallEnded => {
                if state.finish == MushroomFinish::GiantEnded {
                    self.core.status.mushroom.giant = false;
                } else {
                    self.core.status.mushroom.small = false;
                }
                let base = self.core.base_scale;
                self.core.take_size(base, assets);
                if state.clears_movement {
                    self.core.clear_movement();
                }
            }
        }
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            return self.change_motion_state(S::Wait.into(), assets);
        }
        match state.exit {
            MushroomExit::Fall => self.change_motion_state(S::Fall.into(), assets),
            MushroomExit::FallAerial => self.change_motion_state(S::FallAerial.into(), assets),
            MushroomExit::FallSpecial => {
                if self.core.holding_hammer() {
                    unimplemented!("ftCo_800968C8 (ftCo_FallSpecial.c:25): FallSpecial with the Hammer");
                }
                self.enter_ordinary_special_fall(assets)
            }
        }
    }
}

/// The Kinoko states' Anim (fn_800D2A3C): a step of the scale's animation
/// sets the scale between the two sizes (fmadds); once it ended the change
/// finishes.
pub fn size_anim(f: &mut Fighter, p: super::state::AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let MotionData::Mushroom(state) = &mut f.core.state_data else {
        panic!("a Kinoko state's scratch")
    };
    let curve = &p
        .assets
        .mushroom_sizes
        .as_ref()
        .expect("the Super Mushroom's size animations")
        .curves[state.curve];
    let factor = curve[state.steps.min(curve.len() - 1)];
    let ended = state.steps + 1 >= curve.len();
    state.steps += 1;
    // retail 800D2A90..94: fsubs, then fmadds.
    let scale = gekko_math::fma::fmadds(factor, state.to - state.from, state.from);
    f.core.set_scale(scale);
    if ended {
        f.finish_size_change(p.assets)?;
    }
    Ok(None)
}

/// The Kinoko states have no Phys: Fighter_procUpdate's tail runs alone
/// (the movement is cleared): on the ground the floor carries the fighter
/// (mpGetSpeed), in the air the knockback decays.
pub fn physics(f: &mut Fighter, p: super::state::PhysicsPhase<'_>) {
    let super::state::PhysicsPhase { assets, map, wind } = p;
    if f.core.physics.ground_or_air == GroundOrAir::Ground {
        crate::physics::integrate::integrate_velocity(&mut f.core.physics);
        let speed = map.line_speed(f.core.collision.data.floor.index, &f.core.physics.position);
        crate::physics::integrate::integrate_environment(&mut f.core.physics, speed, wind);
    } else {
        f.core.finish_air_update(assets, wind);
    }
}

/// The grounded Kinoko states' Coll (ft_8008403C): off the floor, the
/// airborne twin at the same frame (fn_800D2CD4: airborne, the saved
/// ground speed and vertical animation speed zeroed).
pub fn ground_collision(f: &mut Fighter, p: super::state::CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    let supported = ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported;
    if supported {
        return Ok(());
    }
    let assets = p.assets.expect("Kinoko collision assets");
    f.leave_ground();
    if let MotionData::Mushroom(state) = &mut f.core.state_data {
        state.saved.ground_velocity = 0.0;
        state.saved.animation_velocity.y = 0.0;
    }
    let twin = air_twin(f.core.motion_state.id);
    switch_twin(f, twin, assets)
}

/// The airborne Kinoko states' Coll (ft_80082C74): a landing takes the
/// grounded twin (fn_800D2CA0: grounded, the saved ground speed the saved
/// horizontal speed).
pub fn air_collision(f: &mut Fighter, p: super::state::CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(&c.physics, &mut c.collision, &mut c.skeleton, c.animation.root);
    let landed = air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    );
    if !landed {
        return Ok(());
    }
    let assets = p.assets.expect("Kinoko collision assets");
    f.land();
    if let MotionData::Mushroom(state) = &mut f.core.state_data {
        state.saved.ground_velocity = state.saved.self_velocity.x;
    }
    let twin = ground_twin(f.core.motion_state.id);
    switch_twin(f, twin, assets)
}

fn switch_twin(f: &mut Fighter, twin: S, assets: &FighterAssets) -> Result<()> {
    let ends = matches!(
        twin,
        S::KinokoGiantEnd | S::KinokoGiantEndAir | S::KinokoSmallEnd | S::KinokoSmallEndAir
    );
    let flags = if ends {
        MotionEntryFlags(SWITCH.0 | MotionEntryFlags::KEEP_FAST_FALL.0)
    } else {
        SWITCH
    };
    let state = f.core.state_data.clone();
    let (frame, rate) = (f.core.animation.frame, f.core.animation.speed);
    f.change_motion_state_with_flags(twin.into(), assets, flags, frame, rate)?;
    f.core.state_data = state;
    Ok(())
}

fn air_twin(state: S) -> S {
    match state {
        S::KinokoGiantStart => S::KinokoGiantStartAir,
        S::KinokoGiantEnd => S::KinokoGiantEndAir,
        S::KinokoSmallStart => S::KinokoSmallStartAir,
        S::KinokoSmallEnd => S::KinokoSmallEndAir,
        other => unreachable!("{other:?} is no grounded Kinoko state"),
    }
}

fn ground_twin(state: S) -> S {
    match state {
        S::KinokoGiantStartAir => S::KinokoGiantStart,
        S::KinokoGiantEndAir => S::KinokoGiantEnd,
        S::KinokoSmallStartAir => S::KinokoSmallStart,
        S::KinokoSmallEndAir => S::KinokoSmallEnd,
        other => unreachable!("{other:?} is no airborne Kinoko state"),
    }
}
