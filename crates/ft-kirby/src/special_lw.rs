//! Stone, ftkirbyspeciallw.c (800F331C..800F5398).
//!
//! Three rows on the ground (393 the change, 394 the rock, 395 the change
//! back) and three in the air (396..398). The start's script asks for the
//! rock (cmd_vars[0] = 1): the next form is drawn (`HSD_Randi(4)` past the
//! last one, so never the same twice), the shell takes the attribute's
//! health (`dmg.x1834`) and the model flickers into the rock over 22
//! frames. The rock row holds its animation still. While it lasts, hits
//! come off the shell (`x221C_b4`, melee-ft's `through_shell`) and Kirby
//! does not react; a hit that breaks it reaches him with what is left.
//!
//! The rock ends when its timer runs out, or on B once the minimum time is
//! up. The grounded end leaves the floor (ftCommon_8007D5D4): both end rows
//! are airborne, move on their TransN and finish in Fall.
//!
//! In the air the rock falls at the attribute's speed. Its landing shakes
//! the screen and continues in the grounded row; a grounded rock that
//! loses its floor continues in the aerial one. On the ground it slides
//! down a slope steeper than the attribute's angle, by the floor's normal
//! averaged over four frames.
use crate::{common, init::Kirby};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{
            callbacks::physics, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase,
        },
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
    physics::grounded,
};
use melee_lb::trigf::atan2f;
use melee_mp::CollMap;

/// ftKb_MS_SpecialLw1, SpecialLw, SpecialLwEnd (393..395) and
/// ftKb_MS_SpecialAirLwStart, SpecialAirLw, SpecialAirLwEnd (396..398).
pub const START: ActionId = ActionId(393);
pub const ROCK: ActionId = ActionId(394);
pub const END: ActionId = ActionId(395);
pub const AIR_START: ActionId = ActionId(396);
pub const AIR_ROCK: ActionId = ActionId(397);
pub const AIR_END: ActionId = ActionId(398);

/// 0x04440012, into the rock and out of it: KeepGfx, SkipModel,
/// SkipItemVis, SkipModelPartVis and SkipModelFlags.
const STAGE_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0444_0012);
/// 0x0C4C5092, a ground/air counterpart: ftCommon_GroundAirColl_MF with
/// KeepGfx and SkipModel.
const COUNTERPART_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5092);
/// 0x0C4C1092, the grounded rock into the falling one: that without
/// UpdateCmd.
const ROCK_FALL_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_1092);

/// `fp->parts[FtPart_YRotN]`: the joint both puffs follow.
const PUFF_JOINT: usize = 3;
/// efAlt 0x49B / 0x49C: hsd_8039EFAC(0, 5, 0x138F / 0x1395, jobj), the
/// puff as Kirby changes and as he changes back.
const CHANGE_PUFF: u16 = 0x49B;
const RETURN_PUFF: u16 = 0x49C;

/// ftKb_Init_803CB490: whether the rock shows on each frame of the change.
const FLICKER: [bool; 23] = [
    false, true, false, false, false, false, true, true, false, false, false, true, true, true,
    false, false, true, true, true, true, false, true, true,
];
/// ftKb_SpecialHi_800F331C plays the first 22 steps (`x14 < 0x16`).
const FLICKER_STEPS: i32 = 0x16;
/// ftKb_Init_803CB4EC: model group 0's selection for each form, 1..=5.
const FORM_MODELS: [i32; 6] = [0, 2, 3, 4, 5, 6];
/// ftParts_80074B0C's "no variant".
const HIDDEN: i32 = -1;
/// ftKb_SpecialHi_800F346C: `(form + HSD_Randi(4)) % 5 + 1`.
const FORM_DRAW: i32 = 4;
const FORMS: i32 = 5;
/// What the script writes to cmd_vars[0].
const BECOME_ROCK: u32 = 1;
const BECOME_KIRBY: u32 = 2;

/// ftKb_Init_803CB4EC's vector: a level floor's normal.
const LEVEL: Vec3 = Vec3::new(0.0, 1.0, 0.0);
/// Below one degree the rock does not slide (retail @400).
const ONE_DEGREE: f32 = 0.017453292;
/// The slide's factor once a recent frame's floor is inside the sliding
/// angle again (800F37EC).
const SETTLING_DAMPING: f32 = 0.9;

