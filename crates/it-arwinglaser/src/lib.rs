//! The Star Fox stages' ship lasers (It_Kind_Arwing_Laser),
//! it/kinds/itarwinglaser.c (802E70BC..802E8790).
//!
//! A far Arwing or Wolfen (grVenom_80205F30, grCorneria_801DED50) fires
//! [`it_802E7654`](launch) at a fighter: the laser starts at the ship's gun,
//! flies straight at the fighter's position (in depth too) and only touches
//! the stage near the stage's plane. Types 1 (an Arwing's) and 3 (a
//! Wolfen's) are the aimed lasers; on Corneria they drift with the Great
//! Fox (grCorneria_801DDCF0).
//!
//! A near ship on Corneria fires its level cannon
//! ([`it_802E72E0`](launch_cannon)): the shot sits at the gun until its
//! animation ends, then flies along the ship's facing in the stage's plane
//! and ends at the first line its tip crosses. An Arwing's (type 0) is
//! drawn hidden, with two side beams (types 4 and 5) at the gun's depth on
//! either side that ride it; a Wolfen's (type 2) has none.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct ArwingLaser;

/// it_803F8DE8's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [0, 1, 2, 3, 4, 5];
/// ArwingLaserAttr: x0 (the ItemAttr pointer), x4, x8.
pub const SPECIAL_ATTRIBUTES: u32 = 3;
/// it_802E7654's it_80275158(gobj, 400.0f).
const LIFETIME: f32 = 400.0;
/// it_802E72E0's it_80275158(gobj, 300.0f).
const CANNON_LIFETIME: f32 = 300.0;
/// The tip, xBBC_dynamicBoneTable->bones[4].
pub const TIP_BONE: usize = 4;
/// it_802E838C / the collision pop: efSync_Spawn(0x40E) and
/// Item_8026AE84(ip, 0x74, ..).
const POP_EFFECT: u16 = 0x40E;
const POP_SOUND: u32 = 0x74;
/// itArwinglaser_UnkMotion3_Coll's first sound near the plane: a Wolfen's
/// (type 3 collision joint) and an Arwing's.
const WOLFEN_SOUND: u32 = 0x5573B;
const ARWING_SOUND: u32 = 0x6B6CA;
/// The depth band in which the laser meets the stage (|z| < 30).
const PLANE_BAND: f32 = 30.0;
/// it_803F8E48: it_802E70BC's four probes around the tip.
const PROBES: [(f32, f32); 4] = [(0.0, 200.0), (0.0, -200.0), (-200.0, 0.0), (200.0, 0.0)];
/// mpLineGetFlags 0x100: a line the probes ignore.
const PASS_THROUGH_LINE: u32 = 0x100;
/// The level cannons: an Arwing's (with side beams) and a Wolfen's.
pub const ARWING_CANNON: i32 = 0;
pub const WOLFEN_CANNON: i32 = 2;
/// An Arwing cannon's side beams: in front of the plane and behind it.
pub const BEAMS: [i32; 2] = [4, 5];

/// ArwingLaserAttr, from the special attribute words. x0 points at an
/// ItemAttr whose x4_throw_speed_mul the scene resolves at load into word 0.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// `attrs->x0->x4_throw_speed_mul`: the level cannons' speed, and the
    /// aimed lasers' speed's scale.
    fn speed_scale(&self) -> f32 {
        self.0[0]
    }
    /// x4: the aimed lasers' speed.
    fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8: the aimed lasers' thickness (the y and z scale).
    fn thickness(&self) -> f32 {
        self.0[2]
    }
}

