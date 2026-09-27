//! Bob-omb (It_Kind_BombHei), itbombhei.c (8027D670..80280F40): the Sudden
//! Death rain's bomb. Ported so far: the rain spawn (it_8027D670), the lit
//! fuse on the ground, in the air and in a fighter's hand, throwing and
//! dropping, landing (a soft landing starts the walk) and the explosion.
//! The walk and turn rows themselves are explicit `unimplemented!` rows.
use gekko_math::msl::{fabsf, fctiwz};
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::*, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct BombHei;

/// it_803F54D8's anim_id column: the article state each motion state plays.
pub const ARTICLE_STATES: [i32; 13] = [-1, -1, 0, 0, 6, 4, 4, 1, 4, 2, 5, 3, -1];
/// itBombHeiAttributes has eleven floats (x0..x28).
pub const SPECIAL_ATTRIBUTES: u32 = 11;

/// Item_80268E5C flags this file passes (it/forward.h).
const UNK_0X1: u32 = 1;

mod motion {
    pub const FALL: u16 = 1;
    pub const LIT: u16 = 5;
    /// Held unlit / lit (itBombhei_Logic6_PickedUp).
    pub const HELD: u16 = 7;
    pub const HELD_LIT: u16 = 8;
    /// Walking after a soft landing (itBombhei_UnkMotion2).
    pub const WALK: u16 = 2;
    /// Thrown or dropped, lit (it_3F14_Logic6_Thrown).
    pub const THROWN_LIT: u16 = 10;
    pub const LIT_FALL: u16 = 6;
    pub const EXPLODE: u16 = 11;
}

/// itBombHeiAttributes (special attributes, x0..x28).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x4, x10, x14: the turning, wandering and walking phases.
    fn turn_frames(&self) -> f32 {
        self.0[1]
    }
    /// x8: frames the lit fuse burns.
    fn fuse_frames(&self) -> f32 {
        self.0[2]
    }
    /// xC: the walking speed.
    fn walk_speed(&self) -> f32 {
        self.0[3]
    }
    fn wander_frames(&self) -> f32 {
        self.0[4]
    }
    fn walk_frames(&self) -> f32 {
        self.0[5]
    }
    /// x18: frames per blink half-period.
    fn blink_period(&self) -> f32 {
        self.0[6]
    }
    /// x1C: scale added per blink frame.
    fn blink_scale(&self) -> f32 {
        self.0[7]
    }
    /// x20 / x24: a landing faster than this explodes.
    fn explode_speed(&self) -> (f32, f32) {
        (self.0[8], self.0[9])
    }
}

