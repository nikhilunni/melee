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
use hsd_types::Mtx;
use std::sync::Arc;

pub const MAIN_SKIP_MASK: u32 = 0x0006_0000;
pub const AUX_SKIP_MASK: u32 = 0x0001_0000;

/// Effect-layer input to `hsd_8039F05C` and its attachment/position wrappers.
/// Velocity overrides are applied after descriptor initialization, as C callers do.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub bank: u8,
    pub kind: u32,
    pub link: u8,
    pub position: [f32; 3],
    pub velocity: Option<[f32; 3]>,
    pub joint: Option<(usize, Mtx)>,
    pub application_transform: Option<crate::generator::ApplicationTransform>,
    pub mirror: bool,
}
impl SpawnRequest {
    pub fn new(bank: u8, kind: u32, link: u8) -> Self {
        Self {
            bank,
            kind,
            link,
            position: [0.0; 3],
            velocity: None,
            joint: None,
            application_transform: None,
            mirror: false,
        }
    }
}

/// Owned equivalents of `hsd_804D78FC` and `hsd_804D0908[16]`.
/// Port storage bound for simultaneously retained AppSRT versions.
const APPLICATION_TRANSFORM_CAPACITY: usize = 256;

/// Vectors are in linked-list order; removal preserves all remaining order.
#[derive(Debug)]
pub struct ParticleSystem {
    pub generators: Vec<Generator>,
    pub particles: [Vec<Particle>; 16],
    /// Optional finite pool to reproduce failed particle allocations. Geometry
    /// draws still happen on failure; immediate interpreter draws do not.
    pub particle_capacity: usize,
    next_id: usize,
    // psInitDataBank's bank < 65 guard (particle.c:329); cloning a bank only
    // shares its immutable descriptor and texture tables, allocated at load.
    banks: [Option<ParticleBank>; 65],
    /// lbl_804D6368 (0x804D6368), the u16 family-ID allocator.
    pub family_counter: u16,
    /// hsd_804D08E8: eight optional point-joint positions. Unbound slots are null.
    pub point_joints: [Option<[f32; 3]>; 8],
    /// hsd_804D78F4 SList.data (+0x04), owned generator IDs in pending order.
    pub pending_generators: Vec<Option<usize>>,
    generator_cursor: Option<usize>,
    application_pool: Vec<Arc<crate::generator::ApplicationTransform>>,
}
impl Default for ParticleSystem {
    fn default() -> Self {
        // Initialize the shared empty texture owner before the first spawn,
        // including scenes whose imported particle lists are initially empty.
        drop(crate::bank::empty_images());
        Self {
            generators: Vec::new(),
            particles: std::array::from_fn(|_| Vec::new()),
            particle_capacity: usize::MAX,
            next_id: 0,
            banks: std::array::from_fn(|_| None),
            family_counter: 0x100,
            point_joints: [None; 8],
            pending_generators: Vec::new(),
            generator_cursor: None,
            // Port bound: preallocate shared AppSRT owners; exhaustion is explicit.
            application_pool: (0..APPLICATION_TRANSFORM_CAPACITY)
                .map(|_| Arc::new(Default::default()))
                .collect(),
        }
    }
}
impl ParticleSystem {
    /// Immutable descriptor bank retained by the first spawn in that bank.
    pub fn bank(&self, id: u8) -> Option<&ParticleBank> {
        self.banks.get(usize::from(id)).and_then(Option::as_ref)
    }
    /// HSD AppSRT shared lifetime, with storage prepared before simulation.
    /// A slot can be overwritten only after every generator/particle alias expires.
    fn share_application_transform(
        &mut self,
        value: crate::generator::ApplicationTransform,
    ) -> Arc<crate::generator::ApplicationTransform> {
        let slot = self
            .application_pool
            .iter_mut()
            .find(|slot| Arc::strong_count(slot) == 1)
            .expect("application transform pool exhausted");
        *Arc::get_mut(slot).expect("unshared application transform") = value;
        Arc::clone(slot)
    }

