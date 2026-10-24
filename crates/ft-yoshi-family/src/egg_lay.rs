//! Egg Lay, ftyoshispecialn.c (8012CC94..8012DF18), and Kirby's copy of it
//! (ftkirbyspecialyoshi.c, 80109260..8010A8BC): the tongue, the swallow and
//! the egg, the same states over either user's rows, generic over
//! [`EggLayUser`]. The captured fighter's CaptureYoshi / CaptureKirbyYoshi
//! and YoshiEgg / KirbyYoshiEgg states are common code
//! (`melee_ft::fighter::capture_yoshi`).
use hsd_archive::Reader;
use hsd_types::{Vec2, Vec3};
use melee_ft::{
    anim::WaitChoice,
    desc::FighterDescError,
    fighter::{
        assets::{FighterAssets, Result},
        capture_yoshi::{EggLayer, EggParameters},
        grab::GrabLink,
        ledge::GrabExclusions,
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase},
        ActionId, Fighter, MotionPreservation, MotionRow,
    },
};
use melee_types::CommonMotionState as S;

/// The ten rows in table order, from the user's first (Yoshi's 346,
/// Kirby's 413).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum EggLayState {
    /// Tongue out on the ground.
    Start = 0,
    /// Ground: a fighter is on the tongue.
    PullFighter = 1,
    /// Ground: an item is on the tongue.
    PullItem = 2,
    /// Ground: the fighter is swallowed and laid as an egg.
    SwallowFighter = 3,
    /// Ground: the item is swallowed and laid as an egg.
    SwallowItem = 4,
    /// Tongue out in the air.
    AirStart = 5,
    AirPullFighter = 6,
    AirPullItem = 7,
    AirSwallowFighter = 8,
    AirSwallowItem = 9,
}

/// The Egg Lay block of the user's attributes (ftYoshiAttributes
/// +0x10..+0x44, ftKb_DatAttrs +3AC..+3E0 `specialn_ys_*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EggLayAttributes {
    /// +0x10: the egg's launch speed away from the captor.
    pub horizontal_speed: f32,
    /// +0x14: its upward launch speed.
    pub vertical_speed: f32,
    /// +0x18: dmg.x182c_behavior while in the egg.
    pub damage_behavior: f32,
    /// +0x1C: the share of the egg's scale that grows in.
    pub wobble_parameter: f32,
    /// +0x20: the frames over which it grows.
    pub capture_parameter: f32,
    /// +0x24: the escape timer.
    pub duration: f32,
    /// +0x28: the timer's decrement per frame.
    pub escape_frames_per_tick: f32,
    /// +0x2C: ftCommon_GrabMash's decrement per input.
    pub mash_frame_reduction: f32,
    /// +0x30: the mashing animation's hold frames.
    pub mash_animation_duration: f32,
    /// +0x34: the mashing animation's rate.
    pub mash_animation_rate: f32,
    /// +0x38: ftColl_8007B760's intangibility after breaking out.
    pub exit_intangibility_frames: i32,
    /// +0x3C: the velocity out of the shell.
    pub release_velocity: Vec2,
    /// +0x44: the timer a hit's damage takes, over +0x18.
    pub damage_frame_reduction: f32,
}

impl EggLayAttributes {
    /// The block at `base`.
    pub fn read(r: &Reader<'_>, base: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            horizontal_speed: r.f32(base)?,
            vertical_speed: r.f32(base + 0x04)?,
            damage_behavior: r.f32(base + 0x08)?,
            wobble_parameter: r.f32(base + 0x0C)?,
            capture_parameter: r.f32(base + 0x10)?,
            duration: r.f32(base + 0x14)?,
            escape_frames_per_tick: r.f32(base + 0x18)?,
            mash_frame_reduction: r.f32(base + 0x1C)?,
            mash_animation_duration: r.f32(base + 0x20)?,
            mash_animation_rate: r.f32(base + 0x24)?,
            exit_intangibility_frames: r.s32(base + 0x28)?,
            release_velocity: Vec2::new(r.f32(base + 0x2C)?, r.f32(base + 0x30)?),
            damage_frame_reduction: r.f32(base + 0x34)?,
        })
    }
}

