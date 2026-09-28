//! Link's and Young Link's bomb (It_Kind_Link_Bomb / It_Kind_CLink_Bomb),
//! itlinkbomb.c (8029D968..8029FDBC).
//!
//! Pulled straight into the hand, the bomb burns its lifetime wherever it
//! is; below x0C frames the fuse is lit (the model switches to article
//! state 3, and later state changes keep that animation). It goes off when
//! the lifetime runs out, when it strikes or lands too fast, or when a hit
//! deals x10 damage to it. A thrown or dropped bomb that lands slowly rolls
//! to a stop with x2C friction. The fuse joint's shrinking (bone 3, x8 and
//! xC) is drawing only.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, CMD_UPDATE, DROP_UPDATE, HIT_PRESERVE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, LinkBombState,
    SpawnItem,
};
use gekko_math::{fma::fmadds, msl::fabsf};
use hsd_types::Vec3;
use melee_types::ItemKind;

/// it_803F6888's anim_id column.
pub const ARTICLE_STATES: [i32; 7] = [0, 0, 1, 1, 0, 2, 0];
/// itLinkBombAttributes: x0..x30 and vel[3].
pub const SPECIAL_ATTRIBUTES: u32 = 16;
/// ftData.x48_items index.
pub const ARTICLE_INDEX: u32 = 0;

/// The joint animation the lit fuse plays (Item_80268D34 with desc 3).
const LIT_ANIMATION: usize = 3;
/// Article states the model carries: the three the motion states name and
/// the lit fuse's.
pub const ARTICLE_STATE_COUNT: usize = LIT_ANIMATION + 1;
/// it_80272A60: efSync_Spawn(0x40E) and Item_8026AE84(item, 0x74, ...).
const BLAST_EFFECT: u16 = 0x40E;
const BLAST_SOUND: u32 = 0x74;

mod motion {
    /// In the hand (it_8029DEB0).
    pub const HELD: u16 = 0;
    /// Off a floor it rolled on (fn_8029E21C).
    pub const FALLING: u16 = 1;
    pub const THROWN: u16 = 2;
    pub const DROPPED: u16 = 3;
    /// Rolling along the floor (it_8029F18C).
    pub const ROLLING: u16 = 4;
    pub const EXPLODING: u16 = 5;
    /// Logic16_EnteredAir: sliding down a slope.
    pub const SLIDING: u16 = 6;
}

/// itLinkBombAttributes (itCharItems.h), read from the special words.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0 (a u32): the lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0].to_bits() as f32
    }
    /// xC (a u32): the lifetime left when the fuse lights.
    fn lit_at(&self) -> f32 {
        self.0[3].to_bits() as f32
    }
    /// x10 (an s32): damage in one frame that sets it off.
    fn damage_limit(&self) -> i32 {
        self.0[4].to_bits() as i32
    }
    /// x14 / x18: the speeds a hit knocks it about at.
    fn knock_speed(&self) -> (f32, f32) {
        (self.0[5], self.0[6])
    }
    /// x1C / x20: the speeds it rebounds off what it struck at.
    fn rebound_speed(&self) -> (f32, f32) {
        (self.0[7], self.0[8])
    }
    /// x24 / x28: striking or landing faster than these sets it off.
    fn blast_speed(&self) -> (f32, f32) {
        (self.0[9], self.0[10])
    }
    /// x2C: the roll's friction per frame.
    fn friction(&self) -> f32 {
        self.0[11]
    }
    /// x30: the roll stops below this speed.
    fn stop_speed(&self) -> f32 {
        self.0[12]
    }
}

fn state(item: &mut ItemCore) -> &mut LinkBombState {
    match &mut item.scratch {
        ItemScratch::LinkBomb(state) => state,
        _ => unreachable!("a bomb without its state"),
    }
}

