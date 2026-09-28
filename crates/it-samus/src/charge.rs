//! Samus's charge shot (It_Kind_Samus_Charge), itsamuschargeshot.c
//! (802B5518..802B6000). It forms in Samus's hand (it_802B55C8, then
//! Item_8026AB54) and lives there while she charges; leaving the charge's
//! states or letting go of it (u.ss.x222C) ends it. Fired
//! (it_802B56E4), it takes the charge level's state (1..8), speed and
//! scale and flies straight until its lifetime ends, it meets the stage in
//! the direction it flies, or its hit connects.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct SamusCharge;

/// it_803F7288's anim_id column: in hand, then one flight per charge level.
pub const ARTICLE_STATES: [i32; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// itSamusChargeShot_Attributes: eight words.
pub const SPECIAL_ATTRIBUTES: u32 = 8;
/// ftData.x48_items index (ftSs_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;

/// ftSs_MS_SpecialNStart .. SpecialAirN: the owner's charge states in
/// which the shot stays (ftSs_SpecialLw_80129158; ftSs_SpecialN_801291A8
/// excludes the cancel, 345).
const OWNER_CHARGE_STATES: [u16; 5] = [343, 344, 346, 347, 348];
/// The highest flight state's charge (it_802B5CBC's clamp).
const MAXIMUM_LEVEL: i32 = 7;
/// efSync_Spawn(0x47F, item, grandchild): efAlt's hsd_8039EFAC(0, 2, 0x7D4)
/// while in hand; 0x480 / 0x481 on the root, the full shot's sparkles.
pub const HAND_GLOW: u16 = 0x47F;
pub const FULL_SPARKLE_A: u16 = 0x480;
pub const FULL_SPARKLE_B: u16 = 0x481;

/// itSamusChargeShot_Attributes.
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
    /// x10 / x14: xDF8's range (read by nothing ported).
    fn damage(&self) -> (f32, f32) {
        (self.0[4], self.0[5])
    }
    /// x18 / x1C: the model grandchild's scale range.
    fn scale(&self) -> (f32, f32) {
        (self.0[6], self.0[7])
    }
}

static STATES: [ItemStateRow; 9] = {
    let flight = ItemStateRow {
        animation_id: 0,
        animation: flight_animation,
        physics: flight_physics,
        collision: flight_collision,
    };
    let mut rows = [flight; 9];
    rows[0] = ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: hand_animation,
        physics: no_physics,
        collision: no_collision,
    };
    let mut i = 1;
    while i < 9 {
        rows[i].animation_id = ARTICLE_STATES[i];
        i += 1;
    }
    rows
};

fn shot(item: &ItemCore) -> &ChargeShotState {
    let ItemScratch::ChargeShot(state) = &item.scratch else {
        panic!("charge shot scratch missing")
    };
    state
}
fn shot_mut(item: &mut ItemCore) -> &mut ChargeShotState {
    let ItemScratch::ChargeShot(state) = &mut item.scratch else {
        panic!("charge shot scratch missing")
    };
    state
}