const AIMED: ItemStateRow = ItemStateRow {
    animation_id: 1,
    animation: aimed_animation,
    physics: aimed_physics,
    collision: aimed_collision,
};
fn no_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {}
const CANNON: ItemStateRow = ItemStateRow {
    animation_id: 0,
    animation: cannon_animation,
    physics: cannon_physics,
    collision: cannon_collision,
};
const BEAM: ItemStateRow = ItemStateRow {
    animation_id: 4,
    animation: no_animation,
    physics: no_physics,
    collision: beam_collision,
};
static STATES: [ItemStateRow; 6] = {
    let mut rows = [CANNON, AIMED, CANNON, AIMED, BEAM, BEAM];
    let mut i = 0;
    while i < 6 {
        rows[i].animation_id = ARTICLE_STATES[i];
        i += 1;
    }
    rows
};

impl ItemLogic for ArwingLaser {
    const KIND: ItemKind = ItemKind::ArwingLaser;
    const STATES: &'static [ItemStateRow] = &STATES;

    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802E838C (dmg_dealt): it pops at its tip.
    fn damage_dealt(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        pop_at_tip(item, context.assets);
        true
    }
    /// it_802E838C (clanked): as a hit it deals.
    fn clanked(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        pop_at_tip(item, context.assets);
        true
    }
    /// it_802E8418 (absorbed). Not witnessed.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itarwinglaser.c:559: it_802E8418, an absorber takes a ship's laser")
    }
    /// it_802E8784 (802E8784, hit_shield): a shield stops the laser.
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_802E8420 (802E8420, reflected): turned; an aimed laser moving
    /// against the reflector's facing (ftLib_800865C0) turns its facing
    /// while its course stays. A level cannon would move to its tip and
    /// fly back; no witness has reflected one.
    fn reflected(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        let laser = laser(item);
        laser.turned = true;
        if matches!(laser.kind, ARWING_CANNON | WOLFEN_CANNON) {
            unimplemented!("itarwinglaser.c:576: it_802E8420, a reflector turns a level cannon");
        }
        if item.velocity.x * context.reflected_facing < 0.0 {
            item.facing = -item.facing;
        }
        false
    }
    /// it_802E85F4 (802E85F4, shield_bounced): turned; the velocity
    /// mirrored off the shield (lbVector_Mirror on xC58), the laser moved
    /// to its tip, and a level cannon faces its new course (an aimed
    /// laser's next physics step restores its course).
    fn shield_bounced(item: &mut ItemCore, context: &ItemEventContext<'_>) -> bool {
        laser(item).turned = true;
        item.velocity = melee_lb::vector::mirror(item.velocity, context.shield_normal);
        let position = tip(item, context.assets);
        item.position = position;
        let laser = laser(item);
        laser.previous_tip = position;
        laser.tip = position;
        if matches!(laser.kind, ARWING_CANNON | WOLFEN_CANNON) {
            item.facing = if item.velocity.x < 0.0 { -1.0 } else { 1.0 };
            item.rotation.y = facing_angle(item.facing);
            item.joints_dirty = true;
        }
        false
    }
}

/// `(f32) (1.5707963267948966 * (f64) facing)`: the double product rounded
/// once (fmul, frsp).
fn facing_angle(facing: f32) -> f32 {
    (core::f64::consts::FRAC_PI_2 * f64::from(facing)) as f32
}

/// Ground_801C0800 -> it_8026B40C: GrVe.dat's (or GrCn.dat's) laser
/// article, with ArwingLaserAttr x0 (an ItemAttr of its own, not the
/// article's) resolved to its x4_throw_speed_mul in special word 0, and
/// the pose the tip reads. `None` when the stage has no laser.
pub fn load(
    archive: &hsd_archive::Archive,
) -> Result<Option<ItemAssets>, String> {
    let Some(mut assets) = ItemAssets::from_stage_item(
        archive,
        ItemKind::ArwingLaser,
        &ARTICLE_STATES,
        SPECIAL_ATTRIBUTES,
        &[],
    )
    .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    assets.special_attributes[0] = speed_scale(archive).map_err(|e| e.to_string())?;
    assets
        .read_pose(archive)
        .map_err(|e| format!("Arwing laser pose: {e}"))?;
    Ok(Some(assets))
}

