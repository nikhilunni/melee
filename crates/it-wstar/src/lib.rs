//! Warp Star (It_Kind_WStar), itwstar.c (80294364..802948D8): it falls in
//! and rests, spinning, until a fighter picks it up; the pickup is a ride
//! (ftCo_WarpStar.c, melee-ft's `warp_star`): the star flies its rider up
//! along one of seven paths, falls back with it and bursts on landing.
//!
//! A sliding star (EnteredAir, state 4) and a dropped one (its rider hit,
//! which intangibility rules out in the witnesses) fail closed.
use hsd_archive::{desc::anim::AnimJoint as ArchiveAnimJoint, Archive};
use hsd_types::{Mtx, Vec3};
use melee_it::{
    desc::ItemAssets,
    state_change::ANIM_UPDATE,
    AirLanding, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, WarpStarState,
};
use melee_types::ItemKind;

pub struct WarpStar;

/// it_803F61B0's anim_id column.
pub const ARTICLE_STATES: [i32; 6] = [0, -1, -1, 2, -1, 1];
/// The ridden fall's article state (it_802946B0); held before it, the star
/// plays none.
pub const RIDDEN_FALL_ARTICLE_STATE: usize = ARTICLE_STATES[motion::RIDDEN_FALL as usize] as usize;
/// itWstarAttributes: x0..x24 and seven 8-byte flight rows.
pub const SPECIAL_ATTRIBUTES: u32 = 10 + 2 * 7;

mod motion {
    pub const REST: u16 = 0;
    pub const FALL: u16 = 1;
    /// Picked up: its rider flies.
    pub const HELD: u16 = 2;
    /// Falling with its rider (it_802946B0).
    pub const RIDDEN_FALL: u16 = 3;
    /// The landing's burst (it_802947CC).
    pub const BLAST: u16 = 5;
}

/// itWstarAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the resting spin, degrees a frame (it_80274658).
    fn rest_spin(&self) -> f32 {
        self.0[0]
    }
    /// x4: the fighter half width the model's size is authored for.
    fn reference_half_width(&self) -> f32 {
        self.0[1]
    }
    /// x8: the model scale at that width.
    fn reference_scale(&self) -> f32 {
        self.0[2]
    }
    /// xC..x20: the falling rider's collision box, per unit of scale
    /// (it_802944AC): top, bottom, left x and y, right x and y.
    fn ride_box(&self) -> [f32; 6] {
        [self.0[3], self.0[4], self.0[5], self.0[6], self.0[7], self.0[8]]
    }
    /// x24: how many flight paths x28 lists.
    fn path_count(&self) -> i32 {
        self.0[9].to_bits() as i32
    }
}

/// it_3F14_Logic29_PickedUp's efAsync_Spawn(gobj, &xBC0, 0, 0x43D,
/// it_80272CC0(gobj, 3)): the sparkle on the held star.
const HELD_SPARKLE: u16 = 0x43D;
const HELD_SPARKLE_BONE: usize = 3;
/// it_80272C08's efSync_Spawn and sound.
const BURST_EFFECT: u16 = 0x410;
const BURST_SOUND: u32 = 0x74;

static STATES: [ItemStateRow; 6] = [
    row(0, no_animation, rest_physics, rest_collision),
    row(1, no_animation, gravity, fall_collision),
    row(2, no_animation, no_physics, no_collision),
    row(3, no_animation, no_physics, no_collision),
    row(4, no_animation, no_physics, slide_collision),
    row(5, blast_animation, no_physics, no_collision),
];

const fn row(
    state: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[state],
        animation,
        physics,
        collision,
    }
}