/// it_3F2F.c's Logic108 row.
impl ItemLogic for SamusCharge {
    const KIND: ItemKind = ItemKind::SamusCharge;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802B55C8 (802B55C8) once Item_80268B18 returns: the command
    /// variables, the lifetime, and the shot's owner (xE00); Item_8026AB54
    /// follows (the scene's SpawnInHand).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        set_lifetime(item, Attributes(&assets.special_attributes).lifetime(), common.half_life_scale);
        item.scratch = ItemScratch::ChargeShot(ChargeShotState {
            original_owner: item.owner,
            ..Default::default()
        });
    }
    /// it_2725_Logic108_PickedUp (802B5A48): state 0 and the hand glow on
    /// the model's grandchild.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, context.assets);
        item.events.push(ItemEvent::OwnEffect { id: HAND_GLOW });
        shot_mut(item).glowing = true;
    }
    /// it_802B5974 (802B5974), Samus's ftSamus_UnkAndDestroyAllEF: the
    /// effects go, then Item_8026A8EC at once.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => {
                item.events.push(ItemEvent::DestroyEffects);
                item.effects_destroyed = true;
                shot_mut(item).glowing = false;
                item.destroyed = true;
            }
            _ => unimplemented!("charge shot item control {control:?}"),
        }
    }
    /// it_2725_Logic108_Destroyed: a shot still in hand lets its owner go
    /// of it (ftSs_SpecialN_801291F0).
    fn notifies_owner(item: &ItemCore) -> bool {
        let state = shot(item);
        !state.launched && state.original_owner.is_some() && item.owner == state.original_owner
    }
    /// it_802B56E4 (802B56E4): the charge clamped to 0..full, the level's
    /// flight state (it_802B5CBC), out of the hand (it_8027429C), then the
    /// speed (802B57DC: fmadds), xDF8 (802B57F4: fmadds) and the scale of
    /// the integer levels (802B5850: fmadds) over the range, the owner's
    /// facing, the aimed point and the velocity along the angle.
    fn launch(
        item: &mut ItemCore,
        launch: &Launch,
        common: &melee_it::desc::ItemCommonData,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        let aim = launch.aim.expect("charge shot aim");
        let a = Attributes(&assets.special_attributes);
        shot_mut(item).angle = aim.angle;
        set_lifetime(item, a.lifetime(), common.half_life_scale);
        let mut level = aim.charge;
        if level < 0.0 {
            level = 0.0;
        }
        if level > aim.full_charge {
            level = aim.full_charge;
        }
        let whole = gekko_math::msl::fctiwz(level);
        let full = gekko_math::msl::fctiwz(aim.full_charge);
        {
            let state = shot_mut(item);
            state.level = whole;
            state.full = full;
        }
        let state = shot(item);
        if state.original_owner.is_none() || item.owner != state.original_owner {
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
        let speed = gekko_math::fma::fmadds(level, (fast - slow) / aim.full_charge, slow);
        let (weak, strong) = a.damage();
        let damage = gekko_math::fma::fmadds(level, (strong - weak) / aim.full_charge, weak);
        let (small, large) = a.scale();
        let state = shot_mut(item);
        let levels = (large - small) / state.full as f32;
        state.speed = speed;
        state.damage = damage as u32;
        state.scale = gekko_math::fma::fmadds(state.level as f32, levels, small);
        state.sparkle = 0;
        state.glowing = false;
        state.launched = true;
        let angle = state.angle;
        item.facing = aim.facing;
        item.position = aim.position;
        item.velocity = Vec3::new(
            speed * gekko_math::msl::cosf(angle),
            speed * gekko_math::msl::sinf(angle),
            0.0,
        );
    }
    /// it_802B5EDC (802B5EDC), installed at the launch: a full shot
    /// sparkles every third frame.
    fn accessory(item: &mut ItemCore, _owner: Option<&ItemOwner>, _assets: &ItemAssets) {
        let state = shot_mut(item);
        if !state.launched {
            return;
        }
        let sparkle = state.level == state.full && state.sparkle == 0;
        state.sparkle = (state.sparkle + 1) % 3;
        if sparkle {
            item.events.push(ItemEvent::OwnEffect { id: FULL_SPARKLE_A });
            item.events.push(ItemEvent::OwnEffect { id: FULL_SPARKLE_B });
        }
    }
    /// itSamusChargeshot_Logic108_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itSamusChargeshot_Logic108_Clanked.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itSamusChargeshot_Logic108_Absorbed.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itSamusChargeshot_Logic108_HitShield.
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_2725_Logic108_Reflected (802B5F80): the facing flips, the model
    /// turns ((float) (M_PI_2 * facing)), the angle turns half a circle (in
    /// double, then rounded) and wraps into 0..2pi.
    fn reflected(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        item.facing = -item.facing;
        item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        let state = shot_mut(item);
        state.angle = (f64::from(state.angle) + std::f64::consts::PI) as f32;
        state.angle = wrap_angle(state.angle);
        false
    }
    /// it_2725_Logic108_ShieldBounced (802B6070): the velocity mirrored off
    /// the shield (lbVector_Mirror with xC58), its angle (atan2f) wrapped,
    /// and the facing and model turned to it.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, context.shield_normal);
        let angle = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        shot_mut(item).angle = wrap_angle(angle);
        item.facing = if item.velocity.x >= 0.0 { 1.0 } else { -1.0 };
        item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        false
    }
}

