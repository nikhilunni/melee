//! Reachable Wait, Fall, EntryStart and Landing command interpreter, ftAction_80073240 (0x80073240).
//! Archive pointers are converted to instruction indices by assets.rs.
use super::{assets::FighterAssets, RetailTrig};
use crate::{
    anim::{attach::PartFlags, FighterAnimation},
    collision::pose::GroundPoseFlags,
};
use hsd_anim::{
    aobj::AObjEndCallback,
    jobj::{JObjTree, JOBJ_USE_QUATERNION},
};

#[derive(Clone, Debug)]
pub enum Command {
    End,
    LandingEffect(u16),
    Wait(f32),
    AtFrame(f32),
    Call {
        target: usize,
        continuation: usize,
    },
    Return,
    Part {
        group: usize,
        variant: usize,
        blend: f32,
    },
    GroundPose(u8),
    Texture {
        indices: Vec<usize>,
        frame: f32,
    },
}
#[derive(Clone, Debug, Default)]
pub struct CommandState {
    /// x3E4_fighterCmdScript.u (+3EC); index, not a retail address.
    pub instruction: Option<usize>,
    /// CommandInfo.timer, Fighter +3E4.
    pub timer: f32,
    /// CommandInfo.frame_count, Fighter +3E8.
    pub frame: f32,
    /// CommandInfo.event_return / loop_count (+3F4/+3F0).
    pub return_stack: Vec<usize>,
    /// Requests to costume TObjs (ftAnim_800704F0). Rendering consumes these;
    /// TObj/GX execution remains M8, like the existing HSD model loader.
    pub texture_frames: Vec<(usize, f32)>,
    /// ftAction_80072E4C requests, resolved at the calling proc boundary.
    pub landing_effects: Vec<u16>,
}
impl CommandState {
    pub fn restart(&mut self, instruction: usize) {
        self.instruction = Some(instruction);
        self.timer = 0.0;
        self.return_stack.clear();
    }
    /// ftaction.c:1318-1348; retail --fused has no multiply-add sites.
    pub fn step(
        &mut self,
        animation: &mut FighterAnimation,
        tree: &mut JObjTree,
        pose: &mut GroundPoseFlags,
        assets: &FighterAssets,
    ) {
        self.frame = animation.frame + animation.remainder;
        if self.instruction.is_none() {
            return;
        }
        self.timer -= animation.speed;
        while self.timer <= 0.0 {
            let Some(pc) = self.instruction else { break };
            self.instruction = Some(pc + 1);
            match &assets.commands[pc] {
                Command::End => self.instruction = None,
                Command::LandingEffect(id) => self.landing_effects.push(*id),
                Command::Wait(frames) => self.timer += frames,
                Command::AtFrame(frame) => self.timer = frame - self.frame,
                Command::Call {
                    target,
                    continuation,
                } => {
                    self.return_stack.push(*continuation);
                    self.instruction = Some(*target);
                }
                Command::Return => {
                    self.instruction = Some(
                        self.return_stack
                            .pop()
                            .expect("command return without call"),
                    );
                }
                Command::GroundPose(flags) => {
                    pose.0 = *flags;
                    if flags & 4 == 0 {
                        tree.set_rotation_x(animation.root, 0.0);
                    }
                }
                Command::Part {
                    group,
                    variant,
                    blend,
                } => apply_part(animation, tree, assets, *group, *variant, *blend),
                Command::Texture { indices, frame } => {
                    for &index in indices {
                        if let Some(entry) =
                            self.texture_frames.iter_mut().find(|(i, _)| *i == index)
                        {
                            entry.1 = *frame;
                        } else {
                            self.texture_frames.push((index, *frame));
                        }
                    }
                }
            }
        }
    }
}

/// ftAnim_ApplyPartAnim (0x80070B0C), ftanim.c:1257-1278, and
/// ftAnim_80070904 (0x80070904), ftanim.c:1165-1185.
fn apply_part(
    animation: &mut FighterAnimation,
    _tree: &mut JObjTree,
    assets: &FighterAssets,
    group: usize,
    variant: usize,
    blend: f32,
) {
    let resource = &assets.part_animations[&(group, variant)];
    let state = &mut animation.part_animations[group];
    state.current = variant as i8;
    state.duration = blend;
    state.progress = 0.0;
    // ftAnim_ApplyPartAnim: fdivs after lb_8000BFF0; no eligible FMA.
    state.rate = if blend != 0.0 {
        resource.duration / blend
    } else {
        0.0
    };
    state.joints = assets.bones.animation_sets[group]
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .map(|&x| usize::from(x))
        .collect();
    let mut index = resource.root;
    for node in &resource.nodes {
        while animation.parts[index].motion_mask != 0 {
            index += 1;
        }
        let part = &mut animation.parts[index];
        if !part.flags.contains(PartFlags::LOCKED) && node.aobjdesc.is_some() {
            let joint = part.joint;
            animation.blend_tree.add_anim(joint, Some(node), None);
            animation.blend_tree.clear_flags(joint, JOBJ_USE_QUATERNION);
            animation.blend_tree.req_anim(joint, 0.0);
            animation
                .blend_tree
                .anim::<RetailTrig>(joint, &mut AObjEndCallback::default());
            part.flags.0 |= PartFlags::PART_ANIMATION;
        }
        index += 1;
    }
}
