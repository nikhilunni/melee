//! Complete a savestate paused in the spherical emitter's final cosf call.
//! This is boundary restoration, using saved stack operands and the ordinary
//! particle interpreter. No oracle row or reconstructed RNG history is used.
use super::{float, word, SavedPose};
use anyhow::{ensure, Context, Result};
use gekko_math::{fma::fmadds, msl::cosf, HsdRng};
use hsd_particle::{
    generator::EmissionShape, particle::Particle, rng_sites::DrawLog, system::ParticleSystem,
};
use hsd_types::{Mtx, Vec3};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct PendingEmission {
    particle: Particle,
}
impl PendingEmission {
    pub(super) fn restore(
        saved: &SavedPose,
        system: &ParticleSystem,
        metadata: &Value,
    ) -> Result<Self> {
        let (registers, pc) = saved.cpu_general_registers()?;
        ensure!(
            registers[27] == word(saved.bytes(0x804D_7838, 4), 0) && registers[28] == 15,
            "saved CPU and scheduler disagree"
        );
        let stack = registers[1];
        let emitter_stack = word(saved.bytes(stack, 4), 0);
        // cosf frame is 0x28 bytes; LR in the caller's linkage area points
        // to hsd_8039DAD4's final direction component store (8039EC84).
        ensure!(
            (0x8032_6240..0x8032_63D4).contains(&pc)
                && emitter_stack == stack + 0x28
                && word(saved.bytes(emitter_stack + 4, 4), 0) == 0x8039_EC84,
            "unsupported suspended particle instruction {pc:08X}"
        );
        let update_stack = word(saved.bytes(emitter_stack, 4), 0);
        ensure!(
            update_stack == emitter_stack + 0x230
                && word(saved.bytes(update_stack + 4, 4), 0) == 0x8039_EF2C,
            "unsupported suspended generator caller"
        );
        ensure!(
            system.generators.len() == 1 && system.pending_generators.is_empty(),
            "partial emitter requires a single generator and no pending requests"
        );
        let generator = &system.generators[0];
        ensure!(
            metadata["particles"]["generators"][0]["pointer"].as_u64()
                == Some(u64::from(registers[30])),
            "suspended emitter generator does not match captured list"
        );
        let EmissionShape::Sphere { speed, .. } = generator.shape else {
            anyhow::bail!("suspended emitter is not spherical");
        };
        ensure!(
            generator.descriptor.radius < 0.0
                && (1.0..2.0).contains(&generator.count)
                && generator.remaining_life > 1,
            "unsupported partial emission radius/count/lifetime"
        );
        let frame = saved.bytes(emitter_stack, 0x164);
        let basis = Mtx(std::array::from_fn(|r| {
            std::array::from_fn(|c| float(frame, 0x110 + r * 16 + c * 4))
        }));
        // 8039EC60/7C already stored X/Y. cosf's input is at callee SP+8;
        // finishing it supplies the pending 8039EC84 Z store.
        let local = Vec3::new(
            float(frame, 0x140),
            float(frame, 0x144),
            cosf(float(saved.bytes(stack + 8, 4), 0)),
        );
        let mut direction = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(&basis, &local, &mut direction);
        let direction = [direction.x, direction.y, direction.z];
        let mut particle = Particle::new(&generator.descriptor, generator.bank, generator.link)?;
        particle.generator_id = Some(generator.id);
        particle.family_id = generator.family_id;
        particle.appsrt_id = generator.appsrt_id;
        particle.application_transform = generator.application_transform.clone();
        particle.texture_images = generator.texture_images.clone();
        // 8039ECA0/ECB0/ECC0: separate fmuls. Negative radius bypasses
        // the velocity-rescale branch; 8039ED18/28/38 are fmadds.
        particle.velocity = direction.map(|component| component * speed);
        particle.position = std::array::from_fn(|i| {
            fmadds(
                -generator.descriptor.radius,
                direction[i],
                generator.position[i],
            )
        });
        Ok(Self { particle })
    }

    pub(crate) fn finish(
        mut self,
        system: &mut ParticleSystem,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<()> {
        let generator = system
            .generators
            .first_mut()
            .context("pending emission lost generator")?;
        ensure!(
            Some(generator.id) == self.particle.generator_id,
            "pending emission changed generator"
        );
        generator.children += 1;
        // psGenerateParticle0 -> hsd_8039930C: immediate interpretation,
        // then the generator loop subtracts one and decrements its lifetime.
        if self
            .particle
            .update::<melee_ft::fighter::RetailTrig>(rng, draws)?
        {
            system.particles[usize::from(self.particle.link)].insert(0, self.particle);
        } else {
            generator.children -= 1;
        }
        // retail 8039EDA8: fsubs; 8039EF40..50: integer lifetime decrement.
        generator.count -= 1.0;
        generator.remaining_life -= 1;
        Ok(())
    }
}
