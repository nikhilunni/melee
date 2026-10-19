//! Shield Breaker / Flare Blade: ftmarsspecialn.c (80136800..8013741C,
//! Marth and Roy) and ftkirbyspecialmars.c (8010B1F4..8010C1E8, Kirby's
//! copied one: the same states over Kirby's rows). The two files differ in
//! what [`ShieldBreaker`] names: the rows, the attributes, the release's
//! effect, the charge flash, and the hips' gusts, which only Marth's and
//! Roy's code makes.
use crate::{attributes::ShieldBreakerAttributes, MarsFamily};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionRow,
    },
    input::Buttons,
    physics::{friction, grounded},
};

#[derive(Clone, Debug, Default)]
pub struct SpecialN {
    /// Fighter +2340, mv.ms.specialn.cur_frame (mv.kb.specialn_ms for
    /// Kirby): accumulated charge ticks.
    pub charge_ticks: i32,
    /// accessory4_cb = ftMs_SpecialN_801365A8 (fn_8010B1F4 for Kirby).
    pub pending_effect: bool,
}

/// The eight Shield Breaker rows of one user, in table order.
#[derive(Clone, Copy, Debug)]
pub struct ShieldBreakerRows {
    /// SpecialNStart; Loop, End0, End1 and the aerial four follow.
    pub first_action: u16,
    /// SpecialNStart's submotion; the others follow.
    pub first_animation: i32,
}
impl ShieldBreakerRows {
    pub const fn action(&self, row: ShieldBreakerRow) -> ActionId {
        ActionId(self.first_action + row as u16)
    }
    /// Which of the eight `action` is.
    fn row(&self, action: ActionId) -> ShieldBreakerRow {
        let index = action.0.wrapping_sub(self.first_action);
        assert!(index < 8, "non-Shield Breaker transition");
        ShieldBreakerRow::ALL[usize::from(index)]
    }
}

/// ftMs_MS_SpecialNStart.. and ftKb_MS_MsSpecialNStart.. /
/// ftKb_MS_FeSpecialNStart.., in table order.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShieldBreakerRow {
    Start,
    Loop,
    End0,
    End1,
    AirStart,
    AirLoop,
    AirEnd0,
    AirEnd1,
}
impl ShieldBreakerRow {
    const ALL: [Self; 8] = [
        Self::Start,
        Self::Loop,
        Self::End0,
        Self::End1,
        Self::AirStart,
        Self::AirLoop,
        Self::AirEnd0,
        Self::AirEnd1,
    ];
    const fn airborne(self) -> bool {
        self as u16 >= Self::AirStart as u16
    }
    /// 0 Start, 1 Loop, 2 and 3 the two ends.
    const fn phase(self) -> u16 {
        self as u16 % 4
    }
    const fn is_end(self) -> bool {
        self.phase() >= 2
    }
    /// The same row on the other side of the floor.
    const fn counterpart(self) -> Self {
        Self::ALL[(self as usize + 4) % 8]
    }
}

/// Who swings a Shield Breaker: Marth and Roy (every [`MarsFamily`]) and
/// Kirby with either one's copied ability. Kirby's two kinds are marker
/// types whose methods reach into Kirby's own state.
pub trait ShieldBreaker: 'static {
    const ROWS: ShieldBreakerRows;
    /// ftMs_SpecialN_801365A8 / 8013666C and fn_8010B1F4: the release's
    /// effect, grounded and aerial.
    const RELEASE_EFFECTS: [u16; 2];
    /// The Start rows' ftCo_800BFFD0 color animation as the charge loop
    /// starts (`kind == FTKIND_MARS ? 99 : 100`).
    const CHARGE_COLOR_ANIMATION: u8;
    /// lb_800119DC at the hips (ftmarsspecialn.c doLoopAnim and inlineA0);
    /// ftkirbyspecialmars.c has no gusts.
    const GUSTS: bool;
    fn attributes(f: &Fighter) -> ShieldBreakerAttributes;
    /// The charge counter and the pending accessory4.
    fn scratch(f: &mut Fighter) -> &mut SpecialN;
    fn scratch_ref(f: &Fighter) -> &SpecialN;
    /// Before the entry's motion change: Marth's and Roy's specials retain
    /// the mv+4 word (`crate::retain_scratch_word`).
    fn before_entry(_f: &mut Fighter) {}
}

