//! Toad, ftpeachspecialn.c (8011E174..8011ED38): Peach holds Toad out and
//! a hit from in front turns into a spore counter.
use crate::init::{Accessory, Peach};
use hsd_types::Vec3;
use melee_coll::defense::AbsorbDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation,
    },
    physics::airborne,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftPe_MS_SpecialN..SpecialAirNHit (365..368).
pub const SPECIAL_N: ActionId = ActionId(365);
pub const SPECIAL_N_HIT: ActionId = ActionId(366);
pub const SPECIAL_AIR_N: ActionId = ActionId(367);
pub const SPECIAL_AIR_N_HIT: ActionId = ActionId(368);
/// fp->parts[109]: the joint Toad and the parasol hang from.
pub const ARTICLE_JOINT: usize = 109;
/// onUnkHit: the counter starts nine frames into its animation.
const COUNTER_START_FRAME: f32 = 9.0;
/// doHitAccessory4: each spore starts 2.5 above the article joint.
const SPORE_RISE: f32 = 2.5;

/// ftPe_SpecialN command variables (cmd_var_idx in ftpeachspecialn.c).
mod var {
    /// 0: the aerial boost, 1 armed by the script, 2 spent.
    pub const PHYSICS: usize = 0;
    /// 0: counter off, 1 raised by the script, 2 installed.
    pub const COUNTER: usize = 1;
    /// Nonzero once Toad has been drawn out.
    pub const TOAD_DRAWN: usize = 2;
    /// Raised by the counter's script once per spore.
    pub const SPORE: usize = 3;
}

/// Toad's counter volume and the strongest hit it caught this frame.
#[derive(Clone, Debug, Default)]
pub struct SpecialN {
    /// ftColl_8007B1B8 with ftPe_DatAttrs +AC: x221B_b0, front-only (b3).
    pub volume: Option<AbsorbDescriptor>,
    /// Fighter shield_unk0 / shield_unk1, attribute +A8.
    pub collision_multiplier: f32,
    /// Fighter +19A4 and specialn_facing_dir: strongest pending contact.
    pub pending: Option<(i32, f32)>,
}

/// ftPe_SpecialN_Enter (8011E3C8) / ftPe_SpecialAirN_Enter (8011E45C).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        // retail 8011E48C: fdivs.
        f.physics.self_velocity.x /= f
            .character
            .get::<Peach>()
            .attributes
            .toad
            .horizontal_momentum_divisor;
    } else {
        f.physics.self_velocity.y = 0.0;
    }
    let state = if air { SPECIAL_AIR_N } else { SPECIAL_N };
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(state, a).expect("Toad assets");
    f.character.get_mut::<Peach>().retained_word = retained_word;
    f.step_animation(a);
    // reset (inlined): the four command variables, then onAccessory4. The
    // facing copy (mv.pe.specialn, fctiwz) is never read.
    f.commands.variables[..4].fill(0);
    let peach = f.character.get_mut::<Peach>();
    peach.special_n = SpecialN::default();
    peach.accessory = Accessory::DrawToad;
    f.core.arm_accessory4();
}

/// onAccessory4 (8011E174): Toad hangs from joint 109 once, at the first
/// accessory pass of the move.
pub fn draw_toad(f: &mut Fighter) {
    if f.commands.variables[var::TOAD_DRAWN] != 0 {
        return;
    }
    f.commands.variables[var::TOAD_DRAWN] = 1;
    let c = &mut f.core;
    let position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        ARTICLE_JOINT,
        Vec3::ZERO,
    );
    let spawn = SpawnItem::attached(ItemKind::PeachToad, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnHeld(spawn));
    // fp->x1984_heldItemSpec, death2/take-damage callbacks and the hitlag
    // callbacks follow the item reference.
    let items = &mut f.character.get_mut::<Peach>().items;
    items.toad = true;
    items.death2_armed = true;
    items.take_damage_armed = true;
}

