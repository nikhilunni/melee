//! Link's and Young Link's boomerang (It_Kind_Link_Boomerang /
//! It_Kind_CLink_Boomerang), itlinkboomerang.c (8029FDBC..802A23CC).
//!
//! The side special creates it in the thrower's hand (it_802A013C) and
//! throws it along the stick's angle (it_802A0534). Outbound it slows by
//! a fixed step; below a floor speed, or when it hits something, it turns
//! back (it_802A1948) and, for a while, homes on the thrower's raised
//! position, speeding up to a cap. Close enough, the thrower catches it
//! (ftLk_SpecialS2_Enter) unless busy. Surfaces met at a glancing angle
//! mirror its flight; head on they send it back. A reflected boomerang
//! just flies until its lifetime ends. The trail models and the return's
//! model spin are drawing only.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::ItemKind;

/// it_803F6920's anim_id column.
pub const ARTICLE_STATES: [i32; 4] = [-1, 0, 1, 2];
/// itLinkBoomerangAttributes x0..x40 (the joints and animations after are
/// the trail models').
pub const SPECIAL_ATTRIBUTES: u32 = 17;
/// ftData.x48_items index (ftLk_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;

mod motion {
    /// In the thrower's hand (itLinkboomerang_UnkMotion0).
    pub const HELD: u16 = 0;
    /// Outbound, before and after its script's turn (UnkMotion1 / 2).
    pub const OUT: u16 = 1;
    pub const OUT_LATE: u16 = 2;
    /// Coming back (UnkMotion3).
    pub const RETURN: u16 = 3;
}

/// MTXDegToRad's factor as MWCC rounds it.
const DEGREES_TO_RADIANS: f32 = 0.017_453_292;
/// Item_8026AF0C's flight sounds: Link's and Young Link's.
const LINK_FLIGHT_SOUND: u32 = 0x2715B;
const YOUNG_LINK_FLIGHT_SOUND: u32 = 0x111CB;
/// The x2071 nibble (Fighter.x2070's x2071_b0_3) ranges the boomerang
/// tests of its thrower (ftLk_SpecialS_Is2071b0_5to13 / _1to13).
const BUSY_CLASSES: std::ops::RangeInclusive<u8> = 1..=13;
const HURT_CLASSES: std::ops::RangeInclusive<u8> = 5..=13;
/// The thrower's part the caught boomerang hangs from
/// (ftLk_SpecialHi_ProcessPartLThumbNb: FtPart_LThumbNb's joint).
pub const CATCH_PART: melee_types::FtPart = melee_types::FtPart::LThumbNb;

/// itLinkBoomerangAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn int(&self, index: usize) -> u32 {
        self.0[index].to_bits()
    }
    /// x0 / x4: the lifetime of an ordinary and of a smash throw.
    fn lifetime(&self, smash: bool) -> f32 {
        self.int(usize::from(smash)) as f32
    }
    /// x8: frames the return's spin runs.
    fn spin_frames(&self) -> i32 {
        self.int(2) as i32
    }
    /// xC: the speed lost (outbound) or gained (returning) per frame.
    fn speed_step(&self) -> f32 {
        self.0[3]
    }
    /// x14: the return's top speed.
    fn return_speed(&self) -> f32 {
        self.0[5]
    }
    /// x18: the outbound speed that turns it back.
    fn turn_speed(&self) -> f32 {
        self.0[6]
    }
    /// x1C: the glancing angle limit (degrees).
    fn glance_degrees(&self) -> f32 {
        self.0[7]
    }
    /// x20 / x24: the return's turn per frame (degrees), after running out
    /// of speed and after a hit.
    fn turn_degrees(&self, hit: bool) -> f32 {
        self.0[if hit { 9 } else { 8 }]
    }
    /// x28: frames the return homes.
    fn homing_frames(&self) -> f32 {
        self.0[10]
    }
    /// x2C: the distance the thrower catches it from.
    fn catch_distance(&self) -> f32 {
        self.0[11]
    }
    /// x38: frames between flight sounds.
    fn sound_interval(&self) -> f32 {
        self.0[14]
    }
    /// x3C / x40: frames before the first and the second trail model.
    fn first_trail(&self) -> f32 {
        self.0[15]
    }
    fn second_trail(&self) -> f32 {
        self.0[16]
    }
}

static STATES: [ItemStateRow; 4] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: held_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: flight_animation,
        physics: out_physics,
        collision: out_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: flight_animation,
        physics: out_physics,
        collision: out_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: return_animation,
        physics: return_physics,
        collision: no_collision,
    },
];