/// it_8029D9A4 (8029D9A4): the state change. Unlit, the state's animation
/// restarts; lit, only the script restarts and the lit animation plays on
/// (the change's HSD_JObjAnimAll advances it a frame). Bone 3's fuse pose
/// is kept across the change (drawing only).
fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    if state(item).lit {
        let frame = item.animation_frame;
        let article = item.article_state as i32;
        item.change_motion_with(motion, article, flags | CMD_UPDATE, assets);
        item.animation_frame = frame + item.animation_rate;
    } else {
        item.change_motion_with(
            motion,
            ARTICLE_STATES[usize::from(motion)],
            flags | ANIM_UPDATE,
            assets,
        );
    }
}

/// it_8029DB5C and its inline copies: out of lifetime it goes off;
/// otherwise the fuse lights at x0C frames left (the model plays article
/// state 3 from its start) and a frame of lifetime burns.
fn burn(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    if item.life_timer <= 0.0 {
        explode(item, ctx.holder.as_mut(), Some(ctx.map), ctx.assets);
        return;
    }
    let lit_at = Attributes(&ctx.assets.special_attributes).lit_at();
    if item.life_timer <= lit_at && !state(item).lit {
        item.play_article_animation(LIT_ANIMATION, ctx.assets);
        state(item).lit = true;
    }
    item.life_timer -= 1.0;
}