    /// An external Arc may outlive its last simulated owner. Transfer that
    /// allocation out of the pool so its lifetime remains caller-owned. Normal
    /// simulation has no external aliases and keeps every prepared slot.
    fn release_external_transforms(&mut self) {
        self.application_pool.retain(|transform| {
            Arc::strong_count(transform) == 1
                || self.generators.iter().any(|g| {
                    g.application_transform
                        .as_ref()
                        .is_some_and(|t| Arc::ptr_eq(t, transform))
                })
                || self.particles.iter().flatten().any(|p| {
                    p.application_transform
                        .as_ref()
                        .is_some_and(|t| Arc::ptr_eq(t, transform))
                })
        });
    }

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
        if let Some(transform) = &mut generator.application_transform {
            generator.appsrt_id = Some(id);
            if transform.family_id != self.family_counter {
                let mut value = (**transform).clone();
                value.family_id = self.family_counter;
                *transform = self.share_application_transform(value);
            }
        }
        let insertion = self
            .generator_cursor
            .and_then(|cursor| self.generators.iter().position(|g| g.id == cursor))
            .filter(|&index| index + 1 < self.generators.len())
            .map_or(usize::from(!self.generators.is_empty()), |index| index + 2);
        self.generators.insert(insertion, generator);
        id
    }

    /// Bank lookup and `hsd_8039F05C` creation. The first successful spawn
    /// registers this bank for synchronous child-generator instructions.
    /// The caller supplies evaluated matrices; attachment position is refreshed
    /// before emission, while explicit velocity overrides apply after creation.
    pub fn spawn<T: InverseTrig>(
        &mut self,
        bank: &ParticleBank,
        request: SpawnRequest,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<Option<usize>, Error> {
        let SpawnRequest {
            bank: bank_id,
            kind: descriptor_id,
            link,
            position,
            velocity,
            joint,
            application_transform,
            mirror,
        } = request;
        if bank_id >= 65 || link >= 8 {
            return Ok(None);
        }
        let Some(descriptor) = bank.descriptor(descriptor_id) else {
            return Ok(None);
        };
        // hsd_8039F05C, generator.c:1210-1217; system owns allocation and aliases.
        let mut application_transform = application_transform.or_else(|| {
            (descriptor.kind & 0x20000 != 0).then(crate::generator::ApplicationTransform::default)
        });
        // hsd_8039F05C (8039F68C): the descriptor sets this byte before
        // the effect caller applies its SRT overrides to the existing object.
        if descriptor.kind & 0x20000 != 0 {
            application_transform.as_mut().unwrap().camera_facing = 1;
        }
        let application_transform = application_transform.map(|mut value| {
            value.family_id = self.family_counter.wrapping_add(1).max(0x100);
            self.share_application_transform(value)
        });
        let mut generator = Generator::with_application_transform::<T>(
            descriptor,
            bank_id,
            link,
            application_transform,
            rng,
            draws,
        )?;
        if let Some(Some(texture)) = bank.textures.get(usize::from(descriptor.texture_group)) {
            generator.texture_images = Arc::clone(&texture.images);
            if texture.palette_flags != 0 {
                generator.descriptor.kind |= 0x10;
            }
        }
        if mirror {
            generator.descriptor.kind |= 0x40000;
        }
        generator.position = position;
        if let Some(velocity) = velocity {
            generator.descriptor.velocity = velocity;
        }
        if let Some((id, matrix)) = joint {
            generator.attachment_id = Some(id);
            generator.attach_joint(matrix);
        }
        self.banks[usize::from(bank_id)].get_or_insert_with(|| bank.clone());
        Ok(Some(self.insert_generator(generator)))
    }

    /// Includes inherited child attachments and zero-rate generators retained by particles.
    pub fn has_joint_attachment(&self, id: usize) -> bool {
        self.generators
            .iter()
            .any(|generator| generator.attachment_id == Some(id))
    }

    /// Refresh a caller-owned animated joint, including inherited child attachments.
    pub fn update_joint(&mut self, id: usize, matrix: Mtx) {
        for generator in &mut self.generators {
            if generator.attachment_id == Some(id) {
                generator.joint_matrix = Some(matrix);
            }
        }
    }

    /// hsd_8039D214 (8039D214), generator.c:38-71: refresh the shared owner
    /// before its emission. Existing particles and child generators retain aliases.
    fn update_application_attachment(&mut self, index: usize) {
        let generator = &mut self.generators[index];
        if generator.flags & 0x100 == 0 {
            return;
        }
        let (Some(matrix), Some(transform), Some(id)) = (
            generator.joint_matrix,
            generator.application_transform.as_mut(),
            generator.appsrt_id,
        ) else {
            return;
        };
        if id != generator.id || generator.flags & 0x1800 == 0 {
            return;
        }
        let mut transform = (**transform).clone();
        if generator.flags & 0x800 != 0 {
            transform.translation =
                hsd_types::Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        }
        if generator.flags & 0x1000 != 0 {
            hsd_anim::mtx::hsd_mtx_get_scale(&matrix, &mut transform.scale);
        }
        self.set_application_transform(id, transform);
    }

    /// Caller-owned AppSRT writes are shared by the generator and its live children.
    pub fn set_application_transform(
        &mut self,
        id: usize,
        transform: crate::generator::ApplicationTransform,
    ) {
        let transform = self.share_application_transform(transform);
        for generator in &mut self.generators {
            if generator.appsrt_id == Some(id) {
                generator.application_transform = Some(transform.clone());
            }
        }
        for particle in self.particles.iter_mut().flatten() {
            if particle.appsrt_id == Some(id) {
                particle.application_transform = Some(transform.clone());
            }
        }
    }

    fn update_particle<T: InverseTrig>(
        &mut self,
        particle: &mut Particle,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<bool, Error> {
        let tornado = particle
            .generator_id
            .and_then(|id| self.generators.iter().find(|g| g.id == id))
            .and_then(|g| {
                if let crate::generator::EmissionShape::Tornado { speed } = g.shape {
                    Some(crate::particle::TornadoPhysics {
                        position: g.position,
                        speed,
                        radius: g.descriptor.radius,
                        angle: g.descriptor.angle,
                        angular_speed: g.descriptor.gravity,
                    })
                } else {
                    None
                }
            });
        let point_joints = self.point_joints;
        particle.update_with_generators::<T>(
            tornado,
            Some(T::atan2f),
            &point_joints,
            rng,
            draws,
            &mut |parent, kind, blend, rng, draws| {
                let bank = self
                    .banks
                    .get(usize::from(parent.bank))
                    .and_then(Clone::clone)
                    .ok_or(Error::UnsupportedFeature(
                        "child generator bank not registered",
                    ))?;
                let attachment = parent
                    .generator_id
                    .and_then(|id| self.generators.iter().find(|g| g.id == id))
                    .map(|g| (g.flags, g.attachment_id, g.joint_matrix));
                let request = SpawnRequest::new(parent.bank, kind, parent.link);
                if let Some(id) = self.spawn::<T>(&bank, request, rng, draws)? {
                    let child = self.generator_mut(id).unwrap();
                    child.family_id = parent.family_id;
                    // particle.c:1098-1108/1162-1174: a child without its own
                    // AppSRT shares the parent's transform and keeps local position.
                    child.appsrt_id = parent.appsrt_id;
                    child.application_transform = parent.application_transform.clone();
                    child.position = parent.position;
                    child.flags |= 0x100;
                    if let Some((flags, attachment_id, matrix)) = attachment {
                        child.flags |= flags & 0x1e00;
                        child.attachment_id = attachment_id;
                        child.joint_matrix = matrix;
                    }
                    if let Some(blend) = blend {
                        child.descriptor.kind =
                            (child.descriptor.kind & 0xf1ff_ffff) | (u32::from(blend & 7) << 25);
                    }
                }
                Ok(())
            },
        )
    }

    /// `particleSort` (psdisp.c, 0x8039FC70): rendering stably buckets the *simulation*
    /// lists by TexEdge and blend mode. Call between scheduler ticks when a
    /// display pass occurred, before the next interpreter traversal.
    pub fn sort_for_display(&mut self, links: u16) {
        for (link, particles) in self.particles.iter_mut().enumerate() {
            if links & (1 << link) != 0 {
                let bucket =
                    |p: &Particle| ((p.kind >> 25) & 7) + if p.kind & 8 == 0 { 8 } else { 0 };
                // Retail links each of 16 buckets in input order. An explicit
                // input ordinal makes the same ordering a total key, allowing
                // Rust's in-place sort without population-sized scratch.
                if !particles.is_sorted_by_key(bucket) {
                    for (order, particle) in particles.iter_mut().enumerate() {
                        particle.display_order = order;
                    }
                    particles.sort_unstable_by_key(|p| (bucket(p), p.display_order));
                }
            }
        }
    }

    /// grLib_801C9834 -> hsd_8039D4DC: stop one generator while retaining its children.
    pub fn expire_generator(&mut self, id: usize) {
        if let Some(index) = self.generators.iter().position(|g| g.id == id) {
            self.expire(index);
        }
        self.release_external_transforms();
    }
    /// hsd_8039D688 (0x8039D688): effect destruction visits each owned joint.
    /// Existing children retain a zero-rate generator, except attached AppSRT
    /// generators, whose particles are removed by the type-0x80 path.
    pub fn expire_joint(&mut self, joint: usize) {
        let mut index = 0;
        while index < self.generators.len() {
            let generator = &mut self.generators[index];
            if generator.attachment_id == Some(joint) {
                if generator.appsrt_id.is_some() && generator.flags & 0x100 != 0 {
                    generator.flags |= 0x80;
                }
                if self.expire(index) {
                    continue;
                }
            }
            index += 1;
        }
        self.release_external_transforms();
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
        self.update_particles::<T>(mask, rng, draws)?;
        self.update_generators::<T>(mask, rng, draws)
    }

    /// `hsd_8039CEAC` (particle.c, 0x8039CEAC): link 0 through 15, each
    /// from head to tail. Expiration updates child counts before generators.
    pub fn update_particles<T: InverseTrig>(
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
                let mut particle = self.particles[link].remove(index);
                if self.update_particle::<T>(&mut particle, rng, draws)? {
                    self.particles[link].insert(index, particle);
                    index += 1;
                } else {
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
        // hsd_8039EE24 (8039EE24), generator.c:970-977: queued one-shot
        // attachments are baked and released before generator iteration.
        for index in 0..self.pending_generators.len() {
            if let Some(id) = self.pending_generators[index] {
                if let Some(generator) = self.generator_mut(id) {
                    generator.detach_joint();
                }
            }
        }
        self.pending_generators.clear();
        self.generator_cursor = None;
        let mut index = 0;
        while index < self.generators.len() {
            let generator = &mut self.generators[index];
            let id = generator.id;
            if mask & (1 << (generator.link + 16)) == 0 && generator.descriptor.kind & PAUSED == 0 {
                self.update_application_attachment(index);
                let generator = &mut self.generators[index];
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
        self.release_external_transforms();
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
                let alive = self.update_particle::<T>(&mut particle, rng, draws)?;
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
        let id = generator.id;
        let owns_attached_transform = generator.flags & 0x100 != 0
            && generator.attachment_id.is_some()
            && generator.flags & 0x1800 != 0
            && generator.appsrt_id == Some(id);
        // hsd_8039D3AC (8039D3AC), generator.c:97-110: the transform's
        // attached owner survives until its last generator/particle alias.
        let shared_transform = owns_attached_transform
            && (self
                .generators
                .iter()
                .any(|g| g.id != id && g.appsrt_id == Some(id))
                || self
                    .particles
                    .iter()
                    .flatten()
                    .any(|p| p.appsrt_id == Some(id)));
        let generator = &mut self.generators[index];
        if generator.children != 0 || shared_transform {
            generator.emission_rate = 0.0;
            generator.remaining_life = 1;
            false
        } else {
            self.generators.remove(index);
            true
        }
    }
}

impl Clone for ParticleSystem {
    fn clone(&self) -> Self {
        use hsd_types::storage::clone_vec;
        let application_pool: Vec<_> = self
            .application_pool
            .iter()
            .map(|value| Arc::new((**value).clone()))
            .collect();
        type TransformOwner = Arc<crate::generator::ApplicationTransform>;
        let mut imported: Vec<(TransformOwner, TransformOwner)> = Vec::new();
        let mut rebind = |value: &mut Option<Arc<crate::generator::ApplicationTransform>>| {
            if let Some(old) = value {
                if let Some(index) = self
                    .application_pool
                    .iter()
                    .position(|slot| Arc::ptr_eq(slot, old))
                {
                    *old = Arc::clone(&application_pool[index]);
                } else {
                    // Preserve aliases among imported owners as well as pool slots.
                    if let Some((_, copied)) =
                        imported.iter().find(|(source, _)| Arc::ptr_eq(source, old))
                    {
                        *old = Arc::clone(copied);
                    } else {
                        let copied = Arc::new((**old).clone());
                        imported.push((Arc::clone(old), Arc::clone(&copied)));
                        *old = copied;
                    }
                }
            }
        };
        let mut generators = clone_vec(&self.generators);
        for generator in &mut generators {
            rebind(&mut generator.application_transform);
        }
        let mut particles = self.particles.each_ref().map(clone_vec);
        for particle in particles.iter_mut().flatten() {
            rebind(&mut particle.application_transform);
        }
        Self {
            generators,
            particles,
            particle_capacity: self.particle_capacity,
            next_id: self.next_id,
            banks: self.banks.clone(),
            family_counter: self.family_counter,
            point_joints: self.point_joints,
            pending_generators: clone_vec(&self.pending_generators),
            generator_cursor: self.generator_cursor,
            application_pool,
        }
    }
}
