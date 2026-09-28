//! Samus's missile (It_Kind_Samus_Missile), itsamusmissile.c
//! (802B62D0..802B7160). Side special fires it from Samus's arm cannon
//! (it_802B62D0): a homing missile turns toward the nearest foe in front
//! while its homing lasts, then slows; a super missile flies straight and
//! accelerates. Either explodes on its lifetime's end, on any map contact,
//! on a hit, or once Samus has fired two newer missiles.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_lb::{shield::angle_xy, vector};
use melee_types::ItemKind;

pub struct SamusMissile;

/// it_803F7340's anim_id column: homing flight, super flight, and their
/// explosions.
pub const ARTICLE_STATES: [i32; 4] = [0, 1, 2, 3];
/// itSamusMissileAttributes: sixteen words, fourteen read.
pub const SPECIAL_ATTRIBUTES: u32 = 16;
/// ftData.x48_items index (ftSs_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;

mod motion {
    pub const HOMING: u16 = 0;
    pub const SUPER: u16 = 1;
    pub const HOMING_EXPLODE: u16 = 2;
    pub const SUPER_EXPLODE: u16 = 3;
}

/// efSync_Spawn(0x485 / 0x484, item, grandchild): efAlt's trail generator
/// (hsd_8039EFAC(0, 2, 0x7DE / 0x7DB, jobj)).
pub const HOMING_TRAIL: u16 = 0x485;
pub const SUPER_TRAIL: u16 = 0x484;
/// The model's child (the homing turn's joint) and grandchild (the trail's),
/// in depth-first order.
const CHILD_JOINT: usize = 1;
const GRANDCHILD_JOINT: usize = 2;
/// it_80272B40 / it_80272A60: the explosion effect at the item and
/// Item_8026AE84(item, 0x74, 0x7F, 0x40).
const HOMING_EXPLOSION: u16 = 0x40C;
const SUPER_EXPLOSION: u16 = 0x40E;
const EXPLOSION_SOUND: u32 = 0x74;
/// it_802B64FC: the difference's y must pass this to turn.
const TURN_EPSILON: f32 = 0.001;

/// The launch word the spawner passes: the super flag and the owner's
/// missile count at launch (it_802B62D0's is_smash_missile and x4).
pub fn spawn_argument(smash: bool, launch_count: u32) -> i32 {
    ((launch_count as i32) << 1) | i32::from(smash)
}

/// itSamusMissileAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x4: the homing missile's lifetime.
    fn homing_lifetime(&self) -> f32 {
        self.0[1]
    }
    /// x8: how long it homes, from launch.
    fn homing_frames(&self) -> f32 {
        self.0[2]
    }
    /// xC: its launch speed.
    fn homing_speed(&self) -> f32 {
        self.0[3]
    }
    /// x10: the slowdown factor once homing ends.
    fn slowdown(&self) -> f32 {
        self.0[4]
    }
    /// x14: it slows only while faster than this.
    fn minimum_speed(&self) -> f32 {
        self.0[5]
    }
    /// x18: the turn per frame.
    fn turn_step(&self) -> f32 {
        self.0[6]
    }
    /// x1C: the largest turn.
    fn maximum_turn(&self) -> f32 {
        self.0[7]
    }
    /// x20: a target nearer the heading than this is not turned toward.
    fn turn_threshold(&self) -> f32 {
        self.0[8]
    }
    /// x24: the super missile's lifetime.
    fn super_lifetime(&self) -> f32 {
        self.0[9]
    }
    /// x28: frames before it accelerates.
    fn super_delay(&self) -> f32 {
        self.0[10]
    }
    /// x2C: its launch speed.
    fn super_speed(&self) -> f32 {
        self.0[11]
    }
    /// x30: its acceleration.
    fn super_acceleration(&self) -> f32 {
        self.0[12]
    }
    /// x34: its top speed.
    fn super_maximum(&self) -> f32 {
        self.0[13]
    }
}

static STATES: [ItemStateRow; 4] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: flight_animation,
        physics: homing_physics,
        collision: flight_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: flight_animation,
        physics: super_physics,
        collision: flight_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[3],
        animation: explosion_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

