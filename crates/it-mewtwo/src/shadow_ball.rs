//! The Shadow Ball (It_Kind_Mewtwo_ShadowBall), itmewtwoshadowball.c
//! (802C4D10..802C61F4).
//!
//! It forms in Mewtwo's hand (it_802C5000, then Item_8026AB54) and lives
//! there while he charges; leaving the charge's states, cancelling, or
//! letting go of it (u.mt.x2230) ends it. Released (it_802C53F0), it takes
//! the charge's state (1..8), speed and strength and flies along its angle
//! while its model's grandchild, which carries the hitbox, wavers: every
//! attribute x20 frames a random turn (HSD_Randf) and a random sound
//! (HSD_Randi). The stage in the direction it flies bursts it
//! (it_802C5E5C): a few frames of a larger hitbox in states 10..17. The
//! forward throw's ball (it_802C519C) starts in state 9 and flies straight.
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemOwner, ItemPhysicsContext, ItemScratch, ItemStateRow, Launch,
    OwnerBlast, ShadowBallState, SpawnItem,
};
use melee_types::ItemKind;

pub struct MewtwoShadowBall;

/// it_803F7760's anim_id column: in hand, one flight per charge, the thrown
/// ball, then one burst per charge playing the flights' article states.
pub const ARTICLE_STATES: [i32; 18] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7, 8];
/// itMewtwoShadowball_DatAttrs: sixteen words.
pub const SPECIAL_ATTRIBUTES: u32 = 16;
/// ftData.x48_items index (ftMt_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;

/// The thrown ball's motion state, and the first burst's.
const THROWN: u16 = 9;
const FIRST_BURST: u16 = 10;
/// ftMt_MS_SpecialNStart .. SpecialAirNEnd without the cancels (344, 349):
/// the owner's states in which the held ball stays
/// (ftMt_SpecialN_CheckShadowBallRemove / CheckShadowBallCancel).
const OWNER_CHARGE_STATES: [u16; 8] = [341, 342, 343, 345, 346, 347, 348, 350];
/// Fighter.x2070's x2071_b6, which also ends the held ball.
const OWNER_FLAG_2071_B6: u32 = 1 << 17;
/// itGetJObjGrandchild: the model's third joint, which wavers and carries
/// the hitbox.
const GRANDCHILD: usize = 2;
/// efSync_Spawn(0x40E, gobj, &pos): the burst.
const BURST_EFFECT: u16 = 0x40E;
/// Item_8026AE84(ip, 0x74, 127, 64): the burst's sound.
const BURST_SOUND: u32 = 0x74;
/// it_803F7880: the waver's sounds.
const WAVER_SOUNDS: [u32; 3] = [0x30DAA, 0x30DAD, 0x30DB0];

/// itMewtwoShadowball_DatAttrs (it/itCommonItems.h).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x8 / xC: the speed at no charge and at full charge.
    fn speed(&self) -> (f32, f32) {
        (self.0[2], self.0[3])
    }
    /// x10 / x14: x24's range (read by nothing ported).
    fn damage(&self) -> (f32, f32) {
        (self.0[4], self.0[5])
    }
    /// x18 / x1C: the model grandchild's scale range.
    fn scale(&self) -> (f32, f32) {
        (self.0[6], self.0[7])
    }
    /// x20 (an s32): frames between the waver's turns.
    fn waver_period(&self) -> i32 {
        self.0[8].to_bits() as i32
    }
    /// x24: the burst's hitbox radius, as a multiple of the flight's.
    fn burst_radius_scale(&self) -> f32 {
        self.0[9]
    }
    /// x28: the burst's lifetime.
    fn burst_lifetime(&self) -> f32 {
        self.0[10]
    }
    /// x2C: the thrown ball's speed.
    fn thrown_speed(&self) -> f32 {
        self.0[11]
    }
}

static STATES: [ItemStateRow; 18] = {
    let flight = ItemStateRow {
        animation_id: 0,
        animation: flight_animation,
        physics: flight_physics,
        collision: flight_collision,
    };
    let mut rows = [flight; 18];
    rows[0] = ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: hand_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows[THROWN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[THROWN as usize],
        animation: thrown_animation,
        physics: no_physics,
        collision: flight_collision,
    };
    let mut i = 1;
    while i < 18 {
        rows[i].animation_id = ARTICLE_STATES[i];
        if i >= FIRST_BURST as usize {
            rows[i].animation = burst_animation;
            rows[i].physics = burst_physics;
            rows[i].collision = no_collision;
        }
        i += 1;
    }
    rows
};

