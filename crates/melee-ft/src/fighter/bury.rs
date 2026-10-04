//! Buried in the ground: ftCo_Bury.c (800C0CB8..800C16E8).
//!
//! A Ground-element hit (Donkey Kong's Headbutt) on a grounded fighter
//! takes the ordinary damage entry and then plants the victim in the floor:
//! Bury sinks the collision box for PlCo +5F4 frames, BuryWait holds, and a
//! mash timer (or losing the floor) pops the fighter out in BuryJump. While
//! buried a hit deals damage without a reaction (x2220_b3).
use super::{
    assets::{FighterAssets, Result},
    caches::bone_position,
    state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
    Fighter, MotionData, MotionEntryFlags,
};
use crate::{
    anim::WaitChoice,
    collision::{air, ecb::EcbPose, ground},
};
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_types::{mp::FtCollisionBox, CommonMotionState as S, FtPart, GroundOrAir, HitElement};

/// PlCo +5F4..+620.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// +5F4: frames the fighter takes to sink.
    pub sink_frames: i32,
    /// +5F8..+60C: the mash timer's base, handicap, rank and percent terms.
    pub base_timer: f32,
    pub handicap_scale: f32,
    pub handicap_origin: f32,
    pub rank_scale: f32,
    pub rank_origin: f32,
    pub percent_scale: f32,
    /// +610: what each frame takes off the timer.
    pub decrement: f32,
    /// +614: what each mash input takes off.
    pub mash_decrement: f32,
    /// +618: the jump out's vertical speed.
    pub jump_speed: f32,
    /// +61C: frames of BuryJump before the aerial interrupts.
    pub jump_interrupt_frames: f32,
    /// +620: frames of invincibility on the way out.
    pub jump_invincible_frames: i32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            sink_frames: r.s32(base + 0x5F4)?,
            base_timer: r.f32(base + 0x5F8)?,
            handicap_scale: r.f32(base + 0x5FC)?,
            handicap_origin: r.f32(base + 0x600)?,
            rank_scale: r.f32(base + 0x604)?,
            rank_origin: r.f32(base + 0x608)?,
            percent_scale: r.f32(base + 0x60C)?,
            decrement: r.f32(base + 0x610)?,
            mash_decrement: r.f32(base + 0x614)?,
            jump_speed: r.f32(base + 0x618)?,
            jump_interrupt_frames: r.f32(base + 0x61C)?,
            jump_invincible_frames: r.s32(base + 0x620)?,
        })
    }
}

/// grab_timer and its mash latches with mv.co.bury.
#[derive(Clone, Debug)]
pub struct BuryState {
    pub timer: f32,
    stick_directions: [i8; 2],
    /// x0: frames of sinking left (BuryWait once it reaches zero).
    pub frames_left: i32,
    /// x20: the floor line the fighter was buried in.
    pub floor: i32,
    /// x1C: how far the collision box's bottom rises each frame.
    pub sink: f32,
    /// coll_box: the ECB at entry, its bottom rising as the fighter sinks.
    pub collision_box: FtCollisionBox,
    /// translate: where the dirt mound sits.
    pub mound: Vec3,
}

/// efSync_Spawn(1095, gobj, &cur_pos, &scale.y): the dirt mound, common
/// model 0x28.
const MOUND_EFFECT: u16 = 0x447;
pub const MOUND_MODEL: u32 = 0x28;

/// Ft_MF_SkipMatAnim | Ft_MF_SkipColAnim, the flags of the Bury entry.
const BURY_FLAGS: MotionEntryFlags =
    MotionEntryFlags(MotionEntryFlags::SKIP_MAT_ANIM.0 | MotionEntryFlags::SKIP_COL_ANIM.0);

impl Fighter {
    /// ftCo_800C0CB8 (800C0CB8): a Ground-element hit buries a grounded
    /// fighter (x2227_b6, set by the death states, never holds here).
    pub(super) fn bury_hit(&self, element: HitElement) -> bool {
        element == HitElement::Ground && self.core.physics.ground_or_air == GroundOrAir::Ground
    }

