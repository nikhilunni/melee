//! The Hammer in hand: ftCo_HammerWait.c .. ftCo_HammerLanding.c and
//! ftCo_0C60.c.
//!
//! Picked up (ftpickupitem.c:182 -> ftCo_800C52F4) the Hammer starts its
//! timer (x2330.x, PlCo +6AC) and may set its head loose after x2330.y
//! frames (it_80293DCC). The seven Hammer states (motions 307..313) keep
//! the swing's animation frame across their changes (ftCo_800C548C) with
//! the hit, model and colour state kept (ftCo_800C54C4), and each entry
//! turns the Hammer's hitboxes on (ftCo_800C4E94). Every loop of the swing
//! starts a new attack instance for the item (inlineB0). The countdowns run
//! in Fighter_8006A360 (ftCo_800C511C).
use super::{
    assets::{FighterAssets, Result},
    jump::{JumpInput, JumpState},
    state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    turn::TurnState,
    walk::WalkState,
    Fighter, FighterCore, MotionData, MotionEntryFlags,
};
use crate::{anim::WaitChoice, collision::{air, ground}, input::Buttons};
use melee_types::{CommonMotionState as S, GroundOrAir, ItemKind};

/// The Hammer's colour animation (ftCo_800C5408).
pub(super) const HAMMER_COLOR: u8 = 106;

/// PlCo +6AC..+6B8.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HammerParameters {
    /// +6AC: the frames the Hammer lasts.
    pub frames: i32,
    /// +6B0: HammerTurn's frames.
    pub turn_frames: i32,
    /// +6B4: HammerLanding's frames.
    pub landing_frames: i32,
    /// +6B8: after hitstun, the frames in which A with L or R drops it
    /// (ftCo_800C554C).
    pub drop_window: i32,
}
impl HammerParameters {
    pub fn read(r: &hsd_archive::Reader<'_>, common: u32) -> Result<Self> {
        Ok(Self {
            frames: r.s32(common + 0x6AC)?,
            turn_frames: r.s32(common + 0x6B0)?,
            landing_frames: r.s32(common + 0x6B4)?,
            drop_window: r.s32(common + 0x6B8)?,
        })
    }
}

/// itHammerData as the pickup reads it (it_80293DCC): the head's countdown
/// range x0..x4 and the percent chance x8 it comes off at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HammerHeadOdds {
    pub low: i32,
    pub high: i32,
    pub percent: f32,
}
impl HammerHeadOdds {
    /// it_80293DCC (80293DCC): HSD_Randf against x8 / 100 (fdivs), then
    /// HSD_Randi(x4 - x0) + x0; 0 for a head that stays on.
    pub fn countdown(&self, rng: &mut gekko_math::HsdRng) -> i32 {
        if rng.randf() < self.percent / 100.0 {
            return rng.randi(self.high - self.low) + self.low;
        }
        0
    }
}

/// x2330 / x2338: the Hammer's countdowns, kept between holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HammerHold {
    /// x2330.x: frames until the Hammer is gone.
    pub frames: i32,
    /// x2330.y: frames until the head comes off (0: it stays on). Counts
    /// only in the Hammer states.
    pub head_frames: i32,
    /// x2338.x: the head is off.
    pub headless: bool,
    /// x2338.y: frames left to drop it with A and L/R.
    pub drop_window: i32,
}

/// The Hammer states' scratch (mv.co.hammerkneebend, hammerlanding).
#[derive(Clone, Debug)]
pub enum HammerState {
    /// HammerWait and HammerFall write nothing: mv+4 is the word the state
    /// before left (`None` where the port does not model it).
    Carry { retained_word: Option<f32> },
    /// mv.co.hammerkneebend: the hop decision, the jump's input, the
    /// frames left (x8).
    KneeBend {
        short_hop: bool,
        input: JumpInput,
        frames: f32,
    },
    /// mv.co.hammerlanding.x4: the frames left.
    Landing { frames: f32 },
}

