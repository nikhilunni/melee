//! Cape, ftmariospecials.c (800E1248..800E1A54).
//!
//! Accessory4 (ftMr_SpecialS_CreateCape) creates the cape in Mario's right
//! hand once per swing. The script walks cmd_vars[0] through the swing's
//! gust (1 -> 2), and holds cmd_vars[1] at 1 while the cape reflects
//! (ftColl_CreateReflectHit with ftMario_DatAttrs.cape_reflection).
use crate::{
    common,
    init::{Accessory, Mario},
};
use hsd_types::Vec3;
use melee_coll::defense::ReflectDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::{airborne, friction},
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// ftMr_MS_SpecialS (345) and ftMr_MS_SpecialAirS (346).
pub const GROUND: ActionId = ActionId(345);
pub const AIR: ActionId = ActionId(346);

/// lb_800119DC(&hip, 120, strength, decay, M_PI_3): the swing's gust.
const GUST_FRAMES: i32 = 120;
const GROUND_GUST: (f32, f32) = (0.9, 0.02);
const AIR_GUST: (f32, f32) = (3.0, 0.1);
/// (float) M_PI_3, retail @324.
const GUST_PHASE_STEP: f32 = std::f32::consts::FRAC_PI_3;
/// The gust starts 3 units in front of the hip (800E1640: fmadds).
const GUST_FORWARD: f32 = 3.0;

/// transition_flags: KeepColAnimHitStatus | SkipHit | SkipMatAnim |
/// UpdateCmd | SkipColAnim | SkipItemVis | Unk19 | SkipModelPartVis |
/// SkipModelFlags | Unk27.
const GROUND_AIR_FLAGS: MotionEntryFlags = MotionEntryFlags(
    MotionEntryFlags::KEEP_COL_ANIM_HIT_STATUS.0
        | MotionEntryFlags::SKIP_HIT.0
        | MotionEntryFlags::SKIP_MAT_ANIM.0
        | MotionEntryFlags::UPDATE_CMD.0
        | MotionEntryFlags::SKIP_COL_ANIM.0
        | MotionEntryFlags::SKIP_ITEM_VIS.0
        | MotionEntryFlags::SKIP_MODEL_PART_VIS.0,
);

/// Mario's hold on the cape (x223C_capeGObj) and the swing's scratch.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cape {
    /// x223C_capeGObj: the cape exists.
    pub present: bool,
    /// take_dmg_cb / death2_cb = ftMr_Init_OnTakeDamage.
    pub damage_callbacks: bool,
    /// mv.mr.SpecialS.reflecting.
    pub reflecting: bool,
    /// ftColl_CreateReflectHit's descriptor while the reflector is up.
    pub reflector: Option<ReflectDescriptor>,
}

fn attributes(f: &Fighter) -> &crate::attributes::CapeAttributes {
    &f.character.get::<Mario>().attributes.cape
}

fn cape(f: &mut Fighter) -> &mut Cape {
    &mut f.character.get_mut::<Mario>().cape
}

/// ftMr_SpecialS_Enter (800E1450) / ftMr_SpecialAirS_Enter (800E14C8,
/// 800E14F8: fdivs), then changeAction.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.x /= attributes(f).horizontal_velocity_decay;
    } else {
        f.physics.self_velocity.y = 0.0;
    }
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Cape assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    f.commands.variables[2] = 0;
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    cape(f).reflecting = false;
    install_accessory(f);
}

/// accessory4_cb = ftMr_SpecialS_CreateCape.
fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Mario>().accessory = Accessory::CreateCape;
    f.core.arm_accessory4();
}

/// setCallbacks: with a cape, take-damage and death2 remove it; the hitlag
/// pair freezes it (it_802B26C0 / it_802B26E0).
fn install_callbacks(f: &mut Fighter) {
    let c = cape(f);
    if c.present {
        c.damage_callbacks = true;
    }
    f.effect_state.article_hitlag = Some(ItemKind::MarioCape);
}

/// ftMr_SpecialS_CreateCape (800E1248): once per swing (cmd_vars[2]), the
/// cape at the right thumb (it_802B2560: Item_InitSpawn, then
/// Item_AttachToParent at that part); the accessory then uninstalls.
pub fn create_cape(f: &mut Fighter, assets: &FighterAssets) {
    if f.commands.variables[2] != 0 {
        return;
    }
    f.commands.variables[2] = 1;
    let part = assets.parts.joint(FtPart::RThumbNb).expect("RThumbNb part");
    let c = &mut f.core;
    let hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(part),
        Vec3::ZERO,
    );
    let kind = attributes(f).cape_kind;
    let c = &mut f.core;
    let spawn = SpawnItem::attached(kind, c.player.id, hand, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
    });
    cape(f).present = true;
    install_callbacks(f);
    // accessory4_cb = NULL.
    f.character.get_mut::<Mario>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// ftMr_SpecialS_Reset (800E132C): the cape unfreezes and lets go of
/// Mario, taking the damage callbacks with it.
pub fn reset(f: &mut Fighter) {
    if cape(f).present {
        exit_hitlag(f);
    }
    let c = cape(f);
    c.present = false;
    c.damage_callbacks = false;
}

/// ftMr_SpecialS_ExitHitlag (800E13F8) on a live cape.
fn exit_hitlag(f: &mut Fighter) {
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::MarioCape,
        control: ItemControl::OwnerHitlag(false),
    });
}

/// ftMr_Init_OnTakeDamage -> ftMr_SpecialS_RemoveCape (800E1368): the take
/// damage and death2 callbacks while a cape is out.
pub fn remove_cape(f: &mut Fighter) {
    if !cape(f).damage_callbacks {
        return;
    }
    if cape(f).present {
        // it_802B2674: the cape resets its owner and is destroyed.
        reset(f);
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::MarioCape,
            control: ItemControl::Remove,
        });
        reset(f);
    }
}

