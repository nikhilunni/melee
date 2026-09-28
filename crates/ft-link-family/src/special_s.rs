//! Boomerang, ftlinkspecials.c (800EBF2C..800EC9BC).
//!
//! The throw motion creates the boomerang in the left hand on its script's
//! throw flag, then sends it along the stick's angle on command variable 0
//! (onAccessory4). While one is out, the side special plays the empty-handed
//! motion. Catching it back (the article's request) plays the catch motion,
//! whose script drops it on command variable 1.
use crate::{
    common::{self, flags},
    row, Accessory, FamilyState, LinkFamily,
};
use melee_ft::input::WaitPredicate;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
};
use melee_it::{ItemControl, ItemRequest, Launch, SpawnItem};
use melee_types::{CommonMotionState, FtPart, GroundOrAir, ItemKind};

/// The boomerang's kind for the family member (ftLk_DatAttrs x2C).
fn kind<C: LinkFamily>(f: &Fighter) -> ItemKind {
    ItemKind::try_from(f.character.get::<C>().attributes().boomerang.item as i32)
        .expect("boomerang item kind")
}

fn part(assets: &FighterAssets, part: FtPart) -> u8 {
    assets.parts.joint(part).expect("Link hand part")
}

/// ftLk_SpecialS_Enter (800EC354) / ftLk_SpecialAirS_Enter (800EC3F8):
/// the throw flags and command variable 0 clear, on21EC notes a smash
/// input (it runs inside the motion change, before the frame-0 script),
/// then the throw, or the empty-handed motion with a boomerang out.
pub fn enter<C: LinkFamily>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    let smash = smash_input(f, a);
    f.character.get_mut::<C>().specials().vars.smash_boomerang = smash;
    let used = f.character.get::<C>().specials_ref().vars.used_boomerang;
    let state = match (air, used) {
        (false, false) => FamilyState::SpecialS1,
        (false, true) => FamilyState::SpecialS1Empty,
        (true, false) => FamilyState::SpecialAirS1,
        (true, true) => FamilyState::SpecialAirS1Empty,
    };
    common::change(f, state.action(), 0, 0.0, a).expect("Boomerang assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    arm_throw::<C>(f);
}

/// on21EC (800EBF2C): a stick past the dash-smash threshold within the
/// smash window plus PlCo +44 (x673 as a float against the window, an int
/// converted, fadds) throws the long boomerang.
fn smash_input(f: &Fighter, a: &FighterAssets) -> bool {
    let t = &a.input.thresholds;
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    magnitude >= t.dash_smash_stick_threshold
        && f32::from(f.input.horizontal.held)
            < t.dash_smash_window as f32 + a.input.item_smash_window_extension
}

/// accessory4_cb = onAccessory4, until the next motion change.
fn arm_throw<C: LinkFamily>(f: &mut Fighter) {
    crate::arm_accessory::<C>(f, Accessory::BoomerangThrow);
}

/// onAccessory4 (800EC210), every frame while installed: the script's
/// throw flag creates the boomerang in the left hand (it_802A013C); with
/// command variable 0 set and the boomerang still in hand, it flies
/// (calcAnglePos, it_802A0534).
pub(crate) fn throw_accessory<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    if std::mem::take(&mut f.commands.throw_accessory) {
        create::<C>(f, assets);
    }
    let vars = &f.character.get::<C>().specials_ref().vars;
    if f.commands.variables[0] != 0 && vars.boomerang_out && vars.boomerang_in_hand {
        launch::<C>(f, assets);
        f.commands.variables[0] = 0;
        f.commands.variables[1] = 0;
    }
}

/// it_802A013C (802A013C): the boomerang at the thrower's centre
/// (it_8026BB68 -> ftLib_80086990), swept from the left thumb, taken into
/// that hand (Item_8026AB54). The thrower notes it (x1984, u.lk) and
/// installs its removal on damage and death.
fn create<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let hand_part = part(assets, FtPart::LThumbNb);
    let c = &mut f.core;
    let hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(hand_part),
        hsd_types::Vec3::ZERO,
    );
    let center = c.item_holder(hand_part, assets).center;
    let kind = kind::<C>(f);
    let c = &mut f.core;
    let mut spawn = SpawnItem::attached(kind, c.player.id, center, c.physics.facing);
    spawn.previous_position = hand;
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: hand_part,
        hold: false,
    });
    let specials = f.character.get_mut::<C>().specials();
    specials.vars.boomerang_out = true;
    specials.vars.boomerang_in_hand = true;
    specials.vars.used_boomerang = true;
    specials.removal_armed = true;
}

