//! Explicit resources borrowed by each fighter scheduler phase.
//! These carry the existing scheduler arguments; callback results are returned.
use crate::anim::WaitChoice;
use crate::fighter::{
    assets::{FighterAssets, Result},
    Fighter,
};
use gekko_math::rng::HsdRng;
use melee_gr::wind::Wind;
use melee_mp::CollMap;

/// Fighter_8006A360 (8006A360), s_link 1: playback and animation callbacks.
pub struct AnimationPhase<'a> {
    pub assets: &'a FighterAssets,
    pub rng: &'a mut HsdRng,
}
/// Fighter_Spaghetti_8006AD10 (8006AD10), s_link 3: the input sample has already been applied.
pub struct InputPhase<'a> {
    pub assets: &'a FighterAssets,
}
/// Fighter_procUpdate (8006B82C), s_link 4: read-only map and stage wind.
pub struct PhysicsPhase<'a> {
    pub assets: &'a FighterAssets,
    pub map: &'a CollMap,
    pub wind: Wind,
}
/// Fighter_procMap (8006C27C), s_link 6: optional assets preserve the existing
/// grounded map-only API. Landing-effect RNG work remains in the scheduler.
pub struct CollisionPhase<'a> {
    pub assets: Option<&'a FighterAssets>,
    pub map: &'a mut CollMap,
}
/// Fighter camera procedure (8006D9EC), s_link 18.
pub struct CameraPhase<'a> {
    pub assets: &'a FighterAssets,
    /// The stage's camera description (Stage_GetCamFixedZoom and the bounds
    /// the dead-fighter callbacks aim at).
    pub stage: &'a melee_cm::StageCamera,
}

pub type AnimFn = fn(&mut Fighter, AnimationPhase<'_>) -> Result<Option<WaitChoice>>;
pub type InputFn = fn(&mut Fighter, InputPhase<'_>);
pub type PhysicsFn = fn(&mut Fighter, PhysicsPhase<'_>);
pub type CollisionFn = fn(&mut Fighter, CollisionPhase<'_>) -> Result<()>;
pub type CameraFn = fn(&mut Fighter, CameraPhase<'_>);