/// Retail rows: state 1 falls unlit, 5, 6 and 8 carry the lit fuse on the
/// ground, in the air and in a hand, 11 is the explosion.
static STATES: [ItemStateRow; 13] = {
    const UNPORTED: ItemStateRow = ItemStateRow {
        animation_id: -1,
        animation: unported_animation,
        physics: unported_physics,
        collision: unported_collision,
    };
    let mut rows = [UNPORTED; 13];
    rows[motion::FALL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: unported_physics,
        collision: unported_collision,
    };
    rows[motion::LIT as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: lit_animation,
        physics: no_physics,
        collision: lit_collision,
    };
    rows[motion::LIT_FALL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: lit_fall_animation,
        physics: fall_physics,
        collision: lit_fall_collision,
    };
    // it_803F54D8[7..=8]: no physics, no collision callback.
    rows[motion::HELD_LIT as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[8],
        animation: held_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows[motion::THROWN_LIT as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[10],
        animation: thrown_animation,
        physics: fall_physics,
        collision: thrown_collision,
    };
    rows[motion::EXPLODE as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[11],
        animation: explosion_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows
};

impl ItemLogic for BombHei {
    const KIND: ItemKind = ItemKind::BombHei;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itBombhei_Logic6_Spawned (8027D758).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        let a = Attributes(&assets.special_attributes);
        // retail 8027D7AC..B8 / 8027D7D4..E0: separate fadds, innermost first.
        let whole_life = a.turn_frames() + (a.wander_frames() + a.walk_frames());
        let scale = item.scale;
        item.scratch = ItemScratch::Bomb(BombState {
            countdown: fctiwz(a.wander_frames()),
            life_frames: a.fuse_frames() + whole_life,
            // retail 8027D7E4..FC: fdivs then fmuls, -1.5999999 and -pi/12.
            squash_step: (-1.599_999_9 / whole_life) * scale,
            tilt_step: (-0.261_799_4 / whole_life) * scale,
            throw_scale: 1.0,
            ..Default::default()
        });
        // it_8027DE18: from states 0 and 3 only it also resets the rotation.
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// itBombhei_Logic6_PickedUp (8027E0B4): the model's spin axis and
    /// facing lock for the hand, then the held state.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        // xDC8 x19 and x17.
        item.spin_ignores_facing = true;
        item.rotation_axis = 1;
        if !bomb(item).lit {
            // ap->x0 would slow the unlit hold animation; the port keeps
            // item animations at one frame per tick.
            unimplemented!("itBombhei_Logic6_PickedUp: unlit Bob-omb pickup");
        }
        change(item, motion::HELD_LIT, UNK_0X1, context.assets);
    }
    /// it_3F14_Logic6_Thrown (80280380): the thrown state, whose hitboxes
    /// take the throw speed (ITEM_DROP_UPDATE); the blast's owner and kin
    /// flags clear (it_80275474).
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        if !bomb(item).lit {
            unimplemented!("it_3F14_Logic6_Thrown: unlit Bob-omb throw (state 9)");
        }
        enter_thrown_lit(item, context.assets);
    }
    /// it_3F14_Logic6_Dropped (8027E648): spin about X ignoring facing, then
    /// the thrown state without it_80275474's owner/kin changes.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.spin_ignores_facing = true;
        item.rotation_axis = 1;
        if !bomb(item).lit {
            unimplemented!("it_3F14_Logic6_Dropped: unlit Bob-omb drop (state 9)");
        }
        change(
            item,
            motion::THROWN_LIT,
            UNK_0X1 | DROP_UPDATE,
            context.assets,
        );
    }
    /// it_3F14_Logic6_DmgDealt: touching anything detonates it.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        detonate_unless_held(item, context.assets);
        false
    }
    /// it_3F14_Logic6_DmgReceived.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        detonate_unless_held(item, context.assets);
        false
    }
    /// itBombhei_Logic6_Clanked.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        if !bomb(item).exploded {
            explode(item, context.assets);
        }
        false
    }
}

/// it_8027D670 (8027D670): a Bob-omb dropped at `position`, lit at once.
/// `facing` is it_8026B684's (ftLib_800864A8) choice, drawn before creation.
pub fn rain_spawn(position: Vec3, facing: f32) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        secondary_owner: None,
        kind: ItemKind::BombHei,
        // Item_802674AC: common items below It_Kind_L_Gun_Ray.
        hold_kind: 0,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: true,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
    }
}

