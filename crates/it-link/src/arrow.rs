//! Link's arrow and Young Link's fire arrow (It_Kind_Link_Arrow /
//! It_Kind_CLink_Arrow), itlinkarrow.c (802A7D8C..802A9E9C).
//!
//! Nocked, it follows the archer's hands (it_802A8398). Shot
//! (itLinkArrow_802A850C), it flies at the charge's speed and damage along
//! 5 degrees, falling under the attribute gravity and turning to its flight;
//! a ray from its last position finds the map, where it sticks
//! (it_802A9458): it wobbles a few random frames, stays its stuck lifetime,
//! then fades (a destroy effect) and goes 300 frames later. A hit or a
//! clank ends it; a reflector turns it round. The trail models are drawing
//! only.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::ItemKind;

/// it_803F6A28's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [-1, 0, -1, -1, -1];
/// itLinkArrowAttributes x0..x20 (the trail joints follow).
pub const SPECIAL_ATTRIBUTES: u32 = 9;
/// ftData.x48_items index.
pub const ARTICLE_INDEX: u32 = 3;

mod motion {
    pub const NOCKED: u16 = 0;
    pub const FLYING: u16 = 1;
    /// Stuck in a fighter's shield.
    pub const IN_SHIELD: u16 = 2;
    pub const STUCK: u16 = 4;
}

/// The x2071_b6 bit of Fighter.x2070 (the archer's motion flags).
const FLAG_2071_B6: u32 = 0x0002_0000;
/// ftLk_MS_SpecialNStart.
const FIRST_DRAW_ROW: u16 = 344;
/// MTXDegToRad's factor as MWCC rounds it.
const DEGREES_TO_RADIANS: f32 = 0.017_453_292;
/// it_803F6A84: each wobble's base and random range (degrees).
const WOBBLE_BASE: [f32; 8] = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0];
const WOBBLE_RANGE: [f32; 8] = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
/// Frames a faded arrow lingers before it goes.
const FADED_FRAMES: i32 = 300;
/// it_802787B4's effect as the stuck arrow fades.
const FADE_EFFECT: u16 = 0x421;
/// efSync 0x448: the fire arrow's flame where it sticks.
const FIRE_EFFECT: u16 = 0x448;

/// itLinkArrowAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the flight's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the speed uncharged and at full charge.
    fn speed(&self) -> (f32, f32) {
        (self.0[1], self.0[2])
    }
    /// xC / x10: the damage uncharged and at full charge.
    fn damage(&self) -> (f32, f32) {
        (self.0[3], self.0[4])
    }
    /// x14: the horizontal speed kept after a hit.
    fn hit_speed_scale(&self) -> f32 {
        self.0[5]
    }
    /// x18: the stuck lifetime.
    fn stuck_lifetime(&self) -> f32 {
        self.0[6]
    }
    /// x1C: gravity (its magnitude).
    fn gravity(&self) -> f32 {
        self.0[7]
    }
    /// x20: the most the flight turns from level (radians).
    fn max_angle(&self) -> f32 {
        self.0[8]
    }
}

fn state(item: &mut ItemCore) -> &mut ArrowState {
    match &mut item.scratch {
        ItemScratch::Arrow(state) => state,
        _ => unreachable!("an arrow without its state"),
    }
}
fn state_ref(item: &ItemCore) -> &ArrowState {
    match &item.scratch {
        ItemScratch::Arrow(state) => state,
        _ => unreachable!("an arrow without its state"),
    }
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
/// itLinkarrow_UnkMotion3_Anim: nothing (no code enters state 3).
fn idle_anim(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

static STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: nocked_anim,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: flying_anim,
        physics: flying_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: in_shield_anim,
        physics: in_shield_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: idle_anim,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: stuck_anim,
        physics: no_physics,
        collision: stuck_collision,
    },
];

/// The arrow of Link (`YOUNG = false`) or Young Link's fire arrow.
pub struct Arrow<const YOUNG: bool>;

