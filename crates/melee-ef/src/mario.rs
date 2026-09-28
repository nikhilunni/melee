//! Mario's efAlt rows (efalt.c:38-72), bank 1 (EfMrData.dat), and Luigi's
//! efSync rows of the same shape (efsync.c 0x507, 0x509), bank 18
//! (EfLgData.dat).
use super::*;

/// A fireball's hand flash: a model turned to the facing, then a
/// generator of the same bank on the hand joint.
#[derive(Clone, Copy)]
pub(super) struct HandFire {
    pub model: u32,
    pub bank: u8,
    pub generator: u32,
}
/// efAlt_Spawn 0x47A: Mario's flash, model 0x3E8 and hsd_8039EFAC(0, 1,
/// 0x3E9, jobj).
pub(super) const MARIO_HAND_FIRE: HandFire = HandFire {
    model: 0x3E8,
    bank: 1,
    generator: 0x3E9,
};
/// efSync_Spawn 0x507 (efsync.c:566-580): Luigi's flash, model 0x4650 and
/// hsd_8039EFAC(0, 0x12, 0x4650, jobj).
pub(super) const LUIGI_HAND_FIRE: HandFire = HandFire {
    model: 0x4650,
    bank: 18,
    generator: 0x4650,
};
/// efAlt_Spawn 0x47C: the Tornado's model.
pub(super) const TORNADO_MODEL: u32 = 0x3E9;
/// efSync_Spawn 0x509 (efsync.c:584-590): the Cyclone's model.
pub(super) const CYCLONE_MODEL: u32 = 0x4651;

impl Effects {
    /// efAlt_Spawn 0x47A (efalt.c:38-57), from ftMr_SpecialN_ItemFireSpawn,
    /// and efSync_Spawn 0x507 from ftLg_SpecialN_FireSpawn:
    /// efLib_Create_Attach(model) on the hand joint, turned once to the
    /// facing (an f64 -+M_PI_2 rounded by HSD_JObjSetRotationY), then the
    /// generator on the same joint. Both drain their animation queue after
    /// the switch, so the model animates after the generator spawns.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_hand_fire<T: InverseTrig>(
        &mut self,
        fire: HandFire,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(fire.model, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        // efAlt's attached character models retain state_flags=0.
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        let turn = if fighter.effect_facing() < 0.0 {
            -std::f64::consts::FRAC_PI_2
        } else {
            std::f64::consts::FRAC_PI_2
        };
        effect.tree.set_rotation_y(effect.root, turn as f32);
        let matrix = fighter.effect_matrix(Some(bone));
        effect.tree.set_translate(
            effect.root,
            &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        );
        assert!(bone < FIGHTER_JOINT_STRIDE);
        let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
        let mut spawn = SpawnRequest::new(fire.bank, fire.generator, 0);
        spawn.joint = Some((joint_id, matrix));
        self.events.spawn(&spawn, false, false);
        let bank = resources::character_bank(&self.character_banks, i32::from(fire.bank))?;
        spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
        self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
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

    /// efAlt_Spawn 0x47C (efalt.c:61-66), from the Tornado's setGfx, and
    /// efSync_Spawn 0x509 from the Cyclone's: efLib_Create_Attach_Scale(model)
    /// on the fighter's root with the efLib_Cb_ftMr_SpecialLw /
    /// efLib_Cb_ftLg_SpecialLw update (its tilt arrives as OwnedRotationZ;
    /// its child's JOBJ_HIDDEN toggle is display only).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_tornado<T: InverseTrig>(
        &mut self,
        model: u32,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(model, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        // efLib_Create_Attach_Scale: the fighter root's Y scale, broadcast.
        let mut scale = fighter.effect_scale();
        scale.x = scale.y;
        scale.z = scale.y;
        effect.tree.set_scale(effect.root, &scale);
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
}