    /// ftCo_800C0D0C (800C0D0C): the ordinary damage entry
    /// (ftCo_8008DCE0(gobj, -1, 0)), then the fighter is put back on the
    /// floor in Bury with every movement source cleared, the mash timer
    /// started (ftCommon_InitGrab) and the dirt mound spawned.
    pub(super) fn enter_bury(
        &mut self,
        hit: melee_coll::damage::ReceivedHit,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        self.interrupt_actions(assets);
        // ftCo_8009750C: a heavy item would be dropped.
        if self.core.held_item.as_ref().is_some_and(|held| held.heavy) {
            unimplemented!("ftCo_8009750C: buried with a heavy item");
        }
        // ftCo_800DD168: ftCo_8008EC90 only comes here without a grab link.
        assert!(
            self.core.combat.grab.is_none(),
            "ftCo_800DD168: buried in a grab pair"
        );
        self.begin_damage_reaction(hit, None, None, None, false, assets, rng)?;
        self.land();
        self.change_motion_state_with_flags(S::Bury.into(), assets, BURY_FLAGS, 0.0, 1.0)?;
        self.core.clear_movement();
        let p = &assets.bury;
        // retail 800C0DBC..800C0E0C: the rank term (fsubs, fsubs, fmuls),
        // the handicap term (fsubs, fmadds), fadds, fmadds with percent.
        let rank = f32::from(self.core.standing_rank) + 1.0;
        let rank_term = p.rank_scale * (p.rank_origin - rank);
        let base = fmadds(
            p.handicap_scale,
            p.handicap_origin - f32::from(self.core.grab_handicap),
            p.base_timer,
        );
        let timer = fmadds(self.core.physics.percent, p.percent_scale, base + rank_term);
        self.set_buried_flags();
        let ecb = &self.core.collision.data.ecb;
        // ft_80084CB0: the box is the ECB as it stands.
        let collision_box = FtCollisionBox {
            top: ecb.top.y,
            bottom: ecb.bottom.y,
            left: hsd_types::Vec2::new(ecb.left.x, ecb.left.y),
            right: hsd_types::Vec2::new(ecb.right.x, ecb.right.y),
        };
        let root = self.core.animation.root;
        let top = bone_position(&mut self.core.skeleton, root, 0, Vec3::ZERO);
        let hip_bone = usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"));
        let hip = bone_position(&mut self.core.skeleton, root, hip_bone, Vec3::ZERO);
        // retail 800C0ED0..: fsubs, the int frames to float, fdivs.
        let sink = (hip.y - top.y) / p.sink_frames as f32;
        let mound = self.core.physics.position;
        self.core.state_data = MotionData::Bury(BuryState {
            timer,
            stick_directions: [0; 2],
            frames_left: p.sink_frames,
            floor: self.core.collision.data.floor.index,
            sink,
            collision_box,
            mound,
        });
        self.core.effects.push(EffectRequest::PositionalModel {
            id: MOUND_EFFECT,
            position: mound,
        });
        self.core.effect_state.destroy_on_state_change = true;
        Ok(())
    }

    /// ftCommon_8007E2F4(fp, 0x1FF), x221D_b5, x2220_b3 and x2224_b4, set by
    /// both buried rows' entries.
    fn set_buried_flags(&mut self) {
        let status = &mut self.core.status;
        status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        status.ignore_fighter_nudge = true;
        status.no_hit_reaction = true;
        status.buried = true;
    }

    /// The timer's frame: PlCo +610 off, +614 more per mash input. True
    /// when it ran out and the fighter jumped (ftCo_800C13BC).
    fn bury_mash(&mut self, assets: &FighterAssets) -> Result<bool> {
        let p = &assets.bury;
        let threshold = assets.grab_escape.stick_threshold;
        let input = self.core.input.clone();
        let MotionData::Bury(bury) = &mut self.core.state_data else {
            panic!("bury scratch missing")
        };
        bury.timer -= p.decrement;
        super::capture_yoshi::grab_mash(
            &mut bury.timer,
            &mut bury.stick_directions,
            &input,
            threshold,
            p.mash_decrement,
        );
        if bury.timer > 0.0 {
            return Ok(false);
        }
        self.enter_bury_jump(assets)?;
        Ok(true)
    }

