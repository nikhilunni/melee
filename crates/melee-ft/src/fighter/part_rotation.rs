//! ftPartSetRotX/Y/Z (ftparts.c:993-1041): one Euler angle of a fighter part.
use super::FighterCore;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl FighterCore {
    /// ftPartSetRotX (8007592C) and its Y/Z siblings: a quaternion main
    /// joint redirects to FighterBone.x4_jobj2 (the blend skeleton's joint),
    /// whose rotation must be Euler.
    pub fn set_part_rotation(&mut self, part: usize, axis: Axis, angle: f32) {
        let joint = self.animation.parts[part].joint;
        let tree = if self.skeleton.get(joint).flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0 {
            &mut self.animation.blend_tree
        } else {
            &mut self.skeleton
        };
        match axis {
            Axis::X => tree.set_rotation_x(joint, angle),
            Axis::Y => tree.set_rotation_y(joint, angle),
            Axis::Z => tree.set_rotation_z(joint, angle),
        }
    }
}
