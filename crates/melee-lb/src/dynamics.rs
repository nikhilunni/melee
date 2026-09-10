//! Dynamic joint chains, `lb/lb_00F9.c`, retail 0x8001044C.
//! Matrices use the SDK paired-single kernels; spring rotations deliberately
//! use lbvector's polynomial trig, not the MSL trig used by quaternions.
use gekko_math::{
    fma::{fmadds, fmsubs, fnmsubs},
    msl::{sinf, sqrtf},
};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    mtx,
    quat::{self, Quaternion},
};
use hsd_types::{Mtx, Vec3};

pub mod arithmetic;
use arithmetic::{angle, cross, difference, euler_matrix, normalize, rotate};

/// ftData's 0x3C-byte `lb_00F9_UnkDesc1Inner`, applied by lb_80011710.
#[derive(Clone, Debug)]
pub struct SpringParameters {
    pub stiffness: f32,
    pub convergence: f32,
    pub natural_rotation: Quaternion,
    pub max_deviation: f32,
    pub rotation_max: Vec3,
    pub rotation_min: Vec3,
    pub damping: f32,
    pub max_step: f32,
}

/// Owned `DynamicsData` / `lb_00F9_UnkDesc0` (lb/types.h).
#[derive(Clone, Debug)]
pub struct BoneSpring {
    pub joint: JObjId,
    pub rest_rotation: Quaternion,
    pub rest_translate: Vec3,
    pub rest_scale: Vec3,
    pub position: Vec3,
    pub velocity_axis: Vec3,
    pub angular_velocity: f32,
    pub length: f32,
    /// -1 = Z, 0 = X, otherwise Y; determined by the child's rest translation.
    pub dominant_axis: i32,
    pub gravity: f32,
    pub parameters: SpringParameters,
}

#[derive(Clone, Debug)]
pub struct DynamicBoneSet {
    pub bones: Vec<BoneSpring>,
    /// DynamicsDesc.pos; X scales stiffness, Z sets gravity per link length.
    pub multipliers: Vec3,
}

pub struct Collider {
    pub position: Vec3,
    pub radius: f32,
}

/// Scene interaction field from lb_800100B0 / lb_800101C8. The scene owns
/// allocation, lifetime and order; querying a field does not consume RNG.
#[derive(Clone, Copy, Debug, Default)]
pub struct ForceField {
    pub direction_or_center: Vec3,
    pub rectangle: Option<[f32; 4]>,
    pub strength: f32,
    pub phase: i32,
    pub phase_step: f32,
}

pub struct SolverEnvironment<'a> {
    /// Global lb_804D63B8 pause gate.
    pub disabled: bool,
    pub colliders: &'a [Collider],
    pub forces: &'a [ForceField],
    pub first_force_bone: usize,
    pub ground_check: bool,
}

struct RetailTrig;
impl mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        crate::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        crate::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        crate::trigf::acosf(x)
    }
}

impl DynamicBoneSet {
    /// lb_8000FD48 (0x8000FD48) and lb_80011710 (0x80011710).
    pub fn new(
        tree: &mut JObjTree,
        root: JObjId,
        parameters: &[SpringParameters],
        multipliers: Vec3,
    ) -> Self {
        let mut joint = Some(root);
        let mut bones = Vec::new();
        for parameters in parameters {
            let id = joint.expect("dynamics descriptor exceeds child chain");
            tree.set_mtx_dirty(id);
            tree.setup_matrix(id);
            let node = tree.get(id);
            bones.push(BoneSpring {
                joint: id,
                rest_rotation: node.rotate,
                rest_translate: node.translate,
                rest_scale: node.scale,
                position: translation(&node.mtx),
                velocity_axis: Vec3::new(1.0, 0.0, 0.0),
                angular_velocity: 0.0,
                length: 0.0,
                dominant_axis: 0,
                gravity: 0.0,
                parameters: parameters.clone(),
            });
            joint = tree.child(id);
        }
        for i in 0..bones.len().saturating_sub(1) {
            let delta = difference(bones[i].position, bones[i + 1].position);
            // retail 0x8000FFA8/FFAC: fmadds, Y squared first.
            bones[i].length = sqrtf(fmadds(
                delta.z,
                delta.z,
                fmadds(delta.x, delta.x, delta.y * delta.y),
            ));
            let t = bones[i + 1].rest_translate;
            bones[i].dominant_axis = if t.z.abs() > t.y.abs() {
                if t.z.abs() > t.x.abs() {
                    -1
                } else {
                    0
                }
            } else if t.y.abs() > t.x.abs() {
                1
            } else {
                0
            };
            if bones[i].length != 0.0 {
                bones[i].gravity = multipliers.z / bones[i].length;
            }
        }
        Self { bones, multipliers }
    }

