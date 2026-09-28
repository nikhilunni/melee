//! Dynamic-joint subaction ownership, ftCo_8009E318 (8009E318).
use super::assets::FighterAssets;
use super::FighterCore;
use crate::anim::attach::PartFlags;
impl FighterCore {
    pub(super) fn apply_dynamic_commands(&mut self, assets: &FighterAssets) {
        for bone in self.commands.dynamic_toggles.take_all() {
            let part = &mut self.animation.parts[bone];
            let joint = part.joint;
            let Some((set, index)) = self.dynamics.iter().enumerate().find_map(|(set, data)| {
                data.bones
                    .iter()
                    .position(|b| b.joint == joint)
                    .map(|index| (set, index))
            }) else {
                continue;
            };
            if part.flags.contains(PartFlags::LOCKED) {
                part.flags.0 &= !PartFlags::LOCKED;
                self.dynamics_first_bone[set] = (index + 1) as u32;
                if self.animation.motion_id >= 0 {
                    self.animation
                        .resume_dynamic_subtree::<super::RetailTrig>(
                            &mut self.skeleton,
                            bone,
                            &assets.motions[&self.animation.motion_id],
                        )
                        .expect("dynamic subtree animation");
                }
            } else {
                part.flags.0 |= PartFlags::LOCKED;
                self.dynamics_first_bone[set] = index as u32;
                self.skeleton.remove_anim_all_by_flags(joint, 1);
                self.animation.blend_tree.remove_anim_all_by_flags(joint, 1);
            }
        }
    }
}
