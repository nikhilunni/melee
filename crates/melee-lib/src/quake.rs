//! Screen-shake models, grLib_801C9CEC (0x801C9CEC): each camera quake
//! request plays one of the stage's `quake_model_set` animations, whose root
//! translation becomes the camera's quake offset every tick.
use anyhow::{Context, Result};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
};
use melee_cm::{GameCamera, QuakeKind};
use melee_ft::fighter::RetailTrig;

/// Instances prepared per one-shot kind; a request beyond them is a port limit.
const INSTANCES_PER_KIND: usize = 4;
const ONE_SHOT_KINDS: [QuakeKind; 3] = [QuakeKind::Small, QuakeKind::Medium, QuakeKind::Large];

#[derive(Clone)]
struct Instance {
    kind: QuakeKind,
    tree: JObjTree,
    root: JObjId,
}

/// The quake models of one stage: preloaded instances and the playing ones
/// in scheduler order (GObj p_link 18, priority = the quake kind).
#[derive(Clone, Default)]
pub(crate) struct Quakes {
    instances: Vec<Instance>,
    /// Indices into `instances`, in retail gobj order.
    playing: melee_types::fixed::FixedVec<usize, { 3 * INSTANCES_PER_KIND }>,
}

impl Quakes {
    pub(crate) fn load(
        archive: &hsd_archive::Archive,
        model: Option<&melee_gr::desc::QuakeModel>,
    ) -> Result<Self> {
        let Some(model) = model else {
            return Ok(Self::default());
        };
        let mut instances = Vec::new();
        for kind in ONE_SHOT_KINDS {
            let animation = model
                .animations
                .get(kind.animation_index())
                .context("quake_model_set animation")?;
            for _ in 0..INSTANCES_PER_KIND {
                let (mut tree, root) = load_joint_tree(archive, &model.joint)?;
                attach_anim_joint(&mut tree, root, animation, archive)?;
                instances.push(Instance { kind, tree, root });
            }
        }
        Ok(Self {
            instances,
            playing: Default::default(),
        })
    }

    /// Camera_RequestQuake (0x80030E44 area) and grLib_801C9CEC.
    pub(crate) fn request(&mut self, camera: &mut GameCamera, kind: QuakeKind) {
        assert!(
            kind != QuakeKind::Loop,
            "grLib_801C9BC8: looping quakes are not ported"
        );
        if !camera.quake.request(kind) || self.instances.is_empty() {
            return;
        }
        let index = (0..self.instances.len())
            .find(|i| self.instances[*i].kind == kind && !self.playing.contains(i))
            .expect("quake model instances exhausted");
        let instance = &mut self.instances[index];
        instance.tree.req_anim_all(instance.root, 0.0);
        // A new gobj follows those of lower or equal priority in its p_link.
        let at = self
            .playing
            .iter()
            .position(|&i| self.instances[i].kind as u8 > kind as u8)
            .unwrap_or(self.playing.len());
        self.playing.insert(at, index);
    }

    /// grLib_801C9C40 (s_link 1) for every playing quake, in gobj order:
    /// animate, publish the root translation, and end with the animation.
    pub(crate) fn animate(&mut self, camera: &mut GameCamera) {
        let instances = &mut self.instances;
        let mut slot = 0;
        while slot < self.playing.len() {
            let index = *self.playing.iter().nth(slot).expect("playing quake");
            let instance = &mut instances[index];
            instance.tree.anim_all::<RetailTrig>(instance.root);
            let joint = instance.tree.get(instance.root);
            camera.quake.offset = hsd_types::Vec2::new(joint.translate.x, joint.translate.y);
            let finished = joint
                .aobj
                .as_ref()
                .is_none_or(|aobj| aobj.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0);
            if finished {
                self.playing.remove(slot);
            } else {
                slot += 1;
            }
        }
    }
}
