//! Yoshi's Story Shy Guy (It_Kind_Heiho), itheiho.c (802D8618..802D9A0C).
//!
//! grStory_801E3418 sends a group across the stage at one of six heights.
//! Each waits its stagger (state 0), then walks through the air (state 1)
//! with its vertical bob authored as a joint's translation
//! (itUpdateVelocityFromBone), turning round at walls. A light hit stuns it
//! and it flees (states 3, 4); a heavier one knocks it spinning off the
//! stage (state 2).
//!
//! heiho.x2C, the frame count past which a walking Shy Guy climbs away
//! (Motion1/4_Phys, `x2C > 960`), is only ever cleared (it_802D8688), so the
//! climb never runs and is not ported. The food a Shy Guy may carry
//! (it_8028FAF4) needs items switched on (it_8026D324), which the port's
//! matches never are.
use core::cell::Cell;
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;
use melee_it::{bone_motion::BoneRate, desc::ItemAssets, state_change::*, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct Heiho;

/// it_803F83F0's anim_id column: the article state each motion state plays.
pub const ARTICLE_STATES: [i32; 5] = [-1, 0, -1, -1, 2];
/// Special attributes x0..x18. x0 is a pointer to an integer, read through
/// as [`POINTER_ATTRIBUTES`].
pub const SPECIAL_ATTRIBUTES: u32 = 7;
pub const POINTER_ATTRIBUTES: [u32; 1] = [0];
/// xBBC_dynamicBoneTable->bones[1]: the joint whose translation is the gait.
pub const GAIT_BONE: usize = 1;

mod motion {
    /// Waiting out the group's stagger (itHeiho_UnkMotion0).
    pub const WAIT: u16 = 0;
    /// Walking across the stage (itHeiho_UnkMotion1).
    pub const WALK: u16 = 1;
    /// Knocked spinning away (itHeiho_UnkMotion2).
    pub const KNOCKED_AWAY: u16 = 2;
    /// Stunned by a light hit (itHeiho_UnkMotion3).
    pub const STUNNED: u16 = 3;
    /// Fleeing at one and a half times its speed (itHeiho_UnkMotion4).
    pub const FLEE: u16 = 4;
}

/// Frames a wall turn lasts (Motion1_Coll / it_802D9168: x24 = 0x14).
const TURN_FRAMES: i32 = 0x14;
/// Frames a light hit stuns it (it_802D8EC8: x24 = 0xC).
const STUN_FRAMES: i32 = 0xC;
/// HSD_JObjAddRotationY per turning frame (0.15707964f, pi/20).
const TURN_STEP: f32 = 0.157_079_64;
/// HSD_JObjSetRotationY facing left (3.1415927f).
const FACING_LEFT_ROTATION: f32 = std::f32::consts::PI;
/// it_802D9714: the margin inside and outside the blast zones.
const SCREEN_MARGIN: f32 = 20.0;
/// Motion4_Phys: fleeing runs at 1.5 times the walking speed.
const FLEE_SPEED_SCALE: f32 = 1.5;

/// itHeiho special attributes.
struct Attributes<'a>(&'a ItemAssets);
impl Attributes<'_> {
    /// x4 + 4 * variant: the walking speeds.
    fn speed(&self, variant: i8) -> f32 {
        self.0.special_attributes[variant as usize + 1]
    }
    /// x14: the random horizontal kick of a heavy hit.
    fn knock_spread(&self) -> f32 {
        self.0.special_attributes[5]
    }
    /// *x0: the damage (times 0.8) past which a hit knocks it away.
    fn knock_away_damage(&self) -> i32 {
        // retail 802D8F44..50: int to float, fmuls 0.8, fctiwz.
        gekko_math::msl::fctiwz(0.8 * self.0.special_pointees[0] as f32)
    }
}

static STATES: [ItemStateRow; 5] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: no_animation,
        physics: wait_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: walk_animation,
        physics: walk_physics,
        collision: walk_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: no_animation,
        physics: knocked_away_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: no_animation,
        physics: stunned_physics,
        collision: stunned_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: flee_animation,
        physics: flee_physics,
        collision: flee_collision,
    },
];

impl ItemLogic for Heiho {
    const KIND: ItemKind = ItemKind::Heiho;
    const STATES: &'static [ItemStateRow] = &STATES;

    /// No pickup callback: Item_IsGrabbable never passes.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }

    /// it_802D8688 (802D8688). it_8027B730's zako fields and ground-push
    /// flags (xDC8 x1C, x1E) only matter on the ground, where a Shy Guy
    /// never is.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.platform_drop = 0;
        item.blast_zone_checked = false;
        item.scratch = ItemScratch::Heiho(HeihoState::default());
        item.position.z = 2.0;
        item.enter_air();
        if item.position.x < 0.0 {
            item.facing = 1.0;
            item.rotation.y = 0.0;
        } else {
            item.facing = -1.0;
            item.rotation.y = FACING_LEFT_ROTATION;
        }
        wait(item, assets);
    }

    /// it_802D8EC8 (802D8EC8): the hit puts the blast zones back in force;
    /// past the attribute's damage it is knocked away, else stunned.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let assets = context.assets;
        item.blast_zone_checked = true;
        if item.damage_percent > Attributes(assets).knock_away_damage() {
            // it_8027CE44 -> grZakoGenerator_801CACB8: no generator slot
            // (zako.idx is -1); it_8027CE18's bonus tally is not modelled.
            let rng = context.rng.expect("OnTakeDamage draws");
            knock_away(item, assets, rng);
        } else {
            let (velocity, launched) = item.knockback_launch(&context.launch);
            item.velocity = velocity;
            if launched {
                item.enter_air();
            }
            heiho_mut(item).countdown = STUN_FRAMES;
            heiho_mut(item).bone_previous = Vec3::ZERO;
            item.change_motion_with(
                motion::STUNNED,
                ARTICLE_STATES[motion::STUNNED as usize],
                ANIM_UPDATE,
                assets,
            );
        }
        false
    }
}

/// it_802D8618 (802D8618): a Shy Guy of a spawned group, created by
/// it_8027B5B0 (the facing is it_8026B684's, drawn before creation).
pub fn spawn(position: Vec3, facing: f32) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        secondary_owner: None,
        kind: ItemKind::Heiho,
        // Item_802674AC: stage enemies.
        hold_kind: 4,
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
        // Item_80268B18.
        ground_or_air: GroundOrAir::Air,
    }
}

/// it_802D8618's tail: the group index, walking speed and stagger, then the
/// wait again.
pub fn join_group(
    item: &mut ItemCore,
    group_index: i32,
    speed_variant: i32,
    delay: i32,
    assets: &ItemAssets,
) {
    let state = heiho_mut(item);
    state.group_index = group_index as i8;
    state.speed_variant = speed_variant as i8;
    state.countdown = delay;
    wait(item, assets);
}

fn heiho(item: &ItemCore) -> &HeihoState {
    match &item.scratch {
        ItemScratch::Heiho(state) => state,
        _ => panic!("Shy Guy scratch"),
    }
}
fn heiho_mut(item: &mut ItemCore) -> &mut HeihoState {
    match &mut item.scratch {
        ItemScratch::Heiho(state) => state,
        _ => panic!("Shy Guy scratch"),
    }
}

/// it_802D8894 (802D8894): stop and wait.
fn wait(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.change_motion_with(
        motion::WAIT,
        ARTICLE_STATES[motion::WAIT as usize],
        ANIM_UPDATE,
        assets,
    );
}

/// Item_80268E5C into an animated state: the gait joint starts over.
fn enter_animated(item: &mut ItemCore, motion: u16, rate: BoneRate, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], ANIM_UPDATE, assets);
    let state = heiho_mut(item);
    state.bone_rate = rate;
    state.bone_step = 0;
}

/// it_802D98C4 (802D98C4) -> itUpdateVelocityFromBone: move by the gait
/// joint's change since the last reading. Retail 802D98C4: fmuls and fsubs,
/// unfused.
fn follow_gait(item: &mut ItemCore, assets: &ItemAssets) {
    let article = ARTICLE_STATES[item.motion as usize] as usize;
    let state = heiho(item);
    let sample = gait(assets).sample(article, state.bone_rate, state.bone_step);
    let previous = state.bone_previous;
    let now = sample.translation;
    item.velocity.x = item.facing * (now.z - previous.z);
    item.velocity.y = now.y - previous.y;
    item.velocity.z = now.x - previous.x;
    heiho_mut(item).bone_previous = now;
}

/// it_80272C6C: the gait joint's animation still runs at this step.
fn gait_running(item: &ItemCore, assets: &ItemAssets) -> bool {
    let article = ARTICLE_STATES[item.motion as usize] as usize;
    let state = heiho(item);
    gait(assets)
        .sample(article, state.bone_rate, state.bone_step)
        .running
}

