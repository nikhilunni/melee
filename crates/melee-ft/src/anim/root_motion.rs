//! Translation extraction in `ftAnim_8006E054` (0x8006E054).
use super::playback::MotionFlags;
use hsd_anim::aobj::AObjEndCallback;
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_anim::mtx::InverseTrig;
use hsd_types::Vec3;

#[derive(Clone, Debug, Default)]
pub struct TranslationHistory {
    /// x68C_transNPos / x6C0.
    pub position: Vec3,
    /// x698 / x6CC.
    pub previous: Vec3,
    /// x6A4_transNOffset / x6D8.
    pub offset: Vec3,
    /// x6B0 / x6E4.
    pub previous_offset: Vec3,
}
impl TranslationHistory {
    fn sample(&mut self, position: Vec3, scale: f32) {
        self.previous = self.position;
        self.position = Vec3::new(position.x * scale, position.y * scale, position.z * scale);
        self.previous_offset = self.offset;
        self.offset = difference(self.position, self.previous);
    }
}

#[derive(Clone, Debug)]
pub struct RootMotion {
    pub translation: JObjId,
    pub secondary: JObjId,
    pub primary_history: TranslationHistory,
    pub secondary_history: TranslationHistory,
    /// Result of ftCommon_GetModelScale; supplied by the fighter size system.
    pub effective_scale: f32,
    /// x2221_b2 && !x2226_b2: subtract extracted motion from this model joint.
    pub compensate_joint: Option<JObjId>,
}
impl RootMotion {
    /// No fused sites in retail (`asm.py ftAnim_8006E054 --fused`).
    pub fn animate<T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        root: JObjId,
        flags: MotionFlags,
        model_scale: f32,
    ) {
        let scale = self.scale(flags, model_scale);
        self.evaluate::<T>(tree, root, flags, scale, true);
        self.compensate(tree, scale);
    }

    /// The blend call at 0x8006EB08 passes the PRIMARY skeleton's secondary
    /// joint as arg3, although it traverses the blend skeleton. Pointer
    /// equality therefore never extracts the secondary joint in this path.
    /// Compensation at the end of E054 also targets the primary skeleton.
    pub fn animate_blend<T: InverseTrig>(
        &mut self,
        blend_tree: &mut JObjTree,
        primary_tree: &mut JObjTree,
        root: JObjId,
        flags: MotionFlags,
        model_scale: f32,
    ) {
        let scale = self.scale(flags, model_scale);
        self.evaluate::<T>(blend_tree, root, flags, scale, false);
        self.compensate(primary_tree, scale);
    }

    fn scale(&self, flags: MotionFlags, model_scale: f32) -> f32 {
        if flags.contains(MotionFlags::ATTRIBUTE_SCALE) {
            model_scale
        } else {
            self.effective_scale
        }
    }

    fn evaluate<T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        root: JObjId,
        flags: MotionFlags,
        scale: f32,
        secondary_in_tree: bool,
    ) {
        let mut cb = AObjEndCallback::default();
        let mut next = Some(root);
        while let Some(id) = next {
            next = tree.next_depth_first(id);
            tree.anim::<T>(id, &mut cb);
            if id == self.translation {
                self.primary_history.sample(tree.translation(id), scale);
                tree.set_translate(id, &Vec3::ZERO);
            } else if secondary_in_tree
                && flags.contains(MotionFlags::SECOND_ROOT)
                && id == self.secondary
            {
                self.secondary_history.sample(tree.translation(id), scale);
                tree.set_translate(id, &Vec3::ZERO);
            }
        }
        if flags.contains(MotionFlags::SECOND_ROOT) {
            tree.set_translate(
                self.translation,
                &difference(
                    self.primary_history.position,
                    self.secondary_history.position,
                ),
            );
            self.primary_history = self.secondary_history.clone();
        }
    }

    fn compensate(&self, tree: &mut JObjTree, scale: f32) {
        if let Some(id) = self.compensate_joint {
            // scale_inline: reciprocal first, then three fmuls; do not divide components.
            let reciprocal = 1.0 / scale;
            let position = self.primary_history.position;
            let unscaled = Vec3::new(
                position.x * reciprocal,
                position.y * reciprocal,
                position.z * reciprocal,
            );
            tree.set_translate(id, &difference(tree.translation(id), unscaled));
        }
    }
}
fn difference(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
