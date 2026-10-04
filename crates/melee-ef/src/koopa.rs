//! Bowser's efSync rows (efsync.c:312-371), bank 12 (EfKpData.dat).
use super::*;

/// efSync_Spawn 0x4DA's models: efLib_Create_Attach_Scale(0x2EE1), then
/// 0x2EE2 chained to it.
const FORTRESS_MODELS: [u32; 2] = [0x2EE1, 0x2EE2];
/// efSync_Spawn 0x4D8's generator: the Bowser Bomb's landing burst.
const BOMB_LANDING_GENERATOR: u32 = 0x61;
/// efSync_Spawn 0x4DF's generator: the Bowser Bomb's drop trail.
const BOMB_DROP_GENERATOR: u32 = 0x143;

impl Effects {
    /// efSync_Spawn 0x4DA (efsync.c:344-356), from ftKp_SpecialHi_Enter:
    /// two efLib_Create_Attach_Scale models on the same joint, the second
    /// chained to the first, with efLib_Cb_ftKp_SpecialHi as the update
    /// callback (eflib.c:1340-1363: the second model shows only in the
    /// grounded row, and both tilt to the floor). The callback changes
    /// only what is drawn, which the gate does not compare.
    pub(super) fn spawn_fortress<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut pair = FORTRESS_MODELS.map(|model| {
            let mut effect = self.acquire(model, particles);
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect
        });
        let matrix = fighter.effect_matrix(Some(bone));
        for effect in pair.iter_mut() {
            effect.owner = Some(ModelOwner::Fighter(player));
            effect.hitlag_pause = HitlagPause::Active;
            effect.attachment = Some(player);
            effect.attachment_bone = Some(bone);
            effect.scale_attachment = false;
            // HSD_JObjGetScale of the fighter root, broadcast from Y.
            let mut scale = fighter.effect_scale();
            scale.x = scale.y;
            scale.z = scale.y;
            effect.tree.set_scale(effect.root, &scale);
            effect.tree.set_translate(
                effect.root,
                &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
            );
        }
        // efSync_Spawn drains efLib_AnimQueue newest first (efsync.c:654-660).
        for effect in pair.iter_mut().rev() {
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
        }
        for effect in pair {
            self.instances.push(effect);
        }
        Ok(())
    }

    /// efSync_Spawn 0x4D8 (efsync.c:312-326), from the Bowser Bomb's
    /// landing (fn_80134518): efLib_CreateGenerator_AddAppSRT(0x61) whose
    /// AppSRT carries the point, the fighter root's Y scale uniformly and
    /// a rotation Y of M_PI_2 (double, rounded on the store).
    pub(super) fn spawn_bomb_landing<T: InverseTrig>(
        &mut self,
        position: Vec3,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let scale = fighter.effect_scale().y;
        let mut spawn = SpawnRequest::new(0, BOMB_LANDING_GENERATOR, 0);
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            translation: position,
            scale: Vec3::new(scale, scale, scale),
            rotation: Vec3::new(0.0, std::f64::consts::FRAC_PI_2 as f32, 0.0),
            status: 1,
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        spawn_particle::<T>(particles, common_bank, spawn, rng, &mut self.draws)?;
        Ok(())
    }

    /// efSync_Spawn 0x4DF (efsync.c:369-371), from the Bowser Bomb's drop
    /// (fn_80134590): efLib_CreateGenerator_Attach_Scale(0x143) on the
    /// joint (eflib.c:791-807): the generator's own AppSRT, type bits 9-10
    /// cleared and PSAPPSRT_UNK_B11 set, scaled by the fighter root's Y.
    pub(super) fn spawn_bomb_drop<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
        let scale = fighter.effect_scale().y;
        let mut spawn = SpawnRequest::new(0, BOMB_DROP_GENERATOR, 0);
        spawn.joint = Some((joint_id, fighter.effect_matrix(Some(bone))));
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            scale: Vec3::new(scale, scale, scale),
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        if let Some(id) = spawn_particle::<T>(particles, common_bank, spawn, rng, &mut self.draws)?
        {
            self.events.flags(joint_id, 0x600, 0x800);
            let generator = particles.generator_mut(id).unwrap();
            generator.flags = (generator.flags & !0x600) | 0x800;
        }
        self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
        Ok(())
    }
}