    /// lb_8001044C (0x8001044C, size 0x11A8). Floor queries are injected to
    /// preserve the lb -> mp layering; return the retail intersection point.
    pub fn solve(
        &mut self,
        tree: &mut JObjTree,
        first: u32,
        env: &SolverEnvironment<'_>,
        floor: &mut impl FnMut(Vec3, Vec3) -> Option<Vec3>,
    ) {
        if env.disabled || first > 0xFF || self.bones.is_empty() {
            return;
        }
        let first = first as usize;
        assert!(first < self.bones.len());
        let mut parent = if let Some(id) = tree.parent(self.bones[first].joint) {
            tree.set_mtx_dirty(id);
            tree.setup_matrix(id);
            tree.get(id).mtx
        } else {
            Mtx::IDENTITY
        };
        let mut on_ground = false;
        for i in first..self.bones.len() - 1 {
            let next_position = self.bones[i + 1].position;
            let bone = &mut self.bones[i];
            let node = tree.get(bone.joint);
            let child = tree.child(bone.joint).expect("spring has no child");
            let child_translation = tree.get(child).translate;
            let mut trans = Mtx::IDENTITY;
            mtx::mtx_trans(
                &mut trans,
                node.translate.x,
                node.translate.y,
                node.translate.z,
            );
            let translated = concat(&parent, &trans);
            let mut natural = concat(&translated, &euler_matrix(bone.parameters.natural_rotation));
            let mut current = concat(&translated, &euler_matrix(node.rotate));
            let mut scale = Mtx::IDENTITY;
            mtx::mtx_scale(&mut scale, node.scale.x, node.scale.y, node.scale.z);
            current = concat(&current, &scale);
            natural = concat(&natural, &scale);
            bone.position = translation(&current);
            let natural_dir = normalize(difference(
                transform(&natural, child_translation),
                bone.position,
            ));
            let current_dir = normalize(difference(
                transform(&current, child_translation),
                bone.position,
            ));
            let saved_dir = normalize(difference(next_position, bone.position));
            let mut link = spring_direction(
                bone,
                self.multipliers.x,
                natural_dir,
                current_dir,
                saved_dir,
                env.forces,
                i >= env.first_force_bone,
                next_position,
            );
            avoid_colliders(bone, &mut link, env.colliders);
            if env.ground_check {
                ground_collision(bone, &mut link, &mut on_ground, floor);
            }
            let angle_diff = angle(current_dir, link);
            let axis = cross(current_dir, link);
            link = rotate(current_dir, axis, angle_diff);
            bone.angular_velocity = if on_ground {
                0.0
            } else {
                angle(saved_dir, link)
            };
            if bone.angular_velocity.abs() > 0.0 {
                bone.velocity_axis = cross(saved_dir, link);
            }
            let damping = bone.parameters.damping;
            if bone.angular_velocity != 0.0 {
                bone.angular_velocity = if bone.angular_velocity > damping {
                    bone.angular_velocity - damping
                } else if bone.angular_velocity < -damping {
                    bone.angular_velocity + damping
                } else {
                    0.0
                };
            }
            apply_rotation(tree, bone, angle_diff, axis, &parent);
            parent = concat(&parent, &trans);
            parent = concat(&parent, &euler_matrix(tree.get(bone.joint).rotate));
            parent = concat(&parent, &scale);
        }
        let last = self.bones.last_mut().unwrap();
        let node = tree.get(last.joint);
        let mut temp = Mtx::IDENTITY;
        mtx::mtx_trans(
            &mut temp,
            node.translate.x,
            node.translate.y,
            node.translate.z,
        );
        parent = concat(&parent, &temp);
        parent = concat(&parent, &euler_matrix(node.rotate));
        mtx::mtx_scale(&mut temp, node.scale.x, node.scale.y, node.scale.z);
        parent = concat(&parent, &temp);
        last.position = translation(&parent);
    }
}