    /// ftCo_800C13BC (800C13BC): out of the ground with PlCo +618 upward,
    /// every grab category still excluded and +620 frames of invincibility
    /// (ftColl_8007B7A4, with colour animation 9).
    fn enter_bury_jump(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Bury(bury) = &self.core.state_data else {
            panic!("bury scratch missing")
        };
        let retained_word = bury.collision_box.top;
        self.leave_ground();
        self.core.physics.self_velocity.x = 0.0;
        self.core.physics.self_velocity.y = assets.bury.jump_speed;
        self.change_motion_state(S::BuryJump.into(), assets)?;
        self.core.state_data = MotionData::BuryJump {
            frames: 0.0,
            retained_word,
        };
        let status = &mut self.core.status;
        status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        status.revival_invincibility = status
            .revival_invincibility
            .max(assets.bury.jump_invincible_frames);
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
        Ok(())
    }

    /// efLib_Cb_ftCo_Bury -> ftCo_800C0FCC (800C0FCC): the mound turns to
    /// the buried floor's normal each frame and rides a moving floor.
    fn tilt_mound(&mut self, map: &melee_mp::CollMap) {
        let MotionData::Bury(bury) = &self.core.state_data else {
            return;
        };
        if !map.line_is_active(bury.floor) {
            return;
        }
        let normal = map.line_get_normal(bury.floor);
        // mpGetSpeed: a floor that moved this frame carries the mound; a
        // still one adds nothing.
        if map
            .floor_speed(&self.core.collision.data)
            .is_some_and(|speed| speed != Vec3::ZERO)
        {
            unimplemented!("ftCo_Bury.c:214: a dirt mound on a moving floor (mpGetSpeed)");
        }
        self.core.effects.push(EffectRequest::OwnedRotationZ {
            model: MOUND_MODEL,
            rotation: melee_lb::trigf::atan2f(-normal.x, normal.y),
        });
    }
}

/// ftCo_Bury_Anim (800C0F44): the timer, then the sinking frames; at
/// their end BuryWait (ftCo_800C124C: KeepGfx | SkipMatAnim | SkipColAnim).
pub fn bury_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.tilt_mound(p.map);
    if f.bury_mash(p.assets)? {
        // The sink countdown below then lands in BuryJump's frame counter
        // (mv+0, a float ftCo_800C13BC had just zeroed), as an int
        // decrement: its bits become 0xFFFFFFFF, a NaN that counts no
        // frames, so this jump never reaches its interrupts.
        let MotionData::BuryJump { frames, .. } = &mut f.core.state_data else {
            unreachable!()
        };
        *frames = f32::from_bits(u32::MAX);
        return Ok(None);
    }
    let MotionData::Bury(bury) = &mut f.core.state_data else {
        panic!("bury scratch missing")
    };
    bury.frames_left -= 1;
    if bury.frames_left == 0 {
        let scratch = std::mem::take(&mut f.core.state_data);
        f.change_motion_state_with_flags(
            S::BuryWait.into(),
            p.assets,
            MotionEntryFlags(BURY_FLAGS.0 | MotionEntryFlags::KEEP_GFX.0),
            0.0,
            1.0,
        )?;
        f.core.state_data = scratch;
        f.core.clear_movement();
        f.set_buried_flags();
    }
    Ok(None)
}

/// ftCo_BuryWait_Anim (800C12D4).
pub fn bury_wait_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.tilt_mound(p.map);
    f.bury_mash(p.assets)?;
    Ok(None)
}

