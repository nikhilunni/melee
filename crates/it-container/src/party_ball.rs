//! Party Ball (It_Kind_Kusudama), itkusudama.c (802896CC..8028AE30): falls
//! in and rests; thrown into anything, dropped hard, struck past its
//! threshold or hitting anything it either rises, hangs and opens into a
//! shower of items (the scene's [`ItemEvent::PartyBallContents`]), or, by
//! the weights, blasts. Its script's command variables time the opening
//! (xDB0), the shower (xDAC) and a sound (xDB4).
use crate::{container, container_mut, no_collision, no_physics};
use gekko_math::msl::fctiwz;
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets,
    state_change::{ANIM_UPDATE, DROP_UPDATE},
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow,
};
use melee_types::ItemKind;

pub struct PartyBall;

/// it_803F59F8's anim_id column.
pub const ARTICLE_STATES: [i32; 9] = [-1, 0, -1, 1, -1, 2, 2, 3, 4];
/// itKusudamaAttributes, x0..x2C.
pub const SPECIAL_ATTRIBUTES: u32 = 12;

mod motion {
    pub const REST: u16 = 0;
    /// Hanging after the rise, until its script opens it.
    pub const HANGING: u16 = 1;
    pub const FALL: u16 = 2;
    /// Rising once struck open (it_8028A544).
    pub const RISING: u16 = 3;
    pub const HELD: u16 = 4;
    pub const THROWN: u16 = 5;
    pub const DROPPED: u16 = 6;
    /// The blast (it_8028AC74).
    pub const BLAST: u16 = 7;
    /// Open, showering its items (it_8028AD44).
    pub const OPEN: u16 = 8;
}

/// it_802897C8 / it_80289910: angles past pi wrap by tau.
const PI: f32 = std::f32::consts::PI;
const TAU: f32 = std::f32::consts::TAU;
/// it_8028A190_inline's M_PI_2, and the angle the rest and fall ease to.
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// it_802897C8: snap within ten degrees, otherwise ease by five degrees.
const PITCH_SNAP_DISTANCE: f32 = 0.174_532_92;
const PITCH_EASING_STEP: f32 = 0.087_266_46;
/// it_8028A544_inline: the rise's yaw.
const RISING_YAW: f32 = 4.694_936;
/// itKusudama_UnkMotion3_inline: frames it hangs before its script opens it
/// (`x4 = 0x5A`, which no callback reads again).
const HANGING_FRAMES: i32 = 0x5A;
/// it_8028AC74 / it_8028AD44: x4 once blasted or opened.
const BLAST_FRAMES: i32 = 0x28;
const OPEN_FRAMES: i32 = 0x55;
/// efSync 0x433 (efasync.c:688): the shower's sparkle (common generator
/// 0x115) on dynamic bones 3 and 5.
const SPARKLE: u16 = 0x433;
const SPARKLE_BONES: [usize; 2] = [3, 5];
/// The rest pitch's write after Item_80268E5C flushes queued joint effects
/// (it_8028A190's tail; itKusudama_UnkMotion6_Coll's it_80289910).
const REST_PITCH: u8 = 0;