fn gait(assets: &ItemAssets) -> &melee_it::bone_motion::BoneMotion {
    assets
        .bone_motion
        .as_ref()
        .expect("Shy Guy gait joint samples")
}

/// Item_802694CC's HSD_JObjAnimAll before the state's animation callback.
fn step_gait(item: &mut ItemCore) {
    heiho_mut(item).bone_step += 1;
}

/// it_802D8918 (802D8918): start walking from a fresh gait reading. The
/// accessory (it_802D96B0) carries food, which a Shy Guy never has here.
fn start_walking(item: &mut ItemCore, assets: &ItemAssets) {
    heiho_mut(item).countdown = 0;
    heiho_mut(item).bone_previous = Vec3::ZERO;
    enter_animated(item, motion::WALK, BoneRate::Steady, assets);
    follow_gait(item, assets);
}

/// itHeiho_UnkMotion1_Anim_inline: the walk (or flee) starts over without
/// resetting the gait reading.
fn restart_walk(item: &mut ItemCore, assets: &ItemAssets) {
    heiho_mut(item).countdown = 0;
    enter_animated(item, motion::WALK, BoneRate::Steady, assets);
    follow_gait(item, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itHeiho_UnkMotion0_Phys (802D88D4): count the stagger down, then walk.
fn wait_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if heiho(item).countdown == 0 {
        start_walking(item, ctx.assets);
        return;
    }
    heiho_mut(item).countdown -= 1;
}

/// itHeiho_UnkMotion1_Anim (802D8984).
fn walk_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    step_gait(item);
    follow_gait(item, ctx.assets);
    if !gait_running(item, ctx.assets) {
        restart_walk(item, ctx.assets);
    }
    false
}

/// itHeiho_UnkMotion1_Phys (802D8A54): walk at its speed (retail 802D8A8C:
/// fmuls), watch the blast zones, and turn.
fn walk_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.x = item.facing * Attributes(ctx.assets).speed(heiho(item).speed_variant);
    watch_blast_zones(item, ctx.bounds);
    turn(item);
}

/// The tail of Motion1_Phys and Motion4_Phys: a turn spins the model a
/// twentieth of a half turn a frame; otherwise it faces its direction.
fn turn(item: &mut ItemCore) {
    let state = heiho_mut(item);
    if state.countdown != 0 {
        state.countdown -= 1;
        item.rotation.y += TURN_STEP;
        return;
    }
    item.rotation.y = if item.facing == -1.0 {
        FACING_LEFT_ROTATION
    } else {
        0.0
    };
}

/// it_802D9714 (802D9714): once it has come within the blast zones, leaving
/// them by the margin puts Item_802696CC's check back in force (and would
/// drop carried food).
fn watch_blast_zones(item: &mut ItemCore, bounds: &ItemBounds) {
    let x = item.position.x;
    let y = item.position.y;
    if !heiho(item).entered_screen {
        if x > SCREEN_MARGIN + bounds.left && x < bounds.right - SCREEN_MARGIN {
            heiho_mut(item).entered_screen = true;
        }
    } else if x > SCREEN_MARGIN + bounds.right
        || x < bounds.left - SCREEN_MARGIN
        || y > SCREEN_MARGIN + bounds.top
        || y < bounds.bottom - SCREEN_MARGIN
    {
        item.blast_zone_checked = true;
    }
}

/// itHeiho_UnkMotion1_Coll (802D8CC8): it_8026DA70 senses the map; a wall
/// turns it round, a floor restarts the walk.
fn walk_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let floor = item.sense_air_collision(ctx.map);
    if heiho(item).countdown == 0 && item.wall_bits() != 0 {
        turn_round(item, ctx.assets);
    } else if floor {
        restart_walk(item, ctx.assets);
    }
    false
}

/// Motion1_Coll / Motion4_Coll at a wall: reverse and walk back at its
/// plain speed (retail: fneg, fmuls), spinning round for the turn.
fn turn_round(item: &mut ItemCore, assets: &ItemAssets) {
    item.facing = -item.facing;
    item.velocity.x = item.facing * Attributes(assets).speed(heiho(item).speed_variant);
    heiho_mut(item).countdown = TURN_FRAMES;
}

