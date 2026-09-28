//! Peach Parasol, ftpeachspecialhi.c (8011D424..8011E0BC): the rising
//! start, then the shared parasol states (ftCo_ItemParasolOpen and its
//! special-fall sequel) on Peach's own rows 369 and 370.
use crate::init::{Accessory, Peach};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    physics::{airborne, friction},
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{CommonMotionState, GroundOrAir, ItemKind};

/// ftPe_MS_SpecialHiStart..SpecialAirHiEnd (361..364).
pub const SPECIAL_HI_START: ActionId = ActionId(361);
pub const SPECIAL_HI_END: ActionId = ActionId(362);
pub const SPECIAL_AIR_HI_START: ActionId = ActionId(363);
pub const SPECIAL_AIR_HI_END: ActionId = ActionId(364);
/// ftPe_MS_ItemParasolOpen / ItemParasolFall (369 / 370).
pub const PARASOL_OPEN: ActionId = ActionId(369);
pub const PARASOL_FALL: ActionId = ActionId(370);
/// ftCo_800CEFE0: the parasol opens over a ten-frame blend.
const OPEN_BLEND_FRAMES: f32 = 10.0;

/// Command variables of the rising start (ftpeachspecialhi.c).
mod var {
    /// Set by the script once the rise begins: root motion drives the
    /// aerial start and the landing check.
    pub const RISING: usize = 0;
    /// Raised at frame 30 and cleared by checkCmdVar2, which never ends
    /// the state.
    pub const SPENT: usize = 2;
}

/// What fp->item_gobj held when the move started (mv.pe.specialhi.kind).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeldAtStart {
    /// It_Kind_Capsule: nothing relevant; the accessory draws the parasol.
    #[default]
    Nothing,
    /// It_Kind_Parasol: the Parasol item opens instead.
    ParasolItem,
    /// It_Kind_Peach_Parasol: her own parasol is already out.
    PeachParasol,
}

/// fp->mv.pe.specialhi and fp->lstick_angle.
#[derive(Clone, Debug, Default)]
pub struct SpecialHi {
    pub held: HeldAtStart,
    /// Fighter +6BC, zeroed by every motion change (fighter.c:1171).
    pub angle: f32,
}

/// ftPe_SpecialHi_Enter (8011D6E4) / ftPe_SpecialAirHi_Enter (8011D740).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.y = 0.0;
        // retail 8011D76C: fmuls.
        f.physics.self_velocity.x *= f
            .character
            .get::<Peach>()
            .attributes
            .parasol
            .startup_momentum_multiplier;
    }
    let state = if air {
        SPECIAL_AIR_HI_START
    } else {
        SPECIAL_HI_START
    };
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(state, a).expect("Peach Parasol assets");
    f.character.get_mut::<Peach>().retained_word = retained_word;
    f.step_animation(a);
    // doEnter (inlined).
    f.commands.variables[..3].fill(0);
    f.commands.grab_release = false;
    f.commands.throw_reverse = false;
    // x2222_b2 (immunity to Sing, ftCo_800C3538) is not modelled.
    let held = if f.core.held_item.is_some_and(|held| held.kind == ItemKind::Parasol) {
        HeldAtStart::ParasolItem
    } else if f.core.article_in_hand.is_some_and(|a| a.kind == ItemKind::PeachParasol) {
        HeldAtStart::PeachParasol
    } else {
        HeldAtStart::Nothing
    };
    let peach = f.character.get_mut::<Peach>();
    peach.special_hi = SpecialHi {
        held,
        angle: 0.0,
    };
    if held == HeldAtStart::Nothing {
        peach.accessory = Accessory::DrawParasol;
        f.core.arm_accessory4();
    }
}