/// ftMs_Init_MotionStateTable's first eight rows, submotions 295..302.
pub const MARS_FAMILY_ROWS: ShieldBreakerRows = ShieldBreakerRows {
    first_action: crate::FIRST_SPECIAL,
    first_animation: 295,
};

/// Marth's and Roy's own: `fp->kind` picks the effect and the flash (the
/// family's constants).
impl<C: MarsFamily> ShieldBreaker for C {
    const ROWS: ShieldBreakerRows = MARS_FAMILY_ROWS;
    const RELEASE_EFFECTS: [u16; 2] = <C as MarsFamily>::RELEASE_EFFECTS;
    const CHARGE_COLOR_ANIMATION: u8 = <C as MarsFamily>::CHARGE_COLOR_ANIMATION;
    const GUSTS: bool = true;
    fn attributes(f: &Fighter) -> ShieldBreakerAttributes {
        f.character.get::<C>().attributes().shield_breaker
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialN {
        &mut f.character.get_mut::<C>().specials().special_n
    }
    fn scratch_ref(f: &Fighter) -> &SpecialN {
        &f.character.get::<C>().specials_ref().special_n
    }
    fn before_entry(f: &mut Fighter) {
        crate::retain_scratch_word::<C>(f);
    }
}

/// The eight rows for `C`: Start, Loop, End0, End1, then the aerial four.
pub const fn rows<C: ShieldBreaker>() -> [MotionRow; 8] {
    let mut rows = [melee_ft::fighter::state::unimplemented_row(); 8];
    let mut i = 0;
    while i < 8 {
        let row = ShieldBreakerRow::ALL[i];
        rows[i] = MotionRow {
            action: C::ROWS.action(row),
            id: melee_types::CommonMotionState::None,
            animation: C::ROWS.first_animation + i as i32,
            anim: match row.phase() {
                0 => start::<C>,
                1 => hold::<C>,
                _ => end::<C>,
            },
            iasa: if row.phase() == 1 {
                input::<C>
            } else {
                no_input
            },
            // ftMs_SpecialAirN*_Phys / ftKb_MsSpecialAirN*_Phys; the
            // grounded Start's own; ft_80084F3C.
            physics: if row.airborne() {
                air_physics::<C>
            } else if row.phase() == 0 {
                startup_physics::<C>
            } else {
                callbacks::physics::guard_on
            },
            collision: if row.airborne() {
                air_collision::<C>
            } else {
                collision::<C>
            },
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    rows
}

fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}

fn airborne(f: &Fighter) -> bool {
    f.physics.ground_or_air == melee_types::GroundOrAir::Air
}

/// lb_800119DC's arguments for the charge loop's gust (80136B50..60).
const CHARGE_GUST: Gust = Gust {
    frames: 10,
    strength: 0.5,
    decay: 0.05,
};
/// The release's gust (80136FC8..D8), on this animation frame.
const RELEASE_GUST: Gust = Gust {
    frames: 120,
    strength: 0.9,
    decay: 0.02,
};
const RELEASE_GUST_FRAME: f32 = 9.0;

struct Gust {
    frames: i32,
    strength: f32,
    decay: f32,
}

/// lb_800119DC at the HipN joint (lb_8000B1CC with no offset): a radial
/// field that pushes nearby dynamics chains, phase step MTXDegToRad(60).
fn gust(f: &mut Fighter, a: &FighterAssets, gust: Gust) {
    let hip = a
        .parts
        .joint(melee_types::FtPart::HipN)
        .expect("Shield Breaker hip") as usize;
    let c = &mut f.core;
    let center = melee_ft::fighter::caches::bone_position(
        &mut c.skeleton,
        c.animation.root,
        hip,
        hsd_types::Vec3::ZERO,
    );
    c.commands
        .gusts
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: gust.frames,
            strength: gust.strength,
            decay: gust.decay,
            phase_step: (std::f64::consts::PI / 3.0) as f32,
        });
}

