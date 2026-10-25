//! Toad, ftpeachspecialn.c (8011E174..8011ED38): Peach holds Toad out and
//! a hit from in front turns into a spore counter. Kirby's copy
//! (ftkirbyspecialpeach.c, 8010C288..8010CE5C) is the same code over his
//! rows 449..452, his attributes (ftKb_DatAttrs +178..+18C, +3EC), his
//! article kinds and his bone 44; [`ToadUser`] carries those differences.
use hsd_archive::Reader;
use hsd_types::Vec3;
use melee_coll::defense::AbsorbDescriptor;
use melee_ft::{
    anim::WaitChoice,
    desc::FighterDescError,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation, MotionRow,
    },
    physics::airborne,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// onUnkHit: the counter starts nine frames into its animation.
const COUNTER_START_FRAME: f32 = 9.0;
/// doHitAccessory4: each spore starts 2.5 above the article joint.
const SPORE_RISE: f32 = 2.5;

/// The command variables (cmd_var_idx in ftpeachspecialn.c).
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

/// ftPe_DatAttrs +94..+A8 (`specialn_*`), ftKb_DatAttrs +178..+18C
/// (`specialn_pe_*`): the same six words.
#[derive(Clone, Debug, PartialEq)]
pub struct ToadAttributes {
    /// +0: the aerial entry divides the horizontal speed by this.
    pub horizontal_momentum_divisor: f32,
    /// +4: the aerial rows' air friction.
    pub air_friction: f32,
    /// +8: the first aerial Toad of an airtime rises at this speed.
    pub first_air_vertical_speed: f32,
    /// +C, +10: the aerial rows' gravity and terminal speed.
    pub gravity: f32,
    pub terminal_velocity: f32,
    /// +14: shield_unk0 / shield_unk1 while the counter is raised.
    pub collision_parameter: f32,
}
impl ToadAttributes {
    pub fn read(r: &Reader, at: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            horizontal_momentum_divisor: r.f32(at)?,
            air_friction: r.f32(at + 0x4)?,
            first_air_vertical_speed: r.f32(at + 0x8)?,
            gravity: r.f32(at + 0xC)?,
            terminal_velocity: r.f32(at + 0x10)?,
            collision_parameter: r.f32(at + 0x14)?,
        })
    }
}

/// An AbsorbDesc (ftPe_DatAttrs +AC, ftKb_DatAttrs +3EC): Toad's counter
/// volume.
#[derive(Clone, Debug, PartialEq)]
pub struct CounterVolume {
    /// +0: signed joint index.
    pub bone: i32,
    /// +4: local collision offset.
    pub offset: Vec3,
    /// +10: collision radius.
    pub radius: f32,
}
impl CounterVolume {
    pub fn read(r: &Reader, at: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            bone: r.s32(at)?,
            offset: Vec3::new(r.f32(at + 0x4)?, r.f32(at + 0x8)?, r.f32(at + 0xC)?),
            radius: r.f32(at + 0x10)?,
        })
    }
}

/// Toad's counter volume and the strongest hit it caught this frame.
#[derive(Clone, Debug, Default)]
pub struct SpecialN {
    /// ftColl_8007B1B8 with the AbsorbDesc: x221B_b0, front-only (b3).
    pub volume: Option<AbsorbDescriptor>,
    /// Fighter shield_unk0 / shield_unk1, attribute +14.
    pub collision_multiplier: f32,
    /// Fighter +19A4 and specialn_facing_dir: strongest pending contact.
    pub pending: Option<(i32, f32)>,
}

/// The one-shot accessory4 callbacks Toad installs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToadAccessory {
    /// onAccessory4 (8011E174) / fn_8010C288: Toad hangs from the joint.
    DrawToad,
    /// onHitAccessory4 (8011E230) / fn_8010C344: one of Toad's spores.
    ReleaseSpore,
}