/// mv.kb.speciallw.x8: the model change in progress.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Change {
    #[default]
    None,
    /// 1: the flicker runs forward into the rock.
    ToRock,
    /// 2: and backward out of it.
    ToKirby,
}

/// fp->mv.kb.speciallw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stone {
    /// x0: frames the rock still lasts.
    pub timer: i16,
    /// x8.
    pub change: Change,
    /// xC: the form is drawn and the shell has its health.
    pub formed: bool,
    /// x10: the form shown, 1..=5.
    pub form: i32,
    /// x14: the flicker's step.
    pub flicker: i32,
    /// x18: the floor's normal, level once the slope ahead turns back.
    pub normal: Vec3,
    /// x24: that normal over the last four frames, newest first.
    pub normals: [Vec3; 4],
    /// x54[0]: their sum, normalised: the direction the rock slides along.
    pub mean_normal: Vec3,
    /// x84: the floor's angle, signed by the facing.
    pub angle: f32,
    /// x88[0..4]: that angle over the last four frames.
    pub angles: [f32; 4],
    /// x88[4]: their mean, the rock's tilt.
    pub mean_angle: f32,
    /// x88[8]: the factor on the slide's speed and acceleration.
    pub damping: f32,
}

impl Default for Stone {
    fn default() -> Self {
        Self {
            timer: 0,
            change: Change::None,
            formed: false,
            form: -1,
            flicker: -1,
            normal: LEVEL,
            normals: [LEVEL; 4],
            mean_normal: LEVEL,
            angle: 0.0,
            angles: [0.0; 4],
            mean_angle: 0.0,
            damping: 0.0,
        }
    }
}

impl Stone {
    /// ftKb_SpecialHi_800F36DC (800F36DC): a level floor throughout.
    fn level_floor(&mut self) {
        self.normals = [LEVEL; 4];
        self.mean_normal = LEVEL;
        self.angles = [0.0; 4];
        self.mean_angle = 0.0;
        self.normal = LEVEL;
        self.angle = 0.0;
    }

    /// ftKb_SpecialLw_SetStoneVecs: the landing's floor throughout.
    fn floor_throughout(&mut self) {
        self.normals = [self.normal; 4];
        self.mean_normal = self.normal;
        self.angles = [self.angle; 4];
        self.mean_angle = self.angle;
    }

    /// ftKb_SpecialHi_800F37EC (800F37EC): this frame's floor joins the
    /// history; the mean normal (sixteen fadds, then lbVector_Normalize)
    /// and angle (fmuls by 0.25) follow, and the damping: 1, or 0.9 once a
    /// frame newer than a sliding one was inside the sliding angle. (Less a
    /// quarter for each newer frame whose slope faced the other way, which
    /// no scene reaches.)
    fn record_floor(&mut self, minimum_slant: f32) {
        for i in (1..4).rev() {
            self.normals[i] = self.normals[i - 1];
            self.angles[i] = self.angles[i - 1];
        }
        self.normals[0] = self.normal;
        self.angles[0] = self.angle;
        let mut sum = Vec3::ZERO;
        let mut angle = 0.0;
        for i in 0..4 {
            sum.x += self.normals[i].x;
            sum.y += self.normals[i].y;
            sum.z += self.normals[i].z;
            angle += self.angles[i];
        }
        self.mean_normal = melee_lb::vector::normalize(sum);
        self.mean_angle = angle * 0.25;
        self.damping = 1.0;
        let magnitude = |a: f32| if a < 0.0 { -a } else { a };
        'history: for i in (1..4).rev() {
            if magnitude(self.angles[i]) <= minimum_slant {
                continue;
            }
            for j in (0..=i).rev() {
                let other = self.angles[j];
                if magnitude(other) <= minimum_slant {
                    self.damping = SETTLING_DAMPING;
                    break 'history;
                }
                let turned = if self.angles[i] > 0.0 {
                    other < 0.0
                } else {
                    other > 0.0
                };
                if turned {
                    unimplemented!(
                        "ftKb_SpecialHi_800F37EC (ftkirbyspeciallw.c:273-285): a slope that \
                         turned against the slide within four frames takes 0.25 off the damping"
                    );
                }
            }
        }
    }
}