impl<const YOUNG: bool> ItemLogic for Arrow<YOUNG> {
    const KIND: ItemKind = if YOUNG {
        ItemKind::CLinkArrow
    } else {
        ItemKind::LinkArrow
    };
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802A83E0 (802A83E0) once Item_80268B18 returns: command variables
    /// clear, the flight's lifetime, not shot, the archer and his scale, no
    /// line.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        let lifetime = Attributes(&assets.special_attributes).lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.scratch = ItemScratch::Arrow(ArrowState {
            archer: spawn.owner,
            // xC0 = ftLib_800869D4(archer): read from the archer's view at
            // the first animation, before anything uses it (unset: zero).
            scale: 0.0,
            line: -1,
            ..Default::default()
        });
    }
    /// itLinkArrow_Logic98_PickedUp: state 0.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(motion::NOCKED, ARTICLE_STATES[0], ANIM_UPDATE, context.assets);
    }
    /// itLinkArrow_Logic98_Destroyed: nocked, the archer lets go
    /// (ftLk_SpecialN_UnsetArrow, through `notifies_owner`).
    fn destroyed(item: &mut ItemCore) {
        state(item).archer = None;
        item.owner = None;
    }
    fn notifies_owner(item: &ItemCore) -> bool {
        let s = state_ref(item);
        !s.shot && s.archer.is_some() && item.owner == s.archer
    }
    /// it_802A8398 from the archer, and it_802A8A7C (ProcessFv10).
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        // The archer's pointer is the nocked arrow; shot ones are not his.
        if state_ref(item).shot {
            return;
        }
        match control {
            ItemControl::Aim { tip, tail } => {
                item.position = tip;
                state(item).tail = tail;
            }
            ItemControl::Remove => item.destroyed = true,
            _ => unimplemented!("arrow item control {control:?}"),
        }
    }
    /// itLinkArrow_Logic98_DmgDealt (802A9A38): the horizontal speed drops to
    /// x14 of itself; the arrow goes.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.velocity.x *= Attributes(&context.assets.special_attributes).hit_speed_scale();
        true
    }
    /// itLinkArrow_Logic98_Clanked: it goes.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itLinkArrow_Logic98_HitShield (802A9C04): a fighter's shield holds
    /// it (it_80272D40 is zero for fighters): at its scale, the stuck
    /// lifetime, still, state 2 (its hitboxes go); it follows that fighter
    /// (xC4) at the shield's radius times the fighter's scale, at the angle
    /// of the midpoint of its position and tail about the shield's centre
    /// (retail 802A9C88 / 802A9CA0: fmadds).
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let (Some(fighter), Some(view)) = (item.pending_shield_owner, item.shield_view) else {
            return true;
        };
        apply_scale(item);
        // it_80275158 (its half-life feeds nothing the arrow does).
        item.life_timer = Attributes(&context.assets.special_attributes).stuck_lifetime();
        item.velocity = Vec3::ZERO;
        item.change_motion_with(
            motion::IN_SHIELD,
            ARTICLE_STATES[usize::from(motion::IN_SHIELD)],
            ANIM_UPDATE,
            context.assets,
        );
        item.shield_anchor = Some(fighter);
        let radius = view.scale * view.size;
        let tail = state_ref(item).tail;
        // 0.5 * (pos + tail): fadds, then fmuls.
        let half_x = 0.5 * (item.position.x + tail.x);
        let half_y = 0.5 * (item.position.y + tail.y);
        let angle = melee_lb::trigf::atan2f(half_y - view.center.y, half_x - view.center.x);
        let s = state(item);
        s.shield_center = view.center;
        s.shield_radius = radius;
        s.shield_angle = angle;
        orbit_shield(item);
        false
    }
    /// itLinkArrow_Logic98_Reflected (802A9D...): it turns round, steps its
    /// new speed at once, and its angle turns by pi (in double), wrapped by
    /// whole pi steps.
    fn reflected(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        use std::f64::consts::{FRAC_PI_2, PI};
        item.facing = -item.facing;
        item.rotation.y = (FRAC_PI_2 * f64::from(item.facing)) as f32;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        item.velocity.z = 0.0;
        item.position.x += item.velocity.x;
        item.position.y += item.velocity.y;
        item.position.z = 0.0;
        let mut angle = (PI + f64::from(state(item).angle)) as f32;
        while f64::from(angle) > PI {
            angle = (f64::from(angle) - PI) as f32;
        }
        while f64::from(angle) < -PI {
            angle = (f64::from(angle) + PI) as f32;
        }
        state(item).angle = angle;
        item.rotation.z = angle;
        item.root_translation = item.position;
        false
    }
    /// itLinkArrow_Logic98_EvtUnk: it_8026B894.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
    /// itLinkArrow_802A850C (802A850C).
    fn launch(
        item: &mut ItemCore,
        launch: &Launch,
        common: &melee_it::desc::ItemCommonData,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        shoot::<YOUNG>(item, launch, common.half_life_scale, map, assets);
    }
}