/// Pure force/constraint portion of one lb_8001044C bone step.
#[allow(clippy::too_many_arguments)]
pub fn spring_direction(
    bone: &BoneSpring,
    stiffness_scale: f32,
    natural: Vec3,
    current: Vec3,
    saved: Vec3,
    forces: &[ForceField],
    apply_force: bool,
    next_position: Vec3,
) -> Vec3 {
    let mut link = saved;
    let stiffness = bone.parameters.stiffness * stiffness_scale;
    if f64::from(stiffness) < 1.0 {
        let a = angle(link, current).abs();
        if a != 0.0 {
            link = rotate(
                link,
                cross(link, current),
                a * (1.0 - f64::from(stiffness)) as f32,
            );
        }
    }
    let gravity_dir = Vec3::new(0.0, -1.0, 0.0);
    let a = angle(link, gravity_dir);
    if a != 0.0 {
        link = rotate(
            link,
            cross(link, gravity_dir),
            (bone.gravity * sinf(a)).abs(),
        );
    }
    let mut force_mag = 0.0;
    if !forces.is_empty() && apply_force {
        let (magnitude, direction) = force_at(forces, next_position);
        force_mag = magnitude;
        let a = angle(link, direction);
        if a != 0.0 {
            link = rotate(
                link,
                cross(link, direction),
                ((magnitude * sinf(a)) / bone.length).abs(),
            );
        }
    }
    if bone.angular_velocity != 0.0 {
        link = rotate(link, bone.velocity_axis, bone.angular_velocity);
    }
    if force_mag < bone.gravity * bone.length && angle(saved, link) > bone.parameters.max_step {
        link = rotate(saved, cross(saved, link), bone.parameters.max_step);
    }
    if bone.parameters.convergence > 0.0 {
        link = if angle(natural, link) < bone.parameters.convergence {
            natural
        } else {
            rotate(link, cross(link, natural), bone.parameters.convergence)
        };
    }
    let deviation = angle(natural, link);
    if deviation > bone.parameters.max_deviation {
        link = rotate(
            link,
            cross(link, natural),
            deviation - bone.parameters.max_deviation,
        );
    }
    link
}

fn apply_rotation(tree: &mut JObjTree, bone: &BoneSpring, angle: f32, axis: Vec3, parent: &Mtx) {
    if !near_zero(angle) {
        let mut transpose = Mtx::IDENTITY;
        mtx::mtx_transpose(parent, &mut transpose);
        let local_axis = transform(&transpose, axis);
        if !(near_zero(local_axis.x) && near_zero(local_axis.y) && near_zero(local_axis.z)) {
            let mut a = Quaternion::default();
            quat::quat_from_axis_angle(&local_axis, &mut a, angle);
            let r = tree.get(bone.joint).rotate;
            let mut b = Quaternion::default();
            quat::euler_to_quat(&Vec3::new(r.x, r.y, r.z), &mut b);
            let mut result = Quaternion::default();
            quat::quat_mul(&a, &b, &mut result);
            let mut matrix = Mtx::IDENTITY;
            mtx::mtx_quat(&mut matrix, &result);
            let mut euler = Vec3::ZERO;
            quat::mtx_to_euler::<RetailTrig>(&matrix, &mut euler);
            // C initializes only xyz; w is unused in Euler mode. Preserve it.
            tree.set_rotation(bone.joint, &Quaternion::new(euler.x, euler.y, euler.z, r.w));
            tree.clear_flags_all(bone.joint, 0x20000);
        }
    }
    let r = tree.get(bone.joint).rotate;
    match bone.dominant_axis {
        -1 => tree.set_rotation_z(bone.joint, r.z * 0.9),
        0 => tree.set_rotation_x(bone.joint, r.x * 0.9),
        _ => tree.set_rotation_y(bone.joint, r.y * 0.9),
    }
}

fn avoid_colliders(bone: &BoneSpring, link: &mut Vec3, colliders: &[Collider]) {
    if colliders.is_empty() {
        return;
    }
    *link = normalize(*link);
    for collider in colliders {
        let end = endpoint(bone.position, *link, bone.length);
        let to_collider = difference(collider.position, bone.position);
        // retail 80010B30..B4C: separate squares/adds, sqrt refinements B78/B88/B98.
        let distance = sqrtf(
            to_collider.z * to_collider.z
                + (to_collider.x * to_collider.x + to_collider.y * to_collider.y),
        );
        if distance > collider.radius
            && segment_sphere(bone.position, end, collider.position, collider.radius)
        {
            let a = angle(to_collider, *link);
            if a != 0.0 {
                let radius = (0.1 + f64::from(collider.radius)) as f32;
                // retail 80010C04: fmsubs.
                let side = sqrtf(fmsubs(distance, distance, radius * radius));
                let avoidance = crate::trigf::atan2f(radius, side).abs() - a;
                if avoidance > 0.0 {
                    *link = rotate(*link, cross(to_collider, *link), avoidance);
                }
            }
        }
    }
}

