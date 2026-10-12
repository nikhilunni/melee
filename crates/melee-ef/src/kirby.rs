//! Kirby's efAlt rows (efalt.c:212-323), bank 5 (EfKbData.dat).
use super::*;

/// efAlt_Spawn 0x495's model: the dash attack's flame.
const DASH_ATTACK_MODEL: u32 = 0x138B;
/// efAlt_Spawn 0x49D's model: one fist of the rapid jab.
const RAPID_JAB_MODEL: u32 = 0x1390;
/// efAlt 0x49D starts the model's child this far behind the root's own Z
/// (efalt.c:308-309) and efLib_Cb_SetJObjOffsetZ moves it this far forward
/// on every update (eflib.c:1138).
const RAPID_JAB_CHILD_START: f32 = 6.0;
const RAPID_JAB_CHILD_STEP: f32 = 2.0;

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

    /// efAlt_Spawn 0x49D (efalt.c:296-319), from the rapid jab's script
    /// (ftCo_8009F834 block_65: efAsync kind 6 with the command's offset,
    /// the facing and an angle): efLib_Create_Attach_Pos(0x1390) at the
    /// point, the fighter root's Y scale uniformly, the child's Z set from
    /// the root's (fsubs), the root turned to the facing (an f64 +-M_PI_2,
    /// rounded) and tilted about Z by the angle, negated facing left;
    /// efLib_Cb_SetJObjOffsetZ is its update callback.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_kirby_rapid_jab<T: InverseTrig>(
        &mut self,
        player: usize,
        position: Vec3,
        facing: f32,
        angle: f32,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(RAPID_JAB_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = None;
        effect.tree.set_translate(effect.root, &position);
        let scale = fighter.effect_scale().y;
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        let child = effect.tree.child(effect.root).expect("rapid jab fist joint");
        effect
            .tree
            .set_translate_z(child, position.z - RAPID_JAB_CHILD_START);
        let (turn, tilt) = if facing < 0.0 {
            (-std::f64::consts::FRAC_PI_2 as f32, -angle)
        } else {
            (std::f64::consts::FRAC_PI_2 as f32, angle)
        };
        effect.tree.set_rotation_y(effect.root, turn);
        effect.tree.set_rotation_z(effect.root, tilt);
        effect.child_drift_z = Some(RAPID_JAB_CHILD_STEP);
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