/// itLinkarrow_UnkMotion0_Anim (802A8B78): nocked, the arrow goes once the
/// archer leaves the draw (or a row with x2071_b6), or is gone.
fn nocked_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if state(item).scale == 0.0 {
        state(item).scale = ctx.owner.map_or(1.0, |owner| owner.model_scale);
    }
    apply_scale(item);
    let archer = state(item).archer;
    if archer.is_none() {
        return true;
    }
    if item.owner != archer {
        return false;
    }
    let Some(owner) = ctx.owner else {
        return false;
    };
    let in_draw = (FIRST_DRAW_ROW..FIRST_DRAW_ROW + 6).contains(&owner.motion);
    if !in_draw || owner.motion_flags.unwrap_or(0) & FLAG_2071_B6 != 0 {
        state(item).archer = None;
        return true;
    }
    false
}

/// HSD_JObjSetScale(jobj, xC0): the model at the archer's scale.
fn apply_scale(item: &mut ItemCore) {
    let scale = state(item).scale;
    item.model_scale = Vec3::new(scale, scale, scale);
}

/// it_80275158: the lifetime and its half.
fn set_lifetime(item: &mut ItemCore, lifetime: f32, half_life_scale: f32) {
    item.life_timer = lifetime;
    item.half_life = lifetime * half_life_scale;
}

/// itLinkArrow_802A850C: the charge's speed and damage (fmadds of the
/// charge over the attribute ranges, the damage truncated unsigned), the
/// archer's facing, the flight from the tip along `angle`, and a ray from
/// the tail that stops the tip at the map (it_8026EA20).
fn shoot<const YOUNG: bool>(
    item: &mut ItemCore,
    launch: &Launch,
    half_life_scale: f32,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
) {
    let shot = launch.shot.expect("an arrow's shot");
    let a = Attributes(&assets.special_attributes);
    state(item).angle = shot.angle;
    set_lifetime(item, a.lifetime() + a.stuck_lifetime(), half_life_scale);
    state(item).charge = shot.charge;
    let archer = state(item).archer;
    if archer.is_none() || item.owner != archer {
        unimplemented!("itLinkArrow_802A850C: an arrow no longer its archer's");
    }
    // it_802A8C7C: the flight's lifetime, state 1.
    set_lifetime(item, a.lifetime(), half_life_scale);
    item.change_motion_with(motion::FLYING, ARTICLE_STATES[1], ANIM_UPDATE, assets);
    // it_8027429C with no speed, where the hand holds it.
    let t = assets.attachment_translation;
    let mut hand = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(&launch.hand, &Vec3::new(-t.x, -t.y, -t.z), &mut hand);
    item.leave_hand(Vec3::ZERO, hand, assets);
    item.end_hold(launch.center, launch.attack, map, assets);
    item.stale_multiplier = launch.attack_stale;
    // xDC8 x14, and it_8026B3A8.
    item.speed_damage = false;
    item.grabbable = false;
    let (slow, fast) = a.speed();
    let (weak, strong) = a.damage();
    // retail 802A85DC / 802A85F4: fmadds each, after fsubs and fdivs.
    let speed = gekko_math::fma::fmadds(shot.charge, (fast - slow) / shot.max_charge, slow);
    let damage = gekko_math::fma::fmadds(shot.charge, (strong - weak) / shot.max_charge, weak);
    let s = state(item);
    s.speed = speed;
    s.damage = damage as u32;
    // ftLib_800865C0: the archer's facing.
    item.facing = shot.facing;
    item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
    item.position = shot.tip;
    state(item).tail = shot.tail;
    let angle = state(item).angle;
    item.velocity = Vec3::new(
        item.facing * (speed * gekko_math::msl::cosf(angle)),
        speed * gekko_math::msl::sinf(angle),
        0.0,
    );
    item.rotation.z = angle;
    state(item).shot = true;
    let tail = state(item).tail;
    if let Some(hit) = map.check_all_remap(-1, -1, tail.x, tail.y, item.position.x, item.position.y) {
        item.position = hit.pos;
        state(item).line = hit.line_id;
    }
    let _ = YOUNG;
}

