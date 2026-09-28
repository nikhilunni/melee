//! Fox/Falco item family, itfoxlaser.c and itfoxblaster.c.
use melee_it::desc::ItemAssets;
use melee_it::*;
use melee_types::ItemKind;

pub struct FoxLaser;
pub struct FalcoLaser;
pub struct FoxBlaster;
pub struct FalcoBlaster;
/// it_803F67D0: both laser animations share the same three callbacks.
pub static LASER_STATES: [ItemStateRow; 2] = [laser_row(0), laser_row(1)];
const fn laser_row(animation_id: i32) -> ItemStateRow {
    ItemStateRow {
        animation_id,
        animation: laser_animation,
        physics: laser_physics,
        collision: laser_collision,
    }
}
macro_rules! laser {
    ($type:ty,$kind:ident) => {
        impl ItemLogic for $type {
            const KIND: ItemKind = ItemKind::$kind;
            const STATES: &'static [ItemStateRow] = &LASER_STATES;
            // it_3F2F.c: Fox/Falco laser picked_up callback is NULL.
            fn pickup_possible(_item: &ItemCore) -> bool {
                false
            }
            fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
                item.scratch = ItemScratch::Ray(RayState {
                    previous_position: item.position,
                    ..Default::default()
                });
                item.life_timer = assets.special_attributes[0];
                // Retail has no laser spawn callback. it_8029C504 selects the
                // motion once in initialize_laser; running motion 0 here would
                // execute its hitbox commands before the selected throw script.
            }
            fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
                true
            }
            fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
                true
            }
            fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
                true
            }
            fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
                true
            }
            fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
                if item.facing != ctx.reflected_facing {
                    item.facing = ctx.reflected_facing;
                    item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
                }
                let ItemScratch::Ray(ray) = &mut item.scratch else {
                    unreachable!()
                };
                ray.scale = 0.001;
                ray.angle =
                    normalize_stored_angle((f64::from(ray.angle) + std::f64::consts::PI) as f32);
                item.model_scale.z = ray.scale;
                false
            }
            fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
                // lbVector_Mirror, 8000DC84/8C/9C: three fmadds.
                let n = ctx.shield_normal;
                let dot = gekko_math::fma::fmadds(n.x, item.velocity.x, n.y * item.velocity.y);
                let factor = dot * -2.0;
                item.velocity.x = gekko_math::fma::fmadds(n.x, factor, item.velocity.x);
                item.velocity.y = gekko_math::fma::fmadds(n.y, factor, item.velocity.y);
                let ItemScratch::Ray(ray) = &mut item.scratch else {
                    unreachable!()
                };
                ray.scale = 0.001;
                ray.angle = normalize_stored_angle(melee_lb::trigf::atan2f(
                    item.velocity.y,
                    item.velocity.x,
                ));
                false
            }
        }
    };
}
laser!(FoxLaser, FoxLaser);
laser!(FalcoLaser, FalcoLaser);
/// it_8029C504: normalize angle, initialize the selected animation and ray vars.
#[allow(clippy::manual_range_contains)] // Retail comparisons preserve the NaN branch.
pub fn initialize_laser(
    item: &mut ItemCore,
    assets: &ItemAssets,
    angle: f32,
    speed: f32,
    motion: u16,
) {
    let angle = normalize_angle(angle);
    item.facing = if angle < std::f64::consts::FRAC_PI_2 || angle > std::f64::consts::PI * 1.5 {
        1.0
    } else {
        -1.0
    };
    item.change_motion(motion, assets);
    item.life_timer = assets.special_attributes[0];
    item.scratch = ItemScratch::Ray(RayState {
        angle: angle as f32,
        speed,
        scale: 0.0,
        previous_position: item.position,
    });
}
fn normalize_stored_angle(mut angle: f32) -> f32 {
    // Reflection callbacks store to Item each iteration (8029CBF4/CC18 frsp).
    while angle < 0.0 {
        angle = (f64::from(angle) + std::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > std::f64::consts::TAU {
        angle = (f64::from(angle) - std::f64::consts::TAU) as f32;
    }
    angle
}
fn normalize_angle(angle: f32) -> f64 {
    // it_8029C504 8029C544/55C: double fadd/fsub, one final stfs.
    let mut angle = f64::from(angle);
    while angle < 0.0 {
        angle += std::f64::consts::TAU;
    }
    while angle > std::f64::consts::TAU {
        angle -= std::f64::consts::TAU;
    }
    angle
}
/// itFoxlaser_UnkMotion1_Anim (8029C6F4); fmuls at 8029C734/748,
/// fdivs/fadds at 8029C8F4/8. No contraction.
fn laser_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let ItemScratch::Ray(ray) = &mut item.scratch else {
        unreachable!()
    };
    item.velocity.x = ray.speed * gekko_math::msl::cosf(ray.angle);
    item.velocity.y = ray.speed * gekko_math::msl::sinf(ray.angle);
    item.velocity.z = 0.0;
    item.facing = if item.velocity.x > 0.0 { 1.0 } else { -1.0 };
    // 8029C780 fmul and 8029C844 fadd are double precision before JObj stores.
    item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
    let vel_x = if item.facing == 1.0 {
        -item.velocity.x
    } else {
        item.velocity.x
    };
    item.rotation.x =
        (std::f64::consts::PI + f64::from(melee_lb::trigf::atan2f(item.velocity.y, vel_x))) as f32;
    // Retail authored length is 11.25 world units.
    ray.scale += gekko_math::msl::fabsf(ray.speed) / 11.25;
    if ray.scale > ctx.assets.special_attributes[1] {
        ray.scale = ctx.assets.special_attributes[1];
    }
    if ray.scale < 0.00001 {
        ray.scale = 0.001;
    }
    item.model_scale.z = ray.scale;
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}
/// itFoxlaser_UnkMotion1_Phys: save the start of this tick's segment.
fn laser_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    let ItemScratch::Ray(ray) = &mut item.scratch else {
        unreachable!()
    };
    ray.previous_position = item.position;
}
/// itFoxlaser_UnkMotion1_Coll: environment contact expires next animation tick.
fn laser_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    if ctx.stage_contact {
        item.life_timer = 1.0;
    }
    false
}

