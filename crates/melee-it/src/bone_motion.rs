//! itUpdateVelocityFromBone (it/kinds/inlines.h): kinds whose gait is
//! authored as one joint's translation read that joint after every
//! animation step, move by its change and zero it again.
//!
//! The joint's animation is a pure function of the steps taken since the
//! state began (Item_80268D34 restores the rest pose and restarts it at
//! frame zero with rate 1), so each article state's readings are sampled
//! once at load by playing the animation through `hsd-anim`. A kind that
//! changes the joint's rate (itHeiho_UnkMotion4_Anim: HSD_AObjSetRate 2
//! after every step) plays one of the [`BoneRate`] schedules.
use hsd_archive::{desc::item_visual::ItemVisual, Archive};
use hsd_types::Vec3;

/// How the joint's AObj rate evolves after the state begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoneRate {
    /// Rate 1 throughout.
    Steady,
    /// Rate 2 from the first step after the state began: the state began
    /// inside the callback that then doubles the rate.
    Doubled,
    /// One step at rate 1, then rate 2: the state began in a physics or
    /// collision callback, so the first doubling comes a step later.
    DoubledAfterOne,
}

/// One reading: the joint's translation after an HSD_JObjAnimAll step, and
/// whether its animation still runs (lb_8000B09C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneSample {
    pub translation: Vec3,
    pub running: bool,
}

/// One article state's readings for each schedule, index 0 being the step
/// Item_80268E5C takes when the state begins.
#[derive(Clone, Debug, Default)]
pub struct BoneClips {
    pub steady: Vec<BoneSample>,
    pub doubled: Vec<BoneSample>,
    pub doubled_after_one: Vec<BoneSample>,
}
impl BoneClips {
    pub fn schedule(&self, rate: BoneRate) -> &[BoneSample] {
        match rate {
            BoneRate::Steady => &self.steady,
            BoneRate::Doubled => &self.doubled,
            BoneRate::DoubledAfterOne => &self.doubled_after_one,
        }
    }
}

/// The dynamic bone a kind reads (xBBC_dynamicBoneTable->bones[bone]) and
/// its readings per article state (`None` for states without a joint
/// animation).
#[derive(Clone, Debug, Default)]
pub struct BoneMotion {
    pub bone: usize,
    pub states: Vec<Option<BoneClips>>,
}

impl BoneMotion {
    /// Plays every article state's joint animation from frame zero until it
    /// stops, reading `bone` after each step.
    pub fn read(
        archive: &Archive,
        visual: &ItemVisual,
        bone: usize,
    ) -> Result<Self, hsd_anim::load::LoadError> {
        let mut states = Vec::with_capacity(visual.states.len());
        for state in &visual.states {
            let Some(anim) = &state.joint else {
                states.push(None);
                continue;
            };
            let clip = |rate| play(archive, visual, anim, bone, rate);
            states.push(Some(BoneClips {
                steady: clip(BoneRate::Steady)?,
                doubled: clip(BoneRate::Doubled)?,
                doubled_after_one: clip(BoneRate::DoubledAfterOne)?,
            }));
        }
        Ok(Self { bone, states })
    }

    /// The reading `step` animation steps into article state `state`.
    pub fn sample(&self, state: usize, rate: BoneRate, step: usize) -> BoneSample {
        let clips = self.states[state]
            .as_ref()
            .expect("itUpdateVelocityFromBone: article state without a joint animation");
        *clips
            .schedule(rate)
            .get(step)
            .expect("itUpdateVelocityFromBone: a step past the animation's end")
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

/// Longest animation the sampler plays before giving up on a loop.
const MAXIMUM_STEPS: usize = 4096;

fn play(
    archive: &Archive,
    visual: &ItemVisual,
    anim: &hsd_archive::desc::AnimJoint,
    bone: usize,
    rate: BoneRate,
) -> Result<Vec<BoneSample>, hsd_anim::load::LoadError> {
    let (mut tree, root) = hsd_anim::load::load_joint_tree(archive, &visual.model)?;
    let joint = tree.bone(root, bone).expect("item dynamic bone");
    hsd_anim::load::attach_anim_joint(&mut tree, root, anim, archive)?;
    tree.req_anim_all(root, 0.0);
    let mut samples = Vec::new();
    loop {
        let steps = tree.anim_all::<RetailTrig>(root);
        let running = steps.running != 0;
        samples.push(BoneSample {
            translation: tree.translation(joint),
            running,
        });
        // itUpdateVelocityFromBone: HSD_JObjSetTranslate(jobj, &zero).
        tree.set_translate(joint, &Vec3::ZERO);
        if !running {
            return Ok(samples);
        }
        assert!(samples.len() < MAXIMUM_STEPS, "looping item bone animation");
        let doubled = match rate {
            BoneRate::Steady => false,
            BoneRate::Doubled => true,
            BoneRate::DoubledAfterOne => samples.len() >= 2,
        };
        if doubled {
            // HSD_AObjSetRate(child->aobj, 2.0F) on the read joint.
            if let Some(aobj) = tree.get_mut(joint).aobj.as_mut() {
                aobj.set_rate(2.0);
            }
        }
    }
}