/// ftMs_SpecialN_Enter / ftMs_SpecialAirN_Enter and Kirby's
/// ftKb_SpecialNMs_8010B2FC / 8010B4A0: the momentum divided, the scratch
/// reset (x21EC: ftMs_SpecialN_80136730 / fn_8010B2E8, run by the motion
/// change) and the Start row.
pub fn enter<C: ShieldBreaker>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let divisor = C::attributes(f).momentum_divisor;
    if air {
        f.physics.self_velocity.x /= divisor;
        if f.physics.self_velocity.y <= 0.0 {
            f.physics.self_velocity.y = 0.0;
        }
    } else {
        f.physics.ground_velocity /= divisor;
    }
    f.commands.variables[0] = 0;
    *C::scratch(f) = Default::default();
    C::before_entry(f);
    let row = if air {
        ShieldBreakerRow::AirStart
    } else {
        ShieldBreakerRow::Start
    };
    f.change_motion_state(C::ROWS.action(row), a)
        .expect("Shield Breaker assets");
    f.step_animation(a);
}
pub fn start<C: ShieldBreaker>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let row = if airborne(f) {
            ShieldBreakerRow::AirLoop
        } else {
            ShieldBreakerRow::Loop
        };
        f.change_motion_state(C::ROWS.action(row), p.assets)?;
        f.commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: C::CHARGE_COLOR_ANIMATION,
                duration: 0,
            });
    }
    Ok(None)
}
pub fn hold<C: ShieldBreaker>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let maximum = C::attributes(f).maximum_charge_levels * 30;
    // doLoopAnim: every 30 charge ticks a small gust leaves the hips.
    if C::GUSTS && C::scratch_ref(f).charge_ticks % 30 == 0 {
        gust(f, p.assets, CHARGE_GUST);
    }
    let s = C::scratch(f);
    s.charge_ticks += 1;
    if s.charge_ticks > maximum {
        release::<C>(f, true, p.assets)?;
    }
    Ok(None)
}
/// ftMs_SpecialN_80137354 / 801373B8 and ftKb_SpecialNPe_8010C148 /
/// 8010C1E8: End0 (released) or End1 (full), from frame 1, with the
/// release's effect as accessory4.
fn release<C: ShieldBreaker>(f: &mut Fighter, full: bool, a: &FighterAssets) -> Result<()> {
    let row = match (airborne(f), full) {
        (false, false) => ShieldBreakerRow::End0,
        (false, true) => ShieldBreakerRow::End1,
        (true, false) => ShieldBreakerRow::AirEnd0,
        (true, true) => ShieldBreakerRow::AirEnd1,
    };
    f.change_motion_state_at(C::ROWS.action(row), a, 1.0)?;
    f.commands.variables[0] = u32::from(full);
    C::scratch(f).pending_effect = true;
    f.arm_accessory4();
    Ok(())
}
pub fn input<C: ShieldBreaker>(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        release::<C>(f, false, p.assets).expect("Shield Breaker release");
    }
}
pub fn end<C: ShieldBreaker>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] == 0 {
        let a = C::attributes(f);
        let charge_ticks = C::scratch_ref(f).charge_ticks;
        let damage = a.base_damage + charge_ticks / 30 * a.damage_per_level;
        crate::set_enabled_hit_damage(f, damage as u32);
    }
    // inlineA0 (80136F94): the release's gust on animation frame 9.
    if C::GUSTS && f.animation.frame == RELEASE_GUST_FRAME {
        gust(f, p.assets, RELEASE_GUST);
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if airborne(f) {
            melee_types::CommonMotionState::Fall
        } else {
            melee_types::CommonMotionState::Wait
        };
        f.change_motion_state(state.into(), p.assets)?;
    }
    Ok(None)
}
pub fn startup_physics<C: ShieldBreaker>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.physics.ground_acceleration =
        friction::friction_acceleration(f.physics.ground_velocity, C::attributes(f).friction);
    let c = &mut f.core;
    grounded::apply_ground_movement(
        &mut c.physics,
        c.collision.data.floor.normal,
        p.map.floor_speed_scale(&c.collision.data),
    );
    grounded::finish_ground_update(
        &mut c.physics,
        &c.collision.data,
        &grounded::GroundedParameters::from_attributes(&c.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}
pub fn collision<C: ShieldBreaker>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    if !matches!(
        melee_ft::collision::ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        melee_ft::collision::ground::WaitGroundResult::Supported
    ) {
        let row = C::ROWS.row(f.motion_state.action);
        // ftCommon_GroundToAirStateChange -> ftCommon_8007D5D4: one jump spent.
        f.leave_ground();
        f.change_ground_air_motion(
            C::ROWS.action(row.counterpart()),
            p.assets.expect("Shield Breaker collision assets"),
            preservation(row),
        )?;
        keep_hitlag_callbacks(f, row);
    }
    Ok(())
}
/// ftMs_SpecialN_801371FC / 801372A8 (ftKb_SpecialNPe_8010BF90 / 8010C06C):
/// the end states' ground and air counterparts install the effect hitlag
/// callbacks again once the trail is out (x2219_b0); the motion change
/// removed them.
fn keep_hitlag_callbacks(f: &mut Fighter, row: ShieldBreakerRow) {
    if row.is_end() && f.effect_state.destroy_on_state_change {
        f.effect_state.hitlag_callbacks = true;
    }
}
/// accessory4_cb: ftMs_SpecialN_801365A8 / 8013666C, fn_8010B1F4.
pub fn accessory<C: ShieldBreaker>(f: &mut Fighter, _: &FighterAssets) {
    let pending = std::mem::take(&mut C::scratch(f).pending_effect);
    if f.run_accessory4(pending) {
        let [ground, air] = C::RELEASE_EFFECTS;
        let id = if airborne(f) { air } else { ground };
        // At `fp->parts[FtPart_TopN]`, bone 0.
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttached { id, bone: 0 });
        f.effect_state.destroy_on_state_change = true;
        // Fighter_SetEffectHitlagCallbacks, so the trail's model waits out
        // the swing's hitlag.
        f.effect_state.hitlag_callbacks = true;
    }
}