pub const fn rows() -> [MotionRow; 6] {
    [
        // ftKb_SpecialLw1_Phys (800F451C): ft_80084FA8.
        common::row(
            START,
            332,
            start_anim,
            common::no_input,
            physics::jab,
            start_collision,
        ),
        common::row(
            ROCK,
            333,
            rock_anim,
            rock_input,
            rock_physics,
            rock_collision,
        ),
        common::row(
            END,
            334,
            end_anim,
            common::no_input,
            end_physics,
            end_collision,
        ),
        common::row(
            AIR_START,
            335,
            start_anim,
            common::no_input,
            end_physics,
            air_start_collision,
        ),
        common::row(
            AIR_ROCK,
            336,
            rock_anim,
            air_rock_input,
            air_rock_physics,
            air_rock_collision,
        ),
        common::row(
            AIR_END,
            337,
            end_anim,
            common::no_input,
            end_physics,
            end_collision,
        ),
    ]
}

fn scratch(f: &mut Fighter) -> &mut Stone {
    &mut f.character.get_mut::<Kirby>().stone
}

/// ftKb_SpecialN_set_cbs2: take_dmg_cb = ftKb_Init_800EE7B8 and death2_cb
/// = ftKb_Init_800EE74C.
fn install_damage_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Kirby>().damage_callbacks = true;
}

/// ftPartSetRotX(fp, 0, angle): the root's tilt.
fn tilt_root(f: &mut Fighter, angle: f32) {
    f.core.set_part_rotation(0, Axis::X, angle);
}

/// Fighter_ChangeMotionState between Stone's rows, with x221C_b4 put back
/// after it (the change clears it, fighter.c:1058).
fn change_row(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    frame: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    let shell = f.combat.shell_active;
    f.change_motion_state_with_flags(state, assets, flags, frame, rate)?;
    f.combat.shell_active = shell;
    Ok(())
}

/// ftAnim_SetAnimRate(gobj, 0): the rock's pose holds.
fn hold_animation(f: &mut Fighter) {
    f.core.animation.set_rate(&mut f.core.skeleton, 0.0, false);
}

/// ftKb_SpecialLw_Enter (800F3D50) / ftKb_SpecialAirLw_Enter (800F3E68).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    f.commands.variables = [0; 4];
    let kirby = f.character.get_mut::<Kirby>();
    kirby.stone = Stone {
        timer: kirby.attributes.stone.maximum_frames as i16,
        ..Stone::default()
    };
    f.combat.shell_health = 0.0;
    f.combat.shell_active = false;
    f.change_motion_state(if air { AIR_START } else { START }, assets)
        .expect("Stone assets");
    install_damage_callbacks(f);
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    f.effects.push(EffectRequest::SyncAttached {
        id: CHANGE_PUFF,
        bone: PUFF_JOINT,
    });
}

/// ftKb_SpecialHi_800F346C (800F346C), the script's request. 1, once per
/// Stone: the shell's health (800F34C4: the integer attribute to float),
/// the next form drawn past the last (ftKb_FighterVars hat.x4) and the
/// flicker forward. 2, once the rock has formed: the flicker backward.
fn read_script_request(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    let request = std::mem::take(&mut f.commands.variables[0]);
    let stone = *scratch(f);
    if stone.change != Change::None {
        return;
    }
    match request {
        BECOME_ROCK if !stone.formed => {
            // The script's own graphics drew their offsets when their
            // commands ran, before this callback.
            common::seal_graphics(f, assets, rng);
            let kirby = f.character.get_mut::<Kirby>();
            let health = kirby.attributes.stone.health as f32;
            kirby.stone_form = (kirby.stone_form + rng.randi(FORM_DRAW)) % FORMS + 1;
            kirby.stone = Stone {
                change: Change::ToRock,
                form: kirby.stone_form,
                flicker: 0,
                formed: true,
                ..kirby.stone
            };
            f.combat.shell_health = health;
        }
        BECOME_KIRBY if stone.formed => {
            let stone = scratch(f);
            stone.change = Change::ToKirby;
            stone.flicker = FLICKER_STEPS;
        }
        _ => {}
    }
}