/// doHitAccessory4 (8011E1C8, inlined in onHitAccessory4): one spore
/// 2.5 above joint 109, on the stage plane.
pub fn release_spore(f: &mut Fighter) {
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        ARTICLE_JOINT,
        Vec3::ZERO,
    );
    position.y += SPORE_RISE;
    position.z = 0.0;
    let spawn =
        SpawnItem::attached(ItemKind::PeachToadSpore, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// doAnim (8011E4F8): the script raises the counter; each frame before
/// that clears the volume flag.
fn stance_anim(f: &mut Fighter, p: AnimationPhase<'_>, air: bool) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    match f.commands.variables[var::COUNTER] {
        1 => {
            f.commands.variables[var::COUNTER] = 2;
            let peach = f.character.get_mut::<Peach>();
            peach.special_n.volume = Some(counter_volume(peach));
            peach.special_n.collision_multiplier = peach.attributes.toad.collision_parameter;
        }
        0 => f.character.get_mut::<Peach>().special_n.volume = None,
        _ => {}
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.character.get_mut::<Peach>().special_n.volume = None;
        finish(f, p.assets, air)?;
    }
    Ok(None)
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    stance_anim(f, p, false)
}
pub fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    stance_anim(f, p, true)
}

/// doHitAnim (8011E8FC): each spore frame installs onHitAccessory4.
fn hit_anim_common(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    air: bool,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[var::SPORE] != 0 {
        f.commands.variables[var::SPORE] = 0;
        f.character.get_mut::<Peach>().accessory = Accessory::ReleaseSpore;
        f.core.arm_accessory4();
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets, air)?;
    }
    Ok(None)
}
pub fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hit_anim_common(f, p, false)
}
pub fn air_hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hit_anim_common(f, p, true)
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
fn finish(f: &mut Fighter, assets: &FighterAssets, air: bool) -> Result<()> {
    let state = if air {
        melee_types::CommonMotionState::Fall
    } else {
        melee_types::CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ftPe_SpecialN_IASA and every sibling: no interrupts.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPe_SpecialN_Phys / ftPe_SpecialNHit_Phys -> ft_80084F3C.
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    melee_ft::fighter::state::callbacks::physics::guard_on(f, p);
}

/// ftPe_SpecialAirN_Phys (8011E6AC): once the script arms it, the first
/// aerial Toad of an airtime rises; later ones stop. Then Toad's gravity.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let toad = f.character.get::<Peach>().attributes.toad.clone();
    // retail 8011E6D4: cmplwi, the variable is unsigned.
    if f.commands.variables[var::PHYSICS] >= 1 {
        if f.commands.variables[var::PHYSICS] == 1 {
            f.commands.variables[var::PHYSICS] = 2;
            let peach = f.character.get_mut::<Peach>();
            f.core.physics.self_velocity.y = if peach.aerial_toad_used {
                0.0
            } else {
                peach.aerial_toad_used = true;
                toad.first_air_vertical_speed
            };
        }
        f.physics.self_velocity.y =
            airborne::gravity(f.physics.self_velocity.y, toad.gravity, toad.terminal_velocity);
    } else {
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            f.attributes.air.gravity,
            f.attributes.air.terminal_velocity,
        );
    }
    apply_air_friction(f, toad.air_friction);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPe_SpecialAirNHit_Phys (8011EA90).
pub fn air_hit_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let toad = f.character.get::<Peach>().attributes.toad.clone();
    f.physics.self_velocity.y =
        airborne::gravity(f.physics.self_velocity.y, toad.gravity, toad.terminal_velocity);
    apply_air_friction(f, toad.air_friction);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftCommon_ApplyFrictionAir (8007CE94): inclusive clamp, separate subtract.
fn apply_air_friction(f: &mut Fighter, friction: f32) {
    let x = f.physics.self_velocity.x;
    f.physics.animation_velocity.x = if friction.abs() >= x.abs() {
        -x
    } else if x > 0.0 {
        -friction
    } else {
        friction
    };
}

/// ftPe_SpecialN_Coll / ftPe_SpecialNHit_Coll (8011E75C / 8011EAE0):
/// ft_800827A0, and off the edge the aerial counterpart (doColl /
/// doHitColl).
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::ground::{map_escape, WaitGroundResult};
    let c = &mut f.core;
    if !matches!(
        map_escape(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        WaitGroundResult::Supported
    ) {
        f.leave_ground();
        let stance = f.motion_state.action == SPECIAL_N;
        let state = if stance {
            SPECIAL_AIR_N
        } else {
            SPECIAL_AIR_N_HIT
        };
        transition(f, p.assets.expect("Toad collision assets"), state)?;
        if stance && f.commands.variables[var::PHYSICS] == 1 {
            f.commands.variables[var::PHYSICS] = 2;
        }
    }
    Ok(())
}

