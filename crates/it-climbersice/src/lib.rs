//! The Ice Climbers' ice block (It_Kind_IceClimber_Ice), itclimbersice.c
//! (802C1590..802C22C0). A climber's Ice Shot script makes it at the head
//! (it_802C1590), where it drops until the script launches it
//! (it_802C16F8); then it slides along the floor at attribute x10's speed,
//! falls off edges and lands again, until its lifetime runs out. A wall it
//! meets slowly turns it around; a fast one breaks it.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ClimbersIceState, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct ClimbersIce;

/// ftData.x48_items index (ftPp_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;
/// it_803F7668's anim_id column: melting and dropping play no article
/// state; sliding and falling play state 0 (the block's spin).
pub const ARTICLE_STATES: [i32; 4] = [-1, -1, 0, 0];
/// itClimbersIceAttributes x0..x30.
pub const SPECIAL_ATTRIBUTES: u32 = 13;

/// The motion states (it_803F7668).
const MELTING: u16 = 0;
const DROPPING: u16 = 1;
const SLIDING: u16 = 2;
const FALLING: u16 = 3;

/// efAsync_Spawn(gobj, xBC0, 0, 0x4E9, child): made (itClimbersice_Spawn2).
const MADE_EFFECT: u16 = 0x4E9;
/// efAsync_Spawn(gobj, xBC0, 3, 0x4EA, child, &facing): launched.
const LAUNCH_EFFECT: u16 = 0x4EA;
/// efAsync_Spawn(gobj, xBC0, 3, 0x4EB, child, &facing): sliding.
const SLIDE_EFFECT: u16 = 0x4EB;

/// Below this child scale a melting block is gone
/// (itClimbersice_UnkMotion0_Anim).
const MELTED_SCALE: f32 = 0.01;

/// itClimbersIceAttributes (it/itCharItems.h).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the lifetime once launched (it_80275158).
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: lifetime lost at each wall or shield it turns off.
    fn bounce_cost(&self) -> f32 {
        self.0[1]
    }
    /// x8: a shield bounce leaving less lifetime than this breaks it.
    fn shield_break_lifetime(&self) -> f32 {
        self.0[2]
    }
    /// xC: faster than this, a wall or shield breaks it.
    fn bounce_speed_limit(&self) -> f32 {
        self.0[3]
    }
    /// x10: the launch speed.
    fn launch_speed(&self) -> f32 {
        self.0[4]
    }
    /// x14: the slope's pull on the block (it_802C1854).
    fn slope_pull(&self) -> f32 {
        self.0[5]
    }
    /// x18 / x1C: the pull's scale sliding down the slope, and up it.
    fn slope_downhill(&self) -> f32 {
        self.0[6]
    }
    fn slope_uphill(&self) -> f32 {
        self.0[7]
    }
    /// x24: slower than this, the block stops and melts.
    fn stop_speed(&self) -> f32 {
        self.0[9]
    }
    /// x28: the melting child's scale factor per frame.
    fn melt_rate(&self) -> f32 {
        self.0[10]
    }
    /// x2C (s32): the hit's base damage.
    fn base_damage(&self) -> u32 {
        self.0[11].to_bits()
    }
    /// x30 (u32): damage per unit of horizontal speed.
    fn speed_damage(&self) -> u32 {
        self.0[12].to_bits()
    }
}

fn attributes(assets: &ItemAssets) -> Attributes<'_> {
    Attributes(&assets.special_attributes)
}

fn ice(item: &mut ItemCore) -> &mut ClimbersIceState {
    match &mut item.scratch {
        ItemScratch::ClimbersIce(state) => state,
        _ => unreachable!("an ice block without its state"),
    }
}

static STATES: [ItemStateRow; 4] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[MELTING as usize],
        animation: melt,
        physics: still,
        collision: melting_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[DROPPING as usize],
        animation: keep,
        physics: drop,
        collision: dropping_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[SLIDING as usize],
        animation: tick_lifetime,
        physics: slide,
        collision: sliding_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[FALLING as usize],
        animation: tick_lifetime,
        physics: fall,
        collision: falling_collision,
    },
];