/// ftMr_SpecialS_Anim (800E1550) / ftMr_SpecialAirS_Anim (800E158C).
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR;
        common::finish(f, p.assets, air)?;
    }
    Ok(None)
}

/// ftMr_SpecialS_IASA / ftMr_SpecialAirS_IASA are empty.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// The swing's gust at the hip, 3 units forward (lb_800119DC).
fn gust(f: &mut Fighter, assets: &FighterAssets, (strength, decay): (f32, f32)) {
    let hip = assets.parts.joint(FtPart::HipN).expect("HipN part");
    let c = &mut f.core;
    let mut center = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(hip),
        Vec3::ZERO,
    );
    center.x = gekko_math::fma::fmadds(GUST_FORWARD, c.physics.facing, center.x);
    c.commands
        .radial_impulses
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: GUST_FRAMES,
            strength,
            decay,
            phase_step: GUST_PHASE_STEP,
        });
}

/// reflect: cmd_vars[1] raises the reflector (ftColl_CreateReflectHit with
/// no hit callback) and lowers it (fp->reflecting = false); then
/// ftColl_8007AEF8 lets its position update.
fn reflect(f: &mut Fighter) {
    let raised = f.commands.variables[1];
    let reflecting = cape(f).reflecting;
    if raised == 1 && !reflecting {
        let descriptor = descriptor(&f.character.get::<Mario>().attributes.cape_reflection);
        let c = cape(f);
        c.reflecting = true;
        c.reflector = Some(descriptor);
        f.combat.reflector_enabled = true;
    } else if raised == 0 && reflecting {
        cape(f).reflecting = false;
        f.combat.reflector_enabled = false;
    }
    f.shield.reflect.volume.position_cached = false;
}

/// ftColl_CreateReflectHit (8007B240), literal descriptor transfer.
fn descriptor(a: &melee_ft::desc::fox_attributes::ReflectionAttributes) -> ReflectDescriptor {
    ReflectDescriptor {
        bone: a.joint as usize,
        maximum_damage: a.max_damage,
        offset: a.offset,
        radius: a.size,
        damage_multiplier: a.damage_multiplier,
        speed_multiplier: a.speed_multiplier,
        exclude_master_ball_ownership: a.skip_ownership_change != 0,
    }
}

/// The live reflector's contact (ftColl_80077464's reflect volume).
pub fn reflector_contact(
    f: &mut Fighter,
    hit: &melee_coll::hitbox::HitCapsule,
    scale: f32,
) -> Option<ReflectDescriptor> {
    if !f.combat.reflector_enabled {
        return None;
    }
    let descriptor = cape(f).reflector?;
    f.core
        .reflector_contact(hit, scale, &descriptor)
        .then_some(descriptor)
}

/// The cape's reflector has no hit callback (reflect_hit_cb = NULL).
pub fn reflect_hit(_: &mut Fighter, _: f32, _: &FighterAssets) -> Result<()> {
    Ok(())
}

/// ftMr_SpecialS_Phys (800E15D0): the gust once cmd_vars[0] reaches 1,
/// ft_80084F3C's friction, then the reflector.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] == 1 {
        f.commands.variables[0] = 2;
        gust(f, p.assets, GROUND_GUST);
    }
    callbacks::physics::guard_on(f, p);
    reflect(f);
}

/// ftMr_SpecialAirS_Phys (800E16E0): the first aerial cape since landing
/// rises (x2238_isCapeBoost), a later one stops; its own gravity from then
/// on; the cape's air friction; the reflector.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (boost, gravity, terminal, air_friction) = {
        let a = attributes(f);
        (
            a.vertical_boost,
            a.gravity,
            a.terminal_velocity,
            a.air_friction,
        )
    };
    if f.commands.variables[0] >= 1 {
        if f.commands.variables[0] == 1 {
            f.commands.variables[0] = 2;
            let mario = f.character.get_mut::<Mario>();
            let boosted = std::mem::replace(&mut mario.cape_boosted, true);
            f.physics.self_velocity.y = if boosted { 0.0 } else { boost };
            gust(f, p.assets, AIR_GUST);
        }
        f.physics.self_velocity.y = airborne::gravity(f.physics.self_velocity.y, gravity, terminal);
    } else {
        let air = &f.attributes.air;
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            air.gravity,
            air.terminal_velocity,
        );
    }
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, air_friction);
    f.core.finish_air_update(p.assets, p.wind);
    reflect(f);
}

/// collUpdateVars: the reflector stays up across the change, the callbacks
/// and the cape's accessory are installed again.
fn after_ground_air_change(f: &mut Fighter) {
    if cape(f).reflecting {
        f.combat.reflector_enabled = true;
    }
    install_callbacks(f);
    install_accessory(f);
}

/// ftMr_SpecialS_Coll (800E1840): ft_800827A0 stops at the edge; losing
/// the floor continues in the air (ftMr_SpecialS_GroundToAir, 800E18B8).
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Cape collision assets");
    common::ground_to_air(f, AIR, assets, GROUND_AIR_FLAGS)?;
    if f.commands.variables[0] == 1 {
        f.commands.variables[0] = 2;
    }
    after_ground_air_change(f);
    Ok(())
}

/// ftMr_SpecialAirS_Coll (800E187C): landing continues on the ground
/// (ftMr_SpecialAirS_AirToGround, 800E198C), the boost spent flag cleared.
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.character.get_mut::<Mario>().cape_boosted = false;
    let assets = p.assets.expect("Cape landing assets");
    common::air_to_ground(f, GROUND, assets, GROUND_AIR_FLAGS)?;
    after_ground_air_change(f);
    Ok(())
}