/// What Egg Lay's states ask of their user.
pub trait EggLayUser {
    /// The grounded start; the other nine rows follow in table order.
    const FIRST: u16;
    /// The animations of the six fighter rows: Start, PullFighter,
    /// SwallowFighter, AirStart, AirPullFighter, AirSwallowFighter.
    const ANIMATIONS: [i32; 6];
    /// Whose egg the victim is laid in.
    const LAYER: EggLayer;
    /// The Egg Lay attributes.
    fn attributes(f: &Fighter) -> EggLayAttributes;
    /// take_dmg_cb / death2_cb, which Kirby's code installs with every row
    /// (ftKb_Init_800EE74C / 800EE7B8); Yoshi's installs none.
    fn arm_callbacks(f: &mut Fighter);
}

const fn action<C: EggLayUser>(state: EggLayState) -> ActionId {
    ActionId(C::FIRST + state as u16)
}

fn at<C: EggLayUser>(f: &Fighter, state: EggLayState) -> bool {
    f.motion_state.action == action::<C>(state)
}

/// The ten rows. The item rows are entered only by the tongue's item catch
/// (fn_8012CEE0 / fn_8012D004, Kirby's fn_801095DC / fn_80109714), which is
/// not ported.
pub const fn rows<C: EggLayUser>() -> [MotionRow; 10] {
    const fn row<C: EggLayUser>(
        state: EggLayState,
        animation: i32,
        anim: state::AnimFn,
        airborne: bool,
        collision: state::CollisionFn,
    ) -> MotionRow {
        MotionRow {
            action: action::<C>(state),
            id: S::None,
            animation,
            anim,
            iasa: no_input,
            physics: if airborne {
                callbacks::physics::air_friction
            } else {
                callbacks::physics::guard_on
            },
            collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        }
    }
    const fn unported<C: EggLayUser>(state: EggLayState) -> MotionRow {
        let mut row = state::unimplemented_row();
        row.action = action::<C>(state);
        row
    }
    use EggLayState as E;
    let a = C::ANIMATIONS;
    [
        row::<C>(E::Start, a[0], start_anim, false, start_collision::<C>),
        row::<C>(E::PullFighter, a[1], pull_anim::<C>, false, pull_collision::<C>),
        unported::<C>(E::PullItem),
        row::<C>(E::SwallowFighter, a[2], swallow_anim::<C>, false, swallow_collision::<C>),
        unported::<C>(E::SwallowItem),
        row::<C>(E::AirStart, a[3], air_start_anim, true, air_start_collision::<C>),
        row::<C>(E::AirPullFighter, a[4], pull_anim::<C>, true, air_pull_collision::<C>),
        unported::<C>(E::AirPullItem),
        row::<C>(E::AirSwallowFighter, a[5], swallow_anim::<C>, true, air_swallow_collision::<C>),
        unported::<C>(E::AirSwallowItem),
    ]
}

fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftYs_SpecialN_Enter (8012CDE8) / ftYs_SpecialAirN_Enter (8012CE64);
/// Kirby's ftKb_SpecialNYs_8010941C / 801094FC.
pub fn enter<C: EggLayUser>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.variables[0] = 0;
    let state = if airborne {
        EggLayState::AirStart
    } else {
        EggLayState::Start
    };
    f.change_motion_state(action::<C>(state), assets)
        .expect("Egg Lay motion assets");
    f.step_animation(assets);
    // Kirby's entries then start the hat's animation (hat_dynamics[1] /
    // [3]), which only draws.
    arm_tongue(f);
    C::arm_callbacks(f);
}

/// setupCallbacks -> ftCommon_8007E2D0(fp, 4, ...): the tongue's grab
/// category, the fighter and item catches, the victim's grabbed_cb.
fn arm_tongue(f: &mut Fighter) {
    f.core.status.special_grab = Some(TONGUE_GRAB);
}

/// ftCommon_8007E2D0's category argument for the tongue.
const TONGUE_GRAB: GrabExclusions = GrabExclusions(4);