/// ftKb_SpecialHi_800F331C (800F331C): one step of the flicker. A frame
/// that shows the rock selects the form's model (ftParts_80074B0C: group 0
/// by the form, group 1 hidden) and hides the held item and the articles
/// (ftCommon_8007F5CC, x221E_b4) and a copied ability's hat (x2225_b2); a
/// frame that shows Kirby undoes that.
fn step_flicker(f: &mut Fighter) {
    let stone = scratch(f);
    match stone.change {
        Change::ToRock => stone.flicker += 1,
        Change::ToKirby => stone.flicker -= 1,
        Change::None => {}
    }
    if !(0..FLICKER_STEPS).contains(&stone.flicker) {
        stone.change = Change::None;
        return;
    }
    let (rock, form) = (FLICKER[stone.flicker as usize], stone.form);
    if rock {
        show_rock(f, FORM_MODELS[form as usize]);
    } else {
        show_kirby(f);
    }
}

fn show_rock(f: &mut Fighter, model: i32) {
    f.commands.model_selections.insert(0, model);
    f.commands.model_selections.insert(1, HIDDEN);
    show_held_item(f, false);
    f.commands.articles_visible = false;
    f.commands.accessory_visible = false;
}

fn show_kirby(f: &mut Fighter) {
    f.commands.model_selections.insert(0, 0);
    f.commands.model_selections.insert(1, 0);
    show_held_item(f, true);
    f.commands.articles_visible = true;
    f.commands.accessory_visible = true;
}

/// ftCommon_8007F5CC. A special cannot start with an item in hand here
/// (SPECIALS_KEEP_HELD_ITEM is false), so only the visibility bit changes.
fn show_held_item(f: &mut Fighter, visible: bool) {
    assert!(
        f.held_item.is_none(),
        "ftCommon_8007F5CC: Stone holding an item"
    );
    f.commands.held_item_hidden = !visible;
}

/// ftKb_SpecialAirLw_800F5318 (800F5318), from the damage and death
/// callbacks: the shell is gone. The callbacks' caller shows the item and
/// the articles again, levels the root and destroys Kirby's effects.
pub fn drop_shell(f: &mut Fighter) {
    f.combat.shell_health = 0.0;
    f.combat.shell_active = false;
}

/// The script's request and the flicker, as every animation callback of
/// the six rows starts (after `x221C_b4` is set for the row).
fn animate(f: &mut Fighter, p: &mut AnimationPhase<'_>, shell: bool) {
    f.step_animation(p.assets);
    f.combat.shell_active = shell;
    read_script_request(f, p.assets, p.rng);
    step_flicker(f);
}

/// ftKb_SpecialLw1_Anim (800F3F80) / ftKb_SpecialAirLwStart_Anim
/// (800F412C): at the end, the rock row with its animation held.
fn start_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    animate(f, &mut p, false);
    if !f.animation.frames_remaining(&f.skeleton) {
        let rock = if f.motion_state.action == AIR_START {
            AIR_ROCK
        } else {
            ROCK
        };
        common::seal_graphics(f, p.assets, p.rng);
        change_row(f, rock, STAGE_FLAGS, 0.0, 0.0, p.assets)?;
        hold_animation(f);
    }
    Ok(None)
}

/// ftKb_SpecialLw_Anim (800F402C) / ftKb_SpecialAirLw_Anim (800F41D8):
/// the shell is up for as long as the rock row runs.
fn rock_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    animate(f, &mut p, true);
    Ok(None)
}

