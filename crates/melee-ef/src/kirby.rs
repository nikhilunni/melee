//! Kirby's efAlt rows (efalt.c:212-323), bank 5 (EfKbData.dat).
use super::*;

/// efAlt_Spawn 0x495's model: the dash attack's flame.
const DASH_ATTACK_MODEL: u32 = 0x138B;

impl Effects {
    /// efAlt_Spawn 0x495 (efalt.c:253-270), from the dash attack's script
    /// (ftCo_8009F834 block_64: efAsync kind 3 with the fighter's facing):
    /// efLib_Create_Attach_Scale(0x138B) on the joint, its root turned to
    /// the facing the script passed (an f64 +-M_PI_2, rounded on the store).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_kirby_dash_attack<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        matrix: Mtx,
        facing: f32,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(DASH_ATTACK_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        // efAlt's attached character models keep state_flags 0: held
        // through hitlag by efLib_PauseAll.
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        // efLib_Create_Attach_Scale: the fighter root's Y scale, uniformly.
        let scale = fighter.effect_scale().y;
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        effect.tree.set_rotation_y(
            effect.root,
            if facing < 0.0 {
                -std::f64::consts::FRAC_PI_2 as f32
            } else {
                std::f64::consts::FRAC_PI_2 as f32
            },
        );
        effect.tree.set_translate(
            effect.root,
            &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        );
        effect.animate_banks::<T>(
            resources::Banks {
                common: common_bank,
                characters: &self.character_banks,
            },
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
        )?;
        self.instances.push(effect);
        Ok(())
    }
}
