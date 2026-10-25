//! itzako.c: the helpers every stage enemy shares (its model's turn toward
//! the floor, its walk along the floor's slope, its stop).
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{cosf, fctiwz, sinf, sqrtf},
};
use hsd_types::Vec3;
use melee_it::{desc::MonsterCommonData, ItemCore, ItemScratch, MonsterState};
use melee_lb::vector::{angle_xy, cross, rotate_about, Axis};
use melee_types::{mp::collide::FLOOR_MASK, HitElement};

/// itzako.c "@211": an angle below this counts as no turn (0.01 degrees).
const TURN_EPSILON: f32 = 0.000_174_532_92;
/// it_803B85A8[0] and [1]: the model's X and Z axes before its facing turn.
const MODEL_X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const MODEL_Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);
/// it_803B85A8[2..=5]: up, the default target and the model's own up.
const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

pub(crate) fn monster(item: &ItemCore) -> &MonsterState {
    match &item.scratch {
        ItemScratch::Monster(state) => state,
        _ => panic!("monster scratch"),
    }
}
pub(crate) fn monster_mut(item: &mut ItemCore) -> &mut MonsterState {
    match &mut item.scratch {
        ItemScratch::Monster(state) => state,
        _ => panic!("monster scratch"),
    }
}

/// The model's root angle for a facing: retail 0x8027C668 fmul, 0x8027C66C
/// frsp. M_PI_2 is a double, so the product is rounded once.
pub(crate) fn facing_angle(facing: f32) -> f32 {
    (core::f64::consts::FRAC_PI_2 * f64::from(facing)) as f32
}

/// it_8027C56C (8027C56C): the model upright and turned to `facing`; the
/// floor it leans toward is up again.
pub(crate) fn face(item: &mut ItemCore, facing: f32) {
    item.rotation = Vec3::new(0.0, facing_angle(facing), 0.0);
    let state = monster_mut(item);
    state.floor_normal = UP;
    state.turn_frames = 0;
}

/// it_8027C0F0 (8027C0F0): on a floor, the model leans toward `normal`
/// (up for NULL), reaching it over `frames` frames.
pub(crate) fn lean_toward(item: &mut ItemCore, normal: Option<Vec3>, facing: f32, frames: f32) {
    if !on_floor(item) {
        return;
    }
    lean(item, normal.unwrap_or(UP), facing, frames);
}

/// it_8027C0A8 -> it_8027BBF4(gobj, false, ..) (8027BBF4): on a floor, the
/// model leans toward the floor's normal over `frames` frames (retail
/// 0x8027BDF0..BE28 fuse as it_8027C0F0 does).
pub(crate) fn lean_with_floor(item: &mut ItemCore, facing: f32, frames: f32) {
    if !on_floor(item) {
        return;
    }
    let normal = item.collision.as_ref().expect("item map collision").floor.normal;
    lean(item, normal, facing, frames);
}

fn on_floor(item: &ItemCore) -> bool {
    item.collision
        .as_ref()
        .is_some_and(|c| c.env_flags as u32 & FLOOR_MASK != 0)
}

/// The body of it_8027C0F0, which it_8027BBF4 shares.
fn lean(item: &mut ItemCore, target: Vec3, facing: f32, frames: f32) {
    let up = rotate_by_angles(UP, item.rotation);
    let state = monster_mut(item);
    if angle_xy(state.floor_normal, target) > TURN_EPSILON {
        state.floor_normal = target;
        state.turn_frames = fctiwz(frames);
    }
    let remaining = angle_xy(up, state.floor_normal);
    let (x, y) = if remaining < TURN_EPSILON || state.turn_frames < 2 {
        (target.x, target.y)
    } else {
        let mut step = remaining / state.turn_frames as f32;
        state.turn_frames -= 1;
        // retail 0x8027C2B0/B4: fmuls, fmsubs.
        if fmsubs(up.x, target.y, up.y * target.x) < 0.0 {
            step = -step;
        }
        let (c, s) = (cosf(step), sinf(step));
        // retail 0x8027C2E0..EC: fmuls, fmuls, fmsubs, fmadds.
        (fmsubs(up.x, c, up.y * s), fmadds(up.x, s, up.y * c))
    };
    // retail 0x8027C300..14: fmul by the double M_PI_2, frsp once for
    // both lbVector_Rotate calls; the turned X axis is overwritten unread.
    let turn = facing_angle(facing);
    let forward = rotate_about(MODEL_Z, Axis::Y, turn);
    let _ = rotate_about(MODEL_X, Axis::Y, turn);
    let normal = Vec3::new(x, y, 0.0);
    // lbVector_CrossprodNormalized twice, then the basis' Euler angles.
    let side = cross(normal, forward);
    let forward = cross(side, normal);
    item.rotation = melee_lb::orientation::from_basis(forward, side, normal);
}

