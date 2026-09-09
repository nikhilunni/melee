//! Explicit resources borrowed by each fighter scheduler phase.
//! These carry the existing scheduler arguments; callback results are returned.
use crate::anim::WaitChoice;
use crate::fighter::{
    assets::{FighterAssets, Result},
    Fighter,
};
use gekko_math::rng::HsdRng;
use hsd_types::Vec3;
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
    pub wind: Vec3,
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
    pub zoom: f32,
}

pub type AnimFn<C> = fn(&mut Fighter<C>, AnimationPhase<'_>) -> Result<Option<WaitChoice>>;
pub type InputFn<C> = fn(&mut Fighter<C>, InputPhase<'_>);
pub type PhysicsFn<C> = fn(&mut Fighter<C>, PhysicsPhase<'_>);
pub type CollisionFn<C> = fn(&mut Fighter<C>, CollisionPhase<'_>) -> Result<()>;
pub type CameraFn<C> = fn(&mut Fighter<C>, CameraPhase<'_>);