/// ftKb_SpecialLwEnd_Anim (800F4074) / ftKb_SpecialAirLwEnd_Anim
/// (800F4220): at the end the shell is cleared and Kirby falls. A nonzero
/// +114 (zero on the disc) would make it the special fall with that lag.
fn end_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    animate(f, &mut p, false);
    if !f.animation.frames_remaining(&f.skeleton) {
        drop_shell(f);
        let lag = f.character.get::<Kirby>().attributes.stone.free_fall_lag;
        if lag != 0.0 {
            unimplemented!(
                "ftKb_SpecialLwEnd_Anim (ftkirbyspeciallw.c:471, 510): ftCo_80096900 with a \
                 nonzero +114"
            );
        }
        common::enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// The rock rows' IASA test (ftkirbyspeciallw.c:53-74): over when the
/// timer is spent, or on a B press once the minimum time has passed;
/// otherwise a frame comes off the timer.
fn rock_over(f: &mut Fighter) -> bool {
    let pressed = f.input.pressed.intersects(Buttons::B);
    let kirby = f.character.get_mut::<Kirby>();
    let a = &kirby.attributes.stone;
    let cancellable = (a.maximum_frames - a.minimum_frames) as i32;
    let timer = &mut kirby.stone.timer;
    if *timer <= 0 || (i32::from(*timer) <= cancellable && pressed) {
        return true;
    }
    *timer -= 1;
    false
}

/// The change into an end row, with the damage callbacks and the puff.
fn end_rock(f: &mut Fighter, state: ActionId, assets: &FighterAssets) {
    change_row(f, state, STAGE_FLAGS, 0.0, 1.0, assets).expect("Stone end assets");
    install_damage_callbacks(f);
    f.effects.push(EffectRequest::SyncAttached {
        id: RETURN_PUFF,
        bone: PUFF_JOINT,
    });
}

/// ftKb_SpecialLw_IASA (800F42DC): the grounded rock leaves the floor
/// (ftCommon_8007D5D4) as it ends.
fn rock_input(f: &mut Fighter, p: InputPhase<'_>) {
    if rock_over(f) {
        f.leave_ground();
        end_rock(f, END, p.assets);
    }
}

/// ftKb_SpecialAirLw_IASA (800F4404).
fn air_rock_input(f: &mut Fighter, p: InputPhase<'_>) {
    if rock_over(f) {
        end_rock(f, AIR_END, p.assets);
    }
}

/// ftKb_SpecialLw_Phys (800F453C): ft_80084FA8 and `self_vel.y = 0`, all of
/// which ftKb_SpecialHi_800F3B28 then replaces, as it does
/// ftCommon_ApplyGroundMovement's projection.
fn rock_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    slide(f);
    let core = &mut f.core;
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &p.assets.common),
        p.map,
        p.wind,
    );
}

/// ftKb_SpecialHi_800F3B28 (800F3B28; no fused operation). The root tilts
/// to the mean angle. Inside the sliding angle (or under one degree after
/// the clamp to +F8) the rock brakes by the fighter's ground friction
/// along the mean normal; past it the slope accelerates it by +FC, more on
/// a slippery floor. The damping scales both. Speed and acceleration are
/// laid along the mean normal's tangent. (The clamps to +F8 and to +100's
/// top speed are not ported: Yoshi's Story's slopes are 12 degrees and 17
/// units long.)
fn slide(f: &mut Fighter) {
    let stone = *scratch(f);
    let (minimum_slant, maximum_slant, acceleration, maximum_speed) = {
        let a = &f.character.get::<Kirby>().attributes.stone;
        (
            a.minimum_slant,
            a.maximum_slant,
            a.slide_acceleration,
            a.slide_speed,
        )
    };
    let magnitude = |a: f32| if a < 0.0 { -a } else { a };
    let mut angle = stone.mean_angle;
    tilt_root(f, angle);
    if magnitude(angle) < minimum_slant {
        angle = 0.0;
    }
    if angle > maximum_slant || angle < -maximum_slant {
        unimplemented!(
            "ftKb_SpecialHi_800F3B28 (ftkirbyspeciallw.c:308-313): a floor past +F8's angle"
        );
    }
    let c = &mut f.core;
    let traction = grounded::ground_friction_multiplier(&c.physics, &c.collision.data);
    let speed = c.physics.ground_velocity;
    let (push, brake) = if magnitude(angle) < ONE_DEGREE {
        // 800F3BE0, 800F3BE8: two fmuls.
        let friction = traction * (stone.mean_normal.y * c.attributes.ground.ground_friction);
        let brake = if magnitude(friction) > magnitude(speed) {
            -speed
        } else if speed > 0.0 {
            -friction
        } else {
            friction
        };
        (0.0, brake)
    } else {
        // 800F3C48..800F3C5C: fsubs, fmuls, fadds, fmuls.
        let push = (1.0 + (1.0 - traction)) * (stone.mean_normal.x * acceleration);
        (push, 0.0)
    };
    // 800F3C60, 800F3C68: fadds, fmuls.
    let physics = &mut c.physics;
    physics.ground_acceleration = stone.damping * (push + brake);
    physics.ground_velocity *= stone.damping;
    let next = physics.ground_velocity + physics.ground_acceleration;
    let within = |speed: f32| (-maximum_speed..=maximum_speed).contains(&speed);
    if !within(physics.ground_velocity) || !within(next) {
        unimplemented!(
            "ftKb_SpecialHi_800F3B28 (ftkirbyspeciallw.c:338-352): the slide at +100's top speed"
        );
    }
    // 800F3CE0..800F3D30: fmuls, the Y terms through an fneg first.
    let n = stone.mean_normal;
    physics.animation_velocity = Vec3::new(
        n.y * physics.ground_acceleration,
        -n.x * physics.ground_acceleration,
        0.0,
    );
    physics.self_velocity = Vec3::new(
        n.y * physics.ground_velocity,
        -n.x * physics.ground_velocity,
        0.0,
    );
}

