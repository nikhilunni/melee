//! Dr. Mario's Megavitamin (It_Kind_DrMario_Vitamin), itdrmariopill.c
//! (802C0510..802C1590).
//!
//! Two spawns share the kind. The neutral special throws one
//! (itDrMarioPill_Spawn): it falls under ItemAttr gravity, bounces along the
//! floor and ends when its lifetime runs out, a bounce leaves it slower than
//! attribute x10, or its hit connects. The side taunt holds one
//! (itDrMarioPill_Appeal_Spawn) at its owner's position until he leaves the
//! taunt. Motions 3..6 (the sleeping and star-KO poses, itDrMarioPill_802C09C4)
//! are unported.
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct DrMarioPill;

/// it_803F75D0's anim_id column.
pub const ARTICLE_STATES: [i32; 7] = [0, 1, 2, 1, 3, 4, 5];
/// itDrMarioPillAttributes x0..x10.
pub const SPECIAL_ATTRIBUTES: u32 = 5;
/// ftData.x48_items index (ftDr_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;

/// The thrown pill's motion.
const THROWN: u16 = 0;
/// itDrMarioPill_802C0DF8's motion, which Appeal_Spawn leaves at once.
const TAUNT_START: u16 = 1;
/// The taunt pill's motion.
const TAUNT: u16 = 2;
/// itDrMarioPill_802C0DF8: the taunt pill's lifetime (never counted down).
const TAUNT_LIFETIME: f32 = 1200.0;
/// ftMr_MS_AppealSR / AppealSL: the owner's taunt (ftDr_Init_80149844).
const OWNER_TAUNT_MOTIONS: std::ops::RangeInclusive<u16> = 341..=342;
/// Item_8026AE84(ip, 0x15FAE, 0x7F, 0x40): the bounce and hit sound.
const SOUND: u32 = 0x1_5FAE;

/// `spawn_argument` of a taunt pill: this bit over the colour
/// (itDrMarioPill_Appeal_Spawn rather than itDrMarioPill_Spawn).
const TAUNT_SPAWN: i32 = 1 << 8;

/// The `spawn_argument` of a thrown pill of `colour` (0..9).
pub const fn thrown_argument(colour: i32) -> i32 {
    colour
}
/// The `spawn_argument` of a taunt pill of `colour` (0..9).
pub const fn taunt_argument(colour: i32) -> i32 {
    colour | TAUNT_SPAWN
}

/// The pill's special attributes (itDrMarioPillAttributes).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the launch speed.
    fn speed(&self) -> f32 {
        self.0[0]
    }
    /// x4: the launch angle, in radians.
    fn angle(&self) -> f32 {
        self.0[1]
    }
    /// x8: the lifetime, in frames.
    fn lifetime(&self) -> f32 {
        self.0[2]
    }
    /// x10: a bounce slower than this ends the pill.
    fn minimum_bounce_speed(&self) -> f32 {
        self.0[4]
    }
}

const fn unported(motion: u16) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion as usize],
        animation: unported_animation,
        physics: unported_physics,
        collision: unported_collision,
    }
}

static STATES: [ItemStateRow; 7] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: thrown_animation,
        physics: thrown_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: taunt_animation,
        physics: taunt_physics,
        collision: taunt_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: taunt_animation,
        physics: taunt_physics,
        collision: taunt_collision,
    },
    unported(3),
    unported(4),
    unported(5),
    unported(6),
];

/// it_3F2F.c's Dr Mario pill row.
impl ItemLogic for DrMarioPill {
    const KIND: ItemKind = ItemKind::DrMarioVitamin;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8026B3A8 clears xDC8 x15 in every callback: never grabbable.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// Both spawns end with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let colour = (spawn.spawn_argument & !TAUNT_SPAWN) as f32;
        if spawn.spawn_argument & TAUNT_SPAWN == 0 {
            throw(item, assets, common, colour);
        } else {
            hold_for_taunt(item, assets, common, colour);
        }
    }
    /// itDrMarioPill_DmgDealt (802C1384).
    fn damage_dealt(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.sound_requests.push(SOUND);
        true
    }
    /// itDrMarioPill_Reflected (802C13CC) -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itDrMarioPill_Clanked (802C1400).
    fn clanked(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.sound_requests.push(SOUND);
        true
    }
    /// itDrMarioPill_HitShield (802C145C).
    fn hit_shield(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.sound_requests.push(SOUND);
        true
    }
    /// itDrMarioPill_Absorbed (802C14B8).
    fn absorbed(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.sound_requests.push(SOUND);
        true
    }
    /// itDrMarioPill_ShieldBounced (802C1514): the sound and the end, like
    /// the other contacts (no itColl_BounceOffShield).
    fn shield_bounced(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        item.sound_requests.push(SOUND);
        true
    }
    /// itDrMarioPill_802C0DBC: the owner's ftDr_Init_801497CC destroys the
    /// taunt pill (xDD4.x4 cleared first, so its owner is not called back).
    /// A thrown pill of the same owner is not the one +2240 points at.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove if item.motion == TAUNT => item.destroyed = true,
            ItemControl::Remove => {}
            _ => unreachable!("{control:?} sent to a Megavitamin"),
        }
    }
    /// itDrMarioPill_802C061C, the taunt pill's on_accessory: it sits at
    /// its owner's position (the JObj's Y rotation, M_PI_2 * facing, and
    /// Z rotation are the model's alone).
    fn accessory(
        item: &mut ItemCore,
        owner: Option<&ItemOwner>,
        _assets: &ItemAssets,
    ) -> Option<OwnerBlast> {
        if item.motion != TAUNT {
            return None;
        }
        if let Some(owner) = owner {
            item.position = owner.position;
            item.root_translation = owner.position;
        }
        None
    }
    /// Only the taunt pill that let go of its owner itself (ftDr_Init_801498A0
    /// in itDrMarioPill_Motion2_Anim) reaches the owner.
    fn notifies_owner(item: &ItemCore) -> bool {
        item.motion == TAUNT
    }
}

