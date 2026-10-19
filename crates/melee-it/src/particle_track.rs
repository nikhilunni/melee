//! Particle keys in an article's joint animation (HSD_A_J_DPTCL tracks):
//! HSD_JObjAnimAll calls efLib_Cb_DPtcl when one fires, and
//! efLib_SpawnParticleEffect attaches the generator to the firing joint.
//!
//! Which steps fire is a pure function of the AnimAll steps taken since the
//! state's Item_80268D34 request (frame zero, the article's rate), so each
//! article state's keys are sampled once at load by playing its animation
//! through `hsd-anim`, as `bone_motion` samples a gait.
use hsd_archive::{desc::item_visual::ItemVisual, Archive};

/// One DPtcl key: the AnimAll step (0 is Item_80268E5C's own step after the
/// request), the track's bank and generator id, and the joint whose track
/// fired it, by its depth-first index in the model (0 is the root).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParticleKey {
    pub step: u16,
    pub bank: i32,
    pub id: i32,
    pub joint: usize,
}

/// One article state's keys, and how many steps were sampled.
#[derive(Clone, Debug, Default)]
pub struct StateParticles {
    pub keys: Vec<ParticleKey>,
    pub sampled_steps: u16,
}

/// Every article state's keys (`None` for states without a joint animation).
#[derive(Clone, Debug, Default)]
pub struct ParticleTracks {
    pub states: Vec<Option<StateParticles>>,
}

/// Steps sampled for a looping animation; an item still animating past
/// them fails closed.
const MAXIMUM_STEPS: u16 = 1024;

impl ParticleTracks {
    /// Plays every article state's joint animation from frame zero at rate
    /// 1, recording the DPtcl events of each AnimAll step.
    pub fn read(archive: &Archive, visual: &ItemVisual) -> Result<Self, hsd_anim::load::LoadError> {
        let mut states = Vec::with_capacity(visual.states.len());
        for state in &visual.states {
            let Some(anim) = &state.joint else {
                states.push(None);
                continue;
            };
            let (mut tree, root) = hsd_anim::load::load_joint_tree(archive, &visual.model)?;
            let joints: Vec<_> = (0..).map_while(|i| tree.bone(root, i)).collect();
            hsd_anim::load::attach_anim_joint(&mut tree, root, anim, archive)?;
            tree.req_anim_all(root, 0.0);
            let mut keys = Vec::new();
            let mut step = 0;
            while step < MAXIMUM_STEPS {
                let ended = tree.anim_all::<RetailTrig>(root);
                for event in tree.events.drain(..) {
                    if let hsd_anim::jobj::JObjEvent::DPtcl { jobj, lo, hi } = event {
                        keys.push(ParticleKey {
                            step,
                            bank: lo,
                            id: hi,
                            joint: joints
                                .iter()
                                .position(|&j| j == jobj)
                                .expect("a DPtcl key on a model joint"),
                        });
                    }
                }
                step += 1;
                if ended.running == 0 {
                    break;
                }
            }
            states.push(Some(StateParticles {
                keys,
                sampled_steps: step,
            }));
        }
        Ok(Self { states })
    }

    /// Whether a key fires on a joint below the root.
    pub fn below_root(&self) -> bool {
        self.states
            .iter()
            .flatten()
            .any(|state| state.keys.iter().any(|key| key.joint != 0))
    }

    /// The keys that fire on AnimAll step `step` of article state `state`.
    pub fn keys(&self, state: usize, step: u16) -> impl Iterator<Item = &ParticleKey> {
        let particles = self.states.get(state).and_then(Option::as_ref);
        if let Some(particles) = particles {
            assert!(
                step < particles.sampled_steps || particles.sampled_steps < MAXIMUM_STEPS,
                "item particle track played past its sampled steps"
            );
        }
        particles
            .into_iter()
            .flat_map(|p| p.keys.iter())
            .filter(move |key| key.step == step)
    }
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