fn ball(item: &ItemCore) -> &ShadowBallState {
    let ItemScratch::ShadowBall(state) = &item.scratch else {
        panic!("Shadow Ball scratch missing")
    };
    state
}
fn ball_mut(item: &mut ItemCore) -> &mut ShadowBallState {
    let ItemScratch::ShadowBall(state) = &mut item.scratch else {
        panic!("Shadow Ball scratch missing")
    };
    state
}

/// it_80275158: the lifetime and its half (it_804D6D28->x4C).
fn set_lifetime(item: &mut ItemCore, frames: f32, half_life_scale: f32) {
    item.life_timer = frames;
    item.half_life = frames * half_life_scale;
}

/// Whether the creator still owns the ball (x2C, and ip->owner the same).
fn kept_by_creator(item: &ItemCore) -> bool {
    let creator = ball(item).creator;
    creator.is_some() && item.owner == creator
}

/// it_3F2F.c's Logic101 row.
impl ItemLogic for MewtwoShadowBall {
    const KIND: ItemKind = ItemKind::MewtwoShadowBall;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// The spawner's set-up once Item_80268B18 returns. it_802C5000
    /// (802C5000), the held ball: the command variables, the lifetime, the
    /// creator, and the waver's first direction (M_TAU * HSD_Randf(), a
    /// double product rounded) and speed (2.5 + (HSD_Randf() - 0.5), fsubs
    /// then fadds) with its change per frame (fneg, fmuls, fdivs);
    /// Item_8026AB54 follows (the scene's SpawnInHand). it_802C519C
    /// (802C519C), the forward throw's ball (`THROWN_VARIANT`): state 9,
    /// no charge, attribute x2C's speed along the angle and it_8026B3A8.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        if spawn.spawn_variant == THROWN_VARIANT {
            thrown(item, assets, common, spawn);
            return;
        }
        item.command_variables = [0; 4];
        set_lifetime(item, a.lifetime(), common.half_life_scale);
        let period = a.waver_period();
        let waver_angle = (std::f64::consts::TAU * f64::from(rng.randf())) as f32;
        let waver_speed = 2.5 + (rng.randf() - 0.5);
        item.scratch = ItemScratch::ShadowBall(ShadowBallState {
            launched: false,
            creator: item.owner,
            waver_frames: period,
            waver_angle,
            waver_speed,
            waver_acceleration: -waver_speed / (0.5 * period as f32),
            flight_frames: 0,
            strength: 0.0,
            // ftLib_800869D4 of the creator: taken at the pickup.
            owner_scale: 1.0,
            ..Default::default()
        });
    }
    /// it_2725_Logic101_PickedUp (802C5774): state 0 and no strength (x50).
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, context.assets);
        let state = ball_mut(item);
        state.strength = 0.0;
        if let Some(owner) = context.owner {
            state.owner_scale = owner.model_scale;
        }
    }
    /// it_802C573C (802C573C), from Mewtwo: the effects go, then
    /// Item_8026A8EC at once.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => {
                item.events.push(ItemEvent::DestroyEffects);
                item.effects_destroyed = true;
                item.destroyed = true;
            }
            _ => unimplemented!("Shadow Ball item control {control:?}"),
        }
    }
    /// it_2725_Logic101_Destroyed (802C56C4): a ball still in hand lets its
    /// creator go of it (ftMt_SpecialN_SetNULL).
    fn notifies_owner(item: &ItemCore) -> bool {
        !ball(item).launched && kept_by_creator(item)
    }
    /// it_802C53F0 (802C53F0): the angle, the lifetime, the charge clamped
    /// to 0..full (fctiwz each); then, while the creator still owns it, the
    /// charge's flight state (it_802C5B18), out of the hand (it_8027429C),
    /// the speed (802C54E8: fmadds), x24 (802C5500: fmadds) and the scale
    /// of the integer charges (802C555C: fmadds) over their ranges, the
    /// creator's facing, the aimed point and the velocity along the angle
    /// at the strength's share (802C55A0..A4: fmuls, fmuls).
    fn launch(
        item: &mut ItemCore,
        launch: &Launch,
        common: &ItemCommonData,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        let aim = launch.aim.expect("Shadow Ball aim");
        let a = Attributes(&assets.special_attributes);
        ball_mut(item).angle = aim.angle;
        set_lifetime(item, a.lifetime(), common.half_life_scale);
        let mut charge = aim.charge;
        if charge < 0.0 {
            charge = 0.0;
        }
        if charge > aim.full_charge {
            charge = aim.full_charge;
        }
        {
            let state = ball_mut(item);
            state.level = gekko_math::msl::fctiwz(charge);
            state.full = gekko_math::msl::fctiwz(aim.full_charge);
        }
        if !kept_by_creator(item) {
            return;
        }
        take_flight_state(item, a.lifetime(), common.half_life_scale, assets);
        // it_8027429C: it_80273B50 at the hand, then it_80273F34. A hold-kind
        // 8 article leaves from the hand joint moved by its negated
        // attachment translation (lb_8000B1CC -> PSMTXMultVec).
        let t = assets.attachment_translation;
        let mut hand = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(&launch.hand, &Vec3::new(-t.x, -t.y, -t.z), &mut hand);
        item.leave_hand(Vec3::ZERO, hand, assets);
        item.end_hold(launch.center, launch.attack, map, assets);
        item.stale_multiplier = launch.attack_stale;
        item.speed_damage = false;
        item.grabbable = false;
        let (slow, fast) = a.speed();
        let speed = gekko_math::fma::fmadds(charge, (fast - slow) / aim.full_charge, slow);
        let (weak, strong) = a.damage();
        let damage = gekko_math::fma::fmadds(charge, (strong - weak) / aim.full_charge, weak);
        let (small, large) = a.scale();
        let state = ball_mut(item);
        state.speed = speed;
        state.damage = damage as u32;
        state.scale = gekko_math::fma::fmadds(
            state.level as f32,
            (large - small) / state.full as f32,
            small,
        );
        state.cycle = 0;
        state.launched = true;
        state.flight_frames = 0;
        let (angle, strength) = (state.angle, state.strength);
        item.facing = aim.facing;
        item.position = aim.position;
        item.velocity = Vec3::new(
            (speed * strength) * gekko_math::msl::cosf(angle),
            (speed * strength) * gekko_math::msl::sinf(angle),
            0.0,
        );
        // HSD_JObjSetScale(child, 1): the model's child at its own size.
        item.joint_scale = Some((melee_it::CHILD_JOINT, Vec3::new(1.0, 1.0, 1.0)));
    }
    /// fn_802C5E18 (802C5E18), installed by it_802C5B18: a counter 0..2.
    fn accessory(
        item: &mut ItemCore,
        _owner: Option<&ItemOwner>,
        _assets: &ItemAssets,
    ) -> Option<OwnerBlast> {
        let flying = (1..THROWN).contains(&item.motion);
        let state = ball_mut(item);
        if flying {
            state.cycle = (state.cycle + 1) % 3;
        }
        None
    }
    /// itMewtwoShadowball_Logic101_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoShadowball_Logic101_Clanked.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoShadowball_Logic101_Absorbed.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoShadowball_Logic101_HitShield.
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_2725_Logic101_Reflected (802C60BC): the angle turns half a circle
    /// (in double, then rounded) and wraps into 0..2pi; the flight's physics
    /// takes the velocity from it.
    fn reflected(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        let state = ball_mut(item);
        state.angle = (f64::from(state.angle) + std::f64::consts::PI) as f32;
        state.angle = wrap_angle(state.angle);
        false
    }
    /// it_2725_Logic101_ShieldBounced (802C6140): the velocity mirrored off
    /// the shield (lbVector_Mirror with xC58) and its angle (atan2f) wrapped.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, context.shield_normal);
        let angle = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        ball_mut(item).angle = wrap_angle(angle);
        false
    }
}