/// itLinkarrow_UnkMotion1_Anim (802A8CEC): the charged damage once
/// (it_80272460 on hitbox 0), then the lifetime (it_80273130).
fn flying_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    apply_scale(item);
    if item.command_variables[2] == 0 {
        let damage = state(item).damage;
        item.set_hitbox_damage(0, damage);
        item.command_variables[2] = 1;
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLinkarrow_UnkMotion1_Phys (802A8E68): the tail is this frame's start;
/// gravity (fabs of the attribute).
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    state(item).tail = item.position;
    let gravity = Attributes(&ctx.assets.special_attributes).gravity();
    item.velocity.y -= gekko_math::msl::fabsf(gravity);
}

/// itLinkarrow_UnkMotion1_Coll (802A8EA0): the angle of the frame's travel
/// (mirrored facing left), clamped to x20; a ray from the tail that meets a
/// live line sticks it there, riding the line.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let a = Attributes(&ctx.assets.special_attributes);
    let tail = state(item).tail;
    let dy = item.position.y - tail.y;
    let mut angle = if item.facing == 1.0 {
        melee_lb::trigf::atan2f(dy, item.position.x - tail.x)
    } else {
        -melee_lb::trigf::atan2f(dy, tail.x - item.position.x)
    };
    let limit = a.max_angle();
    if gekko_math::msl::fabsf(angle) > limit {
        angle = if angle < 0.0 { -limit } else { limit };
    }
    state(item).angle = angle;
    item.rotation.z = angle;
    if let Some(hit) = ctx
        .map
        .check_all_remap(-1, -1, tail.x, tail.y, item.position.x, item.position.y)
    {
        item.position = hit.pos;
        state(item).line = hit.line_id;
        if ctx.map.line_is_active(hit.line_id) {
            ride_line(item, ctx.map, hit.line_id);
            let normal_angle = melee_lb::trigf::atan2f(hit.normal.y, hit.normal.x);
            let s = state(item);
            s.normal_angle = normal_angle;
            s.previous_normal_angle = normal_angle;
            stick(item, ctx);
        }
    }
    false
}

/// mpColl_80043558 and mpGetSpeed: the line's joint hears of it, and the
/// arrow takes the line's motion.
fn ride_line(item: &mut ItemCore, map: &mut melee_mp::CollMap, line: i32) {
    let mut collision = item.collision.take().expect("arrow map collision");
    map.notify_line_joint(&mut collision, line);
    item.collision = Some(collision);
    if let Some(speed) = map.line_speed(line, &item.position) {
        item.velocity = speed;
    }
}

/// The arrow on the shield's rim: fmadds(radius, cosf / sinf(angle),
/// centre), on the stage plane.
fn orbit_shield(item: &mut ItemCore) {
    let s = *state_ref(item);
    item.position.x = gekko_math::fma::fmadds(
        s.shield_radius,
        gekko_math::msl::cosf(s.shield_angle),
        s.shield_center.x,
    );
    item.position.y = gekko_math::fma::fmadds(
        s.shield_radius,
        gekko_math::msl::sinf(s.shield_angle),
        s.shield_center.y,
    );
    item.position.z = 0.0;
}

