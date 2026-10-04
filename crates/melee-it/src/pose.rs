//! An article's joint poses for kinds whose simulation reads a joint below
//! the root (xBBC_dynamicBoneTable->bones[i] through lb_8000B1CC, and
//! hitboxes on such a joint, it_8027129C).
//!
//! Below the root every joint's local transform is a pure function of the
//! animation steps taken since the article state began (Item_80268D34
//! restores the rest pose and restarts at frame zero), so each state's
//! locals are sampled once at load by playing the animation through
//! `hsd-anim`. The root's transform is the item's own (its position,
//! rotation and scale), which the kind code sets. A world matrix is then
//! composed exactly as HSD_JObjMakeMatrix composes it, parent first.
use hsd_anim::jobj::{JOBJ_CLASSICAL_SCALE, JOBJ_USE_QUATERNION};
use hsd_anim::quat::Quaternion;
use hsd_archive::{desc::item_visual::ItemVisual, Archive};
use hsd_types::{Mtx, Vec3};

/// One joint's local transform after an animation step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalSrt {
    pub flags: u32,
    pub rotate: Quaternion,
    pub scale: Vec3,
    pub translate: Vec3,
}

/// The joints of an article's model in depth-first order, their parents
/// and, per article state, their locals after each animation step
/// (index 0: the step Item_80268E5C takes when the state begins).
#[derive(Clone, Debug, Default)]
pub struct ItemPose {
    pub parents: Vec<Option<usize>>,
    /// The root's flags (the root's transform is the item's).
    pub root_flags: u32,
    pub states: Vec<Option<Vec<Vec<LocalSrt>>>>,
    /// Every joint's locals in the model's rest pose, which a motion state
    /// without an article state keeps.
    pub rest: Vec<LocalSrt>,
    /// Article states whose animation loops, sampled for `LOOP_SAMPLES`
    /// steps only.
    pub looping: Vec<usize>,
}

/// The item's root transform: HSD_JObjSetTranslate / Rotation / Scale on
/// the model's root joint.
#[derive(Clone, Copy, Debug)]
pub struct RootSrt {
    pub translate: Vec3,
    pub rotate: Vec3,
    pub scale: Vec3,
}

/// The retail inverse trigonometry for joint tracks that need it.
struct RetailTrig;
impl hsd_anim::mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}

/// Steps sampled of an animation that loops (the Ice Climbers' spinning
/// ice block): ten seconds.
const LOOP_SAMPLES: usize = 600;

impl ItemPose {
    /// Plays every article state's joint animation from frame zero until it
    /// stops, recording each joint's locals after each step.
    pub fn read(archive: &Archive, visual: &ItemVisual) -> Result<Self, hsd_anim::load::LoadError> {
        let (tree, root) = hsd_anim::load::load_joint_tree(archive, &visual.model)?;
        let joints: Vec<_> = (0..).map_while(|i| tree.bone(root, i)).collect();
        let parents = joints
            .iter()
            .map(|&joint| {
                tree.parent(joint).map(|parent| {
                    joints
                        .iter()
                        .position(|&j| j == parent)
                        .expect("parent joint")
                })
            })
            .collect();
        let root_flags = tree.get(root).flags;
        let rest = joints
            .iter()
            .map(|&joint| {
                let joint = tree.get(joint);
                LocalSrt {
                    flags: joint.flags,
                    rotate: joint.rotate,
                    scale: joint.scale,
                    translate: joint.translate,
                }
            })
            .collect();
        let mut states = Vec::with_capacity(visual.states.len());
        let mut looping = Vec::new();
        for state in &visual.states {
            let Some(anim) = &state.joint else {
                states.push(None);
                continue;
            };
            let (mut tree, root) = hsd_anim::load::load_joint_tree(archive, &visual.model)?;
            hsd_anim::load::attach_anim_joint(&mut tree, root, anim, archive)?;
            tree.req_anim_all(root, 0.0);
            let mut steps = Vec::new();
            loop {
                let running = tree.anim_all::<RetailTrig>(root).running != 0;
                steps.push(
                    joints
                        .iter()
                        .enumerate()
                        .map(|(index, _)| {
                            let joint = tree.get(tree.bone(root, index).expect("joint"));
                            LocalSrt {
                                flags: joint.flags,
                                rotate: joint.rotate,
                                scale: joint.scale,
                                translate: joint.translate,
                            }
                        })
                        .collect(),
                );
                if !running {
                    break;
                }
                if steps.len() == LOOP_SAMPLES {
                    // A looping animation never stops: its first steps
                    // serve until the item has played it longer.
                    looping.push(states.len());
                    break;
                }
            }
            states.push(Some(steps));
        }
        Ok(Self {
            parents,
            root_flags,
            states,
            rest,
            looping,
        })
    }