/// ArwingLaserAttr x0's x4_throw_speed_mul, through the stage's itemdata.
fn speed_scale(archive: &hsd_archive::Archive) -> hsd_archive::desc::Result<f32> {
    let r = archive.reader();
    let table = archive.public("itemdata").expect("the stage's itemdata");
    let mut index = 0;
    let article = loop {
        let entry = r.u32(table + index * 4)?;
        if r.u32(entry)? == ItemKind::ArwingLaser as u32 {
            break r.u32(entry + 4)?;
        }
        index += 1;
    };
    let special = r.u32(article + 4)?;
    Ok(r.f32(r.u32(special)? + 4)?)
}

/// What the ship's proc hands it_802E7654.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    /// The gun's world position (lb_8000B1CC on the ship's bone).
    pub gun: Vec3,
    /// The fighter's position, 5 above its feet.
    pub target: Vec3,
    /// 1 an Arwing's laser, 3 a Wolfen's.
    pub kind: i32,
    /// it_802E7654's `arg4`: the collision joint the line tests keep to.
    pub joint: i32,
    /// yakumono_param x34 (Venom) or x70 (Corneria), and Ground_801C0498.
    pub scale: f32,
    pub map_scale: f32,
    /// grCorneria_801DDCF0 as the shot leaves (zero on Venom).
    pub stage_drift: Vec3,
}

/// What a near ship's proc hands it_802E72E0 (Corneria's level cannon).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CannonLaunch {
    /// The gun bone's world position.
    pub gun: Vec3,
    /// 0 an Arwing's cannon (with its side beams 4 and 5), 2 a Wolfen's.
    pub kind: i32,
    /// yakumono_param x70, and Ground_801C0498.
    pub scale: f32,
    pub map_scale: f32,
}

fn spawn_item(position: Vec3, facing: f32) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind: ItemKind::ArwingLaser,
        // Item_802674AC: kinds from It_Kind_Arwing_Laser on.
        hold_kind: 5,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: false,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// it_802E7654's SpawnItem: at the gun, facing 0, unowned and motionless.
pub fn spawn(gun: Vec3) -> SpawnItem {
    spawn_item(gun, 0.0)
}

/// it_802E72E0's SpawnItem for laser type `kind`: at the gun flattened
/// onto the stage's plane (a cannon) or at the gun's depth in front of it
/// (beam 4) or behind it (beam 5), facing left (the ship's -1.0).
pub fn spawn_cannon(gun: Vec3, kind: i32) -> SpawnItem {
    let mut position = gun;
    let depth = gekko_math::msl::fabsf(gun.z);
    position.z = match kind {
        ARWING_CANNON | WOLFEN_CANNON => 0.0,
        4 => depth,
        5 => -depth,
        _ => unreachable!("it_802E72E0: laser type {kind}"),
    };
    spawn_item(position, -1.0)
}

fn state(item: &ItemCore, map_scale: f32, scale: f32, kind: i32) -> ArwingLaserState {
    ArwingLaserState {
        map_scale,
        scale,
        direction: Vec3::ZERO,
        tip: item.position,
        previous_tip: item.position,
        launched: false,
        kind,
        turned: false,
        joint: 0,
        near_ticks: 0,
        gun: Vec3::ZERO,
        parent: None,
        beams: [None; 2],
        beam_depth: 0.0,
        parent_pose: None,
        stage_drift: Vec3::ZERO,
    }
}