/// it_8029F69C (8029F69C): the blast. A bomb in hand leaves it with no
/// speed (it_8027429C) and takes no hits (it_802756D0); its owner's model
/// group 2 returns with the empty hand (ftLk_AttackAir_SetupParts). Then
/// no pickup, no speed, hidden, the explosion lifetime without a destroy
/// effect (it_8027518C), no hits, the blast effect with hitlag off
/// (it_80272A60), state 5 whose script makes the blast hitbox, and a
/// radial gust (lb_800119DC).
fn explode(
    item: &mut ItemCore,
    holder: Option<&mut melee_it::ItemHolder<'_>>,
    map: Option<&mut melee_mp::CollMap>,
    assets: &ItemAssets,
) {
    // it_80275444: the blast hits its owner and items sharing its owner.
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    if item.held {
        let holder = holder.expect("it_8027429C: a held bomb's holder");
        let map = map.expect("it_8027429C: a held bomb's map");
        item.release_from_holder(Vec3::ZERO, holder, map, assets);
        item.hurt_intangible = true;
    }
    // it_8026B3A8, it_80273454, JOBJ_HIDDEN.
    item.grabbable = false;
    item.velocity = Vec3::ZERO;
    item.hidden = true;
    // it_8027518C.
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    // it_802756D0.
    item.hurt_intangible = true;
    // it_80272A60.
    item.events.push(ItemEvent::Effect {
        id: BLAST_EFFECT,
        position: item.position,
    });
    item.sound_requests.push(BLAST_SOUND);
    item.hitlag_enabled = false;
    change(item, motion::EXPLODING, 0, assets);
    // lb_800119DC(&pos, 0x78, 1.0, 0.02, pi/3).
    item.events.push(ItemEvent::Gust {
        center: item.position,
        frames: 0x78,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
}

/// it_LinkBomb_Inline_VelocityCompare: faster than x24 / x28 on either
/// axis, it goes off. True when it did.
fn blast_if_fast(item: &mut ItemCore, velocity: Vec3, assets: &ItemAssets) -> bool {
    let (x, y) = Attributes(&assets.special_attributes).blast_speed();
    if fabsf(velocity.x) > x || fabsf(velocity.y) > y {
        explode(item, None, None, assets);
        return true;
    }
    false
}

/// it_8029F18C (8029F18C): it rolls the way it was moving (the collision
/// faces that way) in state 4.
fn roll(item: &mut ItemCore, assets: &ItemAssets) {
    let direction = if item.velocity.x >= 0.0 { 1.0 } else { -1.0 };
    let s = state(item);
    s.roll_direction = direction;
    s.rolling = true;
    item.set_collision_facing(if direction == -1.0 { -1 } else { 1 });
    change(item, motion::ROLLING, 0, assets);
}

/// fn_8029E21C (8029E21C): falling (state 1).
fn fall(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALLING, 0, assets);
}

/// it_8029E5D0 / it_8029EC34: thrown (2) or dropped (3): the owner's and
/// its kin's hits land on it (it_802754A4) and its own reach its kin
/// (it_80275414); a repeat keeps the live hitboxes.
fn leave_hand(item: &mut ItemCore, motion: u16, assets: &ItemAssets) {
    item.strikes_kindred_items = true;
    item.hurt_by_owner = true;
    let flags = if item.motion == motion {
        HIT_PRESERVE | DROP_UPDATE
    } else {
        DROP_UPDATE
    };
    change(item, motion, flags, assets);
}

/// The bomb of Link (`YOUNG = false`) or Young Link.
pub struct Bomb<const YOUNG: bool>;

impl<const YOUNG: bool> ItemLogic for Bomb<YOUNG> {
    const KIND: ItemKind = if YOUNG {
        ItemKind::CLinkBomb
    } else {
        ItemKind::LinkBomb
    };
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_8029DD58 (8029DD58) once Item_80268B18 returns: the lifetime
    /// (it_80275158), unlit, the puller noted.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let lifetime = Attributes(&assets.special_attributes).lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.scratch = ItemScratch::LinkBomb(LinkBombState {
            puller: spawn.owner,
            ..LinkBombState::default()
        });
    }
    /// it_8029DEB0 (8029DEB0): state 0 in the hand.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, 0, ctx.assets);
    }
    /// it_8029E5D0 (8029E5D0).
    fn thrown(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        leave_hand(item, motion::THROWN, ctx.assets);
    }
    /// it_8029EC34 (8029EC34).
    fn dropped(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        leave_hand(item, motion::DROPPED, ctx.assets);
    }
    /// Logic16_EnteredAir (8029FAA8): state 6.
    fn entered_air(_item: &mut ItemCore) {
        unimplemented!("itLinkBomb_Logic16_EnteredAir: a bomb sliding down a slope");
    }
    /// it_8029F960 (8029F960): unless it goes off from its speed, it turns
    /// back and rebounds at x1C / x20.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::EXPLODING {
            return false;
        }
        let velocity = item.velocity;
        if !blast_if_fast(item, velocity, ctx.assets) {
            let (x, y) = Attributes(&ctx.assets.special_attributes).rebound_speed();
            item.facing = -item.facing;
            item.velocity.x = x * item.facing;
            item.velocity.y = y;
        }
        false
    }
    /// itLinkBomb_Logic16_DmgReceived (8029FA20): x10 damage in a frame
    /// sets it off; otherwise the first hit knocks it about: away from the
    /// hit when moving past x30, else a random way (2 * (HSD_Randf - 0.5),
    /// facing it, it_80272980), up at x18 by its facing.
    fn damage_received(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        if item.motion == motion::EXPLODING {
            return false;
        }
        let a = Attributes(&ctx.assets.special_attributes);
        if item.pending_damage_taken >= a.damage_limit() {
            explode(item, None, None, ctx.assets);
            return false;
        }
        if state(item).knocked {
            return false;
        }
        let (x, y) = a.knock_speed();
        if item.velocity.x > a.stop_speed() {
            item.facing = -item.hit_direction;
            item.velocity.x = x * item.facing;
        } else {
            let cell = ctx.rng.expect("Logic16_DmgReceived draws");
            let mut rng = cell.get();
            let way = 2.0 * (rng.randf() - 0.5);
            cell.set(rng);
            item.velocity.x = x * way;
            item.face_velocity();
        }
        item.velocity.y = y * item.facing;
        state(item).knocked = true;
        false
    }
    /// Logic16_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// Logic16_HitShield -> itColl_BounceOffVictim.
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(ctx.victim_bounce);
        false
    }
    /// Logic16_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(ctx.shield_normal);
        false
    }
    /// it_8029FD84 (8029FD84): the puller is forgotten, then it_8026B894.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        let s = state(item);
        if s.puller == Some(owner) {
            s.puller = None;
        }
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
}