    /// `bone`'s world matrix `steps` animation steps into article state
    /// `state` (the last sample once the animation has stopped), under the
    /// item's `root` transform. HSD_JObjMakeMatrix: each joint's SRT takes
    /// its parent's accumulated scale unless the parent's is classical, and
    /// the parent's matrix is concatenated in front (PSMTXConcat).
    pub fn bone_matrix(&self, state: usize, steps: u32, bone: usize, root: RootSrt) -> Mtx {
        let samples = self.states[state]
            .as_ref()
            .expect("item pose: article state without a joint animation");
        assert!(
            !self.looping.contains(&state) || (steps as usize) <= samples.len(),
            "item pose: a looping animation played past its samples"
        );
        let index = (steps.max(1) as usize - 1).min(samples.len() - 1);
        self.matrix_from_locals(&samples[index], bone, root)
    }

    /// `bone`'s world matrix in the rest pose, with `scale` replacing one
    /// joint's scale (it_80272F7C on that joint).
    pub fn rest_bone_matrix(&self, bone: usize, root: RootSrt, scale: Option<(usize, f32)>) -> Mtx {
        let mut locals = [LocalSrt {
            flags: 0,
            rotate: Quaternion::default(),
            scale: Vec3::ZERO,
            translate: Vec3::ZERO,
        }; 16];
        assert!(
            self.rest.len() <= locals.len(),
            "item pose: too many joints"
        );
        locals[..self.rest.len()].copy_from_slice(&self.rest);
        if let Some((joint, s)) = scale {
            locals[joint].scale = Vec3::new(s, s, s);
        }
        self.matrix_from_locals(&locals[..self.rest.len()], bone, root)
    }

    /// HSD_JObjMakeMatrix down the path from the root to `bone`.
    fn matrix_from_locals(&self, locals: &[LocalSrt], bone: usize, root: RootSrt) -> Mtx {
        // The path from the root down to `bone`.
        let mut path = [0usize; 16];
        let mut depth = 0;
        let mut joint = Some(bone);
        while let Some(j) = joint {
            path[depth] = j;
            depth += 1;
            joint = self.parents[j];
        }
        assert_eq!(path[depth - 1], 0, "item pose: the path reaches the root");
        let mut matrix = Mtx::default();
        hsd_anim::mtx::hsd_mtx_srt(
            &mut matrix,
            &root.scale,
            &root.rotate,
            &root.translate,
            None,
        );
        let mut accumulated = if self.root_flags & JOBJ_CLASSICAL_SCALE != 0 {
            None
        } else {
            Some(root.scale)
        };
        for &j in path[..depth - 1].iter().rev() {
            let local = &locals[j];
            let parent_scale = accumulated;
            accumulated = if local.flags & JOBJ_CLASSICAL_SCALE != 0 {
                parent_scale
            } else {
                Some(match parent_scale {
                    Some(p) => Vec3::new(
                        local.scale.x * p.x,
                        local.scale.y * p.y,
                        local.scale.z * p.z,
                    ),
                    None => local.scale,
                })
            };
            let mut own = Mtx::default();
            if local.flags & JOBJ_USE_QUATERNION != 0 {
                hsd_anim::mtx::hsd_mtx_srt_quat(
                    &mut own,
                    &local.scale,
                    &local.rotate,
                    &local.translate,
                    parent_scale.as_ref(),
                );
            } else {
                let euler = Vec3::new(local.rotate.x, local.rotate.y, local.rotate.z);
                hsd_anim::mtx::hsd_mtx_srt(
                    &mut own,
                    &local.scale,
                    &euler,
                    &local.translate,
                    parent_scale.as_ref(),
                );
            }
            let parent = matrix;
            hsd_anim::mtx::mtx_concat(&parent, &own, &mut matrix);
        }
        matrix
    }