/// itKusudamaAttributes (itCommonItems.h:968).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    fn int(&self, index: usize) -> i32 {
        self.0[index].to_bits() as i32
    }
    /// it_8028A114's weights in its argument order (x4, x0, x8, xC): the
    /// first three open it, the last blasts it.
    fn open_weights(&self) -> [i32; 4] {
        [self.int(1), self.int(0), self.int(2), self.int(3)]
    }
    /// x18: the rise's speed.
    fn rise_speed(&self) -> f32 {
        self.0[6]
    }
    /// x1C: the rise's frames (a float stored to an s32).
    fn rise_frames(&self) -> f32 {
        self.0[7]
    }
    /// x20: the damage that opens it.
    fn damage_threshold(&self) -> f32 {
        self.0[8]
    }
    /// x28: the yaw's step back toward its side.
    fn yaw_step(&self) -> f32 {
        self.0[10]
    }
    /// x2C: a dropped Party Ball landing faster than this opens.
    fn break_speed(&self) -> f32 {
        self.0[11]
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
        animation: hanging_animation,
        physics: no_physics,
        collision: hanging_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: rest_animation,
        physics: fall_physics,
        collision: fall_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: rising_animation,
        physics: no_physics,
        collision: rising_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[4],
        animation: still_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[5],
        animation: still_animation,
        physics: tumble_physics,
        collision: thrown_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[6],
        animation: still_animation,
        physics: tumble_physics,
        collision: dropped_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[7],
        animation: crate::explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[8],
        animation: open_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for PartyBall {
    const KIND: ItemKind = ItemKind::Kusudama;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itKusudama_Logic4_Spawned (80289784): the ECB upright (xDCE b7),
    /// the script's variables and x0 clear, then the fall (it_8028A3CC).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        item.command_variables[..3].fill(0);
        item.scratch = ItemScratch::Container(Default::default());
        start_falling(item, assets);
    }
    fn resumed(item: &mut ItemCore, step: u8, _assets: &ItemAssets, _rng: &mut gekko_math::HsdRng) {
        assert_eq!(step, REST_PITCH);
        assert_eq!(item.motion, motion::REST);
        pitch_to_side(item);
    }
    /// itKusudama_Logic4_PickedUp.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
    }
    /// itKusudama_Logic4_Thrown: not grabbable, state 5.
    fn thrown(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = false;
        change(item, motion::THROWN, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itKusudama_Logic4_Dropped: not grabbable, state 6.
    fn dropped(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        item.grabbable = false;
        change(item, motion::DROPPED, ANIM_UPDATE | DROP_UPDATE, context.assets);
    }
    /// itKusudama_Logic4_DmgDealt: unopened, it opens or blasts.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_opened(item, context);
        false
    }
    /// itKusudama_Logic4_Clanked -> it_80289B50(gobj, 0): as a hit dealt,
    /// without the opening's sound.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        open_unless_opened(item, context);
        false
    }
    /// itKusudama_Logic4_HitShield -> itKusudama_Logic4_Clanked.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        Self::clanked(item, context)
    }
    /// itKusudama_Logic4_Reflected -> itKusudama_Logic4_Clanked.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        Self::clanked(item, context)
    }
    /// itKusudama_Logic4_DmgReceived: unopened, the damage it has taken
    /// (xC9C, an s32 compared as a float) at x20 opens or blasts it.
    fn damage_received(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let threshold = Attributes(&context.assets.special_attributes).damage_threshold();
        if item.damage_percent as f32 >= threshold {
            open_unless_opened(item, context);
        }
        false
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

/// The angle normalised to (-pi, pi] and turned to the ball's side: the
/// head of it_802897C8 and it_80289910 (fsubs of tau, then fmuls by -1
/// when `facing_dir == -1.0f`).
fn sided_angle(item: &ItemCore, mut angle: f32) -> f32 {
    if angle > PI {
        angle -= TAU;
    }
    if item.facing == -1.0 {
        angle *= -1.0;
    }
    angle
}

/// it_8028A190_inline / it_80289910(M_PI_2): pitched a quarter turn toward
/// its side at once.
fn pitch_to_side(item: &mut ItemCore) {
    item.rotation.x = sided_angle(item, HALF_PI);
}

/// it_802897C8 (802897C8): the pitch eases toward `angle`: within ten
/// degrees it snaps, otherwise it moves five. Retail subtracts, takes the
/// absolute difference and adds or subtracts the step separately; no FMA.
fn ease_pitch(item: &mut ItemCore, angle: f32) {
    let mut angle = sided_angle(item, angle);
    let pitch = item.rotation.x;
    let mut distance = pitch - angle;
    if distance < 0.0 {
        distance = -distance;
    }
    if distance > PITCH_SNAP_DISTANCE {
        angle = if pitch < angle {
            pitch + PITCH_EASING_STEP
        } else {
            pitch - PITCH_EASING_STEP
        };
    }
    item.rotation.x = angle;
}

/// it_80289A00 (80289A00): the yaw steps back by x28 to a quarter turn on
/// its side.
fn ease_yaw(item: &mut ItemCore, assets: &ItemAssets) {
    let step = Attributes(&assets.special_attributes).yaw_step();
    let yaw = item.rotation.y;
    item.rotation.y = if item.facing == 1.0 {
        if yaw > HALF_PI + step {
            yaw - step
        } else {
            HALF_PI
        }
    } else if yaw < -HALF_PI - step {
        yaw + step
    } else {
        -HALF_PI
    };
}

/// it_8028A3CC (8028A3CC): not grabbable, the fall, pitched to its side.
fn start_falling(item: &mut ItemCore, assets: &ItemAssets) {
    item.grabbable = false;
    change(item, motion::FALL, ANIM_UPDATE, assets);
    pitch_to_side(item);
}

/// it_8028A190 (8028A190) and itKusudama_UnkMotion6_Coll_inline: still,
/// grabbable (it_8026B390), at rest. The pitch is written after the state
/// change's efAsync flush (`REST_PITCH`), so a landing spark queued before
/// it is placed by the pre-pitch root.
fn rest(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    item.grabbable = true;
    change(item, motion::REST, ANIM_UPDATE, assets);
    item.defer_callback(REST_PITCH);
}

/// The open-or-blast draw every opening takes, unless it opened already
/// (x0).
fn open_unless_opened(item: &mut ItemCore, context: &ItemEventContext<'_>) {
    if !container(item).opened {
        let assets = context.assets;
        crate::with_event_rng(context, |rng| open_or_blast(item, assets, rng));
    }
}

/// it_8028A114 (8028A114): HSD_Randi over all four weights; below the
/// first three's sum it rises (it_8028A544), otherwise it blasts
/// (it_8028AC74). The rise's sound (it_80289B50's arg) draws nothing.
fn open_or_blast(item: &mut ItemCore, assets: &ItemAssets, rng: &mut gekko_math::HsdRng) {
    let [first, second, third, blast_weight] =
        Attributes(&assets.special_attributes).open_weights();
    let two = first + second;
    let three = third + two;
    if rng.randi(blast_weight + three) < three {
        rise(item, assets);
    } else {
        blast(item, assets);
    }
}

/// it_8028A544 (8028A544): up at x18 for x1C frames (fctiwz), opened,
/// airborne (it_802762BC), hitting its owner and kin (it_80275444), shown
/// (it_8026BB20), not grabbable, state 3, its yaw turned to its side.
fn rise(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    item.velocity = Vec3::new(0.0, a.rise_speed(), 0.0);
    let state = container_mut(item);
    state.despawn_timer = fctiwz(a.rise_frames());
    state.opened = true;
    item.enter_air();
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.hidden = false;
    item.grabbable = false;
    change(item, motion::RISING, ANIM_UPDATE, assets);
    item.rotation.y = if item.facing == 1.0 {
        RISING_YAW
    } else {
        -RISING_YAW
    };
}

/// itKusudama_UnkMotion3_inline: still, hanging (x4 = 0x5A), not
/// grabbable, state 1 keeping its yaw. Its xDD1 b1 lets the stage's own
/// drift (grLib_801C9E60) carry it while airborne, which is zero on every
/// stage the port runs.
fn hang(item: &mut ItemCore, assets: &ItemAssets) {
    item.velocity = Vec3::ZERO;
    container_mut(item).despawn_timer = HANGING_FRAMES;
    item.grabbable = false;
    let yaw = item.rotation.y;
    change(item, motion::HANGING, ANIM_UPDATE, assets);
    item.rotation.y = yaw;
}

/// it_8028AC74 (8028AC74): the blast. The model hides (it_80272A3C), the
/// blast effect and its hitlag's end (it_80272C08), no hits taken
/// (it_802756D0), not grabbable, the explosion lifetime without a destroy
/// effect (it_8027518C), stopped, opened, hitting its owner and kin
/// (it_80275444), state 7. Unlike a crate's it raises no gust.
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    item.hidden = true;
    item.events.push(ItemEvent::Effect {
        id: crate::EXPLOSION_EFFECT,
        position: item.position,
    });
    item.hitlag_enabled = false;
    item.hurt_intangible = true;
    item.grabbable = false;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    let state = container_mut(item);
    state.opened = true;
    state.despawn_timer = BLAST_FRAMES;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    change(item, motion::BLAST, ANIM_UPDATE, assets);
}

