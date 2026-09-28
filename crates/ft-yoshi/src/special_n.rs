//! Egg Lay, ftyoshispecialn.c (8012CC94..8012DF18): the tongue, the swallow
//! and the egg. The captured fighter's CaptureYoshi and YoshiEgg states are
//! common code (ftCo_CaptureYoshi.c, ftCo_YoshiEgg.c) in melee-ft.
use crate::init::Yoshi;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        capture_yoshi::EggParameters,
        grab::GrabLink,
        ledge::GrabExclusions,
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase},
        ActionId, Fighter, MotionPreservation, MotionRow,
    },
};
use melee_types::CommonMotionState as S;

/// ftYs_MS_SpecialN1 .. ftYs_MS_SpecialAirN2_1 (346..355), in table order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum EggLayState {
    /// Tongue out on the ground.
    Start = 346,
    /// Ground: a fighter is on the tongue.
    PullFighter = 347,
    /// Ground: an item is on the tongue.
    PullItem = 348,
    /// Ground: the fighter is swallowed and laid as an egg.
    SwallowFighter = 349,
    /// Ground: the item is swallowed and laid as an egg.
    SwallowItem = 350,
    /// Tongue out in the air.
    AirStart = 351,
    AirPullFighter = 352,
    AirPullItem = 353,
    AirSwallowFighter = 354,
    AirSwallowItem = 355,
}
impl From<EggLayState> for ActionId {
    fn from(state: EggLayState) -> Self {
        ActionId(state as u16)
    }
}

/// First Yoshi-owned row (ftYs_MS_SpecialN1 - ftCo_MS_Count).
pub const FIRST_ROW: usize = 5;
/// Rows 346..355. The item rows are entered only by the tongue's item
/// catch (fn_8012CEE0 / fn_8012D004), which is not ported.
pub const fn rows() -> [MotionRow; 10] {
    const fn row(
        state: EggLayState,
        animation: i32,
        anim: state::AnimFn,
        airborne: bool,
        collision: state::CollisionFn,
    ) -> MotionRow {
        MotionRow {
            action: ActionId(state as u16),
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
    const fn unported(state: EggLayState) -> MotionRow {
        let mut row = state::unimplemented_row();
        row.action = ActionId(state as u16);
        row
    }
    use EggLayState as E;
    [
        row(E::Start, 295, start_anim, false, start_collision),
        row(E::PullFighter, 296, pull_anim, false, pull_collision),
        unported(E::PullItem),
        row(E::SwallowFighter, 297, swallow_anim, false, swallow_collision),
        unported(E::SwallowItem),
        row(E::AirStart, 298, air_start_anim, true, air_start_collision),
        row(E::AirPullFighter, 299, pull_anim, true, air_pull_collision),
        unported(E::AirPullItem),
        row(E::AirSwallowFighter, 300, swallow_anim, true, air_swallow_collision),
        unported(E::AirSwallowItem),
    ]
}

fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftYs_SpecialN_Enter (8012CDE8) / ftYs_SpecialAirN_Enter (8012CE64).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    f.commands.variables[0] = 0;
    let state = if airborne {
        EggLayState::AirStart
    } else {
        EggLayState::Start
    };
    f.change_motion_state(state.into(), assets)
        .expect("Egg Lay motion assets");
    f.step_animation(assets);
    arm_tongue(f);
}

/// setupCallbacks -> ftCommon_8007E2D0(fp, 4, ...): the tongue's grab
/// category, fn_8012CF7C/fn_8012D0A0 on a fighter, ftCo_800BBB8C for it.
fn arm_tongue(f: &mut Fighter) {
    f.core.status.special_grab = Some(TONGUE_GRAB);
}

/// ftCommon_8007E2D0's category argument for the tongue.
const TONGUE_GRAB: GrabExclusions = GrabExclusions(4);

/// ftYs_SpecialN1_Anim (8012D550) / ftYs_SpecialAirN1_0_Anim (8012D58C).
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::Wait.into(), p.assets)?;
    }
    Ok(None)
}
fn air_start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::Fall.into(), p.assets)?;
    }
    Ok(None)
}

/// fn_8012CF7C (8012CF7C) / fn_8012D0A0 (8012D0A0), the tongue's grab_cb
/// on a fighter (flags 0x81092: KeepGfx, SkipModel, SkipMatAnim,
/// SkipColAnim, Unk19), then ftCo_800BBB8C, the grabbed_cb it installed.
pub fn grab(
    yoshi: &mut Fighter,
    victim: &mut Fighter,
    yoshi_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    let pull = if yoshi.motion_state.action == EggLayState::AirStart.into() {
        EggLayState::AirPullFighter
    } else {
        assert_eq!(yoshi.motion_state.action, EggLayState::Start.into());
        EggLayState::PullFighter
    };
    let frame = yoshi.animation.frame;
    yoshi.change_motion_state_keeping_graphics(pull.into(), yoshi_assets, frame, true)?;
    // x2222_b2 (kept by Unk19) and mv.ys.specialn.x0_b0 (false: a fighter,
    // not an item) have no port reader.
    yoshi.status.grab_exclusions = GrabExclusions::ALL;
    yoshi.clear_movement();
    // ftColl_80078A2C chose the victim (fp->victim_gobj).
    yoshi.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    victim.enter_capture_yoshi(&mut yoshi.core, victim_assets, yoshi_assets)
}