const fn row(
    motion: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion as usize],
        animation,
        physics,
        collision,
    }
}

static STATES: [ItemStateRow; 7] = [
    row(motion::HELD, held_animation, no_physics, no_collision),
    row(motion::FALLING, falling_animation, falling_physics, falling_collision),
    row(motion::THROWN, thrown_animation, flight_physics, flight_collision),
    row(motion::DROPPED, dropped_animation, flight_physics, flight_collision),
    row(motion::ROLLING, rolling_animation, rolling_physics, rolling_collision),
    row(motion::EXPLODING, blast_animation, no_physics, no_collision),
    row(motion::SLIDING, sliding_animation, no_physics, sliding_collision),
];

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itLinkbomb_UnkMotion0_Anim (8029DF6C): the hold's animation loops.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        change(item, motion::HELD, 0, ctx.assets);
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion1_Anim (8029E240).
fn falling_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        fall(item, ctx.assets);
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion2_Anim (8029E650).
fn thrown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        leave_hand(item, motion::THROWN, ctx.assets);
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion3_Anim (8029ECB4): the drop's animation loops; if
/// it is still over after that (a lit bomb's, which the loop does not
/// restart), it falls.
fn dropped_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        leave_hand(item, motion::DROPPED, ctx.assets);
    }
    if item.article_animation_ended(ctx.assets) {
        fall(item, ctx.assets);
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion4_Anim (8029F214).
fn rolling_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        roll(item, ctx.assets);
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion5_Anim -> it_802751D8: the blast lasts its lifetime.
fn blast_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itLinkbomb_UnkMotion6_Anim (8029FAC8).
fn sliding_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        item.change_motion_with(
            motion::SLIDING,
            ARTICLE_STATES[usize::from(motion::SLIDING)],
            ANIM_UPDATE,
            ctx.assets,
        );
    }
    burn(item, ctx);
    false
}

/// itLinkbomb_UnkMotion1_Phys -> it_80272860: gravity only.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// Item_ApplyFallingPhysics: gravity, then the falling spin.
fn flight_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itLinkbomb_UnkMotion4_Phys (8029F5F0): a rolling bomb slows by x2C
/// (retail 8029F63C: fmadds) and stops below x30.
fn rolling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let s = *state(item);
    if !s.rolling {
        return;
    }
    if item.velocity.x != 0.0 {
        item.velocity.x = fmadds(a.friction(), s.roll_direction, item.velocity.x);
    }
    if fabsf(item.velocity.x) < a.stop_speed() {
        item.velocity.x = 0.0;
    }
}

/// itLinkbomb_UnkMotion1_Coll -> it_8026E15C: bounces, and a settled
/// landing rolls (it_8029F18C).
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne => false,
        AirLanding::Landed => {
            roll(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itLinkbomb_UnkMotion2_Coll (8029E6F4), and state 3's: any contact at
/// speed sets it off; a slow floor contact grounds it (it_802762B0) with
/// its own box again (it_80275D5C of xC0C) and rolls. Walls and ceilings
/// do not bounce it.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let velocity = item.velocity;
    let bits = item.air_contact_bits(ctx.map);
    if bits & 0xF != 0 && !blast_if_fast(item, velocity, ctx.assets) && bits & 1 != 0 {
        item.land_on_floor();
        item.restore_collision_box(ctx.assets);
        roll(item, ctx.assets);
    }
    false
}

/// itLinkbomb_UnkMotion4_Coll -> it_8026D62C: off the floor it falls.
fn rolling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        fall(item, ctx.assets);
    }
    false
}

/// itLinkbomb_UnkMotion6_Coll -> it_8026E8C4: on the floor it rolls, off
/// it it falls.
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ground_collision_with_slide(ctx.map, ctx.assets) {
        roll(item, ctx.assets);
    } else {
        fall(item, ctx.assets);
    }
    false
}