/// it_802E7654 (802E7654) after Item_80268B18: the lifetime, the scratch,
/// the course toward the target (lbVector_Normalize), the facing and the
/// model's turn toward it, then it_802E7A4C (not grabbable, the laser's
/// state). The scene then runs it_802750F8 and clears xDCC b3 (the item
/// never leaves through a blast zone).
pub fn launch(item: &mut ItemCore, launch: &Launch, assets: &ItemAssets, half_life_scale: f32) {
    item.life_timer = LIFETIME;
    item.half_life = LIFETIME * half_life_scale;
    let gun = launch.gun;
    let direction = melee_lb::vector::normalize(Vec3::new(
        launch.target.x - gun.x,
        launch.target.y - gun.y,
        launch.target.z - gun.z,
    ));
    let mut laser = state(item, launch.map_scale, launch.scale, launch.kind);
    laser.tip = gun;
    laser.previous_tip = gun;
    laser.direction = direction;
    laser.joint = launch.joint;
    laser.stage_drift = launch.stage_drift;
    item.scratch = ItemScratch::ArwingLaser(laser);
    item.facing = if direction.x >= 0.0 { 1.0 } else { -1.0 };
    item.rotation.x = -melee_lb::trigf::atan2f(direction.y, direction.z);
    item.rotation.y = melee_lb::trigf::atan2f(direction.x, direction.z);
    item.rotation.z = 0.0;
    item.joints_dirty = true;
    let state = launch.kind;
    assert!(
        matches!(state, 1 | 3),
        "it_802E7654: laser type {state} (a level cannon is it_802E72E0)"
    );
    enter_state(item, state, assets);
}

/// it_802E7A4C (802E7A4C): not grabbable (it_8026B3A8), then the laser's
/// type as its state (Item_80268E5C with ITEM_ANIM_UPDATE).
fn enter_state(item: &mut ItemCore, state: i32, assets: &ItemAssets) {
    item.grabbable = false;
    item.change_motion_with(state as u16, ARTICLE_STATES[state as usize], ANIM_UPDATE, assets);
}

/// it_802E72E0 (802E72E0) after Item_80268B18, for a level cannon or a
/// side beam: the lifetime (300), the scratch (the tip at the spawn point,
/// not yet out), the model turned to the facing, then it_802E7A4C. A beam
/// keeps the depth it rides at; an Arwing cannon's beams are made next
/// ([`attach_beams`]).
pub fn launch_cannon(
    item: &mut ItemCore,
    kind: i32,
    gun: Vec3,
    scale: f32,
    map_scale: f32,
    assets: &ItemAssets,
    half_life_scale: f32,
) {
    item.life_timer = CANNON_LIFETIME;
    item.half_life = CANNON_LIFETIME * half_life_scale;
    let mut laser = state(item, map_scale, scale, kind);
    laser.gun = gun;
    if BEAMS.contains(&kind) {
        let depth = gekko_math::msl::fabsf(item.position.z);
        laser.beam_depth = if kind == 4 { depth } else { -depth };
    }
    item.scratch = ItemScratch::ArwingLaser(laser);
    item.rotation.y = facing_angle(item.facing);
    item.joints_dirty = true;
    enter_state(item, kind, assets);
}

/// An Arwing cannon's beams (xE40, xE44) and each beam's cannon (xDF4).
pub fn attach_beams(cannon: &mut ItemCore, beams: [u32; 2]) {
    laser(cannon).beams = beams.map(Some);
}
pub fn attach_parent(beam: &mut ItemCore, cannon: u32) {
    laser(beam).parent = Some(cannon);
}

/// The scratch of a laser.
pub fn laser_state(item: &ItemCore) -> &ArwingLaserState {
    match &item.scratch {
        ItemScratch::ArwingLaser(laser) => laser,
        _ => unreachable!("an Arwing laser's scratch"),
    }
}

/// A level cannon still at its gun: where the gun is now.
pub fn follow_gun(item: &mut ItemCore, gun: Vec3) {
    laser(item).gun = gun;
}

/// grCorneria_801DDCF0 this tick, for an aimed laser.
pub fn follow_stage(item: &mut ItemCore, drift: Vec3) {
    laser(item).stage_drift = drift;
}

/// The scene's sample, before a side beam's collision proc, of its cannon
/// as itArwinglaser_UnkMotion5_Coll reads it (`None` once the cannon is
/// gone or no longer holds the beam).
pub fn sample_parent(beam: &mut ItemCore, pose: Option<(Vec3, Vec3, Vec3)>) {
    laser(beam).parent_pose = pose;
}