/// ftKb_SpecialLwEnd_Phys (800F4584), ftKb_SpecialAirLwStart_Phys
/// (800F45A4) and ftKb_SpecialAirLwEnd_Phys (800F4608): ft_80085134.
fn end_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::root_motion_air(f);
    common::finish_air(f, &p);
}

/// ftKb_SpecialAirLw_Phys (800F45C4): ft_80085134, then the fall at +104
/// (fneg).
fn air_rock_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::root_motion_air(f);
    let fall_speed = f.character.get::<Kirby>().attributes.stone.fall_speed;
    f.physics.self_velocity.y = -fall_speed;
    common::finish_air(f, &p);
}

/// ftKb_SpecialHi_800F3570 (800F3570): the floor's normal and its angle
/// signed by the facing (fmuls). On a slope a ray looks down it, from +100
/// above Kirby to +100 below and that far along (800F3618: fmadds); where
/// the floor it meets slopes the other way, the rock is at the bottom: the
/// floor counts as level and it stops (no recorded stage has such a dip).
fn read_floor(f: &mut Fighter, map: &mut CollMap) {
    let normal = f.collision.data.floor.normal;
    let facing = f.physics.facing;
    let stone = scratch(f);
    stone.normal = normal;
    stone.angle = facing * atan2f(normal.x, normal.y);
    if 0.0 == normal.x {
        return;
    }
    let reach = f.character.get::<Kirby>().attributes.stone.slide_speed;
    let position = f.physics.position;
    let downhill = if normal.x > 0.0 { 1.0 } else { -1.0 };
    // 800F360C, 800F3618: fmuls, fmadds.
    let ahead = gekko_math::fma::fmadds(normal.y * reach, downhill, position.x);
    let floor_ahead = map
        .check_all_remap(
            -1,
            -1,
            position.x,
            position.y + reach,
            ahead,
            position.y - reach,
        )
        .map_or(LEVEL, |hit| hit.normal);
    let turns_back = if normal.x > 0.0 {
        floor_ahead.x < 0.0
    } else {
        floor_ahead.x > 0.0
    };
    if turns_back {
        unimplemented!(
            "ftKb_SpecialHi_800F3570 (ftkirbyspeciallw.c:177-188): the floor ahead of a sliding \
             rock slopes back: the floor counts as level and the rock stops"
        );
    }
}

/// What the grounded rows' collision does while the floor holds:
/// ftKb_SpecialHi_800F3570 and ftKb_SpecialHi_800F37EC.
fn follow_floor(f: &mut Fighter, map: &mut CollMap) {
    read_floor(f, map);
    let minimum_slant = f.character.get::<Kirby>().attributes.stone.minimum_slant;
    scratch(f).record_floor(minimum_slant);
}