/// ftYs_SpecialN1_Anim (8012D550) / Kirby's ftKb_YsSpecialN1_Anim: Wait
/// at the end.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::Wait.into(), p.assets)?;
    }
    Ok(None)
}
/// ftYs_SpecialAirN1_0_Anim (8012D58C) / Kirby's
/// ftKb_YsSpecialAirNCapture2_Anim: Fall at the end.
fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// fn_8012CF7C (8012CF7C) / fn_8012D0A0 (8012D0A0), Kirby's fn_80109680 /
/// fn_801097B8: the tongue's grab_cb on a fighter (flags 0x81092: KeepGfx,
/// SkipModel, SkipMatAnim, SkipColAnim, Unk19), then the grabbed_cb it
/// installed (ftCo_800BBB8C / fn_8010A930).
pub fn grab<C: EggLayUser>(
    captor: &mut Fighter,
    victim: &mut Fighter,
    captor_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    let pull = if at::<C>(captor, EggLayState::AirStart) {
        EggLayState::AirPullFighter
    } else {
        assert!(at::<C>(captor, EggLayState::Start));
        EggLayState::PullFighter
    };
    let frame = captor.animation.frame;
    captor.change_motion_state_keeping_graphics(action::<C>(pull), captor_assets, frame, true)?;
    // x2222_b2 (kept by Unk19) and mv.ys.specialn.x0_b0 (false: a fighter,
    // not an item) have no port reader.
    captor.status.grab_exclusions = GrabExclusions::ALL;
    captor.clear_movement();
    C::arm_callbacks(captor);
    // ftColl_80078A2C chose the victim (fp->victim_gobj).
    captor.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    victim.enter_capture_yoshi(&mut captor.core, victim_assets, captor_assets, C::LAYER)
}

fn holding(f: &Fighter) -> bool {
    matches!(f.combat.grab, Some(GrabLink::Holding { .. }))
}

/// ftYs_SpecialN1_0_Anim (8012D658) / ftYs_SpecialAirN1_1_Anim (8012D760);
/// Kirby's ftKb_YsSpecialNCapture1_Anim / AirCapture1_Anim: the script's
/// cmd_vars[0] starts the swallow (flags 0x80012: KeepGfx, SkipModel,
/// Unk19). The pull has no end of its own.
fn pull_anim<C: EggLayUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && holding(f) {
        f.commands.variables[0] = 0;
        f.commands.variables[1] = 0;
        let swallow = if at::<C>(f, EggLayState::AirPullFighter) {
            EggLayState::AirSwallowFighter
        } else {
            EggLayState::SwallowFighter
        };
        f.change_motion_state_keeping_graphics(action::<C>(swallow), p.assets, 0.0, false)?;
        // Kirby's swallow starts the hat's hat_dynamics[2] / [4], which
        // only draws.
        C::arm_callbacks(f);
        f.status.grab_exclusions = GrabExclusions::ALL;
    }
    Ok(None)
}

/// ftYs_SpecialN2_0_Anim (8012D948) / ftYs_SpecialAirN2_0_Anim (8012DB74),
/// inlineA1; Kirby's ftKb_YsSpecialNCapture2_1_Anim / AirN2_0_Anim:
/// cmd_vars[1] hides the swallowed fighter (ftCo_800BBC88 /
/// ftKb_SpecialNYs_8010AA2C), cmd_vars[0] lays it (ftCo_800DE2CC, then
/// ftCo_800BBED4 / 8010AC78). The scene applies both to the pair right
/// after this callback.
fn swallow_anim<C: EggLayUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[1] != 0 && holding(f) {
        f.combat.capture_requests.hide = true;
        f.commands.variables[1] = 0;
    }
    if f.commands.variables[0] != 0 && holding(f) {
        f.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.capture_requests.lay_egg = Some(egg_parameters::<C>(f));
        f.commands.variables[0] = 0;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        assert!(
            f.combat.capture_requests.lay_egg.is_none(),
            "ftYs_SpecialN2_0_Anim: the egg laid on the swallow's last frame"
        );
        let end = if at::<C>(f, EggLayState::AirSwallowFighter) {
            S::Fall
        } else {
            S::Wait
        };
        f.change_motion_state(end.into(), p.assets)?;
    }
    Ok(None)
}