/// What Toad's states read and write of the fighter using it.
pub trait ToadUser: 'static {
    /// The stance, counter, aerial stance and aerial counter rows, in that
    /// order (ftPe_MS_SpecialN..SpecialAirNHit, ftKb_MS_PeSpecialLw..
    /// PeSpecialAirLwHit).
    const FIRST_ROW: u16;
    /// fp->parts[] index Toad hangs from and the spores leave: Peach's
    /// FtPart_109, Kirby's FtPart_R3rdNa (44) used as a bone index.
    const ARTICLE_JOINT: usize;
    const TOAD: ItemKind;
    const SPORE: ItemKind;
    fn attributes(f: &Fighter) -> &ToadAttributes;
    fn volume(f: &Fighter) -> &CounterVolume;
    fn scratch(f: &mut Fighter) -> &mut SpecialN;
    /// u.pe.specialairn_used / u.kb.xCC: the aerial lift is spent.
    fn lift_used(f: &mut Fighter) -> &mut bool;
    /// u.pe.toad_gobj / u.kb.xD0 is set.
    fn toad_out(f: &Fighter) -> bool;
    /// onAccessory4's reference to the new Toad, with the death2 and
    /// take-damage callbacks it brings (Peach's x1984_heldItemSpec too).
    fn hold_toad(f: &mut Fighter);
    /// setupColl / setupHitColl: death2_cb and take_dmg_cb while Toad is
    /// out (ftPe_Init_OnDeath2, or Kirby's ftKb_Init_800EE74C / 800EE7B8).
    fn arm_damage_callbacks(f: &mut Fighter);
    /// accessory4_cb.
    fn install_accessory(f: &mut Fighter, accessory: ToadAccessory);
    /// The entry's view of mv+4 before the motion change (Peach's specials
    /// write only mv+0 and keep the rest).
    fn keep_scratch_word(_f: &mut Fighter, _word: Option<f32>) {}
}

const fn row(
    action: u16,
    animation: i32,
    anim: melee_ft::fighter::state::AnimFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: ActionId(action),
        id: melee_types::CommonMotionState::None,
        animation,
        anim,
        iasa: input,
        physics,
        collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// The four rows from `C::FIRST_ROW`, their submotions from
/// `first_animation`: stance, counter, aerial stance, aerial counter.
pub const fn rows<C: ToadUser>(first_animation: i32) -> [MotionRow; 4] {
    let a = C::FIRST_ROW;
    [
        row(a, first_animation, anim::<C>, physics, collision::<C>),
        row(
            a + 1,
            first_animation + 1,
            hit_anim::<C>,
            physics,
            collision::<C>,
        ),
        row(
            a + 2,
            first_animation + 2,
            air_anim::<C>,
            air_physics::<C>,
            air_collision::<C>,
        ),
        row(
            a + 3,
            first_animation + 3,
            air_hit_anim::<C>,
            air_hit_physics::<C>,
            air_collision::<C>,
        ),
    ]
}

const fn stance<C: ToadUser>() -> ActionId {
    ActionId(C::FIRST_ROW)
}
const fn counter<C: ToadUser>() -> ActionId {
    ActionId(C::FIRST_ROW + 1)
}
const fn air_stance<C: ToadUser>() -> ActionId {
    ActionId(C::FIRST_ROW + 2)
}
const fn air_counter<C: ToadUser>() -> ActionId {
    ActionId(C::FIRST_ROW + 3)
}

/// Whether `action` is one of the user's four Toad rows
/// (ftPe_SpecialN_IsActive / ftKb_SpecialNPe_8010C4AC, negated).
pub fn is_toad_row<C: ToadUser>(action: ActionId) -> bool {
    (C::FIRST_ROW..C::FIRST_ROW + 4).contains(&action.0)
}

/// ftPe_SpecialN_Enter (8011E3C8) / ftPe_SpecialAirN_Enter (8011E45C), and
/// Kirby's ftKb_SpecialNPe_8010C4D4 / 8010C560.
pub fn enter<C: ToadUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        // retail 8011E48C (Kirby's 8010C590): fdivs.
        f.physics.self_velocity.x /= C::attributes(f).horizontal_momentum_divisor;
    } else {
        f.physics.self_velocity.y = 0.0;
    }
    let state = if air {
        air_stance::<C>()
    } else {
        stance::<C>()
    };
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(state, a).expect("Toad assets");
    C::keep_scratch_word(f, retained_word);
    f.step_animation(a);
    // reset (inlined): the four command variables, then onAccessory4. The
    // facing copy (mv.pe.specialn, fctiwz) is never read.
    f.commands.variables[..4].fill(0);
    *C::scratch(f) = SpecialN::default();
    C::install_accessory(f, ToadAccessory::DrawToad);
    f.core.arm_accessory4();
}

/// onAccessory4 (8011E174) / fn_8010C288: Toad hangs from the article
/// joint once, at the first accessory pass of the move.
pub fn draw_toad<C: ToadUser>(f: &mut Fighter) {
    if f.commands.variables[var::TOAD_DRAWN] != 0 {
        return;
    }
    f.commands.variables[var::TOAD_DRAWN] = 1;
    let c = &mut f.core;
    let position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::ARTICLE_JOINT,
        Vec3::ZERO,
    );
    let spawn = SpawnItem::attached(C::TOAD, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnHeld(spawn));
    // The death2/take-damage callbacks and the hitlag callbacks follow the
    // item reference.
    C::hold_toad(f);
    // onEnterHitlag / onExitHitlag (8011E348 / 8011E378), fn_8010C44C /
    // ftKb_SpecialNPe_8010C47C: Toad freezes for the owner's hitlag
    // (it_802BDFA0 / it_802BDFC0).
    f.effect_state.article_hitlag = Some(C::TOAD);
}

