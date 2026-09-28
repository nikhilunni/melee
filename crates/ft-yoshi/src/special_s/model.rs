//! The egg's model, pose and hitbox upkeep shared by the Egg Roll states.
use super::{part, roll, EggRoll, END_AIR, END_GROUND};
use crate::init::Yoshi;
use hsd_types::Vec3;
use melee_ft::fighter::{part_rotation::Axis, Fighter};

/// ftYs_Unk3_803CED48: the shell-closing sequence's model group per step
/// (1 shows the egg).
const SHELL_SEQUENCE: [i32; 15] = [0, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 1, 1];
/// Steps ftYs_SpecialS_8012EB48 plays (the table's last entry is unused).
const SHELL_STEPS: i32 = 14;
/// ftYs_Unk3_803CED84 / ftYs_Unk3_803CED94: the landing squash's Y and Z
/// scales per step.
const SQUASH_Y: [f32; 4] = [0.65, 0.7, 0.8, 1.0];
const SQUASH_Z: [f32; 4] = [1.1, 1.35, 1.3, 1.2];
/// @419: the roll angle's wrap, in double precision.
const TAU: f64 = std::f64::consts::TAU;
/// @310: M_PI_2 in double precision.
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;

/// cmd_vars[0] values the script writes for ftYs_SpecialS_8012EB48.
const SHELL_ADVANCE: u32 = 1;
const SHELL_RESTART: u32 = 2;

/// ftYs_SpecialS_8012EB48 (8012EB48): the script steps the shell closing
/// (cmd_vars[0] = 1) or restarts it and counts (= 2, cmd_vars[1]++).
pub(super) fn step_shell(f: &mut Fighter) {
    match f.commands.variables[0] {
        SHELL_ADVANCE => roll(f).shell_step += 1,
        SHELL_RESTART => {
            roll(f).shell_step = 0;
            f.commands.variables[1] = f.commands.variables[1].wrapping_add(1);
        }
        _ => {}
    }
    let step = roll(f).shell_step;
    if !(0..SHELL_STEPS).contains(&step) {
        f.commands.variables[0] = 0;
        return;
    }
    let egg = SHELL_SEQUENCE[step as usize];
    select_model(f, egg);
    if egg == 1 {
        // ftYs_Init_8012B918: the egg material back to frame 0.
        f.character.get_mut::<Yoshi>().shield_material_frame = 0.0;
        show_held_item(f, false);
        f.commands.articles_visible = false;
    } else {
        show_held_item(f, true);
        f.commands.articles_visible = true;
    }
    if f.commands.variables[0] == SHELL_RESTART {
        roll(f).shell_step = -1;
        f.commands.variables[0] = 0;
    }
}

/// ftParts_80074B0C(gobj, 0, variant): model group 0.
fn select_model(f: &mut Fighter, variant: i32) {
    f.character.get_mut::<Yoshi>().model_group = variant;
    f.commands.model_selections.insert(0, variant);
}

/// ftCommon_8007F5CC. A special cannot start with an item in hand here
/// (SPECIALS_KEEP_HELD_ITEM is false), so only the visibility bit changes.
fn show_held_item(f: &mut Fighter, visible: bool) {
    assert!(
        f.held_item.is_none(),
        "ftCommon_8007F5CC: Egg Roll holding an item"
    );
    f.commands.held_item_hidden = !visible;
}

/// ftYs_SpecialS_UpdateScale: the landing squash for four frames, else the
/// saved scale (HSD_JObjSetScale on the model root).
pub(super) fn squash(f: &mut Fighter) {
    let saved = f.character.get::<Yoshi>().egg_roll_scale;
    let step = roll(f).squash_step;
    let scale = if (0..4).contains(&step) {
        roll(f).squash_step += 1;
        // Retail fmuls per axis.
        Vec3::new(
            saved.x,
            saved.y * SQUASH_Y[step as usize],
            saved.z * SQUASH_Z[step as usize],
        )
    } else {
        saved
    };
    let root = f.animation.root;
    f.skeleton.set_scale(root, &scale);
}