fn missile(item: &ItemCore) -> &MissileState {
    let ItemScratch::Missile(state) = &item.scratch else {
        panic!("missile scratch missing")
    };
    state
}
fn missile_mut(item: &mut ItemCore) -> &mut MissileState {
    let ItemScratch::Missile(state) = &mut item.scratch else {
        panic!("missile scratch missing")
    };
    state
}

/// it_3F2F.c's Logic52 row.
impl ItemLogic for SamusMissile {
    const KIND: ItemKind = ItemKind::SamusMissile;
    const STATES: &'static [ItemStateRow] = &STATES;
    const LOCKS_ON: bool = true;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802B62D0 (802B62D0) once Item_80268B18 returns: the super flag
    /// and the owner's missile count, then it_802B66A8 or it_802B6A60.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let smash = spawn.spawn_argument & 1 != 0;
        item.scratch = ItemScratch::Missile(MissileState {
            smash,
            launch_count: spawn.spawn_argument >> 1,
            turn: 0.0,
        });
        if smash {
            launch_super(item, assets, common.half_life_scale);
        } else {
            launch_homing(item, assets, common.half_life_scale);
        }
    }
    /// The trail generators ride the model's grandchild
    /// (itGetJObjGrandchild), below the child the homing turns about X.
    fn effect_joint_matrix(
        item: &ItemCore,
        assets: &ItemAssets,
        root: hsd_types::Mtx,
    ) -> hsd_types::Mtx {
        let pose = assets.pose.as_ref().expect("missile pose");
        pose.bone_matrix_turned(
            item.article_state,
            item.pose_steps,
            GRANDCHILD_JOINT,
            melee_it::pose::RootSrt {
                translate: Vec3::new(root.0[0][3], root.0[1][3], root.0[2][3]),
                rotate: item.rotation,
                scale: item.model_scale,
            },
            // Only the flights set the child's X rotation; an explosion's
            // state change restored the rest pose (Item_80268D34).
            matches!(item.motion, motion::HOMING | motion::SUPER)
                .then_some((CHILD_JOINT, missile(item).turn)),
        )
    }
    /// it_2725_Logic52_DmgDealt (802B6C1C): the flight explodes.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// it_2725_Logic52_Clanked.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// it_2725_Logic52_HitShield.
    fn hit_shield(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        explode_unless_exploding(item, context.assets);
        false
    }
    /// it_2725_Logic52_ShieldBounced: a super missile bounces off
    /// (itColl_BounceOffShield); a homing one does nothing.
    fn shield_bounced(item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        if missile(item).smash {
            unimplemented!("it_2725_Logic52_ShieldBounced: itColl_BounceOffShield (super missile)");
        }
        false
    }
    /// it_2725_Logic52_Reflected (802B6E58): the owner's count no longer
    /// applies (xDB8), it_80273030 turns it back, a super missile restarts
    /// at launch speed, a falling one rises, the model faces the new way
    /// and the trail starts again.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        item.command_variables[3] = 1;
        item.reverse_on_reflect(context.reflected_speed);
        let a = Attributes(&context.assets.special_attributes);
        let smash = missile(item).smash;
        if smash {
            item.velocity.x = a.super_speed() * item.facing;
        }
        if item.velocity.y < 0.0 {
            item.velocity.y = -item.velocity.y * context.reflected_speed;
        }
        // HSD_JObjSetRotationY(jobj, (float) M_PI_2 * facing): fmuls.
        item.rotation.y = std::f32::consts::FRAC_PI_2 * item.facing;
        item.events.push(ItemEvent::DestroyEffects);
        item.events.push(ItemEvent::OwnEffect {
            id: if smash { SUPER_TRAIL } else { HOMING_TRAIL },
        });
        false
    }
}