/// it_803F6CA8: Fox and Falco share all eleven blaster rows.
pub static BLASTER_STATES: [ItemStateRow; 11] = [
    blaster_row(0),
    blaster_row(1),
    blaster_row(2),
    blaster_row(3),
    blaster_row(4),
    blaster_row(5),
    blaster_row(6),
    blaster_row(7),
    blaster_row(8),
    ItemStateRow {
        animation_id: -1,
        animation: remove_blaster,
        physics: empty_physics,
        collision: remove_collision,
    },
    ItemStateRow {
        animation_id: -1,
        animation: external_blaster,
        physics: empty_physics,
        collision: empty_collision,
    },
];
const fn blaster_row(animation_id: i32) -> ItemStateRow {
    ItemStateRow {
        animation_id,
        animation: blaster_animation,
        physics: blaster_physics,
        collision: empty_collision,
    }
}
macro_rules! blaster {
    ($type:ty,$kind:ident,$open:expr,$holster:expr) => {
        impl ItemLogic for $type {
            const KIND: ItemKind = ItemKind::$kind;
            const STATES: &'static [ItemStateRow] = &BLASTER_STATES;
            // Held setup it_80279CDC clears grabbable via it_8026B3A8.
            // Detached or external states require a complete pickup audit.
            fn pickup_possible(item: &ItemCore) -> bool {
                !item.held
            }
            // ftFox_SpecialN: it_8026BAE8 sets the blaster model scale.
            const MODEL_COPIES: usize = 1;
            const HELD_SCALE: f32 = 0.85;
            const HELD_PART: Option<melee_types::FtPart> = Some(melee_types::FtPart::RThumbNb);
            fn model_pose(
                item: &ItemCore,
                tree: &mut hsd_anim::jobj::JObjTree,
                _copy: usize,
            ) -> bool {
                crate::pose::blaster(item, tree)
            }
            fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
                // Item_8026AB54 at HELD_PART, through it_8026BAE8; the
                // presentation resolves the part, so xDC4 is not modelled.
                item.held = true;
                item.scratch = ItemScratch::Held(HeldState {
                    visibility: 1,
                    ..Default::default()
                });
                item.change_motion(0, assets);
            }
            fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
                // itFoxBlaster_Logic96_PickedUp: it_803F6E68 startup mapping.
                const PICKUP_MOTIONS: [u16; 11] = [0, 9, 9, 3, 9, 9, 6, 7, 8, 9, 10];
                let owner = ctx.owner.expect("held blaster owner");
                item.change_motion(
                    PICKUP_MOTIONS[usize::from(owner.blaster_action)],
                    ctx.assets,
                );
            }
            fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
                let ItemScratch::Held(held) = &mut item.scratch else {
                    unreachable!()
                };
                match control {
                    // it_802ADDD0: sound only on a transition to hidden state 2.
                    ItemControl::Visibility(value) => {
                        if held.visibility != value && value == 2 {
                            item.sound_requests.push($holster);
                        }
                        held.visibility = value;
                    }
                    ItemControl::Open => {
                        if held.opening_frame != 4 {
                            held.opening_frame = 1;
                            held.opening_direction = 1;
                            // it_802AE538: first open only, latched until item reset.
                            if !held.opening_sound_played {
                                item.sound_requests.push($open);
                                held.opening_sound_played = true;
                            }
                        }
                    }
                    ItemControl::Close => {
                        if held.opening_frame != 0 {
                            held.opening_frame = 3;
                            held.opening_direction = -1;
                        }
                    }
                    ItemControl::Fire => {
                        held.recoil_frame = 1;
                        held.shot_pending = true;
                    }
                    ItemControl::Remove => item.destroyed = true,
                    ItemControl::Counter
                    | ItemControl::ParasolOpening(_)
                    | ItemControl::ParasolOpen(_)
                    | ItemControl::OwnerHitlag(_)
                    | ItemControl::Strike
                    | ItemControl::Motion(_)
                    | ItemControl::Orphan
                    | ItemControl::Aim { .. } => {
                        unreachable!("another kind's article control sent to a blaster")
                    }
                }
            }
        }
    };
}
blaster!(FoxBlaster, FoxBlaster, 0x1AE05, 0x1AE14);
blaster!(FalcoBlaster, FalcoBlaster, 0x186F1, 0x18700);
/// itFoxblaster_UnkMotion8_Anim: item motion follows the owner's SpecialN index;
/// ending rows 2/5/6/7/8 retain their animation until cleanup.
fn blaster_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let Some(owner) = ctx.owner else {
        return true;
    };
    if ![2, 5, 6, 7, 8].contains(&item.motion) && item.motion != owner.blaster_action {
        item.change_motion(owner.blaster_action, ctx.assets);
    }
    owner.blaster_action == 9 || owner.remove_blaster
}
fn blaster_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    // itFoxblaster_UnkMotion8_Phys (802AEED4) updates attached model parts;
    // Item.pos remains at the spawn position when the fighter moves.
    let ItemScratch::Held(held) = &mut item.scratch else {
        unreachable!()
    };
    held.opening_pose_frame = held.opening_frame as usize;
    held.recoil_pose_frame = held.recoil_frame;
    if held.opening_frame > 0 && held.opening_frame < 4 {
        held.opening_frame += held.opening_direction;
        if held.opening_frame >= 5 {
            held.opening_frame = 4;
            held.opening_direction = 0;
        }
        if held.opening_frame <= 0 {
            held.opening_frame = 0;
            held.opening_direction = 0;
        }
    }
    if held.recoil_frame > 0 && held.recoil_frame < 14 {
        held.recoil_frame += 1;
        if held.recoil_frame >= 14 {
            held.recoil_frame = 0;
        }
    }
}
fn remove_blaster(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    true
}
fn remove_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    true
}
fn empty_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn empty_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
fn external_blaster(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    unimplemented!("itFoxblaster_UnkMotion10_Anim external-owner scale")
}

mod pose;
