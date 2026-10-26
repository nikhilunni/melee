//! The Super Scope (It_Kind_S_Scope, itsscope.c 80291BE0..80292334) and
//! its shots (It_Kind_S_Scope_Beam, itsscopebeam.c 80298DEC..802992A4).
//!
//! The holder charges the scope while A is held (ItemScopeStart's timer)
//! and fires a shot whose charge level picks its cost in shots (xD4C) and
//! its beam's speed, size and lifetime; held A without charging fires
//! rapid level-0 shots (ItemScopeRapid).
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

/// It_Kind_S_Scope.
pub struct SuperScope;
/// It_Kind_S_Scope_Beam.
pub struct SuperScopeBeam;

/// it_803F5F40's anim_id column.
pub const ARTICLE_STATES: [i32; 5] = [-1, -1, -1, 0, -1];
/// itSScopeAttributes: x0 (shots), x4, x8, xC[10] (costs), x34 (muzzle).
pub const SPECIAL_ATTRIBUTES: u32 = 16;
/// it_803F6568's anim_id column: one state per charge level.
pub const BEAM_ARTICLE_STATES: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// ScopeBeamAttrs: ten {speed, scale, lifetime} rows (x0..x78), x78, x7C.
pub const BEAM_SPECIAL_ATTRIBUTES: u32 = 32;

mod motion {
    pub const GROUNDED: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const FLYING: u16 = 3;
    pub const SLIDING: u16 = 4;
}

/// The highest charge level (it_80291CF4's 9).
pub const MAX_LEVEL: i32 = melee_it::scope::MAX_LEVEL;

/// it_80291CCC (80291CCC): the muzzle, in the hand joint's frame (x34).
pub fn muzzle_offset(assets: &ItemAssets) -> hsd_types::Vec3 {
    let a = &assets.special_attributes;
    hsd_types::Vec3::new(a[13], a[14], a[15])
}

fn charges(assets: &ItemAssets) -> melee_it::scope::ScopeCharges {
    melee_it::scope::ScopeCharges::from_attributes(&assets.special_attributes)
}

static STATES: [ItemStateRow; 5] = [
    row(motion::GROUNDED, no_animation, no_physics, grounded_collision),
    row(motion::FALL, no_animation, gravity_physics, flying_collision),
    row(motion::HELD, no_animation, no_physics, no_collision),
    row(motion::FLYING, no_animation, falling_physics, flying_collision),
    row(motion::SLIDING, no_animation, no_physics, sliding_collision),
];

const fn row(
    state: u16,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state as usize],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for SuperScope {
    const KIND: ItemKind = ItemKind::SScope;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itSScope_Logic21_Spawned (80291C94): the shots (x0), then the fall
    /// (it_802920B8).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ammo = assets.special_attributes[0].to_bits() as i32;
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::FLYING, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    const ENTERED_AIR: Option<EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itSScope_Logic21_EnteredAir (itsscope.c:262): a Super Scope sliding down a slope is not witnessed")
    });
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            // it_80291F14: a shot of `level` spent (the beam itself is the
            // holder's spawn request, it_80291FA8).
            ItemControl::Spend(level) => item.ammo = charges(assets).spend(item.ammo, level),
            other => unimplemented!("itsscope.c: control {other:?}"),
        }
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_80292030 (80292030): stopped, grabbable, at rest.
fn come_to_rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = hsd_types::Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::GROUNDED, ANIM_UPDATE, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itSscope_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itSscope_UnkMotion3_Phys -> Item_ApplyFallingPhysics.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itSscope_UnkMotion0_Coll: it_8026D62C, off the floor it falls.
fn grounded_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itSscope_UnkMotion3_Coll (states 1 and 3): with shots left it lands
/// (it_8026E15C(it_80292030)); empty, the floor pass it_8026DF34 removes it.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.ammo == 0 {
        return item.air_pass(ctx.map);
    }
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            come_to_rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itSscope_UnkMotion4_Coll, reached only through the guarded EnteredAir.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itSscope_UnkMotion4_Coll (itsscope.c:272): a Super Scope sliding down a slope is not witnessed")
}