/// it_2725.c's Logic90 row.
impl ItemLogic for ClimbersIce {
    const KIND: ItemKind = ItemKind::IceClimberIce;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802C1590 clears xDC8 x15 (it_8026B3A8): never grabbable.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802C1590 ends with it_802750F8.
    const PROCS_AT_SPAWN: bool = true;
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::ClimbersIce(ClimbersIceState::default());
    }
    /// it_802C1590 (802C1590) once Item_80268B18 returns: the child's scale
    /// starts at ItemAttr x60, the made effect, then motion 1
    /// (it_802C1A58) and it_8026B3A8.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        *ice(item) = ClimbersIceState {
            scale: assets.scale,
            launched: false,
            half_life_scale: common.half_life_scale,
        };
        item.spawn_async(ItemEvent::ChildEffect {
            id: MADE_EFFECT,
            facing: None,
        });
        item.change_motion_with(DROPPING, ARTICLE_STATES[1], ANIM_UPDATE, assets);
        item.grabbable = false;
    }
    /// Before its launch the block is its owner's fp->u.pp.x222C, which
    /// it_2725_Logic90_Destroyed clears (ftPp_Init_8011F16C); a launched
    /// one is no longer the owner's.
    fn notifies_owner(item: &ItemCore) -> bool {
        matches!(&item.scratch, ItemScratch::ClimbersIce(state) if !state.launched)
    }
    /// The owner's accessory4 (ftPp_SpecialN_8011F500): `Fire` launches the
    /// block it holds (it_802C16F8), `Remove` breaks it (it_802C17DC ->
    /// Item_8026A8EC). A launched block is no longer held.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        if ice(item).launched {
            return;
        }
        match control {
            ItemControl::Fire => launch(item, assets),
            ItemControl::Remove => item.destroyed = true,
            other => unreachable!("{other:?} sent to an ice block"),
        }
    }
    /// itClimbersIce_Logic90_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersIce_Logic90_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itClimbersIce_Logic90_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_2725_Logic90_HitShield (802C2220): a slow block bounces back off
    /// the shield (itColl_BounceOffVictim), faces its way, keeps ItemAttr
    /// x58 of its speed (fmuls each axis) and loses lifetime; a fast one,
    /// or one left with too little lifetime, breaks.
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        let a = attributes(ctx.assets);
        if gekko_math::msl::fabsf(item.velocity.x) > a.bounce_speed_limit() {
            return true;
        }
        item.bounce_off_victim(ctx.victim_bounce);
        item.face_velocity();
        let scale = ctx.assets.bounce_scale;
        item.velocity.x *= scale;
        item.velocity.y *= scale;
        item.velocity.z *= scale;
        item.life_timer -= a.bounce_cost();
        item.life_timer < a.shield_break_lifetime()
    }
    /// itClimbersIce_Logic90_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itClimbersIce_Logic90_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itClimbersIce_Logic90_ShieldBounced: itColl_BounceOffShield")
    }
}

/// it_802C16F8 (802C16F8): attribute x10's speed along the facing (fmuls),
/// the launch lifetime (it_80275158), the facing from the velocity, the
/// launch effect, then sliding (it_802C1AE4).
fn launch(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    item.velocity.x = a.launch_speed() * item.facing;
    item.velocity.z = 0.0;
    item.velocity.y = 0.0;
    let lifetime = a.lifetime();
    let half_life_scale = ice(item).half_life_scale;
    item.life_timer = lifetime;
    item.half_life = lifetime * half_life_scale;
    ice(item).launched = true;
    item.face_velocity();
    item.spawn_async(ItemEvent::ChildEffect {
        id: LAUNCH_EFFECT,
        facing: Some(item.facing),
    });
    start_sliding(item, assets);
}

/// it_802C1AE4 (802C1AE4): the sliding effect, then motion 2.
fn start_sliding(item: &mut ItemCore, assets: &ItemAssets) {
    item.spawn_async(ItemEvent::ChildEffect {
        id: SLIDE_EFFECT,
        facing: Some(item.facing),
    });
    item.change_motion_with(SLIDING, ARTICLE_STATES[2], ANIM_UPDATE, assets);
}

/// it_802C1950 (802C1950): the block stops (itResetVelocity) and melts.
fn stop(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Default::default();
    item.change_motion_with(MELTING, ARTICLE_STATES[0], ANIM_UPDATE, assets);
}

/// itClimbersice_UnkMotion0_Anim (802C19A4): a launched block's child
/// shrinks by attribute x28 each frame (fmuls, it_80272F7C) until it is
/// gone.
fn melt(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let rate = attributes(ctx.assets).melt_rate();
    let state = ice(item);
    if !state.launched {
        return false;
    }
    state.scale *= rate;
    state.scale < MELTED_SCALE
}