impl ItemLogic for WarpStar {
    const KIND: ItemKind = ItemKind::WStar;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itWStar_Logic29_Spawned (802944F4): its ECB stays upright (xDCE b7
    /// off), the ride's model and hitbox scales 1 (xDD4, xDD8), the fall
    /// (it_80294624).
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.ecb_follows_rotation = false;
        item.scratch = ItemScratch::WarpStar(WarpStarState {
            scale: 1.0,
            hitbox_scale: 1.0,
            path: -1,
            joint_scaled: false,
        });
        change(item, motion::FALL, ANIM_UPDATE, assets);
    }
    /// it_3F14_Logic29_PickedUp (8029473C): state 2 and the held sparkle on
    /// bone 3.
    fn picked_up(item: &mut ItemCore, context: &mut ItemAnimationContext<'_>) {
        change(item, motion::HELD, ANIM_UPDATE, context.assets);
        item.spawn_async(ItemEvent::BoneGenerator {
            id: HELD_SPARKLE,
            bone: HELD_SPARKLE_BONE,
        });
    }
    /// itWStar_Logic29_Dropped (802947A0): its rider was hit or KO'd; it
    /// falls (state 1) and its sparkle goes.
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        // Item_80268E5C(gobj, 1, 6) and efLib_DestroyAll; only a rider's hit
        // or KO drops it (ftCo_800C4858, guarded with the rider).
        unimplemented!("itWStar_Logic29_Dropped (itwstar.c:167): a dropped Warp Star")
    }
    const ENTERED_AIR: Option<melee_it::EnteredAir> = Some(|_item, _assets| {
        unimplemented!("itWStar_Logic29_EnteredAir (itwstar.c:230): a Warp Star sliding down a slope is not witnessed")
    });
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::WarpStarRide {
                half_width,
                hitbox_scale,
            } => ride(item, ride_scale(half_width, assets), hitbox_scale),
            ItemControl::WarpStarFall => ridden_fall(item, assets),
            _ => unreachable!("{control:?} sent to a Warp Star"),
        }
    }
}

fn star(item: &ItemCore) -> &WarpStarState {
    match &item.scratch {
        ItemScratch::WarpStar(state) => state,
        _ => unreachable!("a Warp Star's scratch"),
    }
}
fn star_mut(item: &mut ItemCore) -> &mut WarpStarState {
    match &mut item.scratch {
        ItemScratch::WarpStar(state) => state,
        _ => unreachable!("a Warp Star's scratch"),
    }
}

/// The model scale it_80294430 gives a rider of push half width
/// `half_width` (x2C4.y): x8 * (half_width / x4) times ItemAttr x60
/// (retail 80294458..68: fdivs, fmuls, fmuls).
fn ride_scale(half_width: f32, assets: &ItemAssets) -> f32 {
    let a = Attributes(&assets.special_attributes);
    let scale = a.reference_scale() * (half_width / a.reference_half_width());
    scale * assets.scale
}

/// it_802944AC (802944AC): the falling rider's collision box, the
/// attributes' box times xDD4 (six fmuls): top, bottom, left (x, y),
/// right (x, y).
pub fn ride_box(item: &ItemCore, assets: &ItemAssets) -> melee_types::mp::FtCollisionBox {
    let scale = star(item).scale;
    let [top, bottom, left_x, left_y, right_x, right_y] = Attributes(&assets.special_attributes)
        .ride_box()
        .map(|side| side * scale);
    melee_types::mp::FtCollisionBox {
        top,
        bottom,
        left: hsd_types::Vec2::new(left_x, left_y),
        right: hsd_types::Vec2::new(right_x, right_y),
    }
}

/// it_80294430 (80294430): the attach joint takes the rider's scale
/// (it_80272F7C), kept with the hitbox scale for the fall.
fn ride(item: &mut ItemCore, scale: f32, hitbox_scale: f32) {
    let state = star_mut(item);
    state.scale = scale;
    state.hitbox_scale = hitbox_scale;
    state.joint_scaled = true;
}

