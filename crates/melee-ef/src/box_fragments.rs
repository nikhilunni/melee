//! efAsync_Dispatch 0x427 (efasync.c:563-588): a broken crate's six
//! falling boards, common model 0x1B, from the Box's itBox_TryOpen.
use super::*;

/// efasync.c:563: the burst's request id.
pub const BOX_FRAGMENTS_REQUEST: u16 = 0x427;
/// The common model each board is.
pub(crate) const BOX_FRAGMENT_MODEL: u32 = 0x1B;
const FRAGMENT_COUNT: usize = 6;
/// `node->lifetime = 0x28`.
const FRAGMENT_LIFETIME: u16 = 0x28;
/// `node->params.y = (0.4f * HSD_Randf()) + 2.8f` (retail 800656F8: fmadds).
const RISE_SCALE: f32 = 0.4;
const RISE_BASE: f32 = 2.8;

impl Effects {
    /// The boards at `position`, each its own efLib_Create_Attach_Pos model
    /// parented to the crate (`owner`), with a rise for
    /// efLib_Cb_Fall_FromParamY and a heading drawn as it is made; then
    /// efAsync_Dispatch drains the animation queue newest first.
    pub fn spawn_box_fragments<T: InverseTrig>(
        &mut self,
        item: u32,
        position: Vec3,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.make_room(BOX_FRAGMENT_MODEL, FRAGMENT_COUNT, particles);
        let mut boards = FixedVec::<_, FRAGMENT_COUNT>::default();
        for _ in 0..FRAGMENT_COUNT {
            let mut effect = self.acquire(BOX_FRAGMENT_MODEL, particles);
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect.owner = Some(ModelOwner::Item(item));
            effect.tree.set_translate(effect.root, &position);
            effect.lifetime = FRAGMENT_LIFETIME;
            effect.indefinite = false;
            let rise = gekko_math::fma::fmadds(RISE_SCALE, rng.randf(), RISE_BASE);
            effect.fall_from_param_y = Some(rise);
            // retail 80065704: fmul in double by M_TAU, then frsp.
            let heading = (std::f64::consts::TAU * f64::from(rng.randf())) as f32;
            effect.tree.set_rotation_y(effect.root, heading);
            boards.push(effect);
        }
        for effect in boards.iter_mut().rev() {
            effect.animate::<T>(
                bank,
                particles,
                rng,
                &mut self.draws,
                &mut self.events,
                &mut self.animation_events,
            )?;
        }
        while !boards.is_empty() {
            self.instances.push(boards.remove(0));
        }
        Ok(())
    }
}
