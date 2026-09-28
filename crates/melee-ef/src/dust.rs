//! efAsync_Dispatch positional generator branches (efasync.c:255-282).
use super::*;
const REVERSE_BRAKE_DUST_REQUEST: u16 = 0x400;
impl Effects {
    /// efSync_Spawn -> efAsync_Dispatch's positional generator rows for an
    /// owner without effect state (items): the generator starts at once.
    pub fn spawn_positional<T: InverseTrig>(
        &mut self,
        id: u16,
        position: Vec3,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.spawn_dust_generator::<T>(id, position, 1.0, bank, particles, rng)
    }

    /// efSync_Spawn(0x3E8, item, &pos, &damage) -> efAsync_Dispatch: the
    /// plain hit spark an item shows when a hit lands on it (it_80270E30).
    /// One of two spark models at random, scaled by the damage.
    pub fn spawn_item_hit_spark<T: InverseTrig>(
        &mut self,
        position: Vec3,
        damage: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.direct_draws.push(0x8006_3990);
        let id = if rng.randi(8) == 0 { 9 } else { 10 };
        let mut effect = self.acquire(id, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.attachment = None;
        effect.owner = None;
        // efAsync_Dispatch, retail 80063A10: fmadds.
        let scale = gekko_math::fma::fmadds(0.04, damage, 0.3).clamp(0.3, 1.5);
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        effect.tree.set_translate(effect.root, &position);
        effect.animate::<T>(bank, particles, rng, &mut self.draws, &mut self.events)?;
        self.instances.push(effect);
        Ok(())
    }

    /// efSync_Spawn(0x3EC, item, &pos, item) -> efAsync_Dispatch: a slashing
    /// hit's spark on a stage enemy (it_80270E30), model 8 turned about Z
    /// by a random angle (efasync.c:34-36: M_TAU is double, multiply then
    /// round), as on a fighter.
    pub fn spawn_item_slash_spark<T: InverseTrig>(
        &mut self,
        position: Vec3,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(8, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.attachment = None;
        effect.owner = None;
        self.events.external_randf(0x8006_3b70);
        effect.tree.set_rotation_z(
            effect.root,
            (std::f64::consts::TAU * f64::from(rng.randf())) as f32,
        );
        effect.tree.set_translate(effect.root, &position);
        effect.animate::<T>(bank, particles, rng, &mut self.draws, &mut self.events)?;
        self.instances.push(effect);
        Ok(())
    }

    pub(super) fn spawn_dust_generator<T: InverseTrig>(
        &mut self,
        id: u16,
        position: Vec3,
        facing: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let (kind, directional) =
            if let Some(row) = DUST_SPAWNS.iter().find(|row| row.request == id) {
                (row.particle, row.directional)
            } else if id < 0x250 || id / 1000 == 30 {
                (u32::from(id), false)
            } else {
                anyhow::bail!("efasync.c:255-282: unsupported dust {id:#x}");
            };
        // efAsync_Dispatch (80063930), efasync.c:274-278: 0x400 reverses
        // the direction passed to the same 0x5A generator (fneg, no fusion).
        let facing = if matches!(id, REVERSE_BRAKE_DUST_REQUEST | 0x3EF | 0x3F1) {
            -facing
        } else {
            facing
        };
        // efLib_CreateGenerator (0x8005C9FC) selects a particle descriptor,
        // unlike model effects' effCommonDataTable entries.
        let link = match kind {
            0x121 => 2,
            0xFF | 0x7918 | 0xFC | 0xF7 => 1,
            _ => 0,
        };
        let mut request = SpawnRequest::new((kind / 1000) as u8, kind, link);
        if directional {
            // efLib_CreateGenerator_Translate_FacingDir: generator position stays
            // local zero; its shared AppSRT carries the world translation.
            request.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                translation: position,
                rotation: Vec3::new(
                    0.0,
                    if facing < 0.0 {
                        -std::f32::consts::FRAC_PI_2
                    } else {
                        std::f32::consts::FRAC_PI_2
                    },
                    0.0,
                ),
                scale: Vec3::new(1.0, 1.0, 1.0),
                status: 1,
                ..Default::default()
            });
            request.mirror = facing < 0.0;
        } else {
            request.position = [position.x, position.y, position.z];
        }
        ensure!(
            bank.descriptor(kind).is_some(),
            "particle descriptor {kind} absent from supplied bank"
        );
        self.events.spawn(&request, false, false);
        spawn_particle::<T>(particles, bank, request, rng, &mut self.draws)?;
        Ok(())
    }
}
