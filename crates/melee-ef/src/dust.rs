//! efAsync_Dispatch positional generator branches (efasync.c:255-282).
use super::*;
const REVERSE_BRAKE_DUST_REQUEST: u16 = 0x400;
impl Effects {
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
        let facing = if id == REVERSE_BRAKE_DUST_REQUEST {
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