/// The boomerang of Link (`YOUNG = false`) or Young Link.
pub struct Boomerang<const YOUNG: bool>;

/// it_3F2F.c's Link boomerang logic row (the same for Young Link's).
impl<const YOUNG: bool> ItemLogic for Boomerang<YOUNG> {
    const KIND: ItemKind = if YOUNG {
        ItemKind::CLinkBoomerang
    } else {
        ItemKind::LinkBoomerang
    };
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802A013C (802A013C), after Item_80268B18: the lifetime, cleared
    /// command variables, the glance limit (-sinf of the attribute angle)
    /// and the thrower. Item_8026AB54 follows (the pickup callback).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        let lifetime = a.lifetime(false);
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.command_variables[..3].fill(0);
        item.scratch = ItemScratch::Boomerang(BoomerangState {
            thrower: spawn.owner,
            thrown_by_link: !YOUNG,
            glance_limit: -gekko_math::msl::sinf(DEGREES_TO_RADIANS * a.glance_degrees()),
            trail_timer: a.first_trail(),
            ..Default::default()
        });
    }
    /// itLinkBoomerang_Logic18_Destroyed (802A08A4): the thrower lets go
    /// (remove_boomerang, through `notifies_owner`).
    fn destroyed(item: &mut ItemCore) {
        item.owner = None;
        item.held = false;
    }
    /// it_802A0E70 (802A0E70): in the hand, state 0.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.change_motion_with(motion::HELD, ARTICLE_STATES[0], ANIM_UPDATE, context.assets);
    }
    /// it_802A0F84: nothing.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {}
    /// it_802A0F88 (802A0F88): outbound, state 1.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::OUT, ANIM_UPDATE, context.assets);
    }
    /// it_802A1F08: a hit sends it back.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if matches!(item.motion, motion::OUT | motion::OUT_LATE) {
            turn_back_after_hit(item, context.assets);
        }
        false
    }
    /// it_802A20E8 (802A20E8): once, the reflector sends it off away from
    /// itself (ftLib_800866DC) at its speed times xC70 (the length's
    /// fmuls, then xC70's), with the half-life left; the thrower lets go
    /// (ftLk_SpecialS_RemoveBoomerang0, through `forgotten`) and it no
    /// longer steers.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if boomerang(item).reflected {
            return false;
        }
        item.life_timer = item.half_life;
        let from = context.reflector_position;
        let angle = wrap_turns(melee_lb::trigf::atan2f(
            item.position.y - from.y,
            item.position.x - from.x,
        ));
        let length = speed(item.velocity);
        let multiplier = context.reflected_speed;
        item.velocity.x = multiplier * (length * gekko_math::msl::cosf(angle));
        item.velocity.y = multiplier * (length * gekko_math::msl::sinf(angle));
        let state = boomerang_mut(item);
        state.reflected = true;
        state.angle = angle;
        if let Some(thrower) = state.thrower {
            item.owner_request = Some((thrower, OwnerRequest::Released));
        }
        face(item);
        false
    }
    /// it_802A1FA8: as a hit.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        Self::damage_dealt(item, context)
    }
    /// itLinkBoomerang_Logic18_Absorbed: as a hit.
    fn absorbed(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        Self::damage_dealt(item, context)
    }
    /// it_802A2320 (802A2320): off a shield's surface, mirrored.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, context.shield_normal);
        let angle = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        let state = boomerang_mut(item);
        state.angle = wrap_turns(angle);
        face(item);
        false
    }
    /// it_802A2288: a shield sends it back unless it already returns.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if item.motion != motion::RETURN {
            turn_back_after_hit(item, context.assets);
        }
        false
    }
    /// it_802A23CC (802A23CC): it_8026B894, and the thrower is gone.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
        let state = boomerang_mut(item);
        if state.thrower == Some(owner) {
            state.thrower = None;
        }
    }
    /// it_802A07B4 (802A07B4) from the thrower (ftLk_SpecialS_
    /// RemoveBoomerang1, the catch's drop): the boomerang is destroyed.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => item.destroyed = true,
            _ => unimplemented!("boomerang item control {control:?}"),
        }
    }
    /// remove_boomerang (802A07B4, Logic18_Destroyed): the thrower's
    /// ftLk_SpecialS_RemoveBoomerang0 runs unless a reflector took it.
    fn notifies_owner(item: &ItemCore) -> bool {
        let state = boomerang(item);
        state.thrower.is_some() && item.owner == state.thrower && !state.reflected
    }
    /// it_802A0534 (802A0534).
    fn launch(
        item: &mut ItemCore,
        launch: &Launch,
        common: &melee_it::desc::ItemCommonData,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        throw(item, launch, common.half_life_scale, map, assets);
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn boomerang(item: &ItemCore) -> &BoomerangState {
    match &item.scratch {
        ItemScratch::Boomerang(state) => state,
        _ => panic!("boomerang scratch"),
    }
}
fn boomerang_mut(item: &mut ItemCore) -> &mut BoomerangState {
    match &mut item.scratch {
        ItemScratch::Boomerang(state) => state,
        _ => panic!("boomerang scratch"),
    }
}

/// it_802A0534 (802A0534): the lifetime (a smash throw's is longer), the
/// flight angle, the outbound state (the thrown callback), then out of the
/// hand at `launch.velocity` (it_8027429C), never pickable (it_8026B3A8),
/// the model turned to the angle.
fn throw(
    item: &mut ItemCore,
    launch: &Launch,
    half_life_scale: f32,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
) {
    let a = Attributes(&assets.special_attributes);
    let lifetime = a.lifetime(launch.long_lifetime);
    item.life_timer = lifetime;
    item.half_life = lifetime * half_life_scale;
    boomerang_mut(item).angle = launch.angle;
    face(item);
    change(item, motion::OUT, ANIM_UPDATE, assets);
    // it_8027429C: it_80273B50 at the hand, then it_80273F34 (see the
    // Yoshi egg's launch for the hung article's offset).
    let t = assets.attachment_translation;
    let mut hand = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(&launch.hand, &Vec3::new(-t.x, -t.y, -t.z), &mut hand);
    item.leave_hand(launch.velocity, hand, assets);
    item.end_hold(launch.center, launch.attack, map, assets);
    item.stale_multiplier = launch.attack_stale;
    item.grabbable = false;
    item.rotation.z = boomerang(item).facing_angle;
}

/// The flight angle turned for the facing (xF78): the angle itself facing
/// right, less pi (in double) facing left.
fn face(item: &mut ItemCore) {
    let facing = item.facing;
    let state = boomerang_mut(item);
    state.facing_angle = if facing == 1.0 {
        state.angle
    } else {
        (f64::from(state.angle) - std::f64::consts::PI) as f32
    };
}

/// clamp_tau: one turn off an angle past +-2pi (double compares and sums).
fn wrap_turns(angle: f32) -> f32 {
    use std::f64::consts::TAU;
    if f64::from(angle) > TAU {
        (f64::from(angle) - TAU) as f32
    } else if f64::from(angle) < -TAU {
        (f64::from(angle) + TAU) as f32
    } else {
        angle
    }
}

/// clamp_pi_tau: into -pi..pi (double compares and sums).
fn wrap_half_turns(angle: f32) -> f32 {
    use std::f64::consts::{PI, TAU};
    if f64::from(angle) < -PI {
        (f64::from(angle) + TAU) as f32
    } else if f64::from(angle) > PI {
        (f64::from(angle) - TAU) as f32
    } else {
        angle
    }
}

/// clamp_angle_pi: the opposite direction (a float test, a double sum).
fn reverse(angle: f32) -> f32 {
    use std::f64::consts::PI;
    if angle <= 0.0 {
        (f64::from(angle) + PI) as f32
    } else {
        (f64::from(angle) - PI) as f32
    }
}

/// The XY speed through the inline MSL sqrtf, squares summed unfused
/// (802A1004..14).
fn speed(v: Vec3) -> f32 {
    gekko_math::msl::sqrtf(v.x * v.x + v.y * v.y)
}

/// it_802A1948 (802A1948) with `hit` for the after-hit turn rate
/// (it_802A1F08 and siblings inline the same): reversed, turning at most
/// the attribute angle per frame for the homing frames, then the return
/// (it_802A19E0).
fn turn_back(item: &mut ItemCore, assets: &ItemAssets, hit: bool) {
    let a = Attributes(&assets.special_attributes);
    let state = boomerang_mut(item);
    state.angle = reverse(state.angle);
    // 802A19A8 / 802A19BC: fmuls.
    state.turn_limit = DEGREES_TO_RADIANS * a.turn_degrees(hit);
    state.homing_frames = a.homing_frames();
    enter_return(item, assets);
}

/// it_802A1F08 / it_802A1FA8 / Absorbed / it_802A2288's shared body.
fn turn_back_after_hit(item: &mut ItemCore, assets: &ItemAssets) {
    if !boomerang(item).reflected {
        turn_back(item, assets, true);
    }
}

/// it_802A19E0 (802A19E0): the return state (keeping the hitbox; from the
/// first outbound state the animation restarts, from the second the
/// script carries on), the model turned to the facing angle. The model's
/// spin (xF88 from the child's rotation, xDE4) is drawing.
fn enter_return(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    boomerang_mut(item).spin_frames = a.spin_frames();
    let flags = if item.motion == motion::OUT {
        ANIM_UPDATE | HIT_PRESERVE
    } else {
        HIT_PRESERVE | CMD_UPDATE
    };
    change(item, motion::RETURN, flags, assets);
    item.rotation.z = boomerang(item).facing_angle;
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itLinkboomerang_UnkMotion0_Anim (802A0EFC): a thrower struck in hand
/// (x2071 class 5..13) drops it: it is destroyed (it_802A07B4).
fn held_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let Some(owner) = ctx.owner else {
        return false;
    };
    let flags = owner
        .motion_flags
        .expect("the boomerang's thrower's motion flags");
    HURT_CLASSES.contains(&melee_ft_class(flags))
}

/// Fighter.x2070's x2071 nibble.
fn melee_ft_class(flags: u32) -> u8 {
    ((flags >> 20) & 0xF) as u8
}

/// it_802A0C34 (802A0C34), every flying state's animation: the flight
/// sound every x38 frames, the script's turn to the second outbound state
/// (it_802A10E4 on command variable 0), the trail models' start times
/// (command variables 1 and 2), and the lifetime.
fn flight_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let a = Attributes(&ctx.assets.special_attributes);
    let state = boomerang_mut(item);
    if state.sound_timer <= 0.0 {
        let sound = if state.thrown_by_link {
            LINK_FLIGHT_SOUND
        } else {
            YOUNG_LINK_FLIGHT_SOUND
        };
        state.sound_timer = a.sound_interval();
        item.sound_requests.push(sound);
    }
    boomerang_mut(item).sound_timer -= 1.0;
    if item.command_variables[0] == 1 {
        // it_802A10E4: state 2, keeping the model's angles.
        change(item, motion::OUT_LATE, ANIM_UPDATE, ctx.assets);
        item.rotation.z = boomerang(item).facing_angle;
        item.command_variables[0] = 2;
    }
    count_down_trails(item, &a);
    if item.life_timer <= 0.0 {
        return true;
    }
    item.life_timer -= 1.0;
    false
}

