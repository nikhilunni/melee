//! Mr. Saturn (It_Kind_Dosei), itdosei.c (80281164..80283558). Ported: the
//! spawn's fall, the hold, throwing and dropping (a thrown Mr. Saturn pops
//! back off what it hits), landing into the walk, turning at edges and
//! walls, and falling off the floor. The idle and look-round states after a
//! spawn's landing, sliding down a steep floor and being hit fail closed.
//!
//! The model's rotations (HSD_JObjSetRotation on its facing) are drawing
//! only and are not modelled.
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE, MODEL_UPDATE},
    AirLanding, DoseiState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct Dosei;

/// it_803F55D0's anim_id column.
pub const ARTICLE_STATES: [i32; 12] = [-1, 0, 2, -1, 3, 1, -1, -1, -1, 0, -1, 3];
/// itDoseiAttributes: six words.
pub const SPECIAL_ATTRIBUTES: u32 = 6;

/// Item_80268E5C flag bit 0 (it/forward.h), which this file passes with
/// ANIM_UPDATE as 3.
const UNK_0X1: u32 = 1;

mod motion {
    pub const WALK: u16 = 1;
    pub const TURN: u16 = 2;
    pub const FALL: u16 = 3;
    pub const HELD: u16 = 4;
    pub const THROWN: u16 = 5;
    /// The spawn's fall (itDosei_80282BFC).
    pub const SPAWN_FALL: u16 = 8;
}

/// itDoseiAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// unk0: the animation rate in a hand and in flight.
    fn carried_rate(&self) -> f32 {
        self.0[0]
    }
    /// unk4 (an int): frames it sits after a spawn's landing.
    fn idle_frames(&self) -> i32 {
        self.0[1].to_bits() as i32
    }
    /// unk8: the walking speed.
    fn walk_speed(&self) -> f32 {
        self.0[2]
    }
}

static STATES: [ItemStateRow; 12] = {
    const UNPORTED: ItemStateRow = ItemStateRow {
        animation_id: -1,
        animation: unported_animation,
        physics: unported_physics,
        collision: unported_collision,
    };
    let mut rows = [UNPORTED; 12];
    rows[motion::WALK as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: walk_animation,
        physics: walk_physics,
        collision: walk_collision,
    };
    rows[motion::TURN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: turn_animation,
        physics: no_physics,
        collision: turn_collision,
    };
    rows[motion::FALL as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: fall_animation,
        physics: gravity_physics,
        collision: flight_collision,
    };
    // it_803F55D0[4]: no collision callback.
    rows[motion::HELD as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: held_animation,
        physics: no_physics,
        collision: no_collision,
    };
    rows[motion::THROWN as usize] = ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: no_animation,
        physics: falling_physics,
        collision: flight_collision,
    };
    rows
};

