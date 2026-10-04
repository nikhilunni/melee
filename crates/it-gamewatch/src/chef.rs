//! Chef's food (It_Kind_GameWatch_Chef), itgamewatchchef.c
//! (802C837C..802C8B08). The neutral special throws one of five foods from
//! the pan; each flies on its own attribute entry (launch velocity,
//! gravity, fall limit, spin), turns back off walls at a fraction of its
//! speed and, once it touches a floor or ceiling, hits something or its
//! lifetime runs out, sits blinking for a second lifetime before it goes.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ChefState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct Chef;

/// it_803F79E0's anim_id column.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// itGamewatchchefAttributes: x0 (the costume colour's pointer), x4..xC and
/// five 0x14-byte entries.
pub const SPECIAL_ATTRIBUTES: u32 = 4 + 5 * 5;
/// The foods a throw chooses between (ftGw_SpecialN_CreateSausage).
pub const FOODS: usize = 5;

/// Motion 0 flies; motion 1 lies spent.
const FLYING: u16 = 0;
const SPENT: u16 = 1;

/// it_8026DAA8's result bits: either wall, and the floor or ceiling.
const WALL_BITS: u32 = 0xC;
const FLOOR_OR_CEILING_BITS: u32 = 0x3;

struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x4: the share of its speed a food keeps off a wall.
    fn wall_bounce(&self) -> f32 {
        self.0[1]
    }
    /// x8: the flight's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[2]
    }
    /// xC: the spent food's lifetime.
    fn spent_lifetime(&self) -> f32 {
        self.0[3]
    }
    fn entry(&self, food: usize) -> &[f32] {
        &self.0[4 + 5 * food..][..5]
    }
    /// entries[food].x0 / x4: the launch velocity (x along the facing).
    fn launch(&self, food: usize) -> (f32, f32) {
        let e = self.entry(food);
        (e[0], e[1])
    }
    /// entries[food].x8 / xC: gravity and the fall speed it stops at.
    fn fall(&self, food: usize) -> (f32, f32) {
        let e = self.entry(food);
        (e[2], e[3])
    }
    /// entries[food].x10: radians the model turns about X each frame.
    fn spin(&self, food: usize) -> f32 {
        self.entry(food)[4]
    }
}

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: flying_animation,
        physics: flying_physics,
        collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: spent_animation,
        physics: spent_physics,
        collision,
    },
];

fn state(item: &ItemCore) -> ChefState {
    match item.scratch {
        ItemScratch::Chef(state) => state,
        _ => panic!("Chef food scratch"),
    }
}
fn food(item: &ItemCore) -> usize {
    state(item).food
}

/// it_80275158: both timers.
fn set_lifetime(item: &mut ItemCore, lifetime: f32, half_life_scale: f32) {
    item.life_timer = lifetime;
    item.half_life = lifetime * half_life_scale;
}

/// it_3F2F.c's Logic112 row.
impl ItemLogic for Chef {
    const KIND: ItemKind = ItemKind::GameWatchChef;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802C84A0 (802C84A0), after Item_80268B18: the food
    /// (`spawn_argument`), the flight's lifetime, xD5C cleared, the entry's
    /// launch velocity (802C84F4: fmuls by the facing), not grabbable
    /// (it_8026B3A8), motion 0.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Attributes(&assets.special_attributes);
        let food = spawn.spawn_argument as usize;
        item.scratch = ItemScratch::Chef(ChefState {
            food,
            half_life_scale: common.half_life_scale,
        });
        set_lifetime(item, a.lifetime(), common.half_life_scale);
        item.platform_drop = 0;
        let (x, y) = a.launch(food);
        item.velocity.x = x * item.facing;
        item.velocity.y = y;
        item.grabbable = false;
        item.change_motion_with(FLYING, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        // it_802C837C: it_8027CE64 after the launch runs it_80274594, whose
        // it_80275534 gives the hitbox the flight's script has just made
        // x3C, the command size, as its radius: the 1 / scl of the hitbox
        // command is undone, so the food's capsule (scl 1.5) is 1.5 times
        // the command's in contact tests.
        let scale = item.scale;
        item.rescale(scale);
    }
    /// itGameWatchChef_Logic112_DmgDealt (802C8474): spent, not removed.
    fn damage_dealt(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        spend(item, ctx.assets, ctx.half_life_scale);
        false
    }
    /// it_2725_Logic112_Clanked (802C88F0).
    fn clanked(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        spend(item, ctx.assets, ctx.half_life_scale);
        false
    }
    /// it_2725_Logic112_HitShield (802C8964).
    fn hit_shield(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        spend(item, ctx.assets, ctx.half_life_scale);
        false
    }
    /// it_2725_Logic112_Absorbed (802C89D8): spent with no lifetime left.
    fn absorbed(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        spend(item, ctx.assets, ctx.half_life_scale);
        item.life_timer = 0.0;
        false
    }
    /// itGameWatchChef_Logic112_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(ctx.shield_normal);
        false
    }
    /// itGameWatchChef_Logic112_Reflected -> itReflectItemAndUpdateRotation:
    /// it_80273030, then the model turns to the new facing.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        item.rotation.y = std::f32::consts::FRAC_PI_2 * item.facing;
        false
    }
}

/// it_802C875C (802C875C): the food stops, takes the spent lifetime and
/// lies in motion 1.
fn spend(item: &mut ItemCore, assets: &ItemAssets, half_life_scale: f32) {
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    set_lifetime(
        item,
        Attributes(&assets.special_attributes).spent_lifetime(),
        half_life_scale,
    );
    item.change_motion_with(SPENT, ARTICLE_STATES[1], ANIM_UPDATE, assets);
}

/// itGamewatchchef_UnkMotion0_Anim (802C8554): the model turns about X by
/// the food's spin (fadds into the JObj rotation); the flight's lifetime
/// counts down and, spent, the food lies down. It is never removed here.
fn flying_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.rotation.x += Attributes(&ctx.assets.special_attributes).spin(food(item));
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        let half_life_scale = state(item).half_life_scale;
        spend(item, ctx.assets, half_life_scale);
    }
    false
}

/// itGamewatchchef_UnkMotion0_Phys (802C8684): it_80272860 with the
/// food's gravity and fall limit.
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let (gravity, limit) = Attributes(&ctx.assets.special_attributes).fall(food(item));
    item.fall(gravity, limit);
}

/// itGamewatchchef_UnkMotion0_Coll (802C86C4), also motion 1's: while the
/// food moves sideways, an airborne pass (it_8026DAA8); a wall turns it
/// back at the attribute share of its speed (fnmsubs-free: fneg, fmuls),
/// a floor or ceiling spends it.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.velocity.x != 0.0 {
        let contact = item.air_contact_bits(ctx.map);
        if contact & WALL_BITS != 0 {
            item.velocity.x *= -Attributes(&ctx.assets.special_attributes).wall_bounce();
        }
        if contact & FLOOR_OR_CEILING_BITS != 0 {
            spend(item, ctx.assets, ctx.half_life_scale);
        }
    }
    false
}

/// itGamewatchchef_UnkMotion1_Anim (802C87D4): the spent lifetime counts
/// down and ends the food; meanwhile bit 1 of the whole frames left blinks
/// the model (it_8026BB20 shown / it_8026BB44 hidden).
fn spent_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        return true;
    }
    item.hidden = gekko_math::msl::fctiwz(item.life_timer) & 2 == 0;
    false
}

/// itGamewatchchef_UnkMotion1_Phys (802C8858) is empty.
fn spent_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