/// it_802A0C34_sub_1: the first trail model starts after x3C frames, the
/// second x40 frames later (command variables 1 and 2 become 2).
fn count_down_trails(item: &mut ItemCore, a: &Attributes<'_>) {
    let vars = &mut item.command_variables;
    let state = match &mut item.scratch {
        ItemScratch::Boomerang(state) => state,
        _ => panic!("boomerang scratch"),
    };
    if vars[1] == 0 {
        state.trail_timer -= 1.0;
        if state.trail_timer <= 0.0 {
            vars[1] = 2;
            state.trail_timer = a.second_trail();
        }
    } else if vars[2] == 0 {
        state.trail_timer -= 1.0;
        if state.trail_timer <= 0.0 {
            vars[2] = 2;
        }
    }
}

/// itLinkboomerang_UnkMotion3_Anim (802A1CEC): the model's spin (drawing),
/// then it_802A0C34.
fn return_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = boomerang_mut(item);
    if state.spin_frames != 0 {
        state.spin_frames -= 1;
    }
    flight_animation(item, ctx)
}

/// itLinkboomerang_UnkMotion1_Phys / 2_Phys (802A0FD0): unless reflected,
/// the speed drops by xC; below x18 it turns back (it_802A1948, which
/// reverses the angle first) at that speed; the velocity follows the angle.
fn out_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if boomerang(item).reflected {
        return;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    let mut speed = speed(item.velocity) - a.speed_step();
    if speed < a.turn_speed() {
        speed = a.turn_speed();
        turn_back(item, ctx.assets, false);
    }
    let angle = boomerang(item).angle;
    item.velocity.x = speed * gekko_math::msl::cosf(angle);
    item.velocity.y = speed * gekko_math::msl::sinf(angle);
}