/// itClimbersice_UnkMotion1_Anim.
fn keep(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// Item_TickLifetime (it/inlines.h): spent, the block is gone; otherwise a
/// frame passes.
fn tick_lifetime(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 0.0 {
        return true;
    }
    item.life_timer -= 1.0;
    false
}

/// itClimbersice_UnkMotion0_Phys.
fn still(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

/// itClimbersice_UnkMotion1_Phys: it_80272860 at ItemAttr x10 / x14.
fn drop(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itClimbersice_UnkMotion2_Phys (802C1C58): only a block the slope just
/// stopped retakes its hit's damage.
fn slide(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if slope_stops(item, ctx.assets) {
        set_damage(item, ctx.assets);
    }
}

/// itClimbersice_UnkMotion3_Phys (802C1DB4): Item_ApplyFallingPhysics
/// (it_80272860, then it_80274658 at ItCo +68), then the hit's damage.
fn fall(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
    set_damage(item, ctx.assets);
}

/// itClimbersice_Phys_inline (802C1DF4..802C1E74): hitbox 0's damage is
/// the base plus |vel.x * x30| (the u32 converted to single with fsubs,
/// fmuls; the product negated when negative, __cvt_fp2unsigned).
fn set_damage(item: &mut ItemCore, assets: &ItemAssets) {
    let a = attributes(assets);
    let per_speed = a.speed_damage() as f32;
    let mut product = item.velocity.x * per_speed;
    if product < 0.0 {
        product = -product;
    }
    let damage = (product as u32).wrapping_add(a.base_damage());
    item.set_hitbox_damage(0, damage);
}

/// it_802C1854 (802C1854): on a floor (env_flags 0x18000) the slope's pull
/// changes the horizontal speed, x1C of it against the motion and x18 of
/// it with (fmuls, fmuls, fadds); below attribute x24 the block stops:
/// xDCD b3/b4 (it_8026BD6C, it_8026BD84, read only by landings the block
/// never makes), it_802C1950 and efLib_DestroyAll.
fn slope_stops(item: &mut ItemCore, assets: &ItemAssets) -> bool {
    /// Collide_FloorMask's two bits.
    const ON_FLOOR: u32 = 0x18000;
    let a = attributes(assets);
    let collision = item.collision.as_ref().expect("ice block collision");
    if collision.env_flags as u32 & ON_FLOOR == 0 {
        return false;
    }
    let normal_x = collision.floor.normal.x;
    let normal_sign = if normal_x < 0.0 { -1 } else { 1 };
    let velocity_sign = if item.velocity.x < 0.0 { -1 } else { 1 };
    let pull = normal_x * a.slope_pull();
    let change = if velocity_sign != normal_sign {
        a.slope_uphill() * pull
    } else {
        a.slope_downhill() * pull
    };
    item.velocity.x += change;
    if gekko_math::msl::fabsf(item.velocity.x) >= a.stop_speed() {
        return false;
    }
    stop(item, assets);
    item.spawn_async(ItemEvent::DestroyEffects);
    true
}

/// itClimbersice_UnkMotion0_Coll (802C1A1C): it_8026D62C (off the floor:
/// it_802C1A58, dropping again) and the lean with the floor (it_80276CB8).
fn melting_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        item.change_motion_with(DROPPING, ARTICLE_STATES[1], ANIM_UPDATE, ctx.assets);
    }
    item.lean_with_floor();
    false
}

/// itClimbersice_UnkMotion1_Coll -> it_8026E15C (8026E15C): an airborne
/// pass; a touched surface (it_80276FC4) or a landing (it_8026DBC8's
/// checks, then it_802C1950) is not ported. The block drops for the few
/// frames between the script's two commands, from the climber's head: it
/// does not reach the floor first.
fn dropping_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let bits = item.air_contact_bits(ctx.map);
    if bits & 0xF != 0 {
        unimplemented!("it_8026E15C: the unlaunched ice block touches the map ({bits:#x})");
    }
    false
}

/// itClimbersice_UnkMotion2_Coll (802C1CD0): it_8026D62C (off the floor:
/// fn_802C1D44, falling), the lean, then the wall check.
fn sliding_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        // fn_802C1D44: motion 3, then efLib_DestroyAll.
        item.change_motion_with(FALLING, ARTICLE_STATES[3], ANIM_UPDATE, ctx.assets);
        item.spawn_async(ItemEvent::DestroyEffects);
    }
    item.lean_with_floor();
    wall_breaks(item, ctx.assets)
}

/// itClimbersice_UnkMotion3_Coll (802C1E94): an airborne pass
/// (it_8026DAA8); on the floor the sliding effect and motion 2 (without
/// grounding the block: it_8026DAA8 lands nothing); then the wall check.
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_bits(ctx.map) & 1 != 0 {
        start_sliding(item, ctx.assets);
    }
    wall_breaks(item, ctx.assets)
}

/// itClimbersice_Coll (inline): against a wall (it_80276308) a slow block
/// turns back (it_8027770C, it_80272980) and loses x4 of its lifetime; a
/// fast one breaks.
fn wall_breaks(item: &mut ItemCore, assets: &ItemAssets) -> bool {
    if item.wall_bits() == 0 {
        return false;
    }
    let a = attributes(assets);
    if gekko_math::msl::fabsf(item.velocity.x) > a.bounce_speed_limit() {
        return true;
    }
    item.bounce_off_wall(assets);
    item.face_velocity();
    item.life_timer -= a.bounce_cost();
    false
}