/// ftPe_SpecialAirN_Coll / ftPe_SpecialAirNHit_Coll (8011E798 / 8011EB1C):
/// ft_80081D0C, landing into the ground counterpart (doAirColl /
/// doAirHitColl), which gives back the aerial boost.
pub fn air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        f.character.get_mut::<Peach>().aerial_toad_used = false;
        f.land();
        let state = if f.motion_state.action == SPECIAL_AIR_N {
            SPECIAL_N
        } else {
            SPECIAL_N_HIT
        };
        transition(f, p.assets.expect("Toad landing assets"), state)?;
    }
    Ok(())
}

/// Fighter_ChangeMotionState with coll_mf (0x0C4C508C: hit status and
/// hitboxes kept, no effects), then setupColl / setupHitColl.
fn transition(f: &mut Fighter, a: &FighterAssets, state: ActionId) -> Result<()> {
    f.change_ground_air_motion(
        state,
        a,
        MotionPreservation {
            hit_status: true,
            hitboxes: true,
            ..Default::default()
        },
    )?;
    // Fighter_ChangeMotionState zeroes shield_unk0/1 and x221B_b0; setupColl
    // re-installs the volume (without them) once the counter is raised.
    let stance = state == SPECIAL_N || state == SPECIAL_AIR_N;
    let peach = f.character.get_mut::<Peach>();
    peach.special_n.collision_multiplier = 0.0;
    peach.special_n.volume = None;
    // setupColl / setupHitColl: the callbacks return while Toad is out.
    arm_toad_callbacks(peach);
    if stance {
        // setupColl re-installs onAccessory4; Toad is already out, so it
        // does nothing more.
        if f.commands.variables[var::COUNTER] == 2 {
            let peach = f.character.get_mut::<Peach>();
            peach.special_n.volume = Some(counter_volume(peach));
        }
    }
    Ok(())
}

/// setupColl / setupHitColl: death2_cb and take_dmg_cb while Toad is out.
fn arm_toad_callbacks(peach: &mut Peach) {
    if peach.items.toad {
        peach.items.death2_armed = true;
        peach.items.take_damage_armed = true;
    }
}

/// ftPe_DatAttrs +AC, installed by ftColl_8007B1B8.
fn counter_volume(peach: &Peach) -> AbsorbDescriptor {
    let v = &peach.attributes.toad_volume;
    AbsorbDescriptor {
        bone: v.bone as usize,
        offset: v.offset,
        radius: v.radius,
    }
}

/// onUnkHit (8011EC6C), Toad's shield_hit_cb: the counter motion from
/// frame nine, Toad's spore animation. Peach does not turn: the caught
/// facing only reaches mv.pe.specialn, which nothing reads.
pub fn process_hit(f: &mut Fighter, a: &FighterAssets) {
    let Some((damage, _facing)) = f.character.get_mut::<Peach>().special_n.pending.take() else {
        return;
    };
    f.character.get_mut::<Peach>().special_n.volume = None;
    let state = if f.physics.ground_or_air == GroundOrAir::Air {
        SPECIAL_AIR_N_HIT
    } else {
        SPECIAL_N_HIT
    };
    f.change_motion_state_at(state, a, COUNTER_START_FRAME)
        .expect("Toad counter assets");
    f.step_animation(a);
    if f.character.get::<Peach>().items.toad {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: ItemKind::PeachToad,
            control: ItemControl::Counter,
        });
    }
    arm_toad_callbacks(f.character.get_mut::<Peach>());
    // Fighter_ProcessHit: x19A4 sets the hitlag (fighter.c:2965).
    f.combat.dealt_damage = damage;
}

/// Toad's volume where joint `bone` holds it this frame (ftColl_8007B1B8's
/// shield_hit, placed by lbColl_80007BCC). Fighter_ChangeMotionState clears
/// x221B_b0 on every motion change and only the two stance rows raise it
/// again, so a volume left over outside them is stale.
fn toad_volume(f: &mut Fighter) -> Option<melee_ft::fighter::damage::DefenseVolume> {
    if f.motion_state.action != SPECIAL_N && f.motion_state.action != SPECIAL_AIR_N {
        f.character.get_mut::<Peach>().special_n.volume = None;
        return None;
    }
    let volume = f.character.get::<Peach>().special_n.volume.as_ref()?;
    let (bone, offset, radius) = (volume.bone, volume.offset, volume.radius);
    let c = &mut f.core;
    let position =
        melee_ft::fighter::caches::bone_position(&mut c.skeleton, c.animation.root, bone, offset);
    let matrix = *c.skeleton.get_mtx(c.animation.parts[bone].joint);
    Some(melee_ft::fighter::damage::DefenseVolume {
        position,
        matrix,
        radius,
        // onAnim sets x221B_b3 (front only), not x221B_b1.
        fixed_bounce: false,
    })
}