/// The ftYs_SpecialN_Get* accessors (Kirby's ftKb_SpecialNYs_80109260 ..
/// 80109380) the egg reads.
fn egg_parameters<C: EggLayUser>(f: &Fighter) -> EggParameters {
    let a = C::attributes(f);
    let facing = f.physics.facing;
    EggParameters {
        layer: C::LAYER,
        // ftYs_SpecialN_SetupItemVel (8012CC94) / 80109260: fneg, then fmuls.
        launch_velocity: Vec3::new(-facing * a.horizontal_speed, a.vertical_speed, 0.0),
        facing,
        damage_behavior: a.damage_behavior,
        growth: a.wobble_parameter,
        growth_frames: a.capture_parameter,
        duration: a.duration,
        decrement: a.escape_frames_per_tick,
        mash_decrement: a.mash_frame_reduction,
        fast_frames: a.mash_animation_duration,
        fast_rate: a.mash_animation_rate,
        exit_intangibility: a.exit_intangibility_frames,
        release_velocity: a.release_velocity,
        // ftYs_SpecialN_8012CDB4 (8012CDB4) / 80109380: fdivs.
        damage_ratio: a.damage_frame_reduction / a.damage_behavior,
    }
}

/// ftCommon_GroundToAirStateChange / AirToGroundStateChange with
/// ftYs_MF_SpecialN_Coll (KeepGfx) or _CollHit (also SkipHit).
fn change_ground_air<C: EggLayUser>(
    f: &mut Fighter,
    assets: &FighterAssets,
    state: EggLayState,
    keep_hitboxes: bool,
) -> Result<()> {
    f.change_ground_air_motion(
        action::<C>(state),
        assets,
        MotionPreservation {
            hitboxes: keep_hitboxes,
            effects: true,
            ..Default::default()
        },
    )?;
    C::arm_callbacks(f);
    Ok(())
}

/// ft_8008403C: ordinary ground collision, `leave` on departure.
fn ground_collision(
    f: &mut Fighter,
    p: CollisionPhase<'_>,
    leave: fn(&mut Fighter, &FighterAssets) -> Result<()>,
) -> Result<()> {
    use melee_ft::collision::ground::{map_ground_action, WaitGroundResult};
    let c = &mut f.core;
    if map_ground_action(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.input.current.stick.x,
    ) != WaitGroundResult::Supported
    {
        f.leave_ground();
        leave(f, p.assets.expect("Egg Lay collision assets"))?;
    }
    Ok(())
}

/// ft_80082C74: ordinary air collision, `land` on landing.
fn air_collision(
    f: &mut Fighter,
    p: CollisionPhase<'_>,
    land: fn(&mut Fighter, &FighterAssets) -> Result<()>,
) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        f.land();
        land(f, p.assets.expect("Egg Lay landing assets"))?;
    }
    Ok(())
}

/// ftYs_SpecialN1_Coll -> fn_8012D128 (Kirby's fn_8010984C): the tongue
/// stays armed in the air.
fn start_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::AirStart, true)?;
        arm_tongue(f);
        Ok(())
    })
}
/// ftYs_SpecialAirN1_0_Coll -> fn_8012D1AC (Kirby's fn_801098EC).
fn air_start_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::Start, true)?;
        arm_tongue(f);
        Ok(())
    })
}
/// ftYs_SpecialN1_0_Coll -> fn_8012D298 (Kirby's fn_80109A08).
fn pull_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::AirPullFighter, false)
    })
}
/// ftYs_SpecialAirN1_1_Coll -> fn_8012D360 (Kirby's fn_80109B00).
fn air_pull_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::PullFighter, false)
    })
}
/// ftYs_SpecialN2_0_Coll -> fn_8012D428 (Kirby's fn_80109BF8).
fn swallow_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::AirSwallowFighter, false)
    })
}
/// ftYs_SpecialAirN2_0_Coll -> fn_8012D4F0 (Kirby's fn_80109CF0).
fn air_swallow_collision<C: EggLayUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air::<C>(f, a, EggLayState::SwallowFighter, false)
    })
}

/// Whether `f` is in one of the user's ten rows.
pub fn in_rows<C: EggLayUser>(f: &Fighter) -> bool {
    (C::FIRST..C::FIRST + 10).contains(&f.motion_state.action.0)
}