/// SpawnItem.spawn_variant of the forward throw's ball; its other arguments
/// ride in `spawn_argument` (the angle's float bits) and `auxiliary_damage`
/// (the full charge), which Item_80268B18 does not read for this kind.
pub const THROWN_VARIANT: i32 = 1;

/// it_802C519C (802C519C) once Item_80268B18 returns: the forward throw's
/// ball.
fn thrown(item: &mut ItemCore, assets: &ItemAssets, common: &ItemCommonData, spawn: &SpawnItem) {
    let a = Attributes(&assets.special_attributes);
    item.change_motion_with(
        THROWN,
        ARTICLE_STATES[THROWN as usize],
        ANIM_UPDATE,
        assets,
    );
    item.command_variables = [0; 4];
    set_lifetime(item, a.lifetime(), common.half_life_scale);
    let angle = f32::from_bits(spawn.spawn_argument as u32);
    let full = i32::from(spawn.auxiliary_damage);
    let speed = a.thrown_speed();
    let (small, large) = a.scale();
    let (weak, strong) = a.damage();
    // 802C5344 / 802C538C: fmadds on a zero charge.
    let scale = gekko_math::fma::fmadds(0.0, (large - small) / full as f32, small);
    let damage = gekko_math::fma::fmadds(0.0, (strong - weak) / full as f32, weak);
    item.scratch = ItemScratch::ShadowBall(ShadowBallState {
        angle,
        speed,
        scale,
        launched: true,
        level: 0,
        full,
        cycle: 0,
        damage: damage as u32,
        creator: item.owner,
        waver: Vec3::ZERO,
        waver_angle: angle,
        waver_speed: speed,
        waver_acceleration: 0.0,
        waver_frames: a.waver_period(),
        flight_frames: 0,
        strength: 1.0,
        owner_scale: 1.0,
        burst_radius: 0.0,
    });
    item.velocity = Vec3::new(
        speed * gekko_math::msl::cosf(angle),
        speed * gekko_math::msl::sinf(angle),
        0.0,
    );
    item.grabbable = false;
}