/// it_802B66A8 (802B66A8): launch speed along the facing, the homing
/// lifetime, no turn, not pickable, state 0 and its trail.
fn launch_homing(item: &mut ItemCore, assets: &ItemAssets, half_life_scale: f32) {
    let a = Attributes(&assets.special_attributes);
    item.velocity.x = a.homing_speed() * item.facing;
    item.velocity.y = 0.0;
    item.command_variables[0] = 0;
    item.command_variables[2] = 0;
    item.command_variables[3] = 0;
    set_lifetime(item, a.homing_lifetime(), half_life_scale);
    missile_mut(item).turn = 0.0;
    item.grabbable = false;
    item.change_motion_with(motion::HOMING, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    item.events.push(ItemEvent::OwnEffect { id: HOMING_TRAIL });
}

/// it_802B6A60 (802B6A60): the super missile's launch.
fn launch_super(item: &mut ItemCore, assets: &ItemAssets, half_life_scale: f32) {
    let a = Attributes(&assets.special_attributes);
    item.velocity.x = a.super_speed() * item.facing;
    item.velocity.y = 0.0;
    item.command_variables[1] = 0;
    item.command_variables[2] = 0;
    item.command_variables[3] = 0;
    set_lifetime(item, a.super_lifetime(), half_life_scale);
    item.grabbable = false;
    item.change_motion_with(motion::SUPER, ARTICLE_STATES[1], ANIM_UPDATE, assets);
    item.events.push(ItemEvent::OwnEffect { id: SUPER_TRAIL });
}

/// it_80275158: the lifetime and its half (it_804D6D28->x4C).
fn set_lifetime(item: &mut ItemCore, frames: f32, half_life_scale: f32) {
    item.life_timer = frames;
    item.half_life = frames * half_life_scale;
}

/// it_802B63F8 (802B63F8): homing lasts while the lifetime is above x4 - x8,
/// whose last frame ends a homing missile's trail; a super missile
/// accelerates from x24 - x28 on; at zero the missile explodes, before
/// that the lifetime counts down.
fn count_down(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    let homing_end = a.homing_lifetime() - a.homing_frames();
    item.command_variables[0] = u32::from(item.life_timer > homing_end);
    if item.life_timer == homing_end && !missile(item).smash {
        item.events.push(ItemEvent::DestroyEffects);
    }
    if item.life_timer <= a.super_lifetime() - a.super_delay() {
        item.command_variables[1] = 1;
    }
    if item.life_timer <= 0.0 {
        explode(item, assets);
    } else {
        item.life_timer -= 1.0;
    }
}

/// itSamusmissile_UnkMotion0_Anim / UnkMotion1_Anim: the countdown, then
/// unless reflected (xDB8), two newer missiles from the owner explode this
/// one (the unsigned difference of u.ss.x2238 and x4).
fn flight_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    count_down(item, ctx.assets);
    if item.command_variables[3] == 0 {
        if let Some(owner) = ctx.owner {
            let launch = missile(item).launch_count as u32;
            if owner.articles_fired.wrapping_sub(launch) >= 2 {
                explode_unless_exploding(item, ctx.assets);
            }
        }
    }
    false
}

/// it_802B64FC (802B64FC): the nearest fighter in front (ftLib_80086368
/// through it_8026B634), else the nearest lock-on item (it_8026C258); a
/// target off the heading by at least x20 turns the missile x18 toward it,
/// up to x1C either way.
fn steer(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let Some(target) = nearest_target(item, ctx) else {
        return;
    };
    if target.x == 0.0 && target.y == 0.0 {
        return;
    }
    let heading = vector::normalize_xy(Vec3::new(item.velocity.x, item.velocity.y, 0.0));
    let toward = vector::normalize_xy(Vec3::new(
        target.x - item.position.x,
        target.y - item.position.y,
        0.0,
    ));
    if angle_xy(heading, toward) < a.turn_threshold() {
        return;
    }
    let difference = vector::difference(toward, heading);
    let state = missile_mut(item);
    if difference.y > TURN_EPSILON {
        state.turn -= a.turn_step();
    } else if difference.y < TURN_EPSILON {
        state.turn += a.turn_step();
    }
    let maximum = a.maximum_turn();
    let magnitude = if state.turn < 0.0 {
        -state.turn
    } else {
        state.turn
    };
    if magnitude > maximum {
        state.turn = if state.turn > 0.0 { maximum } else { -maximum };
    }
}