/// ftPe_SpecialHi_8011D424 (8011D424): the parasol hangs from joint 109
/// unless one is already out.
pub fn draw_parasol(f: &mut Fighter, assets: &FighterAssets) {
    let peach = f.character.get_mut::<Peach>();
    if peach.special_hi.held == HeldAtStart::Nothing {
        peach.special_hi.held = HeldAtStart::PeachParasol;
        if !peach.items.parasol[0] {
            let c = &mut f.core;
            let position = melee_ft::fighter::caches::part_position(
                &mut c.skeleton,
                &c.animation,
                crate::special_n::ARTICLE_JOINT,
                Vec3::ZERO,
            );
            // A held item (a turnip) is stowed under the parasol: it stays in
            // hand, hidden and frozen, as u.pe.parasol_gobj_1.
            if f.core.held_item.is_some() {
                f.character.get_mut::<Peach>().items.parasol[1] = true;
                f.stow_held_item(assets);
            }
            let c = &mut f.core;
            let spawn =
                SpawnItem::attached(ItemKind::PeachParasol, c.player.id, position, c.physics.facing);
            c.item_requests.push(ItemRequest::SpawnHeld(spawn));
            // fp->item_gobj = parasol_gobj_0; the pickup callback opens it
            // (itPeachParasol_Logic60_PickedUp, motion 2).
            f.core.article_in_hand = Some(melee_ft::fighter::item_pickup::ArticleInHand {
                kind: ItemKind::PeachParasol,
                part: crate::special_n::ARTICLE_JOINT,
                use_kind: it_parasol::USE_KIND,
            });
            let peach = f.character.get_mut::<Peach>();
            peach.items.parasol[0] = true;
            peach.items.parasol_motion = it_parasol::OPEN;
            // death3_cb and take_dmg_cb = ftPe_Init_OnDeath2.
            peach.items.death3_armed = true;
            peach.items.take_damage_armed = true;
        }
    }
    // pre/post-hitlag callbacks (ftPe_SpecialHi_8011D620 / 8011D650) are
    // installed but not modelled: see crate::articles.
}

/// The parasol item's motion ids, as Peach drives them.
pub mod it_parasol {
    /// it_802BDD40: the opening animation.
    pub const OPENING: u16 = 1;
    /// it_802BDDB4: held open.
    pub const OPEN: u16 = 2;
    /// it_804D5518: the frames of the opening and open animations.
    pub const ANIMATION_FRAMES: [i32; 2] = [15, 16];
    /// PlPe items[2] ItemAttr use kind (it_8026B30C): 2, a swinging item.
    pub const USE_KIND: u8 = 2;
}

/// ftPe_SpecialHiStart_Anim (8011D8D0) / ftPe_SpecialAirHiStart_Anim:
/// the parasol opens once the start ends, as if from special fall.
pub fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // checkCmdVar2: clears the variable and always returns false.
    f.commands.variables[var::SPENT] = 0;
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.character.get::<Peach>().special_hi.held == HeldAtStart::ParasolItem {
            unimplemented!("ftPe_SpecialHiStart_Anim: the Parasol item opens");
        }
        // fp->motion_id = ftCo_MS_FallSpecial, then ftCo_800CEFE0, whose
        // timer comes from PlCo +59C whatever the argument.
        f.enter_parasol_open(
            p.assets,
            CommonMotionState::FallSpecial.into(),
            OPEN_BLEND_FRAMES,
        )?;
    }
    Ok(None)
}

/// ftPe_SpecialHiStart_IASA (8011DA30): the stick steers the rise until
/// it begins, and the first frames may turn Peach round.
pub fn start_input(f: &mut Fighter, _: InputPhase<'_>) {
    let x = f.input.current.stick.x;
    let a = f.character.get::<Peach>().attributes.parasol.clone();
    let magnitude = gekko_math::msl::fabsf(x);
    if f.commands.variables[var::RISING] == 0 && magnitude > a.angle_stick_threshold {
        // retail 8011DA80..9C: fsubs, then a double fsub, fdiv and fmul
        // before frsp; then fmuls by pi/180 (f32).
        let degrees = (f64::from(a.maximum_angle_degrees)
            * (f64::from(magnitude - a.angle_stick_threshold)
                / (1.0 - f64::from(a.angle_stick_threshold)))) as f32;
        let mut angle = (std::f32::consts::PI / 180.0) * degrees;
        if x > 0.0 {
            angle = -angle;
        }
        let scratch = &mut f.character.get_mut::<Peach>().special_hi;
        if gekko_math::msl::fabsf(angle) > gekko_math::msl::fabsf(scratch.angle) {
            scratch.angle = angle;
        }
    }
    // ftCheckThrowB3: the script's reverse window, consumed when read.
    if std::mem::take(&mut f.commands.grab_release)
        && gekko_math::msl::fabsf(x) > a.reverse_stick_threshold
    {
        // ftCommon_UpdateFacing, then ftPartSetRotY(fp, 0, M_PI_2 * facing)
        // (retail 8011DB58: a double fmul, frsp).
        f.physics.facing = if x < 0.0 { -1.0 } else { 1.0 };
        let root = f.animation.root;
        let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
        f.skeleton.set_rotation_y(root, rotation);
    }
}