/// doHitAccessory4 (8011E1C8, inlined in onHitAccessory4) / fn_8010C344:
/// one spore 2.5 above the article joint, on the stage plane.
pub fn release_spore<C: ToadUser>(f: &mut Fighter) {
    let c = &mut f.core;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::ARTICLE_JOINT,
        Vec3::ZERO,
    );
    position.y += SPORE_RISE;
    position.z = 0.0;
    let spawn = SpawnItem::attached(C::SPORE, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// doAnim (8011E4F8) / doPeAnim: the script raises the counter; each frame
/// before that clears the volume flag.
fn stance_anim<C: ToadUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    air: bool,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    match f.commands.variables[var::COUNTER] {
        1 => {
            f.commands.variables[var::COUNTER] = 2;
            let volume = counter_volume::<C>(f);
            let multiplier = C::attributes(f).collision_parameter;
            let scratch = C::scratch(f);
            scratch.volume = Some(volume);
            scratch.collision_multiplier = multiplier;
        }
        0 => C::scratch(f).volume = None,
        _ => {}
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        C::scratch(f).volume = None;
        finish(f, p.assets, air)?;
    }
    Ok(None)
}
fn anim<C: ToadUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    stance_anim::<C>(f, p, false)
}
fn air_anim<C: ToadUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    stance_anim::<C>(f, p, true)
}

/// doHitAnim (8011E8FC) / doPeHitAnim: each spore frame installs
/// onHitAccessory4.
fn hit_anim_common<C: ToadUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
    air: bool,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[var::SPORE] != 0 {
        f.commands.variables[var::SPORE] = 0;
        C::install_accessory(f, ToadAccessory::ReleaseSpore);
        f.core.arm_accessory4();
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        finish(f, p.assets, air)?;
    }
    Ok(None)
}
fn hit_anim<C: ToadUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hit_anim_common::<C>(f, p, false)
}
fn air_hit_anim<C: ToadUser>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    hit_anim_common::<C>(f, p, true)
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
fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPe_SpecialN_Phys / ftPe_SpecialNHit_Phys -> ft_80084F3C.
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftPe_SpecialAirN_Phys (8011E6AC) / ftKb_PeSpecialAirLw_Phys: once the
/// script arms it, the first aerial Toad of an airtime rises; later ones
/// stop. Then Toad's gravity.
fn air_physics<C: ToadUser>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let toad = C::attributes(f).clone();
    // retail 8011E6D4: cmplwi, the variable is unsigned.
    if f.commands.variables[var::PHYSICS] >= 1 {
        if f.commands.variables[var::PHYSICS] == 1 {
            f.commands.variables[var::PHYSICS] = 2;
            let used = C::lift_used(f);
            let speed = if *used {
                0.0
            } else {
                *used = true;
                toad.first_air_vertical_speed
            };
            f.core.physics.self_velocity.y = speed;
        }
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            toad.gravity,
            toad.terminal_velocity,
        );
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

/// ftPe_SpecialAirNHit_Phys (8011EA90) / ftKb_PeSpecialAirLwHit_Phys.
fn air_hit_physics<C: ToadUser>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let toad = C::attributes(f).clone();
    f.physics.self_velocity.y = airborne::gravity(
        f.physics.self_velocity.y,
        toad.gravity,
        toad.terminal_velocity,
    );
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
/// doHitColl; Kirby's ftKb_SpecialNPe_8010C8D8 / ftKb_SpecialNGw_8010CC6C).
fn collision<C: ToadUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        let stance = f.motion_state.action == stance::<C>();
        let state = if stance {
            air_stance::<C>()
        } else {
            air_counter::<C>()
        };
        transition::<C>(f, p.assets.expect("Toad collision assets"), state)?;
        if stance && f.commands.variables[var::PHYSICS] == 1 {
            f.commands.variables[var::PHYSICS] = 2;
        }
    }
    Ok(())
}

/// ftPe_SpecialAirN_Coll / ftPe_SpecialAirNHit_Coll (8011E798 / 8011EB1C):
/// ft_80081D0C, landing into the ground counterpart (doAirColl /
/// doAirHitColl; Kirby's 8010C9CC / ftKb_SpecialNGw_8010CCD4), which gives
/// back the aerial boost.
fn air_collision<C: ToadUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
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
        *C::lift_used(f) = false;
        f.land();
        let state = if f.motion_state.action == air_stance::<C>() {
            stance::<C>()
        } else {
            counter::<C>()
        };
        transition::<C>(f, p.assets.expect("Toad landing assets"), state)?;
    }
    Ok(())
}

