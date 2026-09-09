//! Linked-list update order and Melee's two particle procs (`eflib.c`).
use crate::{
    bank::ParticleBank,
    generator::Generator,
    particle::{Particle, PAUSED},
    rng_sites::{DrawLog, EMISSION_COUNT},
    Error,
};
use gekko_math::{fma::fmadds, rng::HsdRng};
use hsd_anim::mtx::InverseTrig;

pub const MAIN_SKIP_MASK: u32 = 0x0006_0000;
pub const AUX_SKIP_MASK: u32 = 0x0001_0000;

/// Owned equivalents of `hsd_804D78FC` and `hsd_804D0908[16]`.
/// Vectors are in linked-list order; removal preserves all remaining order.
#[derive(Debug)]
pub struct ParticleSystem {
    pub generators: Vec<Generator>,
    pub particles: [Vec<Particle>; 16],
    /// Optional finite pool to reproduce failed particle allocations. Geometry
    /// draws still happen on failure; immediate interpreter draws do not.
    pub particle_capacity: usize,
    next_id: usize,
    /// lbl_804D6368 (0x804D6368), the u16 family-ID allocator.
    pub family_counter: u16,
    /// hsd_804D78F4 SList.data (+0x04), owned generator IDs in pending order.
    pub pending_generators: Vec<Option<usize>>,
    generator_cursor: Option<usize>,
}
impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            generators: Vec::new(),
            particles: std::array::from_fn(|_| Vec::new()),
            particle_capacity: usize::MAX,
            next_id: 0,
            family_counter: 0x100,
            pending_generators: Vec::new(),
            generator_cursor: None,
        }
    }
}
impl ParticleSystem {
    /// Restore head-to-tail lists without allocation, insertion or RNG draws.
    /// Generator IDs and particle associations must already be normalized.
    pub fn from_live_lists(
        generators: Vec<Generator>,
        particles: [Vec<Particle>; 16],
        family_counter: u16,
    ) -> Self {
        let next_id = generators.iter().map(|g| g.id + 1).max().unwrap_or(0);
        Self {
            generators,
            particles,
            family_counter,
            next_id,
            ..Self::default()
        }
    }