/// calcAnglePos (800EC0C4) and it_802A0534: the flight angle from the
/// stick past x14 (atan2f of y over |x|, clamped to x18*x1C), the launch
/// velocity by facing, then the angle mirrored for a left facing and
/// wrapped to 0..2pi (double sums).
fn launch<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let (limit_scale, stick_threshold, distance) = {
        let s = f.character.get::<C>().specials_ref();
        let b = &f.character.get::<C>().attributes().boomerang;
        let distance = if s.vars.smash_boomerang {
            b.smash_speed
        } else {
            b.speed
        };
        // 800EC108: fmuls.
        (
            b.max_angle * b.max_angle_scale,
            b.angle_stick_threshold,
            distance,
        )
    };
    let stick = f.input.current.stick;
    let mut angle = 0.0;
    let y_magnitude = if stick.y < 0.0 { -stick.y } else { stick.y };
    if y_magnitude > stick_threshold {
        let x_magnitude = if stick.x < 0.0 { -stick.x } else { stick.x };
        angle = melee_lb::trigf::atan2f(stick.y, x_magnitude);
        if angle > limit_scale {
            angle = limit_scale;
        } else if angle < -limit_scale {
            angle = -limit_scale;
        }
    }
    let facing = f.physics.facing;
    // 800EC16C..90: (distance * cos) then * facing, fmuls each.
    let velocity = hsd_types::Vec3::new(
        facing * (distance * gekko_math::msl::cosf(angle)),
        facing * (distance * gekko_math::msl::sinf(angle)),
        0.0,
    );
    use std::f64::consts::{PI, TAU};
    if facing == -1.0 {
        angle = if angle < 0.0 {
            (-PI - f64::from(angle)) as f32
        } else {
            (PI - f64::from(angle)) as f32
        };
    }
    if angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    let hand_part = part(assets, FtPart::LThumbNb);
    let smash = f.character.get::<C>().specials_ref().vars.smash_boomerang;
    let holder = f.core.item_holder(hand_part, assets);
    // ftLib_80086630: the hand's world matrix, set up on demand.
    let hand = *holder.skeleton.get_mtx(holder.part);
    let launch = Launch {
        velocity,
        offset: hsd_types::Vec3::ZERO,
        spin_degrees: 0.0,
        hand,
        center: holder.center,
        attack: holder.attack,
        attack_stale: holder.attack_stale,
        angle,
        long_lifetime: smash,
        aim: None,
        shot: None,
    };
    let kind = kind::<C>(f);
    f.core.item_requests.push(ItemRequest::Launch {
        owner: f.player.id,
        kind,
        launch,
    });
    f.character.get_mut::<C>().specials().vars.boomerang_in_hand = false;
}

/// The article's catch (itLinkboomerang_UnkMotion3_Phys_sub):
/// ftLk_SpecialS2_Enter (800EC4A0) plays the catch on the ground or in the
/// air and installs the removal on damage; the boomerang hangs from the
/// left thumb (ftLk_SpecialHi_ProcessPartLThumbNb).
pub fn catch<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) -> Option<u8> {
    let state = if f.physics.ground_or_air == GroundOrAir::Air {
        FamilyState::SpecialAirS2
    } else {
        FamilyState::SpecialS2
    };
    // The catch writes no mv field: mv+4 stays the interrupted state's.
    let retained = f.inherited_scratch_word();
    f.character.get_mut::<C>().specials().retained_word = retained;
    common::change(f, state.action(), 0, 0.0, assets).expect("Boomerang catch assets");
    f.character.get_mut::<C>().specials().removal_armed = true;
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    f.character.get_mut::<C>().specials().vars.boomerang_in_hand = true;
    Some(part(assets, FtPart::LThumbNb))
}

/// ftLk_SpecialS_RemoveBoomerang0 (800EC064): the boomerang is gone.
pub fn forget_boomerang<C: LinkFamily>(f: &mut Fighter) {
    let vars = &mut f.character.get_mut::<C>().specials().vars;
    vars.used_boomerang = false;
    vars.boomerang_out = false;
    vars.boomerang_in_hand = false;
}

/// ftLk_SpecialS_RemoveBoomerang1 (800EC08C) / it_802A07B4: a boomerang
/// still out is destroyed.
pub fn remove_boomerang<C: LinkFamily>(f: &mut Fighter) {
    if !f.character.get::<C>().specials_ref().vars.boomerang_out {
        return;
    }
    let kind = kind::<C>(f);
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind,
        control: ItemControl::Remove,
    });
    forget_boomerang::<C>(f);
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air, at the end.
fn finish_at_end(f: &mut Fighter, p: &AnimationPhase<'_>, air: bool) -> Result<()> {
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, air, p.assets)?;
    }
    Ok(())
}

/// ftLk_SpecialS1_Anim / SpecialS1Empty_Anim / SpecialAirS1_Anim /
/// SpecialAirS1Empty_Anim.
fn throw_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    finish_at_end(f, &p, false)?;
    Ok(None)
}
fn air_throw_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    finish_at_end(f, &p, true)?;
    Ok(None)
}