/// The reflection's wrap: add M_TAU below zero, subtract it above (the
/// float promoted to double for each step, as retail does).
fn wrap_angle(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + std::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > std::f64::consts::TAU {
        angle = (f64::from(angle) - std::f64::consts::TAU) as f32;
    }
    angle
}

/// The charge clamped into 0..full (it_802C5B18 / it_802C5E5C).
fn clamp_level(state: &mut ShadowBallState) {
    if state.level < 0 {
        state.level = 0;
    }
    if state.level >= state.full {
        state.level = state.full;
    }
}

/// it_802C5B18 (802C5B18): the lifetime again, the effects go, the clamped
/// charge picks the flight state, the accessory begins, and the strength
/// is 0.5 + 0.5 * charge / full (802C5BD4: fdivs; 802C5BD8: fmadds).
fn take_flight_state(
    item: &mut ItemCore,
    lifetime: f32,
    half_life_scale: f32,
    assets: &ItemAssets,
) {
    set_lifetime(item, lifetime, half_life_scale);
    item.events.push(ItemEvent::DestroyEffects);
    let state = ball_mut(item);
    clamp_level(state);
    let motion = (state.level + 1) as u16;
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
    let state = ball_mut(item);
    state.strength = gekko_math::fma::fmadds(0.5, state.level as f32 / state.full as f32, 0.5);
}

/// itMewtwoshadowball_UnkMotion0_Anim (802C57C0): the ball ends once its
/// creator leaves the charge (ftMt_SpecialN_CheckShadowBallRemove), cancels
/// it (ftMt_SpecialN_CheckShadowBallCancel) or lets go of it
/// (ftMt_SpecialN_GetChargeLevel); otherwise it takes the creator's charge
/// and full charge. The grandchild's scale and translation it then sets
/// are drawn only: the held ball has no hitbox.
fn hand_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if let (Some(owner), true) = (ctx.owner, kept_by_creator(item)) {
        if !OWNER_CHARGE_STATES.contains(&owner.motion) {
            return true;
        }
        if owner
            .motion_flags
            .is_some_and(|flags| flags & OWNER_FLAG_2071_B6 != 0)
        {
            return true;
        }
        let Some((level, full)) = owner.charge else {
            return true;
        };
        let state = ball_mut(item);
        state.level = level;
        state.full = full;
    }
    false
}

/// itMewtwoshadowball_UnkMotion8_Anim (802C5BF8): the grandchild keeps the
/// flight's scale (drawn only: the hitbox sits at its origin); the lifetime
/// counts down (it_80273130).
fn flight_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itMewtwoshadowball_UnkMotion9_Anim (802C5D8C): it_80273130.
fn thrown_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itMewtwoshadowball_UnkMotion8_Phys (802C5CE8): after the first frame the
/// speed along the angle (fmuls after cosf and sinf, without the strength),
/// then the waver.
fn flight_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let state = ball(item);
    if state.flight_frames > 0 {
        let (speed, angle) = (state.speed, state.angle);
        item.velocity.x = speed * gekko_math::msl::cosf(angle);
        item.velocity.y = speed * gekko_math::msl::sinf(angle);
    }
    waver(item, ctx);
}