/// ftMs_SpecialAirNStart_Phys / Loop_Phys / End_Phys: gravity and friction.
pub fn air_physics<C: ShieldBreaker>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    use melee_ft::physics::{airborne, integrate};
    let friction = if C::ROWS.row(f.motion_state.action) == ShieldBreakerRow::AirStart {
        C::attributes(f).friction
    } else {
        f.attributes.air.aerial_friction
    };
    f.physics.self_velocity.y = airborne::gravity(
        f.physics.self_velocity.y,
        f.attributes.air.gravity,
        f.attributes.air.terminal_velocity,
    );
    // ftCommon_ApplyFrictionAir (8007CE94): inclusive magnitude comparison.
    let x = f.physics.self_velocity.x;
    f.physics.animation_velocity.x = if friction.abs() >= x.abs() {
        -x
    } else if x > 0.0 {
        -friction
    } else {
        friction
    };
    f.decay_air_knockback(p.assets);
    integrate::integrate_velocity(&mut f.physics);
    integrate::integrate_environment(&mut f.physics, None, p.wind);
}
/// ftMs_SpecialAirN*_Coll -> ft_80081D0C; preserved ground counterpart.
pub fn air_collision<C: ShieldBreaker>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        let row = C::ROWS.row(f.motion_state.action);
        f.land();
        f.change_ground_air_motion(
            C::ROWS.action(row.counterpart()),
            p.assets.expect("Shield Breaker landing assets"),
            preservation(row),
        )?;
        keep_hitlag_callbacks(f, row);
    }
    Ok(())
}

/// The ground/air changes' flags: Start 0x0C4C5084, Loop 0x0C4C5A86, the
/// ends 0x0C4C508E (Kirby's add 0x2000, Ft_MF_KeepAccessory, for the
/// sword he holds: drawn only).
fn preservation(row: ShieldBreakerRow) -> melee_ft::fighter::MotionPreservation {
    let phase = row.phase();
    melee_ft::fighter::MotionPreservation {
        hit_status: true,
        hitboxes: phase >= 2,
        effects: phase != 0,
        ..Default::default()
    }
}