/// ScopeBeamAttrs: level `level`'s speed, scale and lifetime.
fn beam_row(assets: &ItemAssets, level: usize) -> (f32, f32, f32) {
    let a = &assets.special_attributes;
    (a[level * 3], a[level * 3 + 1], a[level * 3 + 2])
}

fn beam_level(item: &ItemCore) -> usize {
    match item.scratch {
        ItemScratch::Tools(level) => level as usize,
        _ => unreachable!("a Super Scope shot without its level"),
    }
}

static BEAM_STATES: [ItemStateRow; 10] = [
    beam_row_state(0),
    beam_row_state(1),
    beam_row_state(2),
    beam_row_state(3),
    beam_row_state(4),
    beam_row_state(5),
    beam_row_state(6),
    beam_row_state(7),
    beam_row_state(8),
    beam_row_state(9),
];

const fn beam_row_state(state: usize) -> ItemStateRow {
    ItemStateRow {
        animation_id: BEAM_ARTICLE_STATES[state],
        animation: beam_animation,
        physics: no_physics,
        collision: beam_collision,
    }
}

impl ItemLogic for SuperScopeBeam {
    const KIND: ItemKind = ItemKind::SScopeBeam;
    const STATES: &'static [ItemStateRow] = &BEAM_STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_80298DEC (80298DEC) once Item_80268B18 returns: the level
    /// (SpawnItem's argument), then it_80298ED0: ungrabbable, the level's
    /// state, its speed times the facing, its scale on the model and its
    /// lifetime (it_80275158).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let level = spawn.spawn_argument;
        assert!((0..=MAX_LEVEL).contains(&level), "it_80298ED0: level {level}");
        item.scratch = ItemScratch::Tools(level);
        item.grabbable = false;
        item.change_motion_with(
            level as u16,
            BEAM_ARTICLE_STATES[level as usize],
            ANIM_UPDATE,
            assets,
        );
        let (speed, scale, lifetime) = beam_row(assets, level as usize);
        item.velocity.x = speed * item.facing;
        item.model_scale = hsd_types::Vec3::new(scale, scale, scale);
        item.joints_dirty = true;
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
    }
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itSScopeBeam_Logic38_Absorbed (itsscopebeam.c:196): an absorbed Super Scope shot is not witnessed")
    }
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itSScopeBeam_Logic38_ShieldBounced (itsscopebeam.c:201): a Super Scope shot off a shield's bounce is not witnessed")
    }
    fn reflected(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        unimplemented!("itSScopeBeam_Logic38_Reflected (itsscopebeam.c:206): a reflected Super Scope shot is not witnessed")
    }
}

/// itSscopebeam_UnkMotion9_Anim (80299008): on its first frame (the
/// lifetime still whole) a live hitbox takes the level's scale; the
/// lifetime counts down and the shot ends at zero.
fn beam_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let (_, scale, lifetime) = beam_row(ctx.assets, beam_level(item));
    if item.life_timer == lifetime {
        if let Some(hit) = item.hitboxes[0].as_mut() {
            hit.descriptor.radius *= scale;
        }
    }
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        return true;
    }
    false
}

/// itSscopebeam_UnkMotion9_Coll (80299098): it_8026DAA8's pass; a wall or
/// ceiling ends the shot; a floor (with env bits 0x18000) turns or ends it.
fn beam_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let bits = item.air_contact_bits(ctx.map);
    if bits & 0xE != 0 {
        return true;
    }
    if bits & 1 != 0 {
        let env = item.collision.as_ref().expect("item map collision").env_flags as u32;
        if env & 0x18000 != 0 {
            unimplemented!(
                "itSscopebeam_UnkMotion9_Coll (itsscopebeam.c:153-184): a Super Scope shot along a floor is not witnessed"
            );
        }
    }
    false
}
