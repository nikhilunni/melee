//! Box (It_Kind_Box), itbox.c (80286088..80287458): a crate. It falls in
//! and rests; thrown into a floor or wall, dropped hard, struck by a hit
//! of its threshold or hitting anything itself, it breaks (efSync 0x427's
//! boards) into one to three items or, by the weights, an explosion.
//! Kongo Jungle's floating crate (it_80286088, state 8) is not reached on
//! the stages the port plays.
use crate::{container, container_mut, no_animation, no_collision, no_physics};
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, EnteredAir, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemStateRow,
};
use melee_types::ItemKind;

pub struct Box;

/// it_803F5850's anim_id column.
pub const ARTICLE_STATES: [i32; 9] = [-1, -1, -1, 0, 0, 0, 1, 2, -1];
/// itBoxAttributes: x0..x1C.
pub const SPECIAL_ATTRIBUTES: u32 = 8;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    pub const HELD: u16 = 2;
    pub const THROWN: u16 = 3;
    pub const DROPPED: u16 = 4;
    /// Sliding down a floor (itBox_UnkMotion5).
    pub const SLIDE: u16 = 5;
    /// Broken empty: the explosion's article animation (it_80286AA4).
    pub const BLAST: u16 = 6;
    /// Broken open: gone after its despawn timer (it_80286BA0).
    pub const OPENED: u16 = 7;
}

/// The step itBox_TryOpen resumes at once its boards are made.
const TRY_OPEN: u8 = 0;
/// efAsync 0x427, the boards (melee_ef::BOX_FRAGMENTS_REQUEST).
const BOARDS_EFFECT: u16 = 0x427;
/// efAsync 0x421 (it_802787B4), as the crate's lifetime runs out at rest.
const LIFETIME_END_EFFECT: u16 = 0x421;
/// it_80286BA0 / it_80286AA4: `despawn_timer = 40`.
const DESPAWN_FRAMES: i32 = 40;
/// Camera_RequestQuake(QuakeKind_Small, ...): the ItemEvent kind.
const SMALL_QUAKE: u16 = 2;

/// itBoxAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn int(&self, index: usize) -> i32 {
        self.0[index].to_bits() as i32
    }
    /// x0..x8: the weights of one, two and three items.
    fn spawn_weights(&self) -> [i32; 3] {
        [self.int(0), self.int(1), self.int(2)]
    }
    /// xC: the weight of breaking empty (the blast).
    fn empty_weight(&self) -> i32 {
        self.int(3)
    }
    /// x10: one in this many lets out one kind only.
    fn same_kind_odds(&self) -> i32 {
        self.int(4)
    }
    /// x14: the damage that breaks it.
    fn damage_threshold(&self) -> f32 {
        self.0[5]
    }
    /// x18: the steepest floor it settles on when it lands sliding.
    fn upright_angle(&self) -> f32 {
        self.0[6]
    }
}