    /// [`Self::bone_matrix`] with one joint's X rotation replaced (a kind
    /// that turns a joint below the root itself, HSD_JObjSetRotationX), as
    /// `(joint, angle)`. An article state without a joint animation keeps
    /// the rest pose (Item_80268D34 restores it on every state change).
    pub fn bone_matrix_turned(
        &self,
        state: usize,
        steps: u32,
        bone: usize,
        root: RootSrt,
        turned: Option<(usize, f32)>,
    ) -> Mtx {
        let source: &[LocalSrt] = match &self.states[state] {
            Some(samples) => {
                assert!(
                    !self.looping.contains(&state) || (steps as usize) <= samples.len(),
                    "item pose: a looping animation played past its samples"
                );
                &samples[(steps.max(1) as usize - 1).min(samples.len() - 1)]
            }
            None => &self.rest,
        };
        let mut locals = [LocalSrt {
            flags: 0,
            rotate: Quaternion::default(),
            scale: Vec3::ZERO,
            translate: Vec3::ZERO,
        }; 16];
        assert!(source.len() <= locals.len(), "item pose: too many joints");
        locals[..source.len()].copy_from_slice(source);
        if let Some((joint, angle)) = turned {
            assert!(
                locals[joint].flags & JOBJ_USE_QUATERNION == 0,
                "item pose: turning a quaternion joint"
            );
            locals[joint].rotate.x = angle;
        }
        self.matrix_from_locals(&locals[..source.len()], bone, root)
    }

    /// [`Self::bone_matrix_turned`] with joints the kind's code places and
    /// sizes itself after the animation step (HSD_JObjSetTranslate /
    /// HSD_JObjSetScale on a joint below the root): `(joint, value)` each.
    #[allow(clippy::too_many_arguments)]
    pub fn bone_matrix_posed(
        &self,
        state: usize,
        steps: u32,
        bone: usize,
        root: RootSrt,
        turned: Option<(usize, f32)>,
        translated: Option<(usize, Vec3)>,
        scaled: Option<(usize, Vec3)>,
    ) -> Mtx {
        let source: &[LocalSrt] = match &self.states[state] {
            Some(samples) => {
                assert!(
                    !self.looping.contains(&state) || (steps as usize) <= samples.len(),
                    "item pose: a looping animation played past its samples"
                );
                &samples[(steps.max(1) as usize - 1).min(samples.len() - 1)]
            }
            None => &self.rest,
        };
        let mut locals = [LocalSrt {
            flags: 0,
            rotate: Quaternion::default(),
            scale: Vec3::ZERO,
            translate: Vec3::ZERO,
        }; 16];
        assert!(source.len() <= locals.len(), "item pose: too many joints");
        locals[..source.len()].copy_from_slice(source);
        if let Some((joint, angle)) = turned {
            assert!(
                locals[joint].flags & JOBJ_USE_QUATERNION == 0,
                "item pose: turning a quaternion joint"
            );
            locals[joint].rotate.x = angle;
        }
        if let Some((joint, translate)) = translated {
            locals[joint].translate = translate;
        }
        if let Some((joint, scale)) = scaled {
            locals[joint].scale = scale;
        }
        self.matrix_from_locals(&locals[..source.len()], bone, root)
    }

    /// `bone`'s local transform `steps` animation steps into article state
    /// `state` (HSD_JObjGetRotationX and friends read these).
    pub fn local(&self, state: usize, steps: u32, bone: usize) -> &LocalSrt {
        let samples = self.states[state]
            .as_ref()
            .expect("item pose: article state without a joint animation");
        &samples[(steps.max(1) as usize - 1).min(samples.len() - 1)][bone]
    }

    /// lb_8000B1CC(bones[bone], NULL, &pos): the joint's world translation.
    pub fn bone_position(&self, state: usize, steps: u32, bone: usize, root: RootSrt) -> Vec3 {
        let m = self.bone_matrix(state, steps, bone, root);
        Vec3::new(m.0[0][3], m.0[1][3], m.0[2][3])
    }
}