/// The grounded rows off their floor (ft_80082708): a level history, no
/// horizontal speed, the aerial row, the damage callbacks and a level root.
fn lose_floor(
    f: &mut Fighter,
    state: ActionId,
    flags: MotionEntryFlags,
    frame: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    scratch(f).level_floor();
    f.physics.self_velocity.x = 0.0;
    f.leave_ground();
    change_row(f, state, flags, frame, rate, assets)
}

/// ftKb_SpecialLw1_Coll (800F4628): off the floor, the aerial start at the
/// current frame.
fn start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        follow_floor(f, p.map);
        return Ok(());
    }
    let frame = f.animation.frame;
    let assets = p.assets.expect("Stone collision assets");
    lose_floor(f, AIR_START, COUNTERPART_FLAGS, frame, 1.0, assets)?;
    install_damage_callbacks(f);
    tilt_root(f, 0.0);
    Ok(())
}

/// ftKb_SpecialLw_Coll (800F4800): off the floor, the falling rock.
fn rock_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        follow_floor(f, p.map);
        return Ok(());
    }
    let assets = p.assets.expect("Stone collision assets");
    lose_floor(f, AIR_ROCK, ROCK_FALL_FLAGS, 0.0, 0.0, assets)?;
    hold_animation(f);
    install_damage_callbacks(f);
    tilt_root(f, 0.0);
    Ok(())
}

/// ftKb_SpecialLwEnd_Coll (800F49E4) / ftKb_SpecialAirLwEnd_Coll
/// (800F51C0): the airborne pass (ft_80081D0C), a level history and a level
/// root. Retail ignores the pass's result, so an end row on a floor would
/// stay airborne there until Fall; the rows only rise, and none has met one.
fn end_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        unimplemented!(
            "ftKb_SpecialLwEnd_Coll / ftKb_SpecialAirLwEnd_Coll (ftkirbyspeciallw.c:695, 866): \
             an end row touching a floor (ft_80081D0C's result is ignored)"
        );
    }
    scratch(f).level_floor();
    tilt_root(f, 0.0);
    Ok(())
}

/// The aerial rows' landing: the floor read, no vertical speed,
/// ftCommon_8007D7FC, the grounded row, the damage callbacks, the quake
/// (Camera_RequestQuake(QuakeKind_Large); the sound and the rumble are
/// output only), the floor through the whole history, and on a slope past
/// the sliding angle the full slide speed down it at once (fmuls by the
/// normal's x).
fn land(
    f: &mut Fighter,
    map: &mut CollMap,
    state: ActionId,
    frame: f32,
    rate: f32,
    assets: &FighterAssets,
) -> Result<()> {
    follow_floor(f, map);
    f.physics.self_velocity.y = 0.0;
    f.land();
    change_row(f, state, COUNTERPART_FLAGS, frame, rate, assets)?;
    if rate == 0.0 {
        hold_animation(f);
    }
    install_damage_callbacks(f);
    f.core.quake_request = Some(melee_cm::QuakeKind::Large);
    let (minimum_slant, slide_speed) = {
        let a = &f.character.get::<Kirby>().attributes.stone;
        (a.minimum_slant, a.slide_speed)
    };
    let stone = scratch(f);
    stone.floor_throughout();
    let (angle, normal) = (stone.angle, stone.normal);
    if (if angle < 0.0 { -angle } else { angle }) >= minimum_slant {
        f.physics.ground_velocity = normal.x * slide_speed;
    }
    Ok(())
}

/// The aerial rows still in the air: a level history and a level root.
fn stay_airborne(f: &mut Fighter) {
    scratch(f).level_floor();
    tilt_root(f, 0.0);
}

/// ftKb_SpecialAirLwStart_Coll (800F4B3C): on a floor (ft_80081D0C), the
/// grounded start at the current frame.
fn air_start_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        stay_airborne(f);
        return Ok(());
    }
    let frame = f.animation.frame;
    let assets = p.assets.expect("Stone collision assets");
    land(f, p.map, START, frame, 1.0, assets)
}

/// ftKb_SpecialAirLw_Coll (800F4E78): on a floor, the grounded rock.
fn air_rock_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        stay_airborne(f);
        return Ok(());
    }
    let assets = p.assets.expect("Stone collision assets");
    land(f, p.map, ROCK, 0.0, 0.0, assets)
}
