//! efAsync_Dispatch positional generator branches (efasync.c:255-282).
use super::*;
const REVERSE_BRAKE_DUST_REQUEST: u16 = 0x400;
impl Effects {
    /// efSync_Spawn's 0x479 (efsync.c:67-69): efAlt's coin generator while
    /// Mario's bank (1) is loaded, else 0x506 (efLib_CreateGenerator 0x4658).
    pub fn coin_hit_request(&self) -> u16 {
        const MARIO_BANK: u32 = 1;
        if self.loaded_banks & (1 << MARIO_BANK) != 0 {
            0x479
        } else {
            0x506
        }
    }

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

    /// it_80278800's block_4FC (iteffect.c:343-351): efSync_Spawn(id, item,
    /// &pos, &item->facing_dir) for a row that turns its generator to a
    /// facing (efLib_CreateGenerator_Translate_FacingDir, e.g. 0x3EF/0x3F0).
    /// Rows without a facing take [`Self::spawn_positional`]'s path.
    pub fn spawn_positional_facing<T: InverseTrig>(
        &mut self,
        id: u16,
        position: Vec3,
        facing: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let directional = DUST_SPAWNS
            .iter()
            .any(|row| row.request == id && row.directional);
        let facing = if directional { facing } else { 1.0 };
        self.spawn_dust_generator::<T>(id, position, facing, bank, particles, rng)
    }

    /// efSync_Spawn(0x443, item, &pos, &facing) -> efAsync_Dispatch
    /// (efasync.c:786-799): the ice shattering, generator 0x1F1 whose
    /// AppSRT takes `position` and the second argument as a uniform scale
    /// (an item passes its facing, it_80278800's block_4FC).
    pub fn spawn_ice_shatter<T: InverseTrig>(
        &mut self,
        position: Vec3,
        scale: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut spawn = SpawnRequest::new(0, 0x1F1, 0);
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            translation: position,
            scale: Vec3::new(scale, scale, scale),
            status: 1,
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
        Ok(())
    }

    /// efSync_Spawn(0x435..=0x437, item, &pos, &angle) -> efAsync_Dispatch
    /// (efasync.c:694-716): generator 0x14E / 0x153 / 0x156 whose AppSRT takes
    /// `position`, a quarter turn about Y (M_PI_2, rounded) and `angle`
    /// about Z (a ray's burst, it_80278800's block_59C: pi facing left).
    pub fn spawn_ray_burst<T: InverseTrig>(
        &mut self,
        id: u16,
        position: Vec3,
        angle: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let generator = match id {
            0x435 => 0x14E,
            0x436 => 0x153,
            0x437 => 0x156,
            _ => anyhow::bail!("efasync.c:694-716: not a ray burst {id:#x}"),
        };
        let mut spawn = SpawnRequest::new(0, generator, 0);
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            translation: position,
            rotation: Vec3::new(0.0, core::f64::consts::FRAC_PI_2 as f32, angle),
            status: 1,
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
        Ok(())
    }

    /// efSync_Spawn(0x3E8, item, &pos, &damage) -> efAsync_Dispatch: the
    /// plain hit spark an item shows when a hit lands on it (it_80270E30).
    /// One of two spark models at random, scaled by the damage.
    pub fn spawn_item_hit_spark<T: InverseTrig>(
        &mut self,
        item: u32,
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
        // efLib_Create's parent_gobj: the struck item.
        effect.owner = Some(ModelOwner::Item(item));
        // efAsync_Dispatch, retail 80063A10: fmadds.
        let scale = gekko_math::fma::fmadds(0.04, damage, 0.3).clamp(0.3, 1.5);
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        effect.tree.set_translate(effect.root, &position);
        effect.animate::<T>(
            bank,
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
            &mut self.animation_events,
        )?;
        self.instances.push(effect);
        Ok(())
    }

    /// efAsync_Dispatch's 0x3ED (efasync.c:136-152), an Ice hit's spark on
    /// a fighter or a stage enemy (it_80270E30): generator 0x50 at the
    /// point, then 0x54 with the point as its AppSRT's translation, turned
    /// to face the victim (rot.y 0 when it faces left, else -M_PI as a
    /// double stored to a float).
    pub fn spawn_ice_hit_spark<T: InverseTrig>(
        &mut self,
        position: Vec3,
        victim_facing: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.spawn_dust_generator::<T>(0x50, position, victim_facing, bank, particles, rng)?;
        let turn = if victim_facing < 0.0 {
            0.0
        } else {
            -std::f64::consts::PI as f32
        };
        let mut spawn = SpawnRequest::new(0, 0x54, 0);
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            translation: position,
            rotation: Vec3::new(0.0, turn, 0.0),
            status: 1,
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?;
        Ok(())
    }

    /// efSync_Spawn(0x3EC, item, &pos, item) -> efAsync_Dispatch: a slashing
    /// hit's spark on a stage enemy (it_80270E30), model 8 turned about Z
    /// by a random angle (efasync.c:34-36: M_TAU is double, multiply then
    /// round), as on a fighter.
    pub fn spawn_item_slash_spark<T: InverseTrig>(
        &mut self,
        item: u32,
        position: Vec3,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(8, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.attachment = None;
        // efLib_Create's parent_gobj: the struck item.
        effect.owner = Some(ModelOwner::Item(item));
        self.events.external_randf(0x8006_3b70);
        effect.tree.set_rotation_z(
            effect.root,
            (std::f64::consts::TAU * f64::from(rng.randf())) as f32,
        );
        effect.tree.set_translate(effect.root, &position);
        effect.animate::<T>(
            bank,
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
            &mut self.animation_events,
        )?;
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
        // A character's generator id names its bank (efAlt rows).
        let bank = if resources::is_character_bank((kind / 1000) as u8) {
            resources::character_bank(&self.character_banks, (kind / 1000) as i32)?
        } else {
            bank
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
