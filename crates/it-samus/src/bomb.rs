//! Samus's bomb (It_Kind_Samus_Bomb), itsamusbomb.c (802B4AC8..802B5550).
//! Down special drops it from TopN (it_802B4AC8): it pops up, falls and
//! bounces until it rests, and explodes when its lifetime ends or it hits
//! something. The blast launches its owner in a ball when its capsule meets
//! Samus's (it_802B5478, the owner's ftSs_Init_80128944).
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

pub struct SamusBomb;

/// it_803F7220's anim_id column: resting, falling, sliding, exploding.
pub const ARTICLE_STATES: [i32; 4] = [0, 0, 0, 1];
/// itSamusBombAttributes: seven words.
pub const SPECIAL_ATTRIBUTES: u32 = 7;
/// ftData.x48_items index (ftSs_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;

mod motion {
    pub const RESTING: u16 = 0;
    pub const FALLING: u16 = 1;
    pub const EXPLODING: u16 = 3;
}

/// The resting and falling states' mutual change: flags 0x11 (bit 0 and
/// ITEM_HIT_PRESERVE), so the bomb's hitbox carries over.
const KEEP_HITBOX: u32 = 0x1 | state_change::HIT_PRESERVE;
/// it_80272B40: efSync_Spawn(0x40C) at the item and
/// Item_8026AE84(item, 0x74, 0x7F, 0x40).
const EXPLOSION: u16 = 0x40C;
const EXPLOSION_SOUND: u32 = 0x74;
/// itSamusBomb_UnkMotion_Process: speeds below this leave the facing.
const TURN_SPEED: f32 = 0.0001;

/// itSamusBombAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the fuse.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
}

static STATES: [ItemStateRow; 4] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: resting_animation,
        physics: no_physics,
        collision: resting_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: moving_animation,
        physics: falling_physics,
        collision: falling_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: moving_animation,
        physics: sliding,
        collision: sliding_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

fn bomb_mut(item: &mut ItemCore) -> &mut SamusBombState {
    let ItemScratch::SamusBomb(state) = &mut item.scratch else {
        panic!("Samus bomb scratch missing")
    };
    state
}

/// it_3F2F.c's Logic50 row.
impl ItemLogic for SamusBomb {
    const KIND: ItemKind = ItemKind::SamusBomb;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802B4BA0 (802B4BA0) once Item_80268B18 returns: ItemAttr x18's
    /// pop, the owner the blast may launch, the fuse (it_80275158), not
    /// pickable (it_8026B3A8), then the falling state.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.velocity.y = assets.launch_vertical_velocity;
        item.command_variables[2] = 0;
        item.scratch = ItemScratch::SamusBomb(SamusBombState {
            owner: item.owner,
            blast_pending: false,
        });
        let fuse = Attributes(&assets.special_attributes).lifetime();
        item.life_timer = fuse;
        item.half_life = fuse * common.half_life_scale;
        item.grabbable = false;
        change(item, motion::FALLING, ANIM_UPDATE, assets);
    }
    /// it_802B5478 (802B5478), installed by the explosion: once, a blast
    /// with an owner offers it hitbox 0 (ftSs_Init_80128A1C, then
    /// ftSs_Init_80128944 on a touch).
    fn accessory(
        item: &mut ItemCore,
        _owner: Option<&ItemOwner>,
        _assets: &ItemAssets,
    ) -> Option<OwnerBlast> {
        let state = bomb_mut(item);
        if !std::mem::take(&mut state.blast_pending) {
            return None;
        }
        let owner = state.owner?;
        let hit = item.hitboxes[0].as_ref().expect("the blast's hitbox 0");
        let (previous, position) = item.hitbox_trace[0];
        let range = hit.descriptor.radius;
        Some(OwnerBlast {
            owner,
            previous,
            position,
            contact_radius: if hit.descriptor.ignore_scale {
                range
            } else {
                range * item.scale
            },
            range,
            x: item.position.x,
        })
    }
    /// itSamusBomb_Logic50_DmgDealt: a bomb that is not yet a blast explodes.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// itSamusBomb_Logic50_Clanked.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// itSamusBomb_Logic50_HitShield.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// itSamusBomb_Logic50_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itSamusBomb_Logic50_ShieldBounced: itColl_BounceOffShield (it_2725.c:430)")
    }
    /// it_2725_Logic50_Reflected (802B5370): it_80273030 turns it back, a
    /// falling bomb rises by the reflector's multiplier (xC70), and the
    /// blast no longer launches anyone.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        if item.velocity.y < 0.0 {
            item.velocity.y = -item.velocity.y * context.reflected_speed;
        }
        bomb_mut(item).owner = None;
        false
    }
    /// itSamusBomb_Logic50_EvtUnk: it_8026B894, and the blast's owner goes.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
        bomb_mut(item).owner = None;
    }
}