/// it_80294364 (80294364): HSD_Randi over the paths other than the one any
/// star flew last (it_804D6D00), which becomes this one's (xDDC). Returns
/// the path; its x4 sound plays (Item_8026AE84).
pub fn draw_path(
    item: &mut ItemCore,
    last_path: &mut i8,
    rng: &mut gekko_math::HsdRng,
    assets: &ItemAssets,
) -> usize {
    let count = Attributes(&assets.special_attributes).path_count();
    let mut candidates = [0i32; 7];
    let mut len = 0;
    for path in 0..count {
        if path != i32::from(*last_path) {
            candidates[len] = path;
            len += 1;
        }
    }
    let path = candidates[rng.randi(len as i32) as usize];
    *last_path = path as i8;
    star_mut(item).path = path;
    // Item_8026AE84(ip, x28[path].x4_sfx, 0x7F, 0x40).
    let sound = assets.special_attributes[10 + 2 * path as usize + 1].to_bits();
    item.sound_requests.push(sound);
    path as usize
}

/// it_802946B0 (802946B0): state 3, the attach joint at xDD4 again, the
/// hitboxes the state made at xDD8, and the star's attack id 0x5E
/// (it_8027B288's statistics word 0x0044005F, it_8027B330).
fn ridden_fall(item: &mut ItemCore, assets: &ItemAssets) {
    change(item, motion::RIDDEN_FALL, ANIM_UPDATE, assets);
    let hitbox_scale = star(item).hitbox_scale;
    scale_hitboxes(item, hitbox_scale);
}

/// it_802755C0 (802755C0): every live capsule's scale times `scale`.
fn scale_hitboxes(item: &mut ItemCore, scale: f32) {
    for hit in item.hitboxes.iter_mut().flatten() {
        hit.descriptor.radius *= scale;
    }
}

/// it_802947CC (802947CC): the landing's burst. Still, state 5 (whose
/// script makes the blast), out of the hand with no push (it_8027429C,
/// keeping its attack statistics), not grabbable, at the rider's
/// position, hidden, the common explosion lifetime (it_8027518C), its
/// generators gone, a radial gust, the burst effect without hitlag
/// (it_80272C08), the blast at xDD8.
#[allow(clippy::too_many_arguments)]
pub fn burst(
    item: &mut ItemCore,
    position: Vec3,
    hand: &Mtx,
    part_position: Vec3,
    center: Vec3,
    attack: Option<melee_types::combat::AttackInstance>,
    map: &mut melee_mp::CollMap,
    assets: &ItemAssets,
) {
    item.velocity = Vec3::ZERO;
    item.command_variables[0] = 0;
    change(item, motion::BLAST, ANIM_UPDATE, assets);
    let release = item.drop_release_point(part_position, hand, assets);
    item.leave_hand(Vec3::ZERO, release, assets);
    item.end_hold(center, attack, map, assets);
    // it_8026B3A8.
    item.grabbable = false;
    item.position = position;
    item.set_root_translation(position);
    item.hidden = true;
    // it_8027518C.
    item.life_timer = assets.explosion_lifetime;
    item.destroy_effect_suppressed = true;
    item.mark_exploding();
    item.events.push(ItemEvent::DestroyEffects);
    // lb_800119DC(&pos, 0x78, 1.0, 0.02, pi/3).
    item.events.push(ItemEvent::Gust {
        center: position,
        frames: 0x78,
        strength: 1.0,
        decay: 0.02,
        phase_step: std::f32::consts::FRAC_PI_3,
    });
    // it_80272C08: the burst, its sound, and it_80274C60's hitlag off.
    item.events.push(ItemEvent::Effect {
        id: BURST_EFFECT,
        position: item.position,
    });
    item.sound_requests.push(BURST_SOUND);
    item.hitlag_enabled = false;
    let hitbox_scale = star(item).hitbox_scale;
    scale_hitboxes(item, hitbox_scale);
}