/// doS2Anim (800EC4F0): command variable 1 drops the boomerang
/// (it_802A07B4); the motion ends in Wait or Fall.
fn catch_anim_with<C: LinkFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    air: bool,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[1] != 0 {
        remove_boomerang::<C>(f);
        f.commands.variables[1] = 0;
    }
    finish_at_end(f, &p, air)?;
    Ok(None)
}
fn catch_anim<C: LinkFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    catch_anim_with::<C>(f, p, false)
}
fn air_catch_anim<C: LinkFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    catch_anim_with::<C>(f, p, true)
}

/// checkBoomerangSomething: ftCo_SpecialS_CheckInput, ftCo_Attack100_
/// CheckInput (the up special), ftCo_800D6824, ftCo_800D68C0, ftCo_80091A4C
/// (shield), ftCo_Jump_CheckInput and ftCo_Dash_CheckInput, in that order.
const CATCH_INTERRUPTS: [WaitPredicate; 7] = [
    WaitPredicate::SpecialSide,
    WaitPredicate::SpecialUp,
    WaitPredicate::SpecialNeutral,
    WaitPredicate::SpecialDown,
    WaitPredicate::Shield,
    WaitPredicate::Jump,
    WaitPredicate::Dash,
];

/// ftLk_SpecialS2_IASA (800EC5A8): any of the catch's interrupts, whatever
/// the script allows, and the boomerang goes (it_802A07B4).
fn catch_input<C: LinkFamily>(f: &mut Fighter, p: InputPhase<'_>) {
    if f.try_ground_checks(p.assets, &CATCH_INTERRUPTS)
        .expect("boomerang catch interrupt")
    {
        remove_boomerang::<C>(f);
    }
}
/// ftLk_SpecialAirS2_IASA (800EC6F4): the aerial special or an aerial jump.
fn air_catch_input<C: LinkFamily>(f: &mut Fighter, p: InputPhase<'_>) {
    if f.try_air_special_or_jump(p.assets)
        .expect("aerial boomerang catch interrupt")
    {
        remove_boomerang::<C>(f);
    }
}

/// ftLk_SpecialS1_Coll (800EC844): off the floor, the aerial throw at the
/// same frame, the throw accessory installed again.
fn throw_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Boomerang assets");
        common::ground_to_air(f, FamilyState::SpecialAirS1.action(), assets)?;
        arm_throw::<C>(f);
    }
    Ok(())
}
/// ftLk_SpecialS1Empty_Coll.
fn empty_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Boomerang assets");
        common::ground_to_air(f, FamilyState::SpecialAirS1Empty.action(), assets)?;
    }
    Ok(())
}
/// ftLk_SpecialAirS1_Coll: landing, the grounded throw at the same frame.
fn air_throw_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Boomerang assets");
        common::air_to_ground(f, FamilyState::SpecialS1.action(), assets)?;
        arm_throw::<C>(f);
    }
    Ok(())
}
/// ftLk_SpecialAirS1Empty_Coll.
fn air_empty_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Boomerang assets");
        common::air_to_ground(f, FamilyState::SpecialS1Empty.action(), assets)?;
    }
    Ok(())
}
/// ftLk_SpecialS2_Coll: off the floor the boomerang goes and Link falls.
fn catch_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        remove_boomerang::<C>(f);
        let assets = p.assets.expect("Boomerang assets");
        f.change_motion_state(CommonMotionState::Fall.into(), assets)?;
    }
    Ok(())
}
/// ftLk_SpecialAirS2_Coll: landing, the boomerang goes and Link lands.
fn air_catch_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        remove_boomerang::<C>(f);
        let assets = p.assets.expect("Boomerang assets");
        f.enter_landing(assets)?;
    }
    Ok(())
}

fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

pub(crate) const fn rows<C: LinkFamily>() -> [MotionRow; 6] {
    let _ = flags::GROUND_AIR;
    [
        row(
            FamilyState::SpecialS1,
            throw_anim,
            common::no_input,
            ground_physics,
            throw_collision::<C>,
        ),
        row(
            FamilyState::SpecialS2,
            catch_anim::<C>,
            catch_input::<C>,
            ground_physics,
            catch_collision::<C>,
        ),
        row(
            FamilyState::SpecialS1Empty,
            throw_anim,
            common::no_input,
            ground_physics,
            empty_collision,
        ),
        row(
            FamilyState::SpecialAirS1,
            air_throw_anim,
            common::no_input,
            air_physics,
            air_throw_collision::<C>,
        ),
        row(
            FamilyState::SpecialAirS2,
            air_catch_anim::<C>,
            air_catch_input::<C>,
            air_physics,
            air_catch_collision::<C>,
        ),
        row(
            FamilyState::SpecialAirS1Empty,
            air_throw_anim,
            common::no_input,
            air_physics,
            air_empty_collision,
        ),
    ]
}