    /// `hsd_8039D9C8` (0x8039D9C8): insertion is after the current
    /// generator (cursor's successor), or after the head when cursor is null
    /// or at the tail. It is deliberately neither prepend nor append.
    pub fn insert_generator(&mut self, mut generator: Generator) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.family_counter = self.family_counter.wrapping_add(1).max(0x100);
        generator.id = id;
        generator.family_id = self.family_counter;
        let insertion = self
            .generator_cursor
            .and_then(|cursor| self.generators.iter().position(|g| g.id == cursor))
            .filter(|&index| index + 1 < self.generators.len())
            .map_or(usize::from(!self.generators.is_empty()), |index| index + 2);
        self.generators.insert(insertion, generator);
        id
    }

    /// Bank lookup and `hsd_8039F05C` creation. The caller can attach the
    /// evaluated spawn JObj matrix to the returned generator before its pass.
    pub fn spawn<T: InverseTrig>(
        &mut self,
        bank: &ParticleBank,
        bank_id: u8,
        descriptor_id: u32,
        link: u8,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<Option<usize>, Error> {
        if bank_id >= 65 || link >= 8 {
            return Ok(None);
        }
        let Some(descriptor) = bank.descriptor(descriptor_id) else {
            return Ok(None);
        };
        let mut generator = Generator::new::<T>(descriptor, bank_id, link, rng, draws)?;
        if let Some(Some(texture)) = bank.textures.get(usize::from(descriptor.texture_group)) {
            generator.texture_images = texture.images.clone().into();
            if texture.palette_flags != 0 {
                generator.descriptor.kind |= 0x10;
            }
        }
        Ok(Some(self.insert_generator(generator)))
    }

    pub fn generator_mut(&mut self, id: usize) -> Option<&mut Generator> {
        self.generators
            .iter_mut()
            .find(|generator| generator.id == id)
    }
    pub fn live_particles(&self) -> usize {
        self.particles.iter().map(Vec::len).sum()
    }

    /// `efLib_particles_proc_main`: s_link 15, p_link 11; interpreter first.
    pub fn proc_main<T: InverseTrig>(
        &mut self,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        self.update::<T>(MAIN_SKIP_MASK, rng, draws)
    }
    /// `efLib_particles_proc_aux`: s_link 15, p_link 12; interpreter first.
    /// Retail masks only link 0; links 3..15 can consequently update in both
    /// procs. Do not replace these masks with a partition of the link array.
    pub fn proc_aux<T: InverseTrig>(
        &mut self,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        self.update::<T>(AUX_SKIP_MASK, rng, draws)
    }
    pub fn update<T: InverseTrig>(
        &mut self,
        mask: u32,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        self.update_particles(mask, rng, draws)?;
        self.update_generators::<T>(mask, rng, draws)
    }

    /// `hsd_8039CEAC` (particle.c, 0x8039CEAC): link 0 through 15, each
    /// from head to tail. Expiration updates child counts before generators.
    pub fn update_particles(
        &mut self,
        mask: u32,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        for link in 0..16 {
            if mask & (1 << (link + 16)) != 0 {
                continue;
            }
            let mut index = 0;
            while index < self.particles[link].len() {
                if self.particles[link][index].update(rng, draws)? {
                    index += 1;
                } else {
                    let particle = self.particles[link].remove(index);
                    if let Some(generator) =
                        particle.generator_id.and_then(|id| self.generator_mut(id))
                    {
                        generator.children -= 1;
                    }
                }
            }
        }
        Ok(())
    }

    /// `hsd_8039EE24` (generator.c, 0x8039EE24), without pending detached
    /// JObj requests or application callbacks (unsupported at creation).
    pub fn update_generators<T: InverseTrig>(
        &mut self,
        mask: u32,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        self.generator_cursor = None;
        let mut index = 0;
        while index < self.generators.len() {
            let generator = &mut self.generators[index];
            let id = generator.id;
            if mask & (1 << (generator.link + 16)) == 0 && generator.descriptor.kind & PAUSED == 0 {
                generator.update_attachment();
                if generator.emission_rate < 0.0 {
                    generator.count -= generator.emission_rate;
                } else {
                    // retail 0x8039EF0C: fmadds, including rate==0.
                    generator.count = fmadds(
                        generator.emission_rate,
                        draws.draw(rng, EMISSION_COUNT),
                        generator.count,
                    );
                }
                if generator.count >= 1.0 {
                    self.emit::<T>(index, rng, draws)?;
                }
                let generator = &mut self.generators[index];
                if generator.remaining_life != 0 {
                    generator.remaining_life -= 1;
                    if generator.remaining_life == 0 && self.expire(index) {
                        continue;
                    }
                }
            }
            self.generator_cursor = Some(id);
            index += 1;
        }
        Ok(())
    }

    fn emit<T: InverseTrig>(
        &mut self,
        index: usize,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        let mut frame = self.generators[index].prepare::<T>(rng, draws)?;
        while self.generators[index].count >= 1.0 {
            let mut particle = self.generators[index].emit::<T>(&mut frame, rng, draws)?;
            if self.live_particles() < self.particle_capacity {
                let link = usize::from(particle.link);
                self.generators[index].children += 1;
                // The new head is interpreted immediately, before count--.
                let alive = particle.update(rng, draws)?;
                if alive {
                    self.particles[link].insert(0, particle);
                } else {
                    self.generators[index].children -= 1;
                }
            }
            self.generators[index].count -= 1.0;
        }
        Ok(())
    }

    /// `hsd_8039D3AC` (0x8039D3AC): expired parents stay alive with zero
    /// rate while children survive, still drawing once each eligible tick.
    fn expire(&mut self, index: usize) -> bool {
        let generator = &mut self.generators[index];
        if generator.flags & 0x80 != 0 {
            let particles = &mut self.particles[usize::from(generator.link)];
            let before = particles.len();
            particles.retain(|particle| {
                particle.generator_id != Some(generator.id)
                    || particle.family_id != generator.family_id
            });
            generator.children -= (before - particles.len()) as u32;
        }
        if generator.children != 0 {
            generator.emission_rate = 0.0;
            generator.remaining_life = 1;
            false
        } else {
            self.generators.remove(index);
            true
        }
    }
}