/// ftLib_80086368's fighter, then it_8026C258's item: the squared distance
/// (fmuls, fadds) is least among candidates not behind the facing.
fn nearest_target(item: &ItemCore, ctx: &ItemPhysicsContext<'_>) -> Option<Vec3> {
    let position = item.position;
    let facing = item.facing;
    let behind = |x: f32| (facing == -1.0 && x > position.x) || (facing == 1.0 && x < position.x);
    let mut best = None;
    let mut least = f32::MAX;
    for fighter in ctx.targets.fighters.iter() {
        if Some(fighter.player) == item.owner || fighter.disabled || behind(fighter.position.x) {
            continue;
        }
        let dx = position.x - fighter.position.x;
        let dy = position.y - fighter.position.y;
        let distance = dx * dx + dy * dy;
        if distance < least {
            least = distance;
            best = Some(fighter.position);
        }
    }
    if best.is_some() {
        return best;
    }
    let mut least = f32::MAX;
    for candidate in ctx.targets.items.iter() {
        if behind(candidate.position.x) {
            continue;
        }
        let dx = position.x - candidate.position.x;
        let dy = position.y - candidate.position.y;
        let distance = dx * dx + dy * dy;
        if distance < least {
            least = distance;
            best = Some(candidate.position);
        }
    }
    best
}

/// itSamusmissile_UnkMotion0_Phys (802B67E4): while homing, steer and fly
/// at the launch speed turned by x8 about Z (lbVector_Rotate with
/// x8 * -facing); after, slow by x10 while faster than x14 (unfused squares,
/// 802B6930..40). The model's child turns about X by x8.
fn homing_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    if item.command_variables[0] != 0 {
        steer(item, ctx);
        let turn = missile(item).turn;
        let heading = vector::rotate_about(
            Vec3::new(a.homing_speed() * item.facing, 0.0, 0.0),
            vector::Axis::Z,
            turn * -item.facing,
        );
        item.velocity.x = heading.x;
        item.velocity.y = heading.y;
    } else {
        let (x, y) = (item.velocity.x, item.velocity.y);
        let minimum = a.minimum_speed();
        if x * x + y * y > minimum * minimum {
            item.velocity.x *= a.slowdown();
            item.velocity.y *= a.slowdown();
        }
    }
    // HSD_JObjSetRotationX(child, x8): the model's child turns with the
    // heading (MissileState::turn); only effects on its grandchild read it.
}

/// itSamusmissile_UnkMotion1_Phys (802B6B84): once accelerating, x30 along
/// the facing (802B6BB0: fmadds) up to x34.
fn super_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if item.command_variables[1] == 0 {
        return;
    }
    let a = Attributes(&ctx.assets.special_attributes);
    item.velocity.x = gekko_math::fma::fmadds(a.super_acceleration(), item.facing, item.velocity.x);
    let speed = if item.velocity.x < 0.0 {
        -item.velocity.x
    } else {
        item.velocity.x
    };
    let maximum = a.super_maximum();
    if speed > maximum {
        item.velocity.x = if item.facing == 1.0 {
            maximum
        } else {
            -maximum
        };
    }
}

/// itSamusmissile_UnkMotion0_Coll / UnkMotion1_Coll: it_8026E71C, whose
/// callback explodes the missile on any contact.
fn flight_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if item.air_contact_any(ctx.map, ctx.assets) {
        explode(item, ctx.assets);
    }
    false
}

/// it_802751D8: the explosion lasts its lifetime.
fn explosion_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// The event callbacks' guard: a flight explodes (not an explosion).
fn explode_unless_exploding(item: &mut ItemCore, assets: &ItemAssets) {
    if !matches!(item.motion, motion::HOMING_EXPLODE | motion::SUPER_EXPLODE) {
        explode(item, assets);
    }
}

/// it_802B701C / it_802B70A0 (802B701C / 802B70A0): the model hides
/// (it_8026BB44), the common explosion lifetime with no destroy effect
/// (it_8027518C), no velocity (it_80273454), the trail goes
/// (efLib_DestroyAll), then it_80272B40 / it_80272A60's effect, sound and
/// no more hitlag, and the explosion state whose script owns the blast.
fn explode(item: &mut ItemCore, assets: &ItemAssets) {
    let smash = missile(item).smash;
    item.hidden = true;
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.velocity = Vec3::ZERO;
    item.events.push(ItemEvent::DestroyEffects);
    item.events.push(ItemEvent::Effect {
        id: if smash {
            SUPER_EXPLOSION
        } else {
            HOMING_EXPLOSION
        },
        position: item.position,
    });
    item.sound_requests.push(EXPLOSION_SOUND);
    item.hitlag_enabled = false;
    let state = if smash {
        motion::SUPER_EXPLODE
    } else {
        motion::HOMING_EXPLODE
    };
    item.change_motion_with(state, ARTICLE_STATES[state as usize], ANIM_UPDATE, assets);
}