/// itDrMarioPill_Spawn's set-up: itDrMarioPill_802C0B5C (802C0B5C) launches
/// along the attribute angle with the lifetime in motion 0, then
/// it_80273670(gobj, 0, colour) poses the colour's frame and removes the
/// animation and script. No fused sites: separate fmuls.
fn throw(
    item: &mut ItemCore,
    assets: &ItemAssets,
    common: &melee_it::desc::ItemCommonData,
    colour: f32,
) {
    let a = Attributes(&assets.special_attributes);
    let (speed, angle) = (a.speed(), a.angle());
    item.velocity.x = item.facing * (speed * gekko_math::msl::cosf(angle));
    item.velocity.y = speed * gekko_math::msl::sinf(angle);
    item.velocity.z = 0.0;
    set_lifetime(item, common, a.lifetime());
    item.change_motion_with(THROWN, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    item.grabbable = false;
    item.pose_article_frame(colour);
}

/// itDrMarioPill_Appeal_Spawn's set-up (802C0850): motion 1 with the fixed
/// lifetime (itDrMarioPill_802C0DF8), the colour's frame of article state 1
/// (it_80273670), then motion 2; held by its owner (xDC8 x13), without
/// spin (it_80274740) and intangible (it_802756D0).
fn hold_for_taunt(
    item: &mut ItemCore,
    assets: &ItemAssets,
    common: &melee_it::desc::ItemCommonData,
    colour: f32,
) {
    set_lifetime(item, common, TAUNT_LIFETIME);
    item.change_motion_with(TAUNT_START, ARTICLE_STATES[1], ANIM_UPDATE, assets);
    item.grabbable = false;
    item.pose_article_frame(colour);
    item.change_motion_with(TAUNT, ARTICLE_STATES[2], ANIM_UPDATE, assets);
    item.command_variables = [0; 4];
    item.spin_ignores_facing = false;
    item.held = true;
    // it_80274740: the spin joint's angle and speed return to zero.
    item.spin_speed = 0.0;
    match item.rotation_axis {
        0 => item.rotation.z = 0.0,
        1 => item.rotation.x = 0.0,
        _ => item.rotation.y = 0.0,
    }
    item.hurt_intangible = true;
    item.grabbable = false;
}

/// it_80275158: both timers.
fn set_lifetime(item: &mut ItemCore, common: &melee_it::desc::ItemCommonData, lifetime: f32) {
    item.life_timer = lifetime;
    item.half_life = lifetime * common.half_life_scale;
}

/// itDrMarioPill_UnkMotion0_Anim (802C0C0C): the lifetime counts down; the
/// pill ends when it is spent.
fn thrown_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.grabbable = false;
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itDrMarioPill_UnkMotion0_Phys (802C0C68): it_80272860's gravity with the
/// ItemAttr fall speeds, then it_80274658's spin at the common rate.
fn thrown_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.grabbable = false;
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itDrmariopill_UnkMotion0_Coll (802C0CC4): it_8026D9A0's pass that never
/// lands, then it_8027781C's bounce. A bounce that leaves the pill slower
/// than attribute x10 ends it (802C0D04..14: two fmuls and an fadds, then
/// the inlined four-step square root, 802C0D20..70); a faster one plays the
/// bounce sound.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.airborne_pass(ctx.map);
    item.grabbable = false;
    if !item.bounce_velocity(ctx.map, ctx.assets) {
        return false;
    }
    let (x, y) = (item.velocity.x, item.velocity.y);
    let speed = gekko_math::msl::sqrtf_accurate(x * x + y * y);
    if speed < Attributes(&ctx.assets.special_attributes).minimum_bounce_speed() {
        return true;
    }
    item.sound_requests.push(SOUND);
    false
}

/// itDrMarioPill_Motion2_Anim (802C0E48): intangible (it_802756D0); the
/// pill ends once its owner left the side taunt (ftDr_Init_80149844),
/// letting go of him (ftDr_Init_801498A0, the owner's ARTICLE_DESTROYED
/// hook). Otherwise it copies the owner's cmd_vars[1] into its model's
/// visibility and his model scale into its own (neither is traced).
fn taunt_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.hurt_intangible = true;
    item.grabbable = false;
    ctx.owner
        .is_none_or(|owner| !OWNER_TAUNT_MOTIONS.contains(&owner.motion))
}

/// itDrMarioPill_Motion2_Phys (802C1114).
fn taunt_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.hurt_intangible = true;
    item.grabbable = false;
}

/// itDrMariopill_Motion2_Coll (802C1148).
fn taunt_collision(item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.hurt_intangible = true;
    item.grabbable = false;
    false
}

fn unported_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!(
        "itDrMarioPill_Motion6_Anim (motion {}): itDrMarioPill_802C09C4's held poses",
        item.motion
    )
}
fn unported_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    unimplemented!("itDrMarioPill_Motion6_Phys (motion {})", item.motion)
}
fn unported_collision(item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itDrMarioPill_Motion6_Coll (motion {})", item.motion)
}