/// it_8028AD44 (8028AD44): the sound, no hits taken, still, no destroy
/// effect, open for 0x55 frames, not grabbable, state 8.
fn open(item: &mut ItemCore, assets: &ItemAssets) {
    item.hurt_intangible = true;
    item.velocity.x = 0.0;
    item.velocity.y = 0.0;
    item.destroy_effect_suppressed = true;
    let state = container_mut(item);
    state.opened = true;
    state.despawn_timer = OPEN_FRAMES;
    item.grabbable = false;
    change(item, motion::OPEN, ANIM_UPDATE, assets);
}

/// itKusudama_UnkMotion0_Anim / 2: the pitch eases to its side.
fn rest_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    ease_pitch(item, HALF_PI);
    false
}

/// itKusudama_UnkMotion4_Anim / 6.
fn still_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}

/// itKusudama_UnkMotion1_Anim: the script's xDB4 at 1 draws which sound
/// plays (HSD_Randi(2)); its xDB0 opens it (it_8028AD44); the pitch eases
/// level and the yaw back to its side.
fn hanging_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[2] == 1 {
        item.command_variables[2] = 2;
        let cell = ctx.rng.expect("itKusudama_UnkMotion1_Anim draws");
        let mut rng = cell.get();
        rng.randi(2);
        cell.set(rng);
    }
    if item.command_variables[1] != 0 {
        open(item, ctx.assets);
    }
    ease_pitch(item, 0.0);
    ease_yaw(item, ctx.assets);
    false
}

