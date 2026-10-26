//! The Star Fox stages' ship lasers (It_Kind_Arwing_Laser),
//! it/kinds/itarwinglaser.c (802E70BC..802E8790).
//!
//! A far Arwing or Wolfen on Venom (grVenom_80205F30) fires
//! [`it_802E7654`](launch) at a fighter: the laser starts at the ship's gun,
//! flies straight at the fighter's position (in depth too) and only touches
//! the stage near the stage's plane. Types 1 (an Arwing's) and 3 (a
//! Wolfen's) are the aimed lasers; types 0, 2, 4 and 5 (Corneria's cannon
//! and its side beams, it_802E72E0) are not ported.
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

/// ArwingLaserAttr, from the special attribute words. x0 points at an
/// ItemAttr whose x4_throw_speed_mul the scene resolves at load into word 0.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// `attrs->x0->x4_throw_speed_mul`.
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
fn cannon_animation(_: &mut ItemCore, _: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itarwinglaser.c:378: itArwinglaser_UnkMotion2_Anim, Corneria's cannon laser")
}
fn cannon_physics(_: &mut ItemCore, _: &ItemPhysicsContext<'_>) {
    unimplemented!("itarwinglaser.c:396: itArwinglaser_UnkMotion2_Phys, Corneria's cannon laser")
}
fn cannon_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itarwinglaser.c:483: itArwinglaser_UnkMotion2_Coll, Corneria's cannon laser")
}
fn beam_collision(_: &mut ItemCore, _: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itarwinglaser.c:543: itArwinglaser_UnkMotion5_Coll, the cannon's side beams")
}
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
    /// it_802E8418 (absorbed). Not reached in 600 explorer matches.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itarwinglaser.c:595: it_802E8418, an absorber takes a ship's laser")
    }
    /// it_802E8784 (hit_shield). Not reached in 600 explorer matches.
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itarwinglaser.c:673: it_802E8784, a ship's laser hits a shield")
    }
    /// it_802E8420 (reflected): an aimed laser moving against the
    /// reflector's facing turns its facing while its course stays. Not
    /// reached in 600 explorer matches.
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itarwinglaser.c:600: it_802E8420, a reflector turns a ship's laser")
    }
    /// it_802E85F4 (shield_bounced). Not reached in 600 explorer matches.
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itarwinglaser.c:641: it_802E85F4, a ship's laser glances off a shield")
    }
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
    /// yakumono_param x34, and Ground_801C0498.
    pub scale: f32,
    pub map_scale: f32,
}

/// it_802E7654's SpawnItem: at the gun, facing 0, unowned and motionless.
pub fn spawn(gun: Vec3) -> SpawnItem {
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
        position: gun,
        previous_position: gun,
        velocity: Vec3::ZERO,
        facing: 0.0,
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
    item.scratch = ItemScratch::ArwingLaser(ArwingLaserState {
        map_scale: launch.map_scale,
        scale: launch.scale,
        direction,
        tip: gun,
        previous_tip: gun,
        launched: false,
        kind: launch.kind,
        turned: false,
        joint: launch.joint,
        near_ticks: 0,
    });
    item.facing = if direction.x >= 0.0 { 1.0 } else { -1.0 };
    item.rotation.x = -melee_lb::trigf::atan2f(direction.y, direction.z);
    item.rotation.y = melee_lb::trigf::atan2f(direction.x, direction.z);
    item.rotation.z = 0.0;
    item.joints_dirty = true;
    // it_802E7A4C: it_8026B3A8, Item_80268E5C(gobj, xE38, ITEM_ANIM_UPDATE).
    item.grabbable = false;
    let state = launch.kind;
    assert!(
        matches!(state, 1 | 3),
        "it_802E7654: laser type {state} (Corneria's cannon is it_802E72E0)"
    );
    item.change_motion_with(state as u16, ARTICLE_STATES[state as usize], ANIM_UPDATE, assets);
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
/// course), the scale (xDFC * xE34, its y and z times x8). Venom adds no
/// offset (Corneria's grCorneria_801DDCF0 is not ported).
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
}

/// itArwinglaser_UnkMotion3_Coll (802E7E44): the tip found; within the
/// plane's band the first tick sounds and it_802E70BC tests the stage.
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
        // efSync_Spawn(0x40E) at the tip and the pop's sound, then the
        // laser ends. Never reached in 600 explorer matches.
        unimplemented!(
            "itarwinglaser.c:528: itArwinglaser_UnkMotion3_Coll, a ship's laser meets the stage"
        );
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