impl ItemLogic for Dosei {
    const KIND: ItemKind = ItemKind::Dosei;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itDosei_Logic7_Spawned (80281164): still, grabbable (it_8026B390),
    /// then the spawn's fall (itDosei_80282BFC).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        let a = Attributes(&assets.special_attributes);
        item.velocity = Vec3::ZERO;
        item.scratch = ItemScratch::Dosei(DoseiState {
            idle_countdown: a.idle_frames(),
            recover_timer: 0,
            throw_scale: 1.0,
            ..Default::default()
        });
        item.grabbable = true;
        change(item, motion::SPAWN_FALL, UNK_0X1 | ANIM_UPDATE, assets);
        item.animation_rate = 1.0;
        item.owner = None;
    }
    /// it_802BD4AC's Mr. Saturn (Peach's pull): xDE0 takes the item's scale
    /// after it_80274594 (a unit owner scale in every supported mode).
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut HsdRng,
    ) {
        if spawn.owner.is_some() {
            let scale = item.scale;
            dosei_mut(item).throw_scale = scale;
        }
    }
    /// itDosei_Logic7_PickedUp (80282524): the hold at the carried rate,
    /// its spin locked to the facing (xDC8 x17 and x19).
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        enter_held(item, context.assets);
    }
    /// itDosei_Logic7_Thrown (8028288C): the flight at the carried rate,
    /// then it_80274484 at xDE0.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        if item.motion == motion::THROWN {
            change(
                item,
                motion::THROWN,
                ANIM_UPDATE | DROP_UPDATE,
                context.assets,
            );
        } else {
            item.grabbable = true;
            change(
                item,
                motion::THROWN,
                ANIM_UPDATE | DROP_UPDATE,
                context.assets,
            );
            item.animation_rate = Attributes(&context.assets.special_attributes).carried_rate();
        }
        let scale = dosei(item).throw_scale;
        item.rescale(scale);
    }
    /// itDosei_Logic7_Dropped (802827A8): as a throw without the rescale.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = true;
        item.spin_ignores_facing = true;
        item.rotation_axis = 1;
        change(
            item,
            motion::THROWN,
            ANIM_UPDATE | DROP_UPDATE,
            context.assets,
        );
        item.animation_rate = Attributes(&context.assets.special_attributes).carried_rate();
    }
    /// itDosei_Logic7_DmgDealt (802834B8): a voice (Item_8026AF0C of one of
    /// three sounds by HSD_Randi(3)); in flight it pops back and its
    /// hitboxes go (it_802725D4).
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let rng = context.rng.expect("itDosei_Logic7_DmgDealt draws");
        let mut r = rng.get();
        r.randi(3);
        rng.set(r);
        if item.motion == motion::THROWN {
            item.bounce_off_victim(context.victim_bounce);
            item.clear_hitboxes();
        }
        false
    }
    /// itDosei_Logic7_DmgReceived (80283358).
    fn damage_received(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itDosei_Logic7_DmgReceived: Mr. Saturn knocked away (state 11)")
    }
    /// itDosei_Logic7_EnteredAir (80282A48).
    fn entered_air(_item: &mut ItemCore) {
        unimplemented!("itDosei_Logic7_EnteredAir: Mr. Saturn sliding off a steep floor")
    }
    /// itDosei_Logic7_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(context.reflected_speed);
        false
    }
    /// itDosei_Logic7_Clanked -> itColl_BounceOffVictim.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itDosei_Logic7_HitShield -> itColl_BounceOffVictim.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_victim(context.victim_bounce);
        false
    }
    /// itDosei_Logic7_ShieldBounced -> itColl_BounceOffShield.
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.bounce_off_shield(context.shield_normal);
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn dosei(item: &ItemCore) -> &DoseiState {
    match &item.scratch {
        ItemScratch::Dosei(state) => state,
        _ => panic!("Mr. Saturn scratch"),
    }
}
fn dosei_mut(item: &mut ItemCore) -> &mut DoseiState {
    match &mut item.scratch {
        ItemScratch::Dosei(state) => state,
        _ => panic!("Mr. Saturn scratch"),
    }
}

/// itDosei_Logic7_PickedUp and itDosei_UnkMotion4_Anim's restart.
fn enter_held(item: &mut ItemCore, assets: &ItemAssets) {
    if item.motion == motion::HELD {
        change(item, motion::HELD, ANIM_UPDATE, assets);
        return;
    }
    change(item, motion::HELD, ANIM_UPDATE, assets);
    item.animation_rate = Attributes(&assets.special_attributes).carried_rate();
    item.spin_ignores_facing = true;
    item.rotation_axis = 1;
}

/// it_80272C6C == 0: the state's joint animation has no frames left.
fn animation_ended(item: &ItemCore, assets: &ItemAssets) -> bool {
    let article = ARTICLE_STATES[item.motion as usize];
    usize::try_from(article)
        .ok()
        .and_then(|article| assets.animation_ends[article])
        .is_some_and(|end| item.animation_frame >= end)
}

/// itDosei_802817A0 (802817A0): on the floor (it_802762B0), walking along
/// its facing at unk8, its owner forgotten.
fn start_walking(item: &mut ItemCore, assets: &ItemAssets) {
    item.platform_drop = 0;
    item.spin_ignores_facing = true;
    item.rotation_axis = 1;
    item.owner = None;
    item.land_on_floor();
    change(
        item,
        motion::WALK,
        UNK_0X1 | ANIM_UPDATE | MODEL_UPDATE,
        assets,
    );
    item.animation_rate = 1.0;
    // retail 80281A8C: fmuls.
    item.velocity = Vec3::new(
        item.facing * Attributes(&assets.special_attributes).walk_speed(),
        0.0,
        0.0,
    );
}

/// itDosei_80281C6C (80281C6C): back to where the frame began, stopped, and
/// turning round.
fn start_turning(item: &mut ItemCore, assets: &ItemAssets) {
    let state = dosei_mut(item);
    state.turn_phase = 0;
    let position = state.last_position;
    state.turn_angle = 0.0;
    item.position = position;
    item.velocity.x = 0.0;
    item.land_on_floor();
    change(item, motion::TURN, UNK_0X1 | ANIM_UPDATE, assets);
    item.animation_rate = 1.0;
    item.owner = None;
}