/// Item_80268E5C with the state's article row.
fn change(item: &mut ItemCore, state: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(state, ARTICLE_STATES[state as usize], flags, assets);
}

/// itSamusBomb_UnkMotion_PreProcess: the fuse. Its last x4 frames only
/// speed the model's material animation (lb_8000BA0C), which is visual;
/// at zero the bomb explodes, before that it counts down.
fn burn_fuse(item: &mut ItemCore, assets: &ItemAssets) {
    if item.life_timer <= 0.0 {
        explode(item, assets);
    } else {
        item.life_timer -= 1.0;
    }
}

/// itSamusbomb_UnkMotion0_Anim.
fn resting_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    burn_fuse(item, ctx.assets);
    false
}

/// itSamusbomb_UnkMotion1_Anim / UnkMotion2_Anim: the fuse, then a moving
/// bomb faces its way (itSamusBomb_UnkMotion_Process).
#[allow(clippy::neg_cmp_op_on_partial_ord)] // A NaN speed turns, as in retail.
fn moving_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    burn_fuse(item, ctx.assets);
    if !(gekko_math::msl::fabsf(item.velocity.x) < TURN_SPEED) {
        face_velocity(item);
        // HSD_JObjSetRotationY(jobj, 0 or M_PI).
        item.rotation.y = if item.facing == 1.0 {
            0.0
        } else {
            std::f32::consts::PI
        };
    }
    false
}

/// it_80272980 (80272980): a moving item faces its velocity (a still one
/// with no facing faces right), and its collision takes the facing.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // A NaN speed turns, as in retail.
fn face_velocity(item: &mut ItemCore) {
    let speed = gekko_math::msl::fabsf(item.velocity.x);
    if !(speed < 0.00001) || item.facing == 0.0 {
        item.facing = if item.velocity.x >= 0.0 { 1.0 } else { -1.0 };
    }
    let facing = if item.facing == -1.0 { -1 } else { 1 };
    let collision = item.collision.as_mut().expect("item map collision");
    melee_mp::set_facing_dir(collision, facing);
}

/// itSamusbomb_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itSamusbomb_UnkMotion2_Phys: the slide's x7C clamp. it_80277040 never
/// lets a Samus bomb slide on the ported stages' floors.
fn sliding(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    unimplemented!("itSamusbomb_UnkMotion2_Phys: a sliding bomb")
}

/// itSamusbomb_UnkMotion0_Coll: it_8026D62C; off the floor it falls
/// (it_802B4CF4).
fn resting_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        change(item, motion::FALLING, KEEP_HITBOX, ctx.assets);
    }
    false
}

/// itSamusbomb_UnkMotion1_Coll: it_8026E248; at rest it lies on the floor
/// (it_802B4C10).
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.bounce_to_rest(ctx.map, ctx.assets) {
        change(item, motion::RESTING, KEEP_HITBOX, ctx.assets);
    }
    false
}

/// itSamusbomb_UnkMotion2_Coll: it_8026E8C4.
fn sliding_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itSamusbomb_UnkMotion2_Coll: a sliding bomb (it_8026E8C4)")
}

/// itSamusbomb_UnkMotion3_Anim -> it_802751D8: the blast lasts its lifetime.
fn explosion_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// The event callbacks' guard: only a bomb that is not a blast (msid 3).
fn explode_unless_exploding(item: &mut ItemCore, assets: &ItemAssets) {
    if item.motion != motion::EXPLODING {
        explode(item, assets);
    }
}

/// it_802B53CC (802B53CC): the blast state at full animation rate, no
/// velocity (it_80273454), the model hidden (it_8026BB44), the explosion
/// effect, sound and no more hitlag (it_80272B40), not pickable
/// (it_8026B3A8), the common explosion lifetime without a destroy effect
/// (it_8027518C), and the accessory that offers the owner the blast.
/// it_8026BD24's xDD0 b3 has no ported reader.
fn explode(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::EXPLODING, ANIM_UPDATE, assets);
    item.animation_rate = 1.0;
    item.velocity = Vec3::ZERO;
    item.hidden = true;
    item.events.push(ItemEvent::Effect {
        id: EXPLOSION,
        position: item.position,
    });
    item.sound_requests.push(EXPLOSION_SOUND);
    item.hitlag_enabled = false;
    item.grabbable = false;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    bomb_mut(item).blast_pending = true;
}