fn laser(item: &mut ItemCore) -> &mut ArwingLaserState {
    match &mut item.scratch {
        ItemScratch::ArwingLaser(laser) => laser,
        _ => unreachable!("an Arwing laser's scratch"),
    }
}

/// The tip's world position: lb_8000B1CC on dynamic bone 4, the model's
/// root at the JObj's translation (set by the last collision proc).
fn tip(item: &ItemCore, assets: &ItemAssets) -> Vec3 {
    let root = melee_it::pose::RootSrt {
        translate: item.root_translation,
        rotate: item.rotation,
        scale: item.model_scale,
    };
    assets
        .pose
        .as_ref()
        .expect("the laser's pose")
        .bone_position(item.article_state, item.pose_steps, TIP_BONE, root)
}

/// it_802E838C (802E838C): the tip found again, the pop there.
fn pop_at_tip(item: &mut ItemCore, assets: &ItemAssets) {
    let position = tip(item, assets);
    laser(item).tip = position;
    item.events.push(ItemEvent::Effect {
        id: POP_EFFECT,
        position,
    });
    item.sound_requests.push(POP_SOUND);
}

/// itArwinglaser_UnkMotion2_Anim (802E7AC4): the tip a tick ago; once the
/// animation has played out (it_80272C6C) the shot is out; it_80273130's
/// lifetime.
fn cannon_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let out = item.article_animation_ended(ctx.assets);
    let laser = laser(item);
    laser.previous_tip = laser.tip;
    if out {
        laser.launched = true;
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itArwinglaser_UnkMotion2_Phys (802E7D84): out, the shot flies at
/// `x0->x4_throw_speed_mul` along its facing; until then it sits at the
/// gun on the stage's plane. Its scale is xDFC * xE34 on every axis.
fn cannon_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let laser = *laser(item);
    if laser.launched {
        item.velocity = Vec3::new(a.speed_scale() * item.facing, 0.0, 0.0);
    } else {
        item.position = Vec3::new(laser.gun.x, laser.gun.y, 0.0);
        item.velocity = Vec3::ZERO;
    }
    let s = laser.map_scale * laser.scale;
    item.model_scale = Vec3::new(s, s, s);
    item.joints_dirty = true;
}

/// itArwinglaser_UnkMotion2_Coll (802E7F48): the tip found; the segment
/// from last tick's tip to it (both flattened) meeting any line
/// (it_8026EA20) ends the shot there: the line's speed carries it
/// (mpGetSpeed), it moves to the crossing, and pops at the tip.
fn cannon_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let position = tip(item, ctx.assets);
    let laser = laser(item);
    laser.tip = position;
    let previous = laser.previous_tip;
    let Some(hit) = ctx
        .map
        .check_all_remap(-1, -1, previous.x, previous.y, position.x, position.y)
    else {
        return false;
    };
    if let Some(speed) = ctx.map.line_speed(hit.line_id, &item.position) {
        item.velocity = speed;
    }
    item.position.x += hit.pos.x - position.x;
    item.events.push(ItemEvent::Effect {
        id: POP_EFFECT,
        position,
    });
    item.sound_requests.push(POP_SOUND);
    true
}

/// itArwinglaser_UnkMotion5_Coll (802E8250): while its cannon holds it, a
/// side beam takes the cannon's root (at its own depth), scale and
/// rotation; once the cannon is gone, so is the beam.
fn beam_collision(item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    let laser = *laser(item);
    let Some((translation, scale, rotation)) = laser.parent_pose else {
        return true;
    };
    item.position = Vec3::new(translation.x, translation.y, laser.beam_depth);
    item.model_scale = scale;
    item.rotation = rotation;
    item.joints_dirty = true;
    false
}