/// itDosei_80282074 (80282074): falling off the floor.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::FALL, UNK_0X1 | ANIM_UPDATE, assets);
    item.animation_rate = 1.0;
    item.owner = None;
}

fn remember_position(item: &mut ItemCore) {
    let position = item.position;
    dosei_mut(item).last_position = position;
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itDosei_UnkMotion4_Anim (802825F8): the hold restarts when it ends.
fn held_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if animation_ended(item, ctx.assets) {
        enter_held(item, ctx.assets);
    }
    false
}

/// itDosei_UnkMotion1_Anim (80281AB4): the walk's rate follows the floor's
/// slope; the cycle restarts when it ends.
fn walk_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    remember_position(item);
    let normal = item
        .collision
        .as_ref()
        .expect("item map collision")
        .floor
        .normal;
    // retail 80281AF8: fmuls; 80281B00: fmadds.
    item.animation_rate = fmadds(0.5, normal.x * item.facing, 1.0);
    if animation_ended(item, ctx.assets) {
        change(item, motion::WALK, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itDosei_UnkMotion1_Phys (80281B44): the walking speed at the rate,
/// along the facing (fmuls, a sign test, fmuls).
fn walk_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let speed = Attributes(&ctx.assets.special_attributes).walk_speed() * item.animation_rate;
    // fcmpo with bge: only a value not below zero keeps its sign.
    let speed = if speed >= 0.0 { speed } else { -speed };
    item.velocity.x = item.facing * speed;
}

/// itDosei_UnkMotion1_Coll (80281B7C): it_8026D8A4 turns at an edge; a
/// wall turns it; a floor steeper than pi/4 in its normal's X would slide
/// it (EnteredAir); otherwise it leans with the floor. Off the floor it
/// falls.
fn walk_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let (grounded, edge) = item.walk_ground_pass(ctx.map);
    if edge {
        start_turning(item, ctx.assets);
    }
    if !grounded {
        start_falling(item, ctx.assets);
        return false;
    }
    if item.wall_bits() != 0 {
        start_turning(item, ctx.assets);
        return false;
    }
    let normal = item
        .collision
        .as_ref()
        .expect("item map collision")
        .floor
        .normal;
    let steep = gekko_math::msl::fabsf(normal.x) >= core::f32::consts::FRAC_PI_4;
    if steep {
        unimplemented!("itDosei_UnkMotion1_Coll: a steep floor (xD5C = 1, EnteredAir)");
    }
    item.platform_drop = 0;
    item.lean_with_floor();
    false
}

/// itDosei_UnkMotion2_Anim (80281CF8): at the end it walks the other way.
fn turn_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    remember_position(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::WALK, ANIM_UPDATE, ctx.assets);
        item.facing = -item.facing;
    }
    false
}

/// itDosei_UnkMotion2_Coll (80281E6C): it_8026D62C falls off the floor;
/// the look-round's walk-off phase (xDD8 2) cannot occur here.
fn turn_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map) {
        start_falling(item, ctx.assets);
    }
    assert_ne!(
        dosei(item).turn_phase,
        2,
        "itDosei_SetupWalk_FC from a turn"
    );
    item.lean_with_floor();
    false
}

/// itDosei_UnkMotion3_Anim (802820D4): the fall's cycle restarts.
fn fall_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    remember_position(item);
    if animation_ended(item, ctx.assets) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itDosei_UnkMotion3_Phys -> it_80272860: gravity only.
fn gravity_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itDosei_UnkMotion5_Phys -> Item_ApplyFallingPhysics: gravity, then the
/// falling spin.
fn falling_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itDosei_UnkMotion5_Coll (80282194) -> it_8026E15C: a settled landing
/// walks (itDosei_802817A0); a thrown Mr. Saturn may break on its first.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne => false,
        AirLanding::Landed => {
            start_walking(item, ctx.assets);
            false
        }
        AirLanding::Broken => true,
    }
}

fn unported_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itdosei.c: motion state {} animation", item.motion)
}
fn unported_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    unimplemented!("itdosei.c: motion state {} physics", item.motion)
}
fn unported_collision(item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itdosei.c: motion state {} collision", item.motion)
}
