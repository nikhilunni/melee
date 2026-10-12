//! Screen-shake models, grLib_801C9CEC (0x801C9CEC): each camera quake
//! request plays one of the stage's `quake_model_set` animations, whose root
//! translation becomes the camera's quake offset every tick.
use anyhow::{Context, Result};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::{attach_anim_joint, load_joint_tree},
};
use hsd_gobj::{GObjAddress, GObjAddresses, GObjClass};
use melee_cm::{GameCamera, QuakeKind};
use melee_ft::fighter::RetailTrig;

/// Instances prepared per one-shot kind. Retail allocates a gobj per request
/// (grLib_801C9CEC); a multi-hit tumble combo stacks several small quakes,
/// and a request beyond these is a port limit.
const INSTANCES_PER_KIND: usize = 16;
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
    /// Indices into `instances` with each model's GObj, in retail gobj
    /// order.
    playing: melee_types::fixed::FixedVec<(usize, GObjAddress), { 3 * INSTANCES_PER_KIND + 1 }>,
    /// Playing models a fighter's own code requested during the proc still
    /// being dispatched, in request order: their GObjs are created when
    /// the dispatch ends (`create_fighter_gobjs`).
    awaiting_gobj: melee_types::fixed::FixedVec<usize, 8>,
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
        // Camera_RequestQuake keeps at most one Loop model (quake_gobj);
        // grLib_801C9CEC sets AOBJ_LOOP on all of its animations.
        {
            let animation = model
                .animations
                .get(QuakeKind::Loop.animation_index())
                .context("quake_model_set loop animation")?;
            let (mut tree, root) = load_joint_tree(archive, &model.joint)?;
            attach_anim_joint(&mut tree, root, animation, archive)?;
            let mut joints = Vec::new();
            tree.walk_tree(root, &mut |joint, _| joints.push(joint));
            for joint in joints {
                if let Some(aobj) = &mut tree.get_mut(joint).aobj {
                    aobj.flags |= hsd_anim::aobj::AOBJ_LOOP;
                }
            }
            instances.push(Instance {
                kind: QuakeKind::Loop,
                tree,
                root,
            });
        }
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
            awaiting_gobj: Default::default(),
        })
    }

    /// Camera_RequestQuake (0x80030E44 area) and grLib_801C9CEC.
    pub(crate) fn request(
        &mut self,
        camera: &mut GameCamera,
        kind: QuakeKind,
        gobjs: &mut GObjAddresses,
    ) {
        if !camera.quake.request(kind) || self.instances.is_empty() {
            return;
        }
        // grLib_801C9CEC's GObj_Create(HSD_GOBJ_CLASS_STAGE, 18, kind).
        let gobj = gobjs.create(GObjClass::Quake);
        self.start(kind, gobj);
    }

    /// [`request`](Self::request) from a fighter's own code (a hit taken, a
    /// bounce off the floor), noted when its proc returns. The camera and
    /// the animation start now; the model's GObj is created by
    /// [`create_fighter_gobjs`](Self::create_fighter_gobjs), after the
    /// articles that proc destroyed are freed: retail destroys a struck
    /// fighter's article in its take-damage callback, ahead of the hit's
    /// Camera_RequestQuake.
    pub(crate) fn request_from_fighter(&mut self, camera: &mut GameCamera, kind: QuakeKind) {
        if !camera.quake.request(kind) || self.instances.is_empty() {
            return;
        }
        let index = self.start(kind, GObjAddress::default());
        self.awaiting_gobj.push(index);
    }

    /// grLib_801C9CEC's GObj_Create for each quake a fighter requested in
    /// the dispatch now ending.
    pub(crate) fn create_fighter_gobjs(&mut self, gobjs: &mut GObjAddresses) {
        while !self.awaiting_gobj.is_empty() {
            let index = self.awaiting_gobj.remove(0);
            let gobj = gobjs.create(GObjClass::Quake);
            for playing in self.playing.iter_mut().filter(|(i, _)| *i == index) {
                playing.1 = gobj;
            }
        }
    }

    /// The models a replay created at another address
    /// (`hsd_gobj::GObjAddresses::replay`).
    pub(crate) fn rename_gobjs(&mut self, renamed: &hsd_gobj::Renamed) {
        for playing in self.playing.iter_mut() {
            playing.1 = renamed.apply(playing.1);
        }
    }

    /// Whether the stage has screen-shake models (quake_model_set).
    pub(crate) fn has_models(&self) -> bool {
        !self.instances.is_empty()
    }

    /// [`request`](Self::request) for a one-shot quake whose GObj an effect
    /// flush already created (`melee_ef::CameraQuake`).
    pub(crate) fn request_created(
        &mut self,
        camera: &mut GameCamera,
        kind: QuakeKind,
        gobj: Option<GObjAddress>,
    ) {
        let start = camera.quake.request(kind);
        assert_eq!(
            start && !self.instances.is_empty(),
            gobj.is_some(),
            "a flushed quake request and its model's GObj"
        );
        if let Some(gobj) = gobj {
            self.start(kind, gobj);
        }
    }

    /// grLib_801C9CEC around its GObj_Create: the kind's animation from
    /// frame zero. Returns the model instance.
    fn start(&mut self, kind: QuakeKind, gobj: GObjAddress) -> usize {
        let index = (0..self.instances.len())
            .find(|i| self.instances[*i].kind == kind && !self.playing.iter().any(|(p, _)| p == i))
            .expect("quake model instances exhausted");
        let instance = &mut self.instances[index];
        instance.tree.req_anim_all(instance.root, 0.0);
        // A new gobj follows those of lower or equal priority in its p_link.
        let at = self
            .playing
            .iter()
            .position(|&(i, _)| self.instances[i].kind as u8 > kind as u8)
            .unwrap_or(self.playing.len());
        self.playing.insert(at, (index, gobj));
        index
    }

    /// Camera_UpdateQuakes (in the camera proc) deletes the Loop model once
    /// its countdown ends; `GameCamera::update_standard` cleared `looping`.
    pub(crate) fn retire_loop(&mut self, camera: &GameCamera, gobjs: &mut GObjAddresses) {
        if camera.quake.looping {
            return;
        }
        let instances = &self.instances;
        let slot = self
            .playing
            .iter()
            .position(|&(i, _)| instances[i].kind == QuakeKind::Loop);
        if let Some(slot) = slot {
            let (_, gobj) = self.playing.remove(slot);
            gobjs.destroy(GObjClass::Quake, gobj);
        }
    }

    /// grLib_801C9C40 (s_link 1) for every playing quake, in gobj order:
    /// animate, publish the root translation, and end with the animation.
    /// The Loop model runs grLib_801C9BC8 instead, which never ends itself.
    pub(crate) fn animate(&mut self, camera: &mut GameCamera, gobjs: &mut GObjAddresses) {
        let instances = &mut self.instances;
        let mut slot = 0;
        while slot < self.playing.len() {
            let (index, gobj) = *self.playing.iter().nth(slot).expect("playing quake");
            let instance = &mut instances[index];
            instance.tree.anim_all::<RetailTrig>(instance.root);
            let joint = instance.tree.get(instance.root);
            camera.quake.offset = hsd_types::Vec2::new(joint.translate.x, joint.translate.y);
            let finished = instance.kind != QuakeKind::Loop
                && joint
                    .aobj
                    .as_ref()
                    .is_none_or(|aobj| aobj.flags & hsd_anim::aobj::AOBJ_NO_ANIM != 0);
            if finished {
                // HSD_GObjPLink_80390228 on itself, applied when its proc
                // returns.
                self.playing.remove(slot);
                gobjs.destroy(GObjClass::Quake, gobj);
            } else {
                slot += 1;
            }
        }
    }
}

/// efAsync_Dispatch's kind 8 (efasync.c:1365-1367): a fighter's graphics
/// command 0x513-0x515 (ftCo_09F7.c:263-274) reaches Camera_RequestQuake
/// when its queue flushes. Called after each fighter proc, in request order.
pub(crate) fn request_effect_quakes(state: &mut crate::initial_state::InitialState) {
    let (quakes, camera) = (&mut state.quakes, &mut state.camera);
    state
        .effects
        .drain_camera_quakes(|(value, _epicenter, gobj)| {
            let kind = QuakeKind::from_value(value).expect("camera shake kind");
            quakes.request_created(camera, kind, gobj);
        });
}