/// it_8027D670's tail, it_8027F8E0 (8027F8E0): light the fuse and hold.
/// `lifetime` is the common item lifetime (it_804D6D28->x30).
pub fn light(item: &mut ItemCore, assets: &ItemAssets, lifetime: f32) {
    item.velocity.x = 0.0;
    if bomb(item).lit {
        item.grabbable = true;
        change(item, motion::LIT, UNK_0X1, assets);
        return;
    }
    let a = Attributes(&assets.special_attributes);
    // it_8026B390: the common lifetime runs from here.
    item.grabbable = true;
    let state = bomb_mut(item);
    state.countdown = fctiwz(a.blink_period());
    state.blink_direction = 1;
    state.fuse = a.fuse_frames();
    state.lit = true;
    item.life_timer = lifetime;
    change(item, motion::LIT, ANIM_UPDATE, assets);
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn bomb(item: &ItemCore) -> &BombState {
    match &item.scratch {
        ItemScratch::Bomb(state) => state,
        _ => panic!("Bob-omb scratch"),
    }
}
fn bomb_mut(item: &mut ItemCore) -> &mut BombState {
    match &mut item.scratch {
        ItemScratch::Bomb(state) => state,
        _ => panic!("Bob-omb scratch"),
    }
}

/// it_8027D820 (8027D820): the lit fuse. The model pulses (xDD8 flips every
/// x18 frames) and the bomb detonates when the fuse runs out.
fn burn_fuse(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    let assets = ctx.assets;
    let a = Attributes(&assets.special_attributes);
    let step = a.blink_scale();
    let state = bomb_mut(item);
    state.countdown -= 1;
    let direction = state.blink_direction as f32;
    // retail 8027D88C/8A8 (and twice more): fmuls, then HSD_JObjAddScale's fadds.
    item.model_scale.x += step * direction;
    item.model_scale.y += step * direction;
    item.model_scale.z += step * direction;
    let state = bomb_mut(item);
    if state.countdown <= 0 {
        state.countdown = fctiwz(a.blink_period());
        state.blink_direction = -state.blink_direction;
    }
    state.fuse -= 1.0;
    state.life_frames -= 1.0;
    if state.fuse <= 0.0 && !state.exploded {
        // it_80280DC0 -> it_80280B60: xDC8 x13 first lets go of the hand
        // with no velocity (it_8027429C).
        if item.held {
            let holder = ctx.holder.as_mut().expect("held Bob-omb holder");
            item.release_from_holder(Vec3::ZERO, holder, ctx.map, assets);
        }
        explode(item, assets);
    }
}

/// it_3F14_Logic6_DmgDealt / DmgReceived: not while held unlit (state 7).
fn detonate_unless_held(item: &mut ItemCore, assets: &ItemAssets) {
    if item.motion != motion::HELD && !bomb(item).exploded {
        assert!(
            !item.held,
            "it_80280B60: a held Bob-omb detonated by contact"
        );
        explode(item, assets);
    }
}

/// it_80280B60 (80280B60): hide the model, start the explosion lifetime,
/// stop, spawn the blast effect and a radial gust, then state 11 whose
/// script owns the explosion hitbox.
fn explode(item: &mut ItemCore, assets: &ItemAssets) {
    assert!(!item.held, "it_8027429C: held Bob-omb explosion");
    // it_8026B3A8 and it_8026BD24: the common lifetime stops.
    item.grabbable = false;
    item.hidden = true;
    // it_8027518C: the common explosion lifetime.
    item.life_timer = assets.explosion_lifetime;
    // it_80273454 -> itResetVelocity.
    item.velocity = Vec3::ZERO;
    bomb_mut(item).exploded = true;
    // it_80272C08: efSync_Spawn(0x410) at the item; it_80274C60 clears
    // xDC8 xC, so contacts no longer put it into hitlag.
    item.events.push(ItemEvent::Effect {
        id: EXPLOSION_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    // it_802756D0: the blast takes no hits.
    item.hurt_intangible = true;
    // it_80275444: the blast hits its owner, and items sharing its owner.
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    // lb_800119DC(&pos, 0x78, 1.0, 0.02, pi/3).
    item.events.push(ItemEvent::Gust {
        center: item.position,
        frames: 0x78,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
    change(item, motion::EXPLODE, ANIM_UPDATE | HIT_PRESERVE, assets);
}

/// it_80272C08's efSync_Spawn id.
const EXPLOSION_EFFECT: u16 = 0x410;

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itBombhei_UnkMotion5_Anim (8027FC08).
fn lit_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if bomb(item).lit {
        burn_fuse(item, ctx);
    }
    false
}

/// itBombhei_UnkMotion5_Coll (8027FC48): it_8026D62C, falling off -> fn_8027FCA8.
fn lit_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        // fn_8027FCA8: it_8026B390 keeps the common lifetime running.
        item.velocity.x = 0.0;
        item.grabbable = true;
        if bomb(item).lit {
            change(item, motion::LIT_FALL, UNK_0X1, ctx.assets);
        } else {
            unimplemented!("fn_8027FCA8: unlit Bob-omb leaving the ground");
        }
    }
    false
}

/// itBombhei_UnkMotion8_Anim (8027E3E4): the hold animation restarts when
/// it ends (it_80272C6C), and the lit fuse burns.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let end = ctx.assets.animation_ends[ARTICLE_STATES[motion::HELD_LIT as usize] as usize];
    if end.is_some_and(|end| item.animation_frame >= end) {
        change(item, motion::HELD_LIT, UNK_0X1, ctx.assets);
    }
    if bomb(item).lit {
        burn_fuse(item, ctx);
    }
    false
}

/// The lit branch of it_3F14_Logic6_Thrown and itBombhei_UnkMotion10_Anim:
/// state 10 with CMD_UPDATE | DROP_UPDATE (0x104), then it_80275474.
fn enter_thrown_lit(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::THROWN_LIT, CMD_UPDATE | DROP_UPDATE, assets);
    item.hits_owner = false;
    item.strikes_kindred_items = false;
    // xDE8: it_80274484 rescales the model after a Bob-omb was made
    // bigger or smaller, which nothing here does.
    assert_eq!(
        bomb(item).throw_scale,
        1.0,
        "it_80274484: rescaled Bob-omb throw"
    );
}