/// Fighter_ChangeMotionState with coll_mf (0x0C4C508C: hit status and
/// hitboxes kept, no effects), then setupColl / setupHitColl.
fn transition<C: ToadUser>(f: &mut Fighter, a: &FighterAssets, state: ActionId) -> Result<()> {
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
    let is_stance = state == stance::<C>() || state == air_stance::<C>();
    let scratch = C::scratch(f);
    scratch.collision_multiplier = 0.0;
    scratch.volume = None;
    // setupColl / setupHitColl: the callbacks return while Toad is out.
    arm_toad_callbacks::<C>(f);
    // setupColl re-installs onAccessory4; Toad is already out, so it does
    // nothing more.
    if is_stance && f.commands.variables[var::COUNTER] == 2 {
        let volume = counter_volume::<C>(f);
        C::scratch(f).volume = Some(volume);
    }
    Ok(())
}

/// setupColl / setupHitColl: death2_cb and take_dmg_cb while Toad is out,
/// and the hitlag pair always (onEnterHitlag checks for Toad itself).
fn arm_toad_callbacks<C: ToadUser>(f: &mut Fighter) {
    if C::toad_out(f) {
        C::arm_damage_callbacks(f);
    }
    f.effect_state.article_hitlag = Some(C::TOAD);
}

/// The AbsorbDesc, installed by ftColl_8007B1B8.
fn counter_volume<C: ToadUser>(f: &Fighter) -> AbsorbDescriptor {
    let v = C::volume(f);
    AbsorbDescriptor {
        bone: v.bone as usize,
        offset: v.offset,
        radius: v.radius,
    }
}

/// onUnkHit (8011EC6C) / fn_8010CD88, Toad's shield_hit_cb: the counter
/// motion from frame nine, Toad's spore animation. The fighter does not
/// turn: the caught facing only reaches mv.pe.specialn, which nothing reads.
pub fn process_hit<C: ToadUser>(f: &mut Fighter, a: &FighterAssets) {
    let Some((damage, _facing)) = C::scratch(f).pending.take() else {
        return;
    };
    C::scratch(f).volume = None;
    let state = if f.physics.ground_or_air == GroundOrAir::Air {
        air_counter::<C>()
    } else {
        counter::<C>()
    };
    f.change_motion_state_at(state, a, COUNTER_START_FRAME)
        .expect("Toad counter assets");
    f.step_animation(a);
    if C::toad_out(f) {
        f.core.item_requests.push(ItemRequest::Control {
            owner: f.player.id,
            kind: C::TOAD,
            control: ItemControl::Counter,
        });
    }
    arm_toad_callbacks::<C>(f);
    // Fighter_ProcessHit: x19A4 sets the hitlag (fighter.c:2965).
    f.combat.dealt_damage = damage;
}

/// Toad's volume where joint `bone` holds it this frame (ftColl_8007B1B8's
/// shield_hit, placed by lbColl_80007BCC). Fighter_ChangeMotionState clears
/// x221B_b0 on every motion change and only the two stance rows raise it
/// again, so a volume left over outside them is stale.
fn toad_volume<C: ToadUser>(f: &mut Fighter) -> Option<melee_ft::fighter::damage::DefenseVolume> {
    if f.motion_state.action != stance::<C>() && f.motion_state.action != air_stance::<C>() {
        C::scratch(f).volume = None;
        return None;
    }
    let volume = C::scratch(f).volume.as_ref()?;
    let (bone, offset, radius) = (volume.bone, volume.offset, volume.radius);
    let c = &mut f.core;
    let (position, matrix) = c.defense_volume_placement(bone, offset);
    Some(melee_ft::fighter::damage::DefenseVolume {
        position,
        matrix,
        radius,
        // onAnim sets x221B_b3 (front only), not x221B_b1.
        fixed_bounce: false,
        no_bounce: false,
    })
}

/// x221B_b3 (ftcoll.c:1803-1815, 2247-2256): Toad only catches what is in
/// front of its holder.
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
pub fn item_contact<C: ToadUser>(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    let Some(volume) = toad_volume::<C>(f) else {
        return false;
    };
    if !in_front(f, item.position.x) {
        return false;
    }
    let hit = item.hitboxes[id].clone().expect("eligible item hit");
    let Some(contact) = volume_contact(f, &volume, &hit, item.scale) else {
        return false;
    };
    let own_hitlag = C::scratch(f).collision_multiplier;
    let damage = f.record_item_volume_hit(item, id, contact, &volume, own_hitlag, assets);
    let facing = if f.physics.position.x > item.position.x {
        -1.0
    } else {
        1.0
    };
    let scratch = C::scratch(f);
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
pub fn contact<C: ToadUser>(
    f: &mut Fighter,
    attacker: &mut Fighter,
    _: &FighterAssets,
    id: usize,
) -> bool {
    let Some(volume) = toad_volume::<C>(f) else {
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
    let scratch = C::scratch(f);
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