/// ftYs_SpecialS_WrapAndSetRotX: the roll angle into [0, 2pi] (each step a
/// double add rounded to the stored float), then part 3's X rotation.
pub(super) fn wrap_roll(f: &mut Fighter) {
    let angle = &mut roll(f).roll_angle;
    *angle = wrap(*angle);
    let angle = *angle;
    f.core.set_part_rotation(part::ROLL, Axis::X, angle);
}

/// The double-precision wrap loops (e.g. 8012FA30..8012FA6C).
pub(super) fn wrap(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > TAU {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// Advance the roll angle by `step` (a double) times `amount`: retail fmadd
/// in double precision, then frsp (e.g. 8012FA14).
pub(super) fn advance_roll(f: &mut Fighter, step: f64, amount: f32) {
    let scratch = roll(f);
    scratch.roll_angle =
        gekko_math::fma::fmadd(step, f64::from(amount), f64::from(scratch.roll_angle)) as f32;
}

/// ftPartSetRotY(fp, 0, M_PI_2 * facing): double product, rounded.
pub(super) fn face(f: &mut Fighter) {
    let angle = (HALF_PI * f64::from(f.physics.facing)) as f32;
    f.core.set_part_rotation(part::FACING, Axis::Y, angle);
}

/// fn_8012EC7C (8012EC7C): outside the end states the egg model goes; the
/// scale and pose return, a pending facing applies, the item and articles
/// show again.
pub(super) fn restore(f: &mut Fighter) {
    let action = f.motion_state.action;
    if action != END_AIR && action != END_GROUND {
        select_model(f, 0);
    }
    let saved = f.character.get::<Yoshi>().egg_roll_scale;
    let root = f.animation.root;
    f.skeleton.set_scale(root, &saved);
    face(f);
    f.core.set_part_rotation(part::TILT, Axis::Z, 0.0);
    let pending = roll(f).pending_facing;
    if pending != 0.0 {
        f.physics.facing = pending;
    }
    roll(f).pending_facing = 0.0;
    show_held_item(f, true);
    f.commands.articles_visible = true;
}

/// efSync_Spawn(0x4CF, gobj, part 4's position, &co_attrs.xBC): the shell
/// bursts from part 4.
pub(super) fn burst_shell(f: &mut Fighter) {
    let scale = f.attributes.yoshi_egg.size;
    f.effects.push(melee_ef::request::EffectRequest::EggShell {
        bone: part::SHELL,
        scale,
    });
}

/// The loops' hitbox upkeep: every `group_toggle_frames` the live hitbox
/// changes group (so it may hit again), and its damage follows the speed.
pub(super) fn update_hitbox(f: &mut Fighter) {
    let toggle = super::attributes(f).group_toggle_frames;
    let scratch: &mut EggRoll = roll(f);
    scratch.group_timer += 1;
    let due = scratch.group_timer >= toggle;
    if due {
        if let Some(hit) = &mut f.commands.hitboxes[0] {
            // HitCapsule.x4 = (x4 + 1) % 2 (unsigned).
            hit.descriptor.group = ((u32::from(hit.descriptor.group) + 1) % 2) as u8;
            roll(f).group_timer = 0;
        }
    }
    if f.commands.hitboxes[0].is_none() {
        return;
    }
    let a = super::attributes(f);
    let (base, multiplier) = (a.damage_base, a.damage_multiplier);
    let speed = if f.physics.ground_or_air == melee_types::GroundOrAir::Air {
        f.physics.self_velocity.x
    } else {
        f.physics.ground_velocity
    };
    // Retail 8012F990 fadds, 8012F994 fmuls, 8012F998 fctiwz.
    let rate = gekko_math::msl::fctiwz(multiplier * (base + gekko_math::msl::fabsf(speed))).max(1);
    set_hitbox_damage(f, rate);
}

/// ftColl_8007ABD0 (8007ABD0): hitbox 0's damage from `rate`, then staled.
fn set_hitbox_damage(f: &mut Fighter, rate: i32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge; the roll has none.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: charged roll"
    );
    let damage = rate as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[0].as_mut().expect("roll hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}