/// it_802D8EC8's heavy branch (802D8F64..9104): a random sideways kick
/// (retail 802D8F84..94: fsubs, fmuls, fmadds), up and out of the stage
/// plane, then state 2.
fn knock_away(item: &mut ItemCore, assets: &ItemAssets, rng: &Cell<HsdRng>) {
    let spread = 2.0 * (draw(rng, |r| r.randf()) - 0.5);
    item.velocity.x = fmadds(Attributes(assets).knock_spread(), spread, item.velocity.x);
    item.velocity.y = 2.0;
    item.velocity.z = 1.5;
    item.rotation.y = if item.facing == -1.0 {
        FACING_LEFT_ROTATION
    } else {
        0.0
    };
    heiho_mut(item).bone_previous = Vec3::ZERO;
    item.change_motion_with(
        motion::KNOCKED_AWAY,
        ARTICLE_STATES[motion::KNOCKED_AWAY as usize],
        ANIM_UPDATE,
        assets,
    );
}

fn draw<T>(rng: &Cell<HsdRng>, f: impl FnOnce(&mut HsdRng) -> T) -> T {
    let mut state = rng.get();
    let value = f(&mut state);
    rng.set(state);
    value
}

/// itHeiho_UnkMotion2_Phys (802D8DBC): fall (it_80272860), then spin by
/// one of three random rates (retail 802D8E1C: fmadds 8 * r + 1, then
/// fmuls by pi/180) about the model's axis (it_80274A64). Item_802697D4
/// spins it once more after the move.
fn knocked_away_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
    let rate = draw(ctx.rng, |r| r.randi(3)) as f32;
    item.spin_speed = 0.017_453_292 * fmadds(8.0, rate, 1.0);
    item.spin();
}

/// itHeiho_UnkMotion3_Phys (802D8E54): fall for the stun, then flee.
fn stunned_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.velocity.y -= ctx.assets.fall_acceleration;
    let state = heiho_mut(item);
    if state.countdown != 0 {
        state.countdown -= 1;
        return;
    }
    flee(item, ctx.bounds, BoneRate::DoubledAfterOne, ctx.assets);
}

/// itHeiho_UnkMotion3_Coll (802D8EA4): it_8026DA70, its result unused.
fn stunned_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.sense_air_collision(ctx.map);
    false
}

/// it_802D9168 (802D9168): flee away from the camera's centre
/// (Stage_UnkSetVec3TCam_Offset), turning round first if the model faces
/// the wrong way.
fn flee(item: &mut ItemCore, bounds: &ItemBounds, rate: BoneRate, assets: &ItemAssets) {
    heiho_mut(item).countdown = 0;
    let centre = bounds.camera_offset.x;
    if item.position.x < centre {
        if item.rotation.y == 0.0 {
            heiho_mut(item).countdown = TURN_FRAMES;
        }
        item.facing = -1.0;
    } else if item.position.x > centre {
        if item.rotation.y != 0.0 {
            heiho_mut(item).countdown = TURN_FRAMES;
        }
        item.facing = 1.0;
    }
    heiho_mut(item).bone_previous = Vec3::ZERO;
    enter_animated(item, motion::FLEE, rate, assets);
    follow_gait(item, assets);
}

/// itHeiho_UnkMotion4_Anim (802D9274): as the walk, restarting the flee
/// when the gait ends; the gait then plays at double rate.
fn flee_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    step_gait(item);
    follow_gait(item, ctx.assets);
    if !gait_running(item, ctx.assets) {
        heiho_mut(item).countdown = 0;
        enter_animated(item, motion::FLEE, BoneRate::Doubled, ctx.assets);
        follow_gait(item, ctx.assets);
    }
    false
}

/// itHeiho_UnkMotion4_Phys (802D9384): run at 1.5 times the walking speed
/// (retail 802D93C0/C4: fmuls, fmuls), and turn. No blast zone watch.
fn flee_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let speed = Attributes(ctx.assets).speed(heiho(item).speed_variant);
    item.velocity.x = FLEE_SPEED_SCALE * (item.facing * speed);
    turn(item);
}

/// itHeiho_UnkMotion4_Coll (802D95F4): as the walk's; a floor makes it
/// flee afresh.
fn flee_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let floor = item.sense_air_collision(ctx.map);
    if heiho(item).countdown == 0 && item.wall_bits() != 0 {
        turn_round(item, ctx.assets);
    } else if floor {
        flee(item, ctx.bounds, BoneRate::DoubledAfterOne, ctx.assets);
    }
    false
}