/// ftCo_Bury_IASA, ftCo_BuryWait_IASA: empty.
pub fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftCo_Bury_Phys (800C10E4): the box's bottom rises by x1C, stopping one
/// unit below the lower of its side points. Fighter_procUpdate's tail
/// follows, without the stage's wind (x2224_b4).
pub fn bury_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let MotionData::Bury(bury) = &mut f.core.state_data else {
        panic!("bury scratch missing")
    };
    let b = &mut bury.collision_box;
    let side = if b.right.y < b.left.y {
        b.right.y
    } else {
        b.left.y
    };
    b.bottom += bury.sink;
    if 1.0 + b.bottom > side {
        b.bottom = side - 1.0;
    }
    bury_wait_physics(f, p);
}

/// ftCo_BuryWait_Phys is empty: only Fighter_procUpdate's tail.
pub fn bury_wait_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let core = &mut f.core;
    crate::physics::grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &crate::physics::grounded::GroundedParameters::from_attributes(
            &core.attributes,
            &p.assets.common,
        ),
        p.map,
        melee_gr::wind::Wind::CALM,
    );
}

/// ftCo_Bury_Coll (800C1194) / ftCo_BuryWait_Coll: the sunk box on the
/// floor (ft_80082888); off it, on another line, or on a floor the stage
/// marks (Ground_801C5700), the fighter jumps out.
pub fn bury_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let MotionData::Bury(bury) = &f.core.state_data else {
        panic!("bury scratch missing")
    };
    let (floor, collision_box) = (bury.floor, bury.collision_box);
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let supported = ground::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        collision_box,
        false,
    );
    // Ground_801C5700: none of the supported stages marks a floor.
    if !supported || floor != f.core.collision.data.floor.index {
        f.enter_bury_jump(p.assets.expect("bury collision assets"))?;
    }
    Ok(())
}

/// ftCo_BuryJump_Anim (800C1474): the frame counter; Fall at the end.
pub fn bury_jump_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let MotionData::BuryJump { frames, .. } = &mut f.core.state_data else {
        panic!("bury jump scratch missing")
    };
    *frames += 1.0;
    if !f.core.animation.frames_remaining(&f.core.skeleton) {
        f.change_motion_state(S::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// ftCo_BuryJump_IASA (800C14D0): after PlCo +61C frames, the aerial
/// interrupts.
pub fn bury_jump_input(f: &mut Fighter, p: InputPhase<'_>) {
    let MotionData::BuryJump { frames, .. } = f.core.state_data else {
        panic!("bury jump scratch missing")
    };
    if frames >= p.assets.bury.jump_interrupt_frames {
        super::state::callbacks::input::aerial(f, p);
    }
}

/// ftCo_BuryJump_Phys (800C15A0): PlCo gravity to terminal velocity
/// (ftCommon_Fall, no fast fall) and the ordinary drift.
pub fn bury_jump_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let core = &mut f.core;
    let air = &core.attributes.air;
    core.physics.self_velocity.y = crate::physics::airborne::gravity(
        core.physics.self_velocity.y,
        air.gravity,
        air.terminal_velocity,
    );
    core.physics.animation_velocity.x = crate::physics::airborne::drift(
        core.physics.self_velocity.x,
        core.input.current.stick.x,
        air,
    );
    core.finish_air_update(p.assets, p.wind);
}

/// ftCo_BuryJump_Coll (800C15F4): rising, ft_80082D40 (the ECB unlocked,
/// landing into Wait or Landing by the speed); falling,
/// ftCo_AirCatchHit_Coll.
pub fn bury_jump_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if f.core.physics.self_velocity.y < 0.0 {
        return super::state::callbacks::collision::air_catch_hit(f, p);
    }
    let assets = p.assets.expect("bury jump collision assets");
    let core = &mut f.core;
    let cd = &mut core.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = core.physics.position;
    let pose = EcbPose::read(&mut core.skeleton, core.animation.root, cd);
    let landed = p.map.air_collide_ecb18(cd, Some(&|i| pose.position(i)));
    core.physics.position = cd.cur_pos;
    core.skeleton
        .set_translate(core.animation.root, &core.physics.position);
    if landed {
        f.land_from_air(assets)?;
    }
    Ok(())
}