/// itKusudama_UnkMotion3_Anim: the rise's frames; at their end it hangs.
fn rising_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = container_mut(item);
    state.despawn_timer -= 1;
    if state.despawn_timer <= 0 {
        hang(item, ctx.assets);
    }
    ease_pitch(item, 0.0);
    ease_yaw(item, ctx.assets);
    false
}

/// itKusudama_UnkMotion8_Anim: the script's xDAC lets the shower out
/// (it_80289BE8 with x4, x0, x8), then its two sparkles; gone when x4 runs
/// out.
fn open_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] != 0 {
        item.command_variables[0] = 0;
        let a = Attributes(&ctx.assets.special_attributes);
        let ecb = item.collision.as_ref().expect("a Party Ball's collision").ecb;
        // it_8026BB88: 0.5 * (top + bottom), then added to the position.
        let rise = 0.5 * (ecb.top.y + ecb.bottom.y);
        item.events.push(ItemEvent::PartyBallContents {
            weights: [a.int(1), a.int(0), a.int(2)],
            kind: a.int(4),
            count: a.int(5),
            center: Vec3::new(item.position.x, item.position.y + rise, item.position.z),
            position: item.position,
        });
        for bone in SPARKLE_BONES {
            item.events.push(ItemEvent::BoneGenerator { id: SPARKLE, bone });
        }
    }
    let state = container_mut(item);
    state.despawn_timer -= 1;
    state.despawn_timer <= 0
}

/// itKusudama_UnkMotion0_Coll: it_8026D62C, off the floor the fall
/// (it_8028A3CC).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        start_falling(item, ctx.assets);
    }
    false
}

/// itKusudama_UnkMotion1_Coll: it_8026DA08.
fn hanging_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_pass(ctx.map);
    false
}

/// itKusudama_UnkMotion2_Phys -> it_80272860.
fn fall_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// Item_ApplyFallingPhysics: gravity and the falling spin.
fn tumble_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    fall_physics(item, ctx);
    item.update_spin(ctx.assets.fall_spin_degrees);
}

/// itKusudama_UnkMotion2_Coll -> it_8026E15C with it_8028A190: a settled
/// landing rests.
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne => false,
        AirLanding::Landed => {
            rest(item, ctx.assets);
            false
        }
        AirLanding::Sliding => unimplemented!("it_8026E15C: a sliding Party Ball, unwitnessed"),
        AirLanding::Broken => true,
    }
}

/// itKusudama_UnkMotion3_Coll: it_8026DA08; a ceiling (env 0x6000) ends
/// the rise.
fn rising_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_pass(ctx.map);
    if item.touches_ceiling() {
        hang(item, ctx.assets);
    }
    false
}

/// itKusudama_UnkMotion5_Coll: thrown, a floor (it_8026DA08) or a wall
/// (it_80276308) opens or blasts it, with the opening's sound; a ceiling
/// (it_802763E0) stops it rising.
fn thrown_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let assets = ctx.assets;
    let opens = if item.air_pass(ctx.map) {
        true
    } else if item.ceiling_bits() != 0 {
        if item.velocity.y > 0.0 {
            item.velocity.y = 0.0;
        }
        false
    } else {
        item.wall_bits() != 0
    };
    if opens {
        let cell = ctx.rng.expect("it_8028A114 draws");
        let mut rng = cell.get();
        open_or_blast(item, assets, &mut rng);
        cell.set(rng);
    }
    false
}

/// itKusudama_UnkMotion6_Coll: dropped onto a floor faster than x2C it
/// opens or blasts; slower it lands (it_802762B0) and rests, its pitch set
/// to its side at once (it_80289910).
fn dropped_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_pass(ctx.map) {
        let mut speed = item.velocity.y;
        if speed < 0.0 {
            speed = -speed;
        }
        if speed > Attributes(&ctx.assets.special_attributes).break_speed() {
            let cell = ctx.rng.expect("it_8028A114 draws");
            let mut rng = cell.get();
            open_or_blast(item, ctx.assets, &mut rng);
            cell.set(rng);
        } else {
            item.land_on_floor();
            rest(item, ctx.assets);
        }
    }
    false
}