static STATES: [ItemStateRow; 9] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: rest_animation,
        physics: no_physics,
        collision: rest_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: no_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: no_animation,
        physics: tumble_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: no_animation,
        physics: tumble_physics,
        collision: dropped_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: no_animation,
        physics: no_physics,
        collision: slide_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: crate::explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[7],
        animation: opened_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[8],
        animation: floating_unported,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for Box {
    const KIND: ItemKind = ItemKind::Box;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itBox_Logic1_EnteredAir (80287008): on a floor near enough upright
    /// (it_802863BC) it rests, otherwise it slides (state 5).
    const ENTERED_AIR: Option<EnteredAir> = Some(|item, assets| {
        if settles_upright(item, assets) {
            rest(item, assets);
        } else {
            change(item, motion::SLIDE, ANIM_UPDATE, assets);
        }
    });
    /// itBox_Logic1_Spawned (802861CC): the ECB stays upright (xDCE b7),
    /// then the fall (it_8028655C).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        item.scratch = melee_it::ItemScratch::Container(Default::default());
        start_falling(item, assets);
    }
    /// itBox_Logic1_PickedUp.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itBox_Logic1_Thrown: not grabbable (it_8026B3A8), state 3.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = false;
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itBox_Logic1_Dropped: not grabbable, state 4.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = false;
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itBox_Logic1_DmgDealt: unopened, it breaks.
    fn damage_dealt(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// itBox_Logic1_Clanked.
    fn clanked(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// itBox_Logic1_HitShield.
    fn hit_shield(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// itBox_Logic1_Reflected.
    fn reflected(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        break_unless_opened(item);
        false
    }
    /// itBox_Logic1_DmgReceived (80286EB0): unopened, the damage it has
    /// taken (xC9C) at its threshold breaks it.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let threshold = Attributes(&context.assets.special_attributes).damage_threshold();
        if !container(item).opened && item.damage_percent as f32 >= threshold {
            try_open(item);
        }
        false
    }
    /// itBox_TryOpen's body after the boards (it_80286340): the weights
    /// decide between the contents (it_80286BA0) and the blast
    /// (it_80286AA4).
    fn resumed(item: &mut ItemCore, step: u8, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
        assert_eq!(step, TRY_OPEN, "itbox.c resumes only itBox_TryOpen");
        let a = Attributes(&assets.special_attributes);
        let [one, two, three] = a.spawn_weights();
        // it_80286340: sums in this order, the roll over all four.
        let two_or_fewer = one + two;
        let any = three + two_or_fewer;
        let roll = rng.randi(a.empty_weight() + any);
        if roll < any {
            open(item, assets);
        } else {
            blast(item, assets);
        }
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// it_8028655C (8028655C): not grabbable, the fall.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    change(item, motion::FALL, ANIM_UPDATE, assets);
}

/// fn_80286480 (80286480): grabbable (it_8026B390), still
/// (itResetVelocity), at rest.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = true;
    item.velocity = Vec3::ZERO;
    change(item, motion::REST, ANIM_UPDATE, assets);
}

/// it_802863BC (802863BC): touching a floor (it_80276408) whose normal is
/// within x18 of up (lbVector_AngleXY), the slide stops for good: its
/// vectors clear and xD5C takes 1.
fn settles_upright(item: &mut ItemCore, assets: &ItemAssets) -> bool {
    let Some(normal) = item.floor_contact_normal() else {
        return false;
    };
    let up = Vec3::new(0.0, 1.0, 0.0);
    if melee_lb::vector::angle_xy(normal, up) >= Attributes(&assets.special_attributes).upright_angle()
    {
        return false;
    }
    item.slide_velocity = Vec3::ZERO;
    item.slide_acceleration = Vec3::ZERO;
    item.previous_slide_acceleration = Vec3::ZERO;
    item.slide_mode = 1;
    true
}

/// itBox_TryOpen (inline): efSync 0x427's boards at the crate, whose draws
/// come before the rest ([`Box::resumed`]).
fn try_open(item: &mut ItemCore) {
    item.events.push(ItemEvent::Effect {
        id: BOARDS_EFFECT,
        position: item.position,
    });
    item.defer_callback(TRY_OPEN);
}


fn break_unless_opened(item: &mut ItemCore) {
    if !container(item).opened {
        try_open(item);
    }
}

/// it_80286BA0 (80286BA0): the sound, a small quake, the contents
/// (it_80286248), then hidden, taking no hits (it_802756D0), still, no
/// destroy effect (xDCF b2), opened with its despawn timer, not
/// grabbable, state 7.
fn open(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.events.push(ItemEvent::Quake {
        kind: SMALL_QUAKE,
        joint: 0,
        offset: Vec3::ZERO,
    });
    let previous_position = Vec3::new(item.position.x, item.position.y, 0.0);
    let ecb = item.collision.as_ref().expect("a crate's collision").ecb;
    let rise = 0.5 * (ecb.top.y + ecb.bottom.y);
    item.events.push(ItemEvent::CrateContents {
        roll: Some(melee_it::CrateRoll {
            weights: a.spawn_weights(),
            same_kind_odds: a.same_kind_odds(),
        }),
        position: Vec3::new(item.position.x, item.position.y + rise, item.position.z),
        previous_position,
    });
    item.hidden = true;
    item.hurt_intangible = true;
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    item.destroy_effect_suppressed = true;
    let state = container_mut(item);
    state.opened = true;
    state.despawn_timer = DESPAWN_FRAMES;
    item.grabbable = false;
    change(item, motion::OPENED, ANIM_UPDATE, assets);
}

/// it_80286AA4 (80286AA4): broken empty, the blast: hidden (it_8026BB44),
/// the blast effect (it_80272C08), taking no hits, not grabbable, the
/// explosion lifetime (it_8027518C), still, opened with its despawn
/// timer, hitting its owner and kin (it_80275444), a gust, state 6.
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    crate::explode(item, assets);
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    let state = container_mut(item);
    state.opened = true;
    state.despawn_timer = DESPAWN_FRAMES;
    // retail 0x80286B58: Item_80268E5C(gobj, 6, 2).
    change(item, motion::BLAST, ANIM_UPDATE, assets);
}

/// itBox_UnkMotion0_Anim: with one frame of life left, effect 0x421
/// (it_802787B4).
fn rest_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 1.0 {
        item.events.push(ItemEvent::DestroyEffect {
            id: LIFETIME_END_EFFECT,
            root: None,
        });
    }
    false
}

/// itBox_UnkMotion0_Coll: it_8026D62C (off the floor: it_8028655C), then
/// the lean with the floor (it_80276CB8).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    item.lean_with_floor();
    false
}

/// itBox_UnkMotion1_Phys: gravity (it_80272860). A crate with a parent
/// (Kongo Jungle's) also tumbles.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itBox_UnkMotion4_Phys -> Item_ApplyFallingPhysics: gravity and the
/// falling spin.
fn tumble_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itBox_UnkMotion1_Coll -> it_8026E15C: a settled landing rests
/// (fn_80286480).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => false,
        AirLanding::Landed => {
            rest(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

/// itBox_UnkMotion3_Coll: thrown, a floor (it_8026DA08) breaks it; a
/// ceiling (it_802763E0) stops it rising; a wall (it_80276308) breaks it.
fn thrown_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itBox_UnkMotion3_Coll: a thrown Box (itbox.c:322), unwitnessed")
}

/// itBox_UnkMotion4_Coll: dropped onto a floor faster than x1C it breaks;
/// slower it lands (it_802762B0) and rests.
fn dropped_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itBox_UnkMotion4_Coll: a dropped Box (itbox.c:342), unwitnessed")
}

/// itBox_UnkMotion5_Coll -> it_8026E8C4: a stopped slide rests, off the
/// floor it falls.
fn slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itBox_UnkMotion5_Coll: a sliding Box (itbox.c:566), unwitnessed")
}

/// itBox_UnkMotion7_Anim: gone when the despawn timer runs out.
fn opened_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = container_mut(item);
    state.despawn_timer -= 1;
    state.despawn_timer <= 0
}

fn floating_unported(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itBox_UnkMotion8: Kongo Jungle's floating crate (it_80286088)")
}