/// itArwinglaser_UnkMotion3_Anim (802E7B54): the tip a tick ago, the shot
/// out, and it_80273130's lifetime.
fn aimed_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let laser = laser(item);
    laser.previous_tip = laser.tip;
    laser.launched = true;
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itArwinglaser_UnkMotion3_Phys (802E7C90): the speed along the course
/// (retail 802E7CE8/CEC: `x4_throw_speed_mul * x4`, then times the
/// course), the scale (xDFC * xE34, its y and z times x8); on Corneria the
/// Great Fox's drift moves it too (grCorneria_801DDCF0).
fn aimed_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Attributes(&ctx.assets.special_attributes);
    let laser = *laser(item);
    if matches!(laser.kind, 1 | 3) {
        let speed = a.speed_scale() * a.speed();
        item.velocity.x = laser.direction.x * speed;
        item.velocity.y = laser.direction.y * speed;
        item.velocity.z = laser.direction.z * speed;
    }
    let s = laser.map_scale * laser.scale;
    item.model_scale = Vec3::new(s, s * a.thickness(), s * a.thickness());
    item.joints_dirty = true;
    item.position.x += laser.stage_drift.x;
    item.position.y += laser.stage_drift.y;
}

/// itArwinglaser_UnkMotion3_Coll (802E7E44): the tip found; within the
/// plane's band the first tick sounds and it_802E70BC tests the stage; a
/// laser that meets it pops at its tip (efSync_Spawn(0x40E)) and ends.
fn aimed_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let position = tip(item, ctx.assets);

    let (joint, sound) = {
        let laser = laser(item);
        laser.tip = position;
        if !(laser.previous_tip.z > -PLANE_BAND && laser.previous_tip.z < PLANE_BAND) {
            return false;
        }
        laser.near_ticks += 1;
        let mut sound = None;
        if laser.near_ticks == 1 {
            sound = Some(if laser.joint == 3 { WOLFEN_SOUND } else { ARWING_SOUND });
            laser.near_ticks += 1;
        }
        (laser.joint, sound)
    };
    if let Some(sound) = sound {
        item.sound_requests.push(sound);
    }
    if meets_stage(position, joint, ctx.map) {
        item.events.push(ItemEvent::Effect {
            id: POP_EFFECT,
            position,
        });
        item.sound_requests.push(POP_SOUND);
        return true;
    }
    false
}

/// it_802E70BC (802E70BC), with the tip just stored in xE18: the segment
/// from xE18 to the tip (both flattened, so a single point) crosses a
/// line; or, each probe in turn, the segment from the tip to the probe's
/// end crosses one and the segment back from that crossing to the tip
/// crosses one too, or else the segment from the probe's end to the tip
/// crosses one; either second crossing counts unless its line passes
/// fliers through (mpLineGetFlags 0x100). it_8026EA9C keeps the tests to
/// collision joint `joint`, and moves its end point onto a crossing. The
/// first test's mpGetSpeed only writes the velocity of a laser that ends.
fn meets_stage(tip: Vec3, joint: i32, map: &mut melee_mp::CollMap) -> bool {
    if map
        .check_all_remap(-1, joint, tip.x, tip.y, tip.x, tip.y)
        .is_some()
    {
        return true;
    }
    let counts = |map: &mut melee_mp::CollMap, hit: Option<melee_mp::LineHit>| {
        hit.is_some_and(|hit| map.line_get_flags(hit.line_id) & PASS_THROUGH_LINE == 0)
    };
    let mut hits = 0;
    for (dx, dy) in PROBES {
        let end = (tip.x + dx, tip.y + dy);
        let back = match map.check_all_remap(-1, joint, tip.x, tip.y, end.0, end.1) {
            Some(crossing) => {
                map.check_all_remap(-1, joint, crossing.pos.x, crossing.pos.y, tip.x, tip.y)
            }
            None => map.check_all_remap(-1, joint, end.0, end.1, tip.x, tip.y),
        };
        if counts(map, back) {
            hits += 1;
        }
    }
    hits != 0
}
