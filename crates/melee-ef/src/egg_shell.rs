//! efSync_Spawn (8005FDDC), efsync.c:228-292, request 0x4CF.
use super::*;
// efLib_Create, eflib.c:524-528: 32 queued initial animations.
const ANIMATION_QUEUE_CAPACITY: usize = 32;
// efsync.c:228-292 emits twelve shell fragments per burst.
const SHELL_COUNT: usize = 12;
impl Effects {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_egg_shell<T: InverseTrig>(
        &mut self,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
        matrix: &Mtx,
        scale: f32,
    ) -> Result<()> {
        self.make_room(0x1E, SHELL_COUNT, particles);
        let mut shells = FixedVec::<_, ANIMATION_QUEUE_CAPACITY>::default();
        for _ in 0..SHELL_COUNT {
            let id = if rng.randf() < 0.5 { 0x1E } else { 0x1F };
            let mut effect = self.acquire(id, particles);
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect.lifetime = 50;
            effect.indefinite = false;
            // Retail efSync_Spawn: double M_TAU products, then frsp; no FMA.
            let y = (std::f64::consts::TAU * f64::from(rng.randf())) as f32;
            let x = (std::f64::consts::TAU * f64::from(rng.randf())) as f32;
            effect
                .tree
                .set_scale(effect.root, &Vec3::new(scale, scale, scale));
            effect.tree.set_rotation_y(effect.root, y);
            effect.tree.set_rotation_x(effect.root, x);
            effect.tree.set_translate(
                effect.root,
                &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
            );
            // efSync_Spawn's separately rounded fmuls; MSL routines are audited.
            use gekko_math::msl::{cosf, sinf};
            effect.velocity = Some(Vec3::new(
                (2.0 * cosf(x)) * sinf(y),
                2.0 * sinf(x),
                (2.0 * cosf(x)) * cosf(y),
            ));
            shells.push(effect);
        }
        // efLib_AnimQueue is drained in reverse creation order by efSync_Spawn.
        for effect in shells.iter_mut().rev() {
            effect.animate::<T>(bank, particles, rng, &mut self.draws, &mut self.events)?;
        }
        while !shells.is_empty() {
            self.instances.push(shells.remove(0));
        }
        Ok(())
    }
}