/// Ft_MF_SkipHit | SkipModel | SkipMatAnim | SkipRumble | SkipColAnim |
/// UpdateCmd | Unk19 (ftCo_800C54C4), between Hammer states.
const KEEP_SWING: MotionEntryFlags = MotionEntryFlags(
    MotionEntryFlags::SKIP_HIT.0
        | MotionEntryFlags::SKIP_MODEL.0
        | MotionEntryFlags::SKIP_MAT_ANIM.0
        | (1 << 11)
        | MotionEntryFlags::SKIP_COL_ANIM.0
        | MotionEntryFlags::UPDATE_CMD.0
        | (1 << 19),
);

impl FighterCore {
    /// ftCo_800C5240 (800C5240): the hand holds the Hammer.
    pub fn holding_hammer(&self) -> bool {
        self.held_item
            .is_some_and(|held| held.kind == ItemKind::Hammer)
    }

    /// ftCo_800C53E4 (800C53E4): one of the Hammer states.
    pub fn in_hammer_state(&self) -> bool {
        (S::HammerWait as u16..=S::HammerLanding as u16)
            .contains(&(self.motion_state.id as u16))
    }

    /// ftCo_800C54C4 and ftCo_800C548C: the flags and start frame of a
    /// change into a Hammer state.
    fn hammer_entry(&self) -> (MotionEntryFlags, f32) {
        if self.in_hammer_state() {
            (KEEP_SWING, self.animation.frame)
        } else {
            (MotionEntryFlags(0), 0.0)
        }
    }

    /// ftCo_800C5284 (800C5284): the timer (PlCo +6AC) and the head's
    /// countdown (it_80293DCC: its draws are the pickup's), nothing off
    /// yet. pl_800403C0's statistics and ft_800880AC's music are not
    /// simulated.
    pub(super) fn take_up_hammer(
        &mut self,
        assets: &FighterAssets,
        rng: Option<&mut gekko_math::HsdRng>,
    ) {
        self.status.hammer = HammerHold {
            frames: assets.hammer.frames,
            head_frames: 0,
            headless: false,
            drop_window: 0,
        };
        self.hammer_draw_pending = true;
        if let Some(rng) = rng {
            self.resolve_hammer_draw(rng);
        }
    }

    /// it_80293DCC's draws for a take-up made without the random stream
    /// at hand (an IASA's catch, a hit in LightGet): nothing else draws in
    /// between, so making them here keeps retail's order.
    pub fn resolve_hammer_draw(&mut self, rng: &mut gekko_math::HsdRng) {
        if !std::mem::take(&mut self.hammer_draw_pending) {
            return;
        }
        let held = self.held_item.expect("ftCo_800C5284 with the Hammer in hand");
        let head = held.hammer_head.expect("the Hammer's head odds");
        self.status.hammer.head_frames = head.countdown(rng);
    }

    /// ftCo_800C4E94 (800C4E94) after each Hammer entry: its colour unless
    /// shown (ftCo_800C5408), the Hammer's hitboxes on (ftCo_800C5588:
    /// it_8026C16C(item, true), x2219_b4). The rumble is not simulated.
    fn hammer_entered(&mut self, assets: &FighterAssets) {
        if self.shown_color(assets) != HAMMER_COLOR {
            self.install_color_overlay_now(HAMMER_COLOR, assets);
        }
        self.request_hammer_hitboxes(true);
        self.commands.item_hitboxes_active = true;
    }

    /// it_8026C16C on the held Hammer.
    fn request_hammer_hitboxes(&mut self, on: bool) {
        let held = self.held_item.expect("the Hammer in hand");
        self.item_requests
            .push(melee_it::ItemRequest::HammerHitboxes { item: held.item, on });
    }

    /// ft_8007C114 (8007C114), from a motion change without Ft_MF_SkipHit
    /// while x2219_b4 is set: a held Hammer's hitboxes go (ftCo_800C555C).
    pub(super) fn drop_item_hitboxes(&mut self) {
        if !std::mem::take(&mut self.commands.item_hitboxes_active) {
            return;
        }
        if self.holding_hammer() {
            self.request_hammer_hitboxes(false);
        }
    }