/// itLinkboomerang_UnkMotion3_Phys (802A1D5C): unless reflected, the speed
/// grows by xC up to x14 along the angle; then it homes (it_802A13EC) and,
/// close enough while still returning, the thrower catches it, or, busy,
/// it is destroyed.
fn return_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if boomerang(item).reflected {
        return;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    let mut speed = speed(item.velocity) + a.speed_step();
    if speed > a.return_speed() {
        speed = a.return_speed();
    }
    let angle = boomerang(item).angle;
    item.velocity.x = speed * gekko_math::msl::cosf(angle);
    item.velocity.y = speed * gekko_math::msl::sinf(angle);
    let distance = home(item, ctx.owner);
    if item.motion == motion::RETURN && distance < a.catch_distance() {
        catch_or_drop(item, ctx.owner);
    }
}

/// itLinkboomerang_UnkMotion3_Phys_sub: the thrower catches it unless its
/// motion is busy (x2071 class 1..13) or in hitlag; otherwise it goes. The
/// trail models go either way (it_802A0810).
fn catch_or_drop(item: &mut ItemCore, owner: Option<&ItemOwner>) {
    if boomerang(item).thrower.is_none() {
        return;
    }
    let owner = owner.expect("the boomerang's thrower");
    let flags = owner
        .motion_flags
        .expect("the boomerang's thrower's motion flags");
    if !BUSY_CLASSES.contains(&melee_ft_class(flags)) && !owner.in_hitlag {
        let thrower = boomerang(item).thrower.expect("checked above");
        item.owner_request = Some((thrower, OwnerRequest::Catch));
        // Item_8026AB54 runs inside the proc: held, the item no longer
        // moves by its velocity this frame (Item_802697D4). The scene
        // completes the attachment once the proc returns.
        item.held = true;
    } else {
        item.destroyed = true;
    }
}

