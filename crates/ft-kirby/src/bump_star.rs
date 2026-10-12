//! The star Kirby raises where he meets the stage, ftkirby.c
//! (800F1D24..800F1F1C).
//!
//! Fighter_procMap runs ftKb_SpecialN_800F1D24 after every collision
//! callback (fighter.c:2494): a wall, ceiling or floor Kirby touches that
//! he was not against on the previous tick spawns generator 0x206 at that
//! side of his ECB (efAsync kind 2, efAlt 0x49E), then none for five ticks
//! of contact.
use crate::init::Kirby;
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::fighter::Fighter;
use melee_types::mp::collide;

/// efAlt 0x49E: efLib_CreateGenerator(0x206, pos).
const STAR_GENERATOR: u16 = 0x206;
/// u.kb.x60 after a tick against any surface.
const COOLDOWN_TICKS: i32 = 5;
/// `fp->parts[0].joint`: TopN.
const TOP_JOINT: usize = 0;

/// The four contacts ftKb_SpecialN_800F1DAC tests, in its order: the flag
/// that is set now and the flags that must all have been clear before.
const CONTACTS: [(u32, u32); 4] = [
    (collide::RIGHT_WALL_HUG, collide::RIGHT_WALL_MASK),
    (collide::LEFT_WALL_HUG, collide::LEFT_WALL_MASK),
    (collide::CEILING_HUG, collide::CEILING_MASK),
    (collide::FLOOR_HUG, collide::FLOOR_MASK),
];

/// ftKb_SpecialN_800F1F1C (800F1F1C): the star at `offset` from TopN.
///
/// The collision callback's own motion change (a landing) has queued its
/// script's frame-0 dust by now; the queue is last in, first out, so the
/// star spawns ahead of that dust.
pub fn spawn(f: &mut Fighter, offset: Vec3) {
    let facing = f.physics.facing;
    f.core
        .push_effect_after_issued_graphics(EffectRequest::Graphics {
            id: STAR_GENERATOR,
            bone: TOP_JOINT,
            offset,
            facing,
            floor_angle: 0.0,
        });
}

/// ftKb_SpecialN_800F1DAC (800F1DAC): the first new contact's star.
fn star_for_new_contact(f: &mut Fighter) {
    let data = &f.collision.data;
    let (now, before) = (data.env_flags as u32, data.prev_env_flags as u32);
    let ecb = &data.ecb;
    let offsets = [
        Vec3::new(ecb.left.x, ecb.left.y, 0.0),
        Vec3::new(ecb.right.x, ecb.right.y, 0.0),
        Vec3::new(0.0, ecb.top.y, 0.0),
        Vec3::new(0.0, ecb.bottom.y, 0.0),
    ];
    for ((touching, was_touching), offset) in CONTACTS.into_iter().zip(offsets) {
        if now & touching != 0 && before & was_touching == 0 {
            spawn(f, offset);
            return;
        }
    }
}

/// ftKb_SpecialN_800F1D24 (800F1D24).
pub fn after_collision(f: &mut Fighter) {
    let cooldown = f.character.get::<Kirby>().bump_star_cooldown;
    if cooldown != 0 {
        f.character.get_mut::<Kirby>().bump_star_cooldown = cooldown - 1;
    } else {
        star_for_new_contact(f);
    }
    let now = f.collision.data.env_flags as u32;
    if CONTACTS.iter().any(|&(touching, _)| now & touching != 0) {
        f.character.get_mut::<Kirby>().bump_star_cooldown = COOLDOWN_TICKS;
    }
}