/// itLinkarrow_UnkMotion2_Anim (802A934C): it goes once the fighter holds
/// no shield (ftLib_80086A18: not GuardOn, Guard or GuardSetOff) or its
/// lifetime runs out (it_80273130).
fn in_shield_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let guarding = item
        .shield_view
        .expect("a shield-held arrow's fighter view")
        .guarding;
    if !guarding {
        return true;
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLinkarrow_UnkMotion2_Phys (802A93B4): the shield's centre and size
/// now (ftCo_80094098, times ftLib_800869D4), the arrow on its rim.
fn in_shield_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let view = item
        .shield_view
        .expect("a shield-held arrow's fighter view");
    let s = state(item);
    s.shield_center = view.center;
    s.shield_radius = view.size * view.scale;
    orbit_shield(item);
}

/// it_802A9458 (802A9458): the stuck lifetime, 3..6 wobbles (HSD_Randi),
/// state 4 (its hitboxes go), the model at its flight angle; the fire
/// arrow's flame.
fn stick(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    apply_scale(item);
    // it_80275158 (the half-life beside it feeds nothing the arrow does).
    item.life_timer = a.stuck_lifetime();
    let cell = ctx.rng.expect("it_802A9458 draws");
    let mut rng = cell.get();
    let wobbles = rng.randi(4) + 3;
    cell.set(rng);
    let s = state(item);
    s.wobbles = wobbles;
    s.faded_frames = 0;
    item.change_motion_with(motion::STUCK, ARTICLE_STATES[4], ANIM_UPDATE, ctx.assets);
    item.rotation.z = state(item).angle;
    if matches!(item.kind, ItemKind::CLinkArrow) {
        item.events.push(ItemEvent::FollowingEffect {
            id: FIRE_EFFECT,
            offset: Vec3::new(0.0, 0.0, 5.0),
        });
    }
}

/// itLinkarrow_UnkMotion4_Anim (802A9658): the wobble (a random turn each
/// frame for its count: even frames up, odd down, degrees `range * rand +
/// base`, fmadds, then the factor); once the lifetime runs out the fade's
/// effect (once), no hits, hidden; it goes 300 frames later without the
/// destroy effect.
fn stuck_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let wobbles = state(item).wobbles;
    let angle = state(item).angle;
    let z = match wobbles {
        0..=6 => {
            let rng_cell = ctx.rng.expect("a stuck arrow's wobble draws");
            let mut rng = rng_cell.get();
            let rand = rng.randf();
            rng_cell.set(rng);
            let index = wobbles as usize;
            // retail 802A975C / 802A978C: fmadds, then fmuls by the factor.
            let turn = DEGREES_TO_RADIANS
                * gekko_math::fma::fmadds(WOBBLE_RANGE[index], rand, WOBBLE_BASE[index]);
            if wobbles % 2 == 0 {
                angle + turn
            } else {
                angle - turn
            }
        }
        _ => angle,
    };
    item.rotation.z = z;
    state(item).wobbles -= 1;
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        if state(item).faded_frames == 0 {
            item.events.push(ItemEvent::DestroyEffect {
                id: FADE_EFFECT,
                root: None,
            });
            item.hurt_intangible = true;
        }
        // it_8026BB44: hidden; xDCF_flag.b2: no destroy effect when it goes.
        item.hidden = true;
        item.destroy_effect_suppressed = true;
        let s = state(item);
        s.faded_frames += 1;
        if s.faded_frames > FADED_FRAMES {
            return true;
        }
    }
    false
}

/// itLinkarrow_UnkMotion4_Coll (802A9888): it rides its line, turning with
/// the line's normal; off a live line it goes.
fn stuck_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let s = state(item);
    s.previous_normal_angle = s.normal_angle;
    let line = s.line;
    if line == -1 {
        item.velocity = Vec3::ZERO;
        return true;
    }
    if !ctx.map.line_is_active(line) {
        return true;
    }
    ride_line(item, ctx.map, line);
    let normal = ctx.map.line_get_normal(line);
    let normal_angle = melee_lb::trigf::atan2f(normal.y, normal.x);
    let s = state(item);
    s.normal_angle = normal_angle;
    if s.normal_angle != s.previous_normal_angle {
        s.angle += s.normal_angle - s.previous_normal_angle;
        let angle = s.angle;
        item.rotation.z = angle;
    }
    false
}