/// it_802A13EC (802A13EC): the distance to the thrower's raised position
/// (802A143C: fmadds, the x square outer); while homing frames remain, the
/// angle turns toward it by at most xF84 (double wraps).
fn home(item: &mut ItemCore, owner: Option<&ItemOwner>) -> f32 {
    if boomerang(item).thrower.is_none() {
        return 0.0;
    }
    let anchor = owner.expect("the boomerang's thrower").anchor;
    let dy = anchor.y - item.position.y;
    let dx = anchor.x - item.position.x;
    let distance = gekko_math::msl::sqrtf(gekko_math::fma::fmadds(dx, dx, dy * dy));
    let state = boomerang_mut(item);
    if state.homing_frames > 0.0 {
        state.homing_frames -= 1.0;
    } else if state.homing_frames <= 0.0 {
        return distance;
    }
    let mut turn = wrap_half_turns(melee_lb::trigf::atan2f(dy, dx));
    turn = wrap_half_turns(turn - state.angle);
    if turn > state.turn_limit {
        turn = state.turn_limit;
    } else if turn < -state.turn_limit {
        turn = -state.turn_limit;
    }
    state.angle = wrap_turns(state.angle + turn);
    face(item);
    distance
}

/// itLinkboomerang_UnkMotion2_Coll (802A16E4): unless reflected, a pass
/// that never lands (it_8026E0F4), the model's angles kept; a wall ahead
/// (by facing), else the ceiling, else the floor, deflects it
/// (it_802A15EC).
fn out_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if boomerang(item).reflected {
        return false;
    }
    item.airborne_pass(ctx.map);
    item.rotation.z = boomerang(item).facing_angle;
    let collision = item.collision.as_ref().expect("boomerang map collision");
    let env = collision.env_flags as u32;
    use melee_types::mp::collide;
    let normal = if item.facing == -1.0 && env & collide::RIGHT_WALL_MASK != 0 {
        Some(collision.right_facing_wall.normal)
    } else if item.facing != -1.0 && env & collide::LEFT_WALL_MASK != 0 {
        Some(collision.left_facing_wall.normal)
    } else if env & collide::CEILING_MASK != 0 {
        Some(collision.ceiling.normal)
    } else if env & collide::FLOOR_MASK != 0 {
        Some(collision.floor.normal)
    } else {
        None
    };
    if let Some(normal) = normal {
        deflect(item, normal, ctx.assets);
    }
    false
}

/// it_802A15EC (802A15EC): flying into the surface, a glancing meeting
/// (cosine above xF7C) mirrors the flight (lbVector_Mirror) and the item
/// is airborne again (it_802762BC); a straighter one turns it back.
fn deflect(item: &mut ItemCore, normal: Vec3, assets: &ItemAssets) {
    let cosine = melee_lb::vector::cos_angle(item.velocity, normal);
    if cosine >= 0.0 {
        return;
    }
    if cosine > boomerang(item).glance_limit {
        item.velocity = melee_lb::vector::mirror(item.velocity, normal);
        let angle = melee_lb::trigf::atan2f(item.velocity.y, item.velocity.x);
        boomerang_mut(item).angle = wrap_turns(angle);
        face(item);
        item.ground_or_air = melee_types::GroundOrAir::Air;
    } else {
        turn_back(item, assets, false);
    }
}