/// x221B_b3 (ftcoll.c:1803-1815, 2247-2256): Toad only catches what is in
/// front of Peach.
fn in_front(f: &Fighter, x: f32) -> bool {
    if f.physics.facing == -1.0 {
        f.physics.position.x >= x
    } else {
        f.physics.position.x <= x
    }
}

/// lbColl_80007BCC: a swept hit capsule against Toad's volume.
fn volume_contact(
    f: &Fighter,
    volume: &melee_ft::fighter::damage::DefenseVolume,
    hit: &melee_coll::hitbox::HitCapsule,
    attacker_scale: f32,
) -> Option<melee_coll::geometry::Contact> {
    use melee_coll::geometry::{capsule_contact, Capsule};
    capsule_contact(
        Capsule {
            start: hit.previous_position,
            end: hit.position,
            radius: hit.descriptor.radius
                * if hit.descriptor.ignore_scale {
                    1.0
                } else {
                    attacker_scale
                },
        },
        Capsule {
            start: volume.position,
            end: volume.position,
            radius: volume.radius,
        },
        &volume.matrix,
        20.0 * f.core.player.scale,
    )
}

/// ftColl_8007925C's shield step against Toad, then ftColl_80077688.
pub fn item_contact(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    let Some(volume) = toad_volume(f) else {
        return false;
    };
    if !in_front(f, item.position.x) {
        return false;
    }
    let hit = item.hitboxes[id].clone().expect("eligible item hit");
    let Some(contact) = volume_contact(f, &volume, &hit, item.scale) else {
        return false;
    };
    let own_hitlag = f.character.get::<Peach>().special_n.collision_multiplier;
    let damage = f.record_item_volume_hit(item, id, contact, &volume, own_hitlag, assets);
    let facing = if f.physics.position.x > item.position.x {
        -1.0
    } else {
        1.0
    };
    let scratch = &mut f.character.get_mut::<Peach>().special_n;
    if scratch.pending.is_none_or(|(old, _)| damage > old) {
        scratch.pending = Some((damage, facing));
    }
    if item.hit_flags[id].defense_interaction {
        f.combat.minimum_hitlag = own_hitlag;
    }
    true
}

/// ftColl_8007B1B8 / lbColl_80007BCC before ordinary hurt-capsule tests,
/// then ftColl_80076CBC.
pub fn contact(f: &mut Fighter, attacker: &mut Fighter, _: &FighterAssets, id: usize) -> bool {
    let Some(volume) = toad_volume(f) else {
        return false;
    };
    if !in_front(f, attacker.physics.position.x) {
        return false;
    }
    let hit = attacker.commands.hitboxes[id]
        .as_ref()
        .expect("eligible hit");
    let Some(contact) = volume_contact(f, &volume, hit, attacker.player.scale) else {
        return false;
    };
    let damage = gekko_math::msl::fctiwz(hit.descriptor.damage).max(1);
    let group = hit.descriptor.group;
    let shield_damage = hit.descriptor.shield_damage;
    let facing = if f.physics.position.x > attacker.physics.position.x {
        -1.0
    } else {
        1.0
    };
    let scratch = &mut f.character.get_mut::<Peach>().special_n;
    if scratch.pending.is_none_or(|(old, _)| damage > old) {
        scratch.pending = Some((damage, facing));
    }
    let minimum = scratch.collision_multiplier;
    f.combat.minimum_hitlag = minimum;
    attacker.combat.minimum_hitlag = minimum;
    attacker.record_shield_recoil(damage, f.shield.lightshield, -facing);
    melee_coll::detection::record_victim(&mut attacker.commands.hitboxes, group, f.spawn_number);
    f.shield_contact_feedback(damage, shield_damage, contact.position);
    true
}