    /// ftCo_800C5500 (800C5500) from ftCommon_8007E6DC as the Hammer leaves
    /// the hand: its colour goes if it is shown (ftCo_800C544C). The music
    /// (ft_80088110) is not simulated.
    pub(super) fn hammer_released(&mut self, assets: &FighterAssets) {
        if self.shown_color(assets) == HAMMER_COLOR {
            self.release_charge_color(HAMMER_COLOR, assets);
        }
    }

    /// ftCo_800C554C (800C554C): hitstun over, A with L/R may drop it.
    pub(super) fn open_hammer_drop_window(&mut self, assets: &FighterAssets) {
        self.status.hammer.drop_window = assets.hammer.drop_window;
    }
}

impl Fighter {
    /// ftCo_800C52F4 (800C52F4), the Hammer's pickup: its countdowns, then
    /// HammerWait on the ground (inlineA0) or HammerFall in the air.
    pub(super) fn take_hammer(
        &mut self,
        assets: &FighterAssets,
        rng: Option<&mut gekko_math::HsdRng>,
    ) -> Result<()> {
        self.core.take_up_hammer(assets, rng);
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.enter_hammer_wait(assets)
        } else {
            self.enter_hammer_fall(assets)
        }
    }

    fn change_hammer_state(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        let (flags, frame) = self.core.hammer_entry();
        self.change_motion_state_with_flags(state.into(), assets, flags, frame, 1.0)
    }

    /// fn_800D6F58 with the Hammer caught: ftCo_800C52F4, then the pickup
    /// sparkle and x2224_b1.
    pub(super) fn finish_hammer_catch(
        &mut self,
        assets: &FighterAssets,
        rng: Option<&mut gekko_math::HsdRng>,
    ) -> Result<()> {
        self.take_hammer(assets, rng)?;
        // The take-over's colour step (ftCo_800C0408 in the motion change)
        // queued its graphic before the sparkle: the sparkle joins the
        // queue once that graphic is resolved (`resolve_graphics_commands`).
        self.core.catch_sparkle_pending = true;
        self.core.item_catch_locked = true;
        Ok(())
    }

    /// ftCo_800C4ED8 (800C4ED8): HammerWait.
    pub(super) fn enter_hammer_wait(&mut self, assets: &FighterAssets) -> Result<()> {
        let retained_word = self.inherited_scratch_word();
        self.change_hammer_state(S::HammerWait, assets)?;
        self.core.state_data = MotionData::Hammer(HammerState::Carry { retained_word });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C5614 (800C5614): HammerWalk with the scale's walk
    /// acceleration and the ground speed (mv.co.walk.accel_mul and x0);
    /// one animation step (ftAnim_8006EBA4).
    fn enter_hammer_walk(&mut self, assets: &FighterAssets) -> Result<()> {
        let mut multiplier = 1.0;
        if self.core.player.scale != 1.0 {
            multiplier = super::model_scale::scaled(
                1.0,
                self.core.player.scale,
                assets.scale_curves.walk_acceleration(),
            );
        }
        // HammerWalk writes x0 and accel_mul only: mv+4 (the walk's msid
        // word) is its predecessor's.
        let base_motion = self
            .inherited_scratch_word()
            .map_or(0, |word| word.to_bits() as i32);
        let ground_velocity = self.core.physics.ground_velocity;
        self.change_hammer_state(S::HammerWalk, assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Walk(WalkState {
            slippery_animation_velocity: ground_velocity,
            base_motion,
            acceleration_multiplier: multiplier,
        });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C57B4 (800C57B4): ftCo_Turn_Enter on HammerTurn with PlCo
    /// +6B0 frames to turn.
    fn enter_hammer_turn(&mut self, assets: &FighterAssets) -> Result<()> {
        let (flags, frame) = self.core.hammer_entry();
        let turn = MotionData::Turn(TurnState {
            has_turned: false,
            just_turned: false,
            facing_after: -self.core.physics.facing,
            dash_direction: 0.0,
            frames_to_turn: assets.hammer.turn_frames as f32,
            buffered_buttons: Buttons(0),
        });
        self.core.state_data = turn.clone();
        self.change_motion_state_with_flags(S::HammerTurn.into(), assets, flags, frame, 1.0)?;
        self.core.state_data = turn;
        self.step_animation(assets);
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C5B88 (800C5B88): the jump squat for jump_startup_time
    /// frames.
    fn enter_hammer_knee_bend(&mut self, assets: &FighterAssets, input: JumpInput) -> Result<()> {
        let frames = self.core.attributes.jumping.jump_startup_time;
        self.change_hammer_state(S::HammerKneeBend, assets)?;
        self.core.state_data = MotionData::Hammer(HammerState::KneeBend {
            short_hop: false,
            input,
            frames,
        });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C5A98 (800C5A98): off the ground (ftCommon_8007D5D4),
    /// HammerJump and ftCo_800CB110(gobj, false, 1.0).
    pub(super) fn enter_hammer_jump(&mut self, assets: &FighterAssets, short_hop: bool) -> Result<()> {
        self.leave_ground();
        self.change_hammer_state(S::HammerJump, assets)?;
        // ftCo_800CB110 with jump_mul 1 (retail 800CB140..800CB1B8).
        let attrs = &self.core.attributes.jumping;
        let multiplier = 1.0;
        let momentum = self.core.physics.self_velocity.x
            * (attrs.ground_to_air_jump_momentum_multiplier * multiplier);
        let horizontal = momentum
            + multiplier * (self.core.input.current.stick.x * attrs.jump_h_initial_velocity);
        let maximum = attrs.jump_h_max_velocity * multiplier;
        let vertical = if short_hop {
            attrs.hop_v_initial_velocity
        } else {
            attrs.jump_v_initial_velocity
        } * multiplier;
        self.core.physics.self_velocity =
            hsd_types::Vec3::new(horizontal.clamp(-maximum, maximum), vertical, 0.0);
        self.core.input.vertical.tilt = 0xFE;
        self.core.state_data = MotionData::Jump(JumpState {
            short_hop,
            physics_started: 0,
            multiplier,
        });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C5D34 (800C5D34): HammerFall; a grounded fighter leaves the
    /// ground after the change.
    pub(super) fn enter_hammer_fall(&mut self, assets: &FighterAssets) -> Result<()> {
        let retained_word = self.inherited_scratch_word();
        self.change_hammer_state(S::HammerFall, assets)?;
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.leave_ground();
        }
        self.core.state_data = MotionData::Hammer(HammerState::Carry { retained_word });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_HammerLanding_Enter (800C61B8): ftCo_Landing_Enter on
    /// HammerLanding (allow_interrupt 1) for PlCo +6B4 frames.
    pub(super) fn enter_hammer_landing(&mut self, assets: &FighterAssets) -> Result<()> {
        self.land();
        self.change_hammer_state(S::HammerLanding, assets)?;
        self.character.on_landing(true);
        (self.character.table().landing_articles)(self, true);
        self.core.state_data = MotionData::Hammer(HammerState::Landing {
            frames: assets.hammer.landing_frames as f32,
        });
        self.core.hammer_entered(assets);
        Ok(())
    }

    /// ftCo_800C5A50 (800C5A50): ftCo_Jump_GetInput, then the squat.
    pub(super) fn try_hammer_jump(&mut self, assets: &FighterAssets) -> Result<bool> {
        let input = &self.core.input;
        let thresholds = &assets.input.thresholds;
        let jump = if input.current.stick.y >= thresholds.tap_jump_threshold
            && i32::from(input.vertical.tilt) < thresholds.tap_jump_window
        {
            JumpInput::Stick
        } else if input.pressed.intersects(Buttons::XY) {
            JumpInput::Buttons
        } else {
            return Ok(false);
        };
        self.enter_hammer_knee_bend(assets, jump)?;
        Ok(true)
    }

    /// ftCo_800C60C8 (800C60C8): a flick down on a platform
    /// (ftCo_80099F1C) drops into HammerFall (ftCo_800C6110).
    pub(super) fn try_hammer_pass(&mut self, assets: &FighterAssets) -> Result<bool> {
        let on_platform =
            self.core.collision.data.floor.flags & melee_types::mp::line_flag::PLATFORM != 0;
        if self.core.input.current.stick.y <= -assets.movement.platform_drop_threshold
            && f32::from(self.core.input.vertical.tilt) < assets.movement.platform_drop_window
            && on_platform
        {
            unimplemented!("ftCo_800C6110 (ftCo_0C60.c:17): a platform drop with the Hammer");
        }
        Ok(false)
    }

    /// ftCo_800C5DDC (800C5DDC): in the drop window, A with L or R held
    /// (the last A press at least PlCo +1C frames before) lets go
    /// (Item_8026ABD8 at rest, speed 1) into Fall. ftCo_800C544C finds
    /// the colour already gone; pl_8003FDF4 counts statistics.
    pub(super) fn try_hammer_drop(&mut self, assets: &FighterAssets) -> Result<bool> {
        let input = &self.core.input;
        if self.core.status.hammer.drop_window == 0
            || !input.pressed.intersects(Buttons::A)
            || !input.current.held.intersects(Buttons::SHIELD)
            || i32::from(input.buttons.previous_attack) < assets.damage.tech_lockout
        {
            return Ok(false);
        }
        let held = self.core.held_item.expect("the Hammer in hand");
        self.core.drop_held_item(held, 1.0, assets);
        self.change_motion_state(S::Fall.into(), assets)?;
        Ok(true)
    }

    /// inlineB0 (every Hammer state's Anim): headless, the fighter's own
    /// hitboxes go (ftColl_8007AFF8); else at each loop's first frame a new
    /// attack instance (ft_800892A0, ft_80089824) the Hammer takes
    /// (it_8027B070).
    pub(super) fn hammer_swing(&mut self) {
        if self.core.status.hammer.headless {
            self.core.commands.disable_all_hitboxes();
            return;
        }
        let animation = &self.core.animation;
        if animation.frame >= 0.0 && animation.frame < animation.speed {
            self.core.combat.stale.new_instance();
            let held = self.core.held_item.expect("the Hammer in hand");
            let attack = self.core.combat.stale.attack();
            self.core.item_requests.push(melee_it::ItemRequest::TakeAttack {
                item: held.item,
                attack,
            });
        }
    }

    /// ftCo_800C511C (800C511C), Fighter_8006A360 (fighter.c:1636-1641)
    /// while the Hammer is in hand, outside LightGet: the timer; in a
    /// Hammer state the head's countdown, whose end launches it
    /// (it_80293E34); at the timer's end the Hammer is gone and a fighter
    /// in a Hammer state returns to Wait or Fall (ftCommon_8007D92C); the
    /// drop window. The rumble and music are not simulated.
    pub(super) fn tick_hammer(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.holding_hammer() || self.core.motion_state.id == S::LightGet {
            return Ok(());
        }
        let hammer = &mut self.core.status.hammer;
        if hammer.frames != 0 {
            hammer.frames -= 1;
        }
        if self.core.status.hammer.head_frames != 0 && self.core.in_hammer_state() {
            self.core.status.hammer.head_frames -= 1;
            if self.core.status.hammer.head_frames == 0 {
                self.core.status.hammer.headless = true;
                self.core.launch_hammer_head();
            }
        }
        if self.core.status.hammer.frames == 0 {
            let held = self.core.held_item.expect("the Hammer in hand");
            self.core
                .item_requests
                .push(melee_it::ItemRequest::Destroy { item: held.item });
            self.core.release_held_item(held.item, assets);
            if self.core.in_hammer_state() {
                if self.core.physics.ground_or_air == GroundOrAir::Air {
                    self.change_motion_state(S::Fall.into(), assets)?;
                } else {
                    self.change_motion_state(S::Wait.into(), assets)?;
                }
            }
        }
        if self.core.status.hammer.drop_window != 0 {
            self.core.status.hammer.drop_window -= 1;
        }
        Ok(())
    }
}

impl FighterCore {
    /// it_80293E34 (80293E34) on the held Hammer: the scene sends the head
    /// off along the handle (it_80299C48 from joint 3, toward it from joint
    /// 1), at the holder's ECB centre (ftLib_80086990).
    fn launch_hammer_head(&mut self) {
        let held = self.held_item.expect("the Hammer in hand");
        let midpoint = 0.5 * (self.collision.data.ecb.top.y + self.collision.data.ecb.bottom.y);
        let center = hsd_types::Vec3::new(
            self.physics.position.x + 0.0,
            self.physics.position.y + midpoint,
            self.physics.position.z + 0.0,
        );
        self.item_requests
            .push(melee_it::ItemRequest::HammerHeadLoss {
                item: held.item,
                owner: self.player.id,
                center,
            });
    }
}

/// HammerWait / HammerWalk / HammerFall / HammerJump Anim: inlineB0.
pub fn swing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.hammer_swing();
    Ok(None)
}

/// ftCo_HammerWait_IASA (800C50AC): the jump, a platform drop, the turn
/// (ftCo_800C97A8), the walk (ftWalkCommon_800DFC70).
pub fn wait_input(f: &mut Fighter, p: InputPhase<'_>) {
    hammer_wait_transitions(f, p.assets).expect("Hammer assets");
}

/// ftCo_HammerWait_IASA's checks, which a grounded Damage past hitstun
/// also runs (ftCo_Damage_IASA).
pub(super) fn hammer_wait_transitions(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.try_hammer_jump(assets)? || f.try_hammer_pass(assets)? {
        return Ok(());
    }
    let along = f.core.input.current.stick.x * f.core.physics.facing;
    let thresholds = &assets.input.thresholds;
    if along <= thresholds.turn_stick_threshold {
        f.enter_hammer_turn(assets)
    } else if along >= thresholds.walk_stick_threshold {
        f.enter_hammer_walk(assets)
    } else {
        Ok(())
    }
}

/// ftCo_HammerWalk_IASA (800C56D0): the jump, a platform drop, the wait
/// (ft_8008A1FC).
pub fn walk_input(f: &mut Fighter, p: InputPhase<'_>) {
    let assets = p.assets;
    let entered = (|| -> Result<()> {
        if f.try_hammer_jump(assets)? || f.try_hammer_pass(assets)? {
            return Ok(());
        }
        let stick = f.core.input.current.stick.x;
        if stick * f.core.physics.facing < 0.0
            || gekko_math::msl::fabsf(stick) < assets.input.thresholds.walk_stick_threshold
        {
            return f.enter_hammer_wait(assets);
        }
        Ok(())
    })();
    entered.expect("Hammer assets");
}

/// ftCo_HammerTurn_IASA (800C5964): the jump.
pub fn turn_input(f: &mut Fighter, p: InputPhase<'_>) {
    f.try_hammer_jump(p.assets).expect("Hammer assets");
}

/// ftCo_HammerKneeBend_IASA (800C5C78): ftCo_KneeBend_Check_ShortHop.
pub fn knee_bend_input(f: &mut Fighter, p: InputPhase<'_>) {
    let release = p.assets.jumping.release_threshold;
    let current = f.core.input.current;
    let MotionData::Hammer(HammerState::KneeBend {
        short_hop, input, ..
    }) = &mut f.core.state_data
    else {
        panic!("Hammer jump squat data missing")
    };
    *short_hop |= match input {
        JumpInput::Buttons => !current.held.intersects(Buttons::XY),
        JumpInput::Stick => current.stick.y < release,
        JumpInput::CStick => current.cstick.y < release,
    };
}

/// ftCo_HammerFall_IASA (800C5E8C): the drop.
pub fn fall_input(f: &mut Fighter, p: InputPhase<'_>) {
    f.try_hammer_drop(p.assets).expect("Hammer assets");
}

/// The empty IASA callbacks (HammerJump's, HammerLanding's).
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCo_HammerTurn_Anim (800C5838): inlineB0; while frames remain they
/// count down and, unless x2222_b6 (never set for a Vs fighter,
/// fighter.c:785), TransN turns pi * (PlCo +6B0 - left) / +6B0; at none
/// left the facing flips into HammerWait.
pub fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.hammer_swing();
    let MotionData::Turn(turn) = &mut f.core.state_data else {
        panic!("Hammer turn data missing")
    };
    if turn.frames_to_turn == 0.0 {
        return Ok(None);
    }
    turn.frames_to_turn -= 1.0;
    let left = turn.frames_to_turn;
    let frames = p.assets.hammer.turn_frames as f32;
    let part = usize::from(
        p.assets
            .parts
            .joint(melee_types::FtPart::TransN)
            .expect("TransN"),
    );
    let joint = f.core.animation.parts[part].joint;
    // TODO(asm): M_PI times the float quotient, in double, rounded once.
    let angle = (std::f64::consts::PI * f64::from((frames - left) / frames)) as f32;
    f.core
        .set_part_rotation(part, super::part_rotation::Axis::Y, angle);
    f.core.animation.blend_tree.set_rotation_y(joint, angle);
    if left <= 0.0 {
        f.core.physics.facing = -f.core.physics.facing;
        f.enter_hammer_wait(p.assets)?;
    }
    Ok(None)
}

/// ftCo_HammerKneeBend_Anim (800C5BFC): inlineB0; the jump once the frames
/// run out (checked before the decrement, which then lands in the jump's
/// jump_mul).
pub fn knee_bend_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.hammer_swing();
    let MotionData::Hammer(HammerState::KneeBend {
        short_hop, frames, ..
    }) = &mut f.core.state_data
    else {
        panic!("Hammer jump squat data missing")
    };
    if *frames <= 0.0 {
        let short_hop = *short_hop;
        f.enter_hammer_jump(p.assets, short_hop)?;
        if let MotionData::Jump(jump) = &mut f.core.state_data {
            jump.multiplier -= 1.0;
        }
        return Ok(None);
    }
    *frames -= 1.0;
    Ok(None)
}

/// ftCo_HammerLanding_Anim (800C6268): inlineB0; HammerWait once the
/// frames run out (checked before the decrement).
pub fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.hammer_swing();
    let MotionData::Hammer(HammerState::Landing { frames }) = &mut f.core.state_data else {
        panic!("Hammer landing data missing")
    };
    if *frames <= 0.0 {
        f.enter_hammer_wait(p.assets)?;
        return Ok(None);
    }
    *frames -= 1.0;
    Ok(None)
}

/// ftCo_HammerWait_Coll (800C5100) and the other grounded rows: off the
/// floor (ft_8008403C -> ft_80082708), HammerFall.
pub fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    let supported = ground::map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) == ground::WaitGroundResult::Supported;
    if !supported {
        f.enter_hammer_fall(p.assets.expect("Hammer collision assets"))?;
    }
    Ok(())
}

/// ftCo_HammerFall_Coll (800C5F00) / HammerJump's: landing (ft_80082C74
/// -> ft_80081D0C) enters HammerLanding.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    );
    if landed {
        f.enter_hammer_landing(p.assets.expect("Hammer collision assets"))?;
    }
    Ok(())
}

/// HammerWait / HammerTurn / HammerKneeBend / HammerLanding Phys:
/// ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::wait(f, p);
}

/// ftCo_HammerFall_Phys (800C5EDC): ft_80084DB0.
pub fn fall_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::pass(f, p);
}