/// The seven flight paths (itWstarAttributes x28[i].x0_anim_joint): one
/// joint animation each for the rider's root, prepared at load.
pub struct FlightPaths {
    pub paths: Vec<(hsd_anim::jobj::AnimJoint, hsd_anim::aobj::AObj)>,
}
impl FlightPaths {
    /// The paths of ItCo's Warp Star article. Each animates the root alone:
    /// rotation and translation, no RObj.
    pub fn read(archive: &Archive, public: u32) -> hsd_archive::desc::Result<Self> {
        use hsd_anim::{aobj::AObjDesc, fobj::FObjDesc};
        let r = archive.reader();
        let articles = r.u32(public + 4)?;
        let article = r.u32(articles + ItemKind::WStar as u32 * 4)?;
        let special = r.u32(article + 4)?;
        let count = r.u32(special + 0x24)?;
        let paths = (0..count)
            .map(|i| {
                let source = ArchiveAnimJoint::read(archive, r.u32(special + 0x28 + i * 8)?)?;
                assert!(source.robj_anim.is_none(), "a Warp Star path with an RObj");
                let a = source
                    .aobjdesc
                    .as_deref()
                    .expect("a Warp Star path animates the root");
                let desc = AObjDesc {
                    flags: a.flags,
                    end_frame: a.end_frame,
                    obj_id: 0,
                    fobjdesc: a
                        .tracks()
                        .map(|t| FObjDesc {
                            length: t.length,
                            startframe: t.startframe,
                            obj_type: t.type_,
                            frac_value: t.frac_value,
                            frac_slope: t.frac_slope,
                            ad: t.ad.clone(),
                        })
                        .collect(),
                };
                let prepared = hsd_anim::aobj::AObj::load_desc(&desc);
                let joint = hsd_anim::jobj::AnimJoint {
                    aobjdesc: Some(desc),
                    flags: source.flags,
                    children: Vec::new(),
                };
                Ok((joint, prepared))
            })
            .collect::<hsd_archive::desc::Result<_>>()?;
        Ok(Self { paths })
    }
}

fn change(item: &mut ItemCore, motion: u16, flags: u32, assets: &ItemAssets) {
    item.change_motion_with(motion, ARTICLE_STATES[motion as usize], flags, assets);
}

fn no_animation(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itWstar_UnkMotion1_Phys: it_80272860 with ItemAttr x10 / x14.
fn gravity(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// itWstar_UnkMotion0_Phys: the spin at x0 degrees a frame
/// (it_80274658), turned (it_80274A64).
fn rest_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.update_spin(Attributes(&ctx.assets.special_attributes).rest_spin());
    item.spin();
}

/// itWstar_UnkMotion0_Coll -> it_8026D62C: off the floor it falls
/// (it_80294624).
fn rest_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if !item.stay_grounded(ctx.map, ctx.assets, ctx.entered_air) {
        change(item, motion::FALL, ANIM_UPDATE, ctx.assets);
    }
    false
}

/// itWstar_UnkMotion1_Coll -> it_8026E15C: a settled landing rests
/// (it_8029455C: still, grabbable, state 0).
fn fall_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let cell = ctx.rng.expect("it_8026DDFC draws");
    let mut rng = cell.get();
    let landing = item.air_collision_with_landing(ctx.map, ctx.assets, ctx.entered_air, &mut rng);
    cell.set(rng);
    match landing {
        AirLanding::Airborne | AirLanding::Sliding => {}
        AirLanding::Landed => {
            item.velocity = Vec3::ZERO;
            item.grabbable = true;
            change(item, motion::REST, ANIM_UPDATE, ctx.assets);
        }
        AirLanding::Broken => unreachable!("it_8026DDFC: a Warp Star is never thrown"),
    }
    false
}

/// itWstar_UnkMotion4_Coll -> it_8026E8C4 (reached only through the
/// guarded EnteredAir).
fn slide_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    unimplemented!("itWstar_UnkMotion4_Coll (itwstar.c:242): a Warp Star sliding down a slope is not witnessed")
}

/// itWstar_UnkMotion5_Anim (8029484C): a blast hitbox the script raised
/// command variable 0 for takes xDD8 (it_802755C0); the burst lasts its
/// explosion lifetime (it_802751D8).
fn blast_animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] != 0 {
        let hitbox_scale = star(item).hitbox_scale;
        scale_hitboxes(item, hitbox_scale);
        item.command_variables[0] = 0;
    }
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}
