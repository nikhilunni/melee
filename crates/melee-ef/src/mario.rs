//! Mario's efAlt rows (efalt.c:38-72), bank 1 (EfMrData.dat).
use super::*;

/// efAlt_Spawn 0x47A: the fireball's flash, model 0x3E8.
const HAND_FIRE_MODEL: u32 = 0x3E8;
/// hsd_8039EFAC(0, 1, 0x3E9, jobj): the flash's generator on the hand.
const HAND_FIRE_GENERATOR: u32 = 0x3E9;
const MARIO_BANK: u8 = 1;

impl Effects {
    /// efAlt_Spawn 0x47A (efalt.c:38-57), from ftMr_SpecialN_ItemFireSpawn:
    /// efLib_Create_Attach(0x3E8) on the hand joint, turned once to the
    /// facing (an f64 -+M_PI_2 rounded by HSD_JObjSetRotationY), then
    /// generator 0x3E9 on the same joint. efAlt drains its animation queue
    /// after the switch, so the model animates after the generator spawns.
    pub(super) fn spawn_hand_fire<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(HAND_FIRE_MODEL, particles);
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
        let mut spawn = SpawnRequest::new(MARIO_BANK, HAND_FIRE_GENERATOR, 0);
        spawn.joint = Some((joint_id, matrix));
        self.events.spawn(&spawn, false, false);
        let bank = resources::character_bank(&self.character_banks, i32::from(MARIO_BANK))?;
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
}