/// it_802C4D10 (802C4D10): after the first frame, every x20 frames the
/// waver turns by up to an eighth of a circle (802C4DA8..BC: fsubs, fmuls,
/// then fmadd with M_PI / 4 in double, rounded), its speed's change is
/// renewed (fneg, fmuls, fdivs) and one of three sounds plays (HSD_Randi);
/// each frame the speed changes (fadds) and the grandchild moves on in y
/// and z by speed * strength along the direction (802C4E48..4C and
/// 802C4E68..6C: fmuls, fmadds).
fn waver(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let period = Attributes(&ctx.assets.special_attributes).waver_period();
    let mut sound = None;
    let state = ball_mut(item);
    if state.flight_frames > 0 {
        state.waver_frames += 1;
        if state.waver_frames >= period {
            state.waver_frames = 0;
            let mut rng = ctx.rng.get();
            let random = rng.randf();
            let turn = 2.0 * (random - 0.5);
            state.waver_angle = gekko_math::fma::fmadd(
                std::f64::consts::FRAC_PI_4,
                f64::from(turn),
                f64::from(state.waver_angle),
            ) as f32;
            state.waver_acceleration = -state.waver_speed / (0.5 * period as f32);
            sound = Some(WAVER_SOUNDS[rng.randi(3) as usize]);
            ctx.rng.set(rng);
        }
        state.waver_speed += state.waver_acceleration;
        state.waver.x = 0.0;
        let push = state.waver_speed * state.strength;
        state.waver.y = gekko_math::fma::fmadds(
            push,
            gekko_math::msl::sinf(state.waver_angle),
            state.waver.y,
        );
        let push = state.waver_speed * state.strength;
        state.waver.z = gekko_math::fma::fmadds(
            push,
            gekko_math::msl::cosf(state.waver_angle),
            state.waver.z,
        );
    } else {
        state.waver = Vec3::ZERO;
    }
    state.flight_frames += 1;
    let waver = state.waver;
    item.joint_translation = Some((GRANDCHILD, waver));
    if let Some(sound) = sound {
        item.sound_requests.push(sound);
    }
}

/// it_802C4F50 (802C4F50): an airborne pass (it_8026DA08), then the ceiling
/// when rising or the floor otherwise, and the left wall when flying right
/// or the right wall otherwise.
fn meets_stage(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    use melee_types::mp::collide;
    item.air_pass(ctx.map);
    let env = item
        .collision
        .as_ref()
        .expect("item map collision")
        .env_flags as u32;
    let mut hit = false;
    if item.velocity.y > 0.0 {
        hit |= env & collide::CEILING_MASK != 0;
    } else {
        hit |= env & collide::FLOOR_MASK != 0;
    }
    if item.velocity.x > 0.0 {
        hit |= env & collide::LEFT_WALL_MASK != 0;
    } else {
        hit |= env & collide::RIGHT_WALL_MASK != 0;
    }
    hit
}

/// itMewtwoshadowball_UnkMotion8_Coll (802C5D3C) / UnkMotion9_Coll
/// (802C5DB0): the stage bursts the ball.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if meets_stage(item, ctx) {
        burst(item, ctx);
    }
    false
}

/// it_802C5E5C (802C5E5C): the model hides, the lifetime is attribute x28,
/// the effects go, the clamped charge picks the burst state, the accessory
/// ends, the new hitbox's radius times attribute x24 is kept (fmuls), and
/// the burst's effect and sound play at the grandchild.
fn burst(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    item.hidden = true;
    set_lifetime(item, a.burst_lifetime(), ctx.half_life_scale);
    item.events.push(ItemEvent::DestroyEffects);
    let state = ball_mut(item);
    clamp_level(state);
    let motion = FIRST_BURST + state.level as u16;
    item.change_motion_with(
        motion,
        ARTICLE_STATES[motion as usize],
        ANIM_UPDATE,
        ctx.assets,
    );
    let radius = item.hitboxes[0]
        .as_ref()
        .map_or(0.0, |hit| hit.descriptor.radius);
    ball_mut(item).burst_radius = radius * a.burst_radius_scale();
    // lb_8000B1CC(grandchild, NULL, &pos).
    let pose = ctx
        .assets
        .pose
        .as_ref()
        .expect("Shadow Ball pose for its grandchild");
    let root = melee_it::pose::RootSrt {
        translate: item.root_translation,
        ..item.root_srt()
    };
    let matrix = pose.bone_matrix_posed(
        item.article_state,
        item.pose_steps,
        GRANDCHILD,
        root,
        None,
        item.joint_translation,
        item.joint_scale,
    );
    let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
    item.events.push(ItemEvent::Effect {
        id: BURST_EFFECT,
        position,
    });
    item.sound_requests.push(BURST_SOUND);
}

/// itMewtwoshadowball_UnkMotion17_Anim (802C5F80): a live hitbox takes the
/// burst's radius; the lifetime counts down (it_80273130).
fn burst_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let radius = ball(item).burst_radius;
    if let Some(hit) = &mut item.hitboxes[0] {
        hit.descriptor.radius = radius;
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itMewtwoshadowball_UnkMotion17_Phys (802C6064): itResetVelocity.
fn burst_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