/// ft_80085154 (80085154): the animation's TransN step, rotated by the
/// steering angle, becomes the velocity.
fn steered_root_motion(f: &mut Fighter) {
    let offset = f
        .animation
        .root_motion
        .as_ref()
        .expect("Peach Parasol TransN")
        .primary_history
        .offset;
    let angle = f.character.get::<Peach>().special_hi.angle;
    let cosine = gekko_math::msl::cosf(angle);
    let sine = gekko_math::msl::sinf(angle);
    let horizontal = offset.z * f.physics.facing;
    // retail 80085198 fmsubs, 8008519C fmadds.
    f.physics.self_velocity.x = gekko_math::fma::fmsubs(horizontal, cosine, offset.y * sine);
    f.physics.self_velocity.y = gekko_math::fma::fmadds(horizontal, sine, offset.y * cosine);
}

/// ftPe_SpecialHiStart_Phys (8011DC94): steered root motion once airborne,
/// ft_80084FA8 before the script lifts Peach off the ground.
pub fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Air {
        steered_root_motion(f);
        f.core.finish_air_update(p.assets, p.wind);
    } else {
        callbacks::physics::jab(f, p);
    }
}

/// ftPe_SpecialAirHiStart_Phys (8011DCF8): the aerial start falls slowly
/// until the rise, which is damped by attribute +8C.
pub fn air_start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = f.character.get::<Peach>().attributes.parasol.clone();
    if f.commands.variables[var::RISING] != 0 {
        steered_root_motion(f);
        // retail 8011DD30..50: three fmuls.
        f.physics.self_velocity.x *= a.ending_momentum_multiplier;
        f.physics.self_velocity.y *= a.ending_momentum_multiplier;
        f.physics.self_velocity.z *= a.ending_momentum_multiplier;
    } else {
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            a.gravity,
            f.attributes.air.terminal_velocity,
        );
        air_drift_friction(f, p.assets);
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCommon_8007CF58: aerial friction, or PlCo +1FC above the drift maximum.
fn air_drift_friction(f: &mut Fighter, assets: &FighterAssets) {
    let air = &f.attributes.air;
    f.physics.animation_velocity.x = friction::air_drift_friction_acceleration(
        f.physics.self_velocity.x,
        air.aerial_friction,
        air.air_drift_max,
        assets.common.over_drift_air_friction,
    );
}

/// ftPe_SpecialHiStart_Coll / ftPe_SpecialAirHiStart_Coll (doColl,
/// 8011DDB8): on the ground ft_80084104; in the air ft_80083B68 until the
/// rise turns downward, then ft_800831CC with special landing.
pub fn start_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return callbacks::collision::escape(f, p);
    }
    let assets = p.assets.expect("Peach Parasol collision assets");
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if c.commands.variables[var::RISING] == 0 || c.physics.self_velocity.y >= 0.0 {
        // ft_80083B68 -> ft_80082578 -> mpColl_800477E0.
        let cd = &mut c.collision.data;
        cd.last_pos = cd.cur_pos;
        cd.cur_pos = c.physics.position;
        let pose = melee_ft::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
        p.map.air_collide_stay(cd, Some(&|i| pose.position(i)));
        c.physics.position = cd.cur_pos;
        c.skeleton
            .set_translate(c.animation.root, &c.physics.position);
        return Ok(());
    }
    if melee_ft::collision::air::collide_fall(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    ) {
        // ftPe_SpecialHi_8011DD8C: LandingFallSpecial with attribute +74.
        let lag = f.character.get::<Peach>().attributes.parasol.landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else if !f.try_wall_jump(assets, p.map)? {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

/// ftPe_SpecialHiEnd_* (362 / 364): no retail code path enters them.
pub fn unreachable_end(_: &mut Fighter, _: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    unimplemented!("ftPe_SpecialHiEnd_Anim: SpecialHiEnd has no entry")
}