/// The reflection's wrap: add M_TAU below zero, subtract it above (the
/// float promoted to double for each compare and step, as retail does).
fn wrap_angle(mut angle: f32) -> f32 {
    while f64::from(angle) < 0.0 {
        angle = (f64::from(angle) + std::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > std::f64::consts::TAU {
        angle = (f64::from(angle) - std::f64::consts::TAU) as f32;
    }
    angle
}

/// it_80275158: the lifetime and its half (it_804D6D28->x4C).
fn set_lifetime(item: &mut ItemCore, frames: f32, half_life_scale: f32) {
    item.life_timer = frames;
    item.half_life = frames * half_life_scale;
}

/// it_802B5CBC (802B5CBC): the lifetime again, the hand glow goes, the
/// level clamped to 0..7 picks the flight state, and the accessory begins.
fn take_flight_state(item: &mut ItemCore, lifetime: f32, half_life_scale: f32, assets: &ItemAssets) {
    set_lifetime(item, lifetime, half_life_scale);
    item.events.push(ItemEvent::DestroyEffects);
    let state = shot_mut(item);
    state.glowing = false;
    state.level = state.level.clamp(0, MAXIMUM_LEVEL);
    let motion = (state.level + 1) as u16;
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
}

/// itSamuschargeshot_UnkMotion0_Anim (802B5ABC): the shot ends once its
/// owner leaves the charge (ftSs_SpecialLw_80129158 /
/// ftSs_SpecialN_801291A8) or lets go of it (ftSs_SpecialLw_80129100);
/// otherwise it takes the owner's charge level and full level, whose
/// scale (802B5C00: fmadds) its grandchild shows.
fn hand_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let original = shot(item).original_owner;
    if let (Some(owner), true) = (ctx.owner, original.is_some() && item.owner == original) {
        // x2071_b6 is clear in every Samus charge state.
        if !OWNER_CHARGE_STATES.contains(&owner.motion) {
            return true;
        }
        let Some((level, full)) = owner.charge else {
            return true;
        };
        let state = shot_mut(item);
        state.level = level;
        state.full = full;
    }
    let (small, large) = Attributes(&ctx.assets.special_attributes).scale();
    let state = shot_mut(item);
    state.scale = gekko_math::fma::fmadds(
        state.level as f32,
        (large - small) / state.full as f32,
        small,
    );
    false
}

/// itSamuschargeshot_UnkMotion8_Anim: the grandchild keeps the flight's
/// scale; the lifetime counts down (it_80273130).
fn flight_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itSamuschargeshot_UnkMotion8_Phys: speed along the angle (fmuls after
/// cosf and sinf), every frame.
fn flight_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let state = shot(item);
    let (speed, angle) = (state.speed, state.angle);
    item.velocity.x = speed * gekko_math::msl::cosf(angle);
    item.velocity.y = speed * gekko_math::msl::sinf(angle);
}

/// itSamuschargeshot_UnkMotion8_Coll -> it_802B5518 (802B5518): an
/// airborne pass (it_8026DA08), then the ceiling when rising or the floor
/// otherwise, and the left wall when flying right or the right wall
/// otherwise, end it.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    use melee_types::mp::collide;
    item.air_pass(ctx.map);
    let env = item.collision.as_ref().expect("item map collision").env_flags as u32;
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

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