/// itBombhei_UnkMotion10_Anim (802806CC): the thrown animation restarts when
/// it ends, and the lit fuse burns.
fn thrown_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let end = ctx.assets.animation_ends[ARTICLE_STATES[motion::THROWN_LIT as usize] as usize];
    if end.is_some_and(|end| item.animation_frame >= end) {
        enter_thrown_lit(item, ctx.assets);
    }
    if bomb(item).lit {
        burn_fuse(item, ctx);
    }
    false
}

/// itBombhei_UnkMotion10_Coll (80280B18): as the lit fall, a fast landing
/// detonates; a soft one starts walking (fn_80280974).
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    bomb_mut(item).landing_velocity = item.velocity;
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        // fn_80280974 (80280974).
        let (x, y) = Attributes(&ctx.assets.special_attributes).explode_speed();
        let v = bomb(item).landing_velocity;
        if fabsf(v.x) > x || fabsf(v.y) > y {
            if !bomb(item).exploded {
                explode(item, ctx.assets);
            }
        } else {
            start_walking(item, ctx.assets);
        }
    }
    false
}

/// fn_80280974's soft landing (fn_80280974_inline): off the hand's pickup
/// list, walking at xC along its facing, and its hits reach its owner and
/// kindred items again (it_80275444; xDCD b6 is not modelled). The state
/// change keeps the squash joint (itBombhei_UpdateStatePreserveBoneMotion10).
fn start_walking(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    // retail 80280A10: fmuls.
    item.velocity.x = Attributes(&assets.special_attributes).walk_speed() * item.facing;
    item.platform_drop = 0;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    change(
        item,
        motion::WALK,
        ANIM_UPDATE | MODEL_UPDATE | HIT_PRESERVE,
        assets,
    );
}

/// itBombhei_UnkMotion6_Anim (8027FE70).
fn lit_fall_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    burn_fuse(item, ctx);
    false
}

/// Item_ApplyFallingPhysics: it_80272860's gravity, then it_80274658 with
/// the common falling spin.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    fall(
        item,
        ctx.assets.fall_acceleration,
        ctx.assets.fall_speed_limit,
    );
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// it_80272860 (80272860): accelerate while below the limit or while the
/// velocity still points against gravity. There is no clamp at the limit.
fn fall(item: &mut ItemCore, acceleration: f32, limit: f32) {
    let gravity_sign = if acceleration < 0.0 { -1 } else { 1 };
    let speed = item.velocity.y;
    let velocity_sign = if speed < 0.0 { -1 } else { 1 };
    if velocity_sign == gravity_sign || fabsf(speed) < limit {
        item.velocity.y -= acceleration;
    }
}

/// itBombhei_UnkMotion6_Coll (80280010): remember the fall velocity, then
/// it_8026E414 with fn_8028007C on landing.
fn lit_fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    bomb_mut(item).landing_velocity = item.velocity;
    if item.airborne_collision(ctx.map, ctx.assets).floor {
        // fn_8028007C (8028007C): a fast landing detonates; a soft one sits lit.
        let (x, y) = Attributes(&ctx.assets.special_attributes).explode_speed();
        let v = bomb(item).landing_velocity;
        if fabsf(v.x) > x || fabsf(v.y) > y {
            if !bomb(item).exploded {
                explode(item, ctx.assets);
            }
        } else {
            item.velocity.x = 0.0;
            item.grabbable = true;
            if bomb(item).lit {
                change(item, motion::LIT, UNK_0X1, ctx.assets);
            } else {
                unimplemented!("fn_8028007C: unlit Bob-omb landing");
            }
        }
    }
    false
}

/// itBombhei_UnkMotion11_Anim -> it_802751D8: the explosion's lifetime.
fn explosion_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

fn unported_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itbombhei.c: motion state {} animation", item.motion)
}
fn unported_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    unimplemented!("itbombhei.c: motion state {} physics", item.motion)
}
fn unported_collision(item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itbombhei.c: motion state {} collision", item.motion)
}