fn holding(f: &Fighter) -> bool {
    matches!(f.combat.grab, Some(GrabLink::Holding { .. }))
}

/// ftYs_SpecialN1_0_Anim (8012D658) / ftYs_SpecialAirN1_1_Anim (8012D760):
/// the script's cmd_vars[0] starts the swallow (flags 0x80012: KeepGfx,
/// SkipModel, Unk19). The pull has no end of its own.
fn pull_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && holding(f) {
        f.commands.variables[0] = 0;
        f.commands.variables[1] = 0;
        let swallow = if f.motion_state.action == EggLayState::AirPullFighter.into() {
            EggLayState::AirSwallowFighter
        } else {
            EggLayState::SwallowFighter
        };
        f.change_motion_state_keeping_graphics(swallow.into(), p.assets, 0.0, false)?;
        f.status.grab_exclusions = GrabExclusions::ALL;
    }
    Ok(None)
}

/// ftYs_SpecialN2_0_Anim (8012D948) / ftYs_SpecialAirN2_0_Anim (8012DB74),
/// inlineA1: cmd_vars[1] hides the swallowed fighter (ftCo_800BBC88),
/// cmd_vars[0] lays it (ftCo_800DE2CC, ftCo_800BBED4). The scene applies
/// both to the pair right after this callback.
fn swallow_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[1] != 0 && holding(f) {
        f.combat.capture_requests.hide = true;
        f.commands.variables[1] = 0;
    }
    if f.commands.variables[0] != 0 && holding(f) {
        f.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.capture_requests.lay_egg = Some(egg_parameters(f));
        f.commands.variables[0] = 0;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        assert!(
            f.combat.capture_requests.lay_egg.is_none(),
            "ftYs_SpecialN2_0_Anim: the egg laid on the swallow's last frame"
        );
        let end = if f.motion_state.action == EggLayState::AirSwallowFighter.into() {
            S::Fall
        } else {
            S::Wait
        };
        f.change_motion_state(end.into(), p.assets)?;
    }
    Ok(None)
}

/// The ftYs_SpecialN_Get* accessors ftCo_800BBED4 and ftCo_YoshiEgg_Anim read.
fn egg_parameters(f: &Fighter) -> EggParameters {
    let a = &f.character.get::<Yoshi>().attributes.egg_lay;
    let facing = f.physics.facing;
    EggParameters {
        // ftYs_SpecialN_SetupItemVel (8012CC94): fneg, then fmuls.
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
        // ftYs_SpecialN_8012CDB4 (8012CDB4): fdivs.
        damage_ratio: a.damage_frame_reduction / a.damage_behavior,
    }
}

/// ftCommon_GroundToAirStateChange / AirToGroundStateChange with
/// ftYs_MF_SpecialN_Coll (KeepGfx) or _CollHit (also SkipHit).
fn change_ground_air(
    f: &mut Fighter,
    assets: &FighterAssets,
    state: EggLayState,
    keep_hitboxes: bool,
) -> Result<()> {
    f.change_ground_air_motion(
        state.into(),
        assets,
        MotionPreservation {
            hitboxes: keep_hitboxes,
            effects: true,
            ..Default::default()
        },
    )
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

/// ftYs_SpecialN1_Coll -> fn_8012D128: the tongue stays armed in the air.
fn start_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::AirStart, true)?;
        arm_tongue(f);
        Ok(())
    })
}
/// ftYs_SpecialAirN1_0_Coll -> fn_8012D1AC.
fn air_start_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::Start, true)?;
        arm_tongue(f);
        Ok(())
    })
}
/// ftYs_SpecialN1_0_Coll -> fn_8012D298.
fn pull_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::AirPullFighter, false)
    })
}
/// ftYs_SpecialAirN1_1_Coll -> fn_8012D360.
fn air_pull_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::PullFighter, false)
    })
}
/// ftYs_SpecialN2_0_Coll -> fn_8012D428.
fn swallow_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    ground_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::AirSwallowFighter, false)
    })
}
/// ftYs_SpecialAirN2_0_Coll -> fn_8012D4F0.
fn air_swallow_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    air_collision(f, p, |f, a| {
        change_ground_air(f, a, EggLayState::SwallowFighter, false)
    })
}
