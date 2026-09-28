//! Samus's efAlt rows (efalt.c:74-114), bank 2 (EfSsData.dat).
use super::*;

/// efAlt_Spawn 0x487's model: efLib_Create_AttachChild_Scale(0x7D2).
const JUMP_THRUSTER_MODEL: u32 = 0x7D2;

/// efAlt_Spawn 0x482's model: the Screw Attack's efLib_Create_Attach(0x7D0).
const SCREW_ATTACK_MODEL: u32 = 0x7D0;

impl Effects {
    /// efSync_Spawn 0x482 -> efAlt_Spawn (efalt.c:82-87), from
    /// ftSs_SpecialHi_Enter: efLib_Create_Attach(0x7D0) on YRotN with
    /// efLib_Cb_SetScaleRotY_FromFighter, which after each update's
    /// animation multiplies the model's scale by the fighter root's (one at
    /// ordinary size) and turns it to the fighter's facing, an f64
    /// +-M_PI_2 rounded (eflib.c:1228-1259). The turn is taken from the
    /// facing at creation; a later turnaround only turns the model, which
    /// the gate does not compare.
    pub(super) fn spawn_screw_attack<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(SCREW_ATTACK_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        effect.facing_rotation = Some(if fighter.effect_facing() < 0.0 {
            -std::f64::consts::FRAC_PI_2 as f32
        } else {
            std::f64::consts::FRAC_PI_2 as f32
        });
        let matrix = fighter.effect_matrix(Some(bone));
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

    /// efAsync kind 0 -> efSync_Spawn 0x487 -> efAlt_Spawn (efalt.c:109-111),
    /// from Samus's jump script: efLib_Create_AttachChild_Scale(0x7D2) on
    /// the live joint. efLib_Create_Attach's position constraint follows
    /// the joint (lb_8000C1C0); lb_8000C290's orientation constraint only
    /// turns the model, which the gate does not compare. The fighter
    /// root's Y scale is broadcast once, and the model keeps
    /// EF_SCALE_NO_INHERIT.
    pub(super) fn spawn_jump_thruster<T: InverseTrig>(
        &mut self,
        player: usize,
        (bone, matrix): (usize, Mtx),
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(JUMP_THRUSTER_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        effect.tree.set_translate(
            effect.root,
            &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        );
        let mut scale = fighter.effect_scale();
        scale.x = scale.y;
        scale.z = scale.y;
        effect.tree.set_scale(effect.root, &scale);
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