/// it_8027BB1C (8027BB1C): `v` turned about X, Y, then Z by `angles`, each
/// with MSL cosf/sinf (retail 0x8027BB58..BBCC: fmuls pairs, then
/// fmsubs/fmadds).
fn rotate_by_angles(v: Vec3, angles: Vec3) -> Vec3 {
    let (c, s) = (cosf(angles.x), sinf(angles.x));
    let v = Vec3::new(v.x, fmsubs(v.y, c, v.z * s), fmadds(v.y, s, v.z * c));
    let (c, s) = (cosf(angles.y), sinf(angles.y));
    let v = Vec3::new(fmadds(v.x, c, v.z * s), v.y, fmsubs(v.z, c, v.x * s));
    let (c, s) = (cosf(angles.z), sinf(angles.z));
    Vec3::new(fmsubs(v.x, c, v.y * s), fmadds(v.x, s, v.y * c), v.z)
}

/// it_8027C8D0 (8027C8D0): `velocity`'s speed along the floor with normal
/// `normal`, toward `facing`; none at a zero facing. The tangent's tiny
/// normal component (6.8074177e-9) is retail's.
pub(crate) fn along_floor(velocity: Vec3, normal: Vec3, facing: f32) -> Vec3 {
    // retail 0x8027C8DC..F4: three fmuls, (x + y) then z, unfused.
    let speed = sqrtf(velocity.z * velocity.z + (velocity.x * velocity.x + velocity.y * velocity.y));
    const NORMAL_LEAK: f32 = 6.807_417_7e-9;
    let direction = if facing > 0.0 {
        -1.0
    } else if facing < 0.0 {
        1.0
    } else {
        return Vec3::ZERO;
    };
    // retail 0x8027C9A4..C0: fmuls, fmuls, fmsubs, fmuls; fmadds, fmuls.
    Vec3::new(
        speed * fmsubs(normal.x, NORMAL_LEAK, normal.y * direction),
        speed * fmadds(normal.x, direction, normal.y * NORMAL_LEAK),
        0.0,
    )
}

/// it_8027CAD8 (8027CAD8): stopped (X and Y only), striking its owner and
/// kindred items again (it_8027542C, then it_80275444), the spin reset
/// (it_80274740), unowned, and catchable by a fighter's grab (it_80275270).
pub(crate) fn halt(item: &mut ItemCore) {
    item.velocity.y = 0.0;
    item.velocity.x = 0.0;
    item.strikes_kindred_items = false;
    item.hits_owner = true;
    item.strikes_kindred_items = true;
    item.reset_spin();
    item.owner = None;
    item.catchable = true;
}

/// it_8027C9D8 (8027C9D8): one of four hurt voices (0x131..0x134) by
/// HSD_Randi(4), drawn again until it differs from the last.
pub(crate) fn hurt_voice(item: &mut ItemCore, rng: &core::cell::Cell<gekko_math::HsdRng>) {
    const VOICES: [i32; 4] = [0x131, 0x132, 0x133, 0x134];
    let mut r = rng.get();
    let mut voice = VOICES[r.randi(4) as usize];
    let last = monster(item).last_voice;
    while voice == last {
        voice = VOICES[r.randi(4) as usize];
    }
    rng.set(r);
    monster_mut(item).last_voice = voice;
    item.sound_requests.push(voice as u32);
}

/// it_8027B964 (8027B964): a launch at it_804D6D40 +4 along the hit's
/// knockback (it_8027B798; a still one goes up and away from the hitter
/// at 0.1, retail 8027B9B4..B8: fneg, fmuls) or, `bounced`, along the
/// velocity it_8027781C mirrored off what it touched.
pub(crate) fn launch(
    item: &mut ItemCore,
    common: &MonsterCommonData,
    launch: &melee_it::hurt::ItemLaunch,
) {
    let (velocity, _) = item.knockback_launch(launch);
    item.velocity = velocity;
    if item.velocity.x == 0.0 && item.velocity.y == 0.0 {
        item.velocity.x = 0.1 * -item.hit_direction;
        item.velocity.y = 0.1;
    }
    if item.velocity.y == 0.0 {
        item.velocity.y = 0.1;
    }
    scale_launch(item, common);
}

/// it_8027B964's tail: lbVector_Normalize, then fmuls by it_804D6D40 +4.
pub(crate) fn scale_launch(item: &mut ItemCore, common: &MonsterCommonData) {
    let v = melee_lb::vector::normalize(item.velocity);
    let speed = common.launch_speed;
    item.velocity = Vec3::new(v.x * speed, v.y * speed, v.z * speed);
}

/// it_8027CBFC (8027CBFC): the damage a hit's element adds: elements 6 and
/// 7 (Nap, Sleep) it_804D6D40 +C, element 9 (Ground) +10, each truncated
/// to an int (fctiwz) and back.
pub(crate) fn element_damage(item: &ItemCore, common: &MonsterCommonData) -> f32 {
    let extra = match item.hit_element {
        HitElement::Nap | HitElement::Sleep => fctiwz(common.sleep_damage),
        HitElement::Ground => fctiwz(common.ground_damage),
        _ => 0,
    };
    extra as f32
}