/// lbColl_80005C44 (0x80005C44), segment vs inflated sphere.
fn segment_sphere(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> bool {
    let radius = 0.1 + radius;
    for (a, b, c) in [
        (start.x, end.x, center.x),
        (start.y, end.y, center.y),
        (start.z, end.z, center.z),
    ] {
        if a > b {
            if a + radius < c || b - radius > c {
                return false;
            }
        } else if a - radius > c || b + radius < c {
            return false;
        }
    }
    let d = difference(end, start);
    let offset = difference(start, center);
    // retail 80005DDC/DDE8, DDF0/DDF8: fmadds, X squared/product first.
    let square = fmadds(d.z, d.z, fmadds(d.y, d.y, d.x * d.x));
    let dot = fmadds(d.z, offset.z, fmadds(d.y, offset.y, d.x * offset.x));
    let amount = if near_zero(square) {
        0.0
    } else {
        (gekko_math::fma::negate_rounded(dot) / square).clamp(0.0, 1.0)
    };
    // retail 80005E5C/E60/E64: fmadds.
    let nearest = endpoint(start, d, amount);
    let d = difference(nearest, center);
    // retail 80005E98/E9C: fmadds.
    radius * radius >= fmadds(d.z, d.z, fmadds(d.y, d.y, d.x * d.x))
}

fn ground_collision(
    bone: &BoneSpring,
    link: &mut Vec3,
    on_ground: &mut bool,
    floor: &mut impl FnMut(Vec3, Vec3) -> Option<Vec3>,
) {
    let down = Vec3::new(0.0, -1.0, 0.0);
    *link = normalize(*link);
    if *on_ground {
        let end = endpoint(bone.position, *link, bone.length);
        let hit = floor(
            Vec3::new(end.x, (1.0 + f64::from(end.y)) as f32, 0.0),
            Vec3::new(end.x, (f64::from(end.y) - 1.0) as f32, 0.0),
        );
        *link = rotate(down, cross(down, *link), std::f32::consts::FRAC_PI_2);
        *on_ground = hit.is_some();
    } else {
        let start = bone.position;
        if floor(
            Vec3::new(start.x, (1.0 + f64::from(start.y)) as f32, 0.0),
            start,
        )
        .is_some()
        {
            *link = rotate(down, cross(down, *link), std::f32::consts::FRAC_PI_2);
            *on_ground = true;
        } else if let Some(hit) = floor(start, endpoint(start, *link, bone.length)) {
            let height = (start.y - hit.y).abs();
            // retail 80010F3C: fnmsubs.
            let horizontal = sqrtf(fnmsubs(height, height, bone.length * bone.length));
            let a = crate::trigf::atan2f(horizontal, height).abs();
            *link = rotate(down, cross(down, *link), a);
            *on_ground = true;
        } else {
            *on_ground = false;
        }
    }
}

/// lb_800103D8 (0x800103D8), fighter-height plane intersection.
pub fn floor_plane(start: Vec3, end: Vec3, height: f32) -> Option<Vec3> {
    let a = start.y - height;
    let b = end.y - height;
    if a == b {
        return (a > 1e-10).then_some(Vec3::ZERO);
    }
    if a > 0.0 && b < 0.0 {
        // retail 80010428: fmadds.
        Some(Vec3::new(
            fmadds(-b / (a - b), start.x - end.x, end.x),
            height,
            0.0,
        ))
    } else {
        None
    }
}

/// lb_800101C8 (0x800101C8), also exported by lb_800103B8.
pub fn force_at(fields: &[ForceField], position: Vec3) -> (f32, Vec3) {
    let mut total = Vec3::ZERO;
    for field in fields {
        let phase = field.phase as f32 * field.phase_step;
        let scale = (0.5
            * f64::from(field.strength)
            * (1.0 + f64::from(gekko_math::msl::cosf(phase)))) as f32;
        if let Some([left, top, right, bottom]) = field.rectangle {
            if position.x > left && position.x < right && position.y < top && position.y > bottom {
                // retail 800102B0/C0/D0: fmadds.
                total = endpoint(total, field.direction_or_center, scale);
            }
        } else {
            let delta = difference(position, field.direction_or_center);
            let length = arithmetic::length(delta);
            let direction = normalize(delta);
            let distance = ((0.05 * f64::from(length)) as f32).max(1.0);
            let attenuation = (1.0 / f64::from(distance * distance)) as f32;
            // retail 80010340/354/368: fmadds; scale direction first.
            total.x = fmadds(attenuation, direction.x * scale, total.x);
            total.y = fmadds(attenuation, direction.y * scale, total.y);
            total.z = fmadds(attenuation, direction.z * scale, total.z);
        }
    }
    (arithmetic::length(total), normalize(total))
}

fn near_zero(x: f32) -> bool {
    x < 0.00001 && x > -0.00001
}
fn concat(a: &Mtx, b: &Mtx) -> Mtx {
    let mut m = Mtx::IDENTITY;
    mtx::mtx_concat(a, b, &mut m);
    m
}
fn translation(m: &Mtx) -> Vec3 {
    Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3])
}
fn transform(m: &Mtx, v: Vec3) -> Vec3 {
    let mut out = Vec3::ZERO;
    mtx::mtx_mult_vec(m, &v, &mut out);
    out
}
fn endpoint(start: Vec3, dir: Vec3, length: f32) -> Vec3 {
    // Solver retail 80010AE4/AF8/B0C, CF8/CFC, EC4/EC8: fmadds.
    Vec3::new(
        fmadds(dir.x, length, start.x),
        fmadds(dir.y, length, start.y),
        fmadds(dir.z, length, start.z),
    )
}
