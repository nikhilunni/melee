//! Canonical boundary adapter promoted from hsd-particle/tests/support/restore.
use gekko_math::rng::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    generator::{EmissionShape, Generator},
    particle::Particle,
    rng_sites::DrawLog,
    system::ParticleSystem,
};
use melee_diff::{Record, Value};
use melee_ft::fighter::RetailTrig;
use std::collections::BTreeMap;

pub trait Banks {
    fn bank(&self, id: u8) -> &ParticleBank;
}
impl Banks for ParticleBank {
    fn bank(&self, _: u8) -> &ParticleBank {
        self
    }
}
impl Banks for BTreeMap<u8, ParticleBank> {
    fn bank(&self, id: u8) -> &ParticleBank {
        self.get(&id).expect("captured bank loaded")
    }
}

pub fn uint(record: &Record, key: &str) -> u64 {
    match record.state.get(key) {
        Some(Value::UInt(value)) => *value,
        other => panic!("{key}: expected unsigned integer, got {other:?}"),
    }
}
trait Scalar: Copy {
    fn encode(self) -> Value;
    fn decode(value: &Value) -> Self;
}
macro_rules! integer {
    ($($ty:ty),*) => { $(impl Scalar for $ty {
        fn encode(self) -> Value { Value::UInt(self as u64) }
        fn decode(value: &Value) -> Self {
            match value { Value::UInt(n) => Self::try_from(*n).expect("field width"), _ => panic!("expected integer: {value:?}") }
        }
    })* };
}
integer!(u8, u16, u32, usize);
impl Scalar for f32 {
    fn encode(self) -> Value {
        Value::f32(self)
    }
    fn decode(value: &Value) -> Self {
        match value {
            Value::F32 { bits, .. } => Self::from_bits(*bits),
            _ => panic!("expected f32: {value:?}"),
        }
    }
}
impl Scalar for Option<usize> {
    fn encode(self) -> Value {
        self.map_or(Value::Null, |n| n.encode())
    }
    fn decode(value: &Value) -> Self {
        match value {
            Value::Null => None,
            _ => Some(usize::decode(value)),
        }
    }
}
/// What a snapshot field belongs to. With the field's name and optional
/// array component it spells the dump's key (`harness/particle_dump.py`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// A whole key such as `rng.seed`.
    Global,
    /// `particles.generator[i]`, in generator-list order.
    Generator(usize),
    /// `particles.link[l]`, one of the sixteen particle lists.
    Link(usize),
    /// `particles.link[l].particle[i]`, in list order.
    Particle { link: usize, index: usize },
    /// `particles.appsrt[i]`, in first-encounter order.
    AppSrt(usize),
    /// `particles.pending[i]`, the pending-generator list.
    Pending(usize),
}

/// The dump's key of one field.
pub fn key(scope: Scope, name: &str, component: Option<usize>) -> String {
    let mut key = match scope {
        Scope::Global => name.to_string(),
        Scope::Generator(index) => format!("particles.generator[{index}].{name}"),
        Scope::Link(link) => format!("particles.link[{link}].{name}"),
        Scope::Particle { link, index } => {
            format!("particles.link[{link}].particle[{index}].{name}")
        }
        Scope::AppSrt(index) => format!("particles.appsrt[{index}].{name}"),
        Scope::Pending(index) => format!("particles.pending[{index}].{name}"),
    };
    if let Some(component) = component {
        key.push_str(&format!("[{component}]"));
    }
    key
}

/// The bank descriptor whose program `program` is. Some banks hold
/// byte-identical programs (bank 0's 116 and 424), so the shared allocation
/// identifies it; bytes are the fallback.
pub fn program_id(bank: &ParticleBank, program: &std::sync::Arc<[u8]>) -> Option<u32> {
    let find = |matches: &dyn Fn(&std::sync::Arc<[u8]>) -> bool| {
        bank.descriptors.iter().enumerate().find_map(|(index, d)| {
            d.as_ref()
                .filter(|d| matches(&d.program))
                .map(|_| bank.first_descriptor_id + index as u32)
        })
    };
    find(&|p| std::sync::Arc::ptr_eq(p, program))
        .or_else(|| find(&|p| p.as_ref() == program.as_ref()))
}

/// Where a snapshot's fields go: a record's key map, or a digest that never
/// spells the keys (`melee-sim`'s particle gate).
pub trait FieldSink {
    fn put(&mut self, scope: Scope, name: &str, component: Option<usize>, value: Value);
    /// [`program_id`]; a sink that sees every tick may remember the answer.
    fn program_id(
        &mut self,
        _bank_id: u8,
        bank: &ParticleBank,
        program: &std::sync::Arc<[u8]>,
    ) -> Option<u32> {
        program_id(bank, program)
    }
    /// Whether the sink takes the AppSRT display caches (`particles.appsrt[i]`).
    fn wants_appsrts(&self) -> bool {
        true
    }
}
impl FieldSink for BTreeMap<String, Value> {
    fn put(&mut self, scope: Scope, name: &str, component: Option<usize>, value: Value) {
        self.insert(key(scope, name, component), value);
    }
}

/// The same field map reads and writes. Round-trip equality detects unknown
/// capture fields; replay compares independently updated state with the oracle.
struct Fields<'a, S: FieldSink> {
    scope: Scope,
    source: Option<&'a Record>,
    output: &'a mut S,
}
impl<S: FieldSink> Fields<'_, S> {
    fn scalar<T: Scalar>(&mut self, name: &str, value: &mut T) {
        self.component(name, None, value);
    }
    fn component<T: Scalar>(&mut self, name: &str, component: Option<usize>, value: &mut T) {
        if let Some(source) = self.source {
            let key = key(self.scope, name, component);
            *value = T::decode(
                source
                    .state
                    .get(&key)
                    .unwrap_or_else(|| panic!("missing {key}")),
            );
        }
        self.output.put(self.scope, name, component, value.encode());
    }
    fn array<T: Scalar, const N: usize>(&mut self, name: &str, values: &mut [T; N]) {
        for (index, value) in values.iter_mut().enumerate() {
            self.component(name, Some(index), value);
        }
    }
    fn program(&mut self, bank_id: u8, bank: &ParticleBank, program: &std::sync::Arc<[u8]>) {
        // The dump names the descriptor whose program the pointer is.
        let id = self.output.program_id(bank_id, bank, program);
        let resolved = u8::from(id.is_some());
        let index = id.map(|n| n as usize);
        // These are derived from the actual program, never copied from expected.
        self.output
            .put(self.scope, "program_kind", None, index.encode());
        self.output
            .put(self.scope, "program_resolved", None, resolved.encode());
        if let Some(source) = self.source {
            assert_eq!(
                source.state[&key(self.scope, "program_kind", None)],
                index.encode()
            );
            assert_eq!(
                source.state[&key(self.scope, "program_resolved", None)],
                resolved.encode()
            );
        }
    }
}
fn generator_fields<S: FieldSink>(
    value: &mut Generator,
    fields: &mut Fields<'_, S>,
    banks: &impl Banks,
) {
    let bank = banks.bank(value.bank);
    fields.scalar("kind", &mut value.descriptor.kind);
    fields.scalar("random", &mut value.emission_rate);
    fields.scalar("count", &mut value.count);
    fields.scalar("generator_life", &mut value.remaining_life);
    fields.scalar("type", &mut value.flags);
    fields.scalar("bank", &mut value.bank);
    fields.scalar("link", &mut value.link);
    fields.scalar("texture_group", &mut value.descriptor.texture_group);
    fields.scalar("id", &mut value.family_id);
    fields.scalar("particle_life", &mut value.descriptor.particle_life);
    fields.array("position", &mut value.position);
    fields.array("velocity", &mut value.descriptor.velocity);
    fields.scalar("gravity", &mut value.descriptor.gravity);
    fields.scalar("friction", &mut value.descriptor.friction);
    fields.scalar("size", &mut value.descriptor.size);
    fields.scalar("radius", &mut value.descriptor.radius);
    fields.scalar("angle", &mut value.descriptor.angle);
    fields.scalar("child_count", &mut value.children);
    fields.scalar("appsrt_index", &mut value.appsrt_id);
    fields.program(value.bank, bank, &value.descriptor.program);
    match &mut value.shape {
        EmissionShape::Sphere {
            speed,
            latitude_midpoint,
            latitude_range,
            longitude_midpoint,
            longitude_range,
        } => {
            fields.scalar("aux.speed", speed);
            fields.scalar("aux.latitude_mid", latitude_midpoint);
            fields.scalar("aux.latitude_range", latitude_range);
            fields.scalar("aux.longitude_mid", longitude_midpoint);
            fields.scalar("aux.longitude_range", longitude_range);
        }
        EmissionShape::Disc {
            minimum_angle,
            maximum_angle,
            ..
        } => {
            fields.scalar("aux.minimum_angle", minimum_angle);
            fields.scalar("aux.maximum_angle", maximum_angle);
        }
        EmissionShape::Tornado { speed } => fields.scalar("aux.tornado_velocity", speed),
        EmissionShape::Rectangle { dimensions } => {
            // hsd_8039F05C: rectangle dimensions and its axis cache are named
            // separately in the oracle. The current emitter retains the
            // authored diagonal axes; rotated imported axes need a real owner.
            for (name, value) in ["aux.x", "aux.y", "aux.z"]
                .into_iter()
                .zip(dimensions.iter_mut())
            {
                fields.scalar(name, value);
            }
            for row in 0..3 {
                for column in 0..3 {
                    let expected = if row == column { dimensions[row] } else { 0.0 };
                    let mut value = expected;
                    let name = format!("aux.{}{}", ["x", "y", "z"][row], ["x", "y", "z"][column]);
                    fields.scalar(&name, &mut value);
                    assert_eq!(
                        value.to_bits(),
                        expected.to_bits(),
                        "rotated rectangle import"
                    );
                }
            }
            let expected = dimensions
                .iter()
                .enumerate()
                .fold(0_u16, |flags, (i, value)| {
                    flags | if *value < 0.0 { 1 << i } else { 0 }
                });
            let mut flags = expected;
            fields.scalar("aux.flags", &mut flags);
            assert_eq!(flags, expected, "rectangle surface flags");
        }
        EmissionShape::Line { end } => {
            for (name, value) in ["aux.line_x", "aux.line_y", "aux.line_z"]
                .into_iter()
                .zip(end.iter_mut())
            {
                fields.scalar(name, value);
            }
        }
        EmissionShape::Cone {
            mode,
            minimum_angle,
            maximum_angle,
            height,
        } => {
            // The dump holds no mode: it is the shape number in `type`.
            let _ = mode;
            fields.scalar("aux.minimum_angle", minimum_angle);
            fields.scalar("aux.maximum_angle", maximum_angle);
            fields.scalar("aux.height", height);
        }
    }
}
fn particle_fields<S: FieldSink>(
    value: &mut Particle,
    fields: &mut Fields<'_, S>,
    banks: &impl Banks,
) {
    let bank = banks.bank(value.bank);
    fields.scalar("kind", &mut value.kind);
    fields.scalar("bank", &mut value.bank);
    fields.scalar("texture_group", &mut value.texture_group);
    fields.scalar("pose", &mut value.pose);
    fields.scalar("palette", &mut value.palette);
    fields.scalar("size_count", &mut value.size_timer);
    fields.scalar("primary_count", &mut value.primary.duration);
    fields.scalar("environment_count", &mut value.environment.duration);
    fields.array("primary_color", &mut value.primary.current);
    fields.array("environment_color", &mut value.environment.current);
    fields.scalar("command_wait", &mut value.wait);
    fields.scalar("loop_count", &mut value.loop_count);
    fields.scalar("link", &mut value.link);
    fields.scalar("id", &mut value.family_id);
    fields.scalar("pc", &mut value.pc);
    fields.scalar("mark_pc", &mut value.mark);
    fields.scalar("loop_pc", &mut value.loop_start);
    fields.scalar("life", &mut value.life);
    fields.array("velocity", &mut value.velocity);
    fields.scalar("gravity", &mut value.gravity);
    fields.scalar("friction", &mut value.friction);
    fields.array("position", &mut value.position);
    fields.scalar("size", &mut value.size);
    fields.scalar("rotation", &mut value.rotation);
    fields.scalar("alpha_compare_count", &mut value.alpha_compare.duration);
    fields.scalar("alpha_compare_mode", &mut value.alpha_compare_mode);
    fields.array("alpha_compare_parameters", &mut value.alpha_compare.current);
    fields.scalar("point_joint_offset", &mut value.point_joint_offset);
    fields.scalar("material_count", &mut value.material.duration);
    fields.scalar("ambient_count", &mut value.ambient.duration);
    fields.scalar("rotation_count", &mut value.rotation_timer);
    fields.scalar("size_target", &mut value.size_target);
    fields.scalar("rotation_target", &mut value.rotation_target);
    fields.scalar("primary_remaining", &mut value.primary.remaining);
    fields.scalar("environment_remaining", &mut value.environment.remaining);
    fields.array("primary_target", &mut value.primary.target);
    fields.array("environment_target", &mut value.environment.target);
    fields.scalar("material_remaining", &mut value.material.remaining);
    fields.scalar("ambient_remaining", &mut value.ambient.remaining);
    fields.scalar(
        "alpha_compare_remaining",
        &mut value.alpha_compare.remaining,
    );
    fields.array("alpha_compare_targets", &mut value.alpha_compare.target);
    fields.array("material", &mut value.material.current);
    fields.array("ambient", &mut value.ambient.current);
    fields.array("material_target", &mut value.material.target);
    fields.array("ambient_target", &mut value.ambient.target);
    fields.scalar("trail", &mut value.trail);
    fields.scalar("generator_index", &mut value.generator_id);
    fields.scalar("appsrt_index", &mut value.appsrt_id);
    fields.program(value.bank, bank, &value.program);
}

pub fn restore(record: &Record, banks: &impl Banks) -> ParticleSystem {
    let mut output = BTreeMap::new();
    let mut generators = Vec::new();
    for index in 0..uint(record, "particles.generator_count") {
        let prefix = format!("particles.generator[{index}]");
        let scope = Scope::Generator(index as usize);
        let id = uint(record, &format!("{prefix}.program_kind")) as u32;
        let bank_id = uint(record, &format!("{prefix}.bank")) as u8;
        let bank = banks.bank(bank_id);
        let descriptor = bank.descriptor(id).expect("captured program in bank");
        // Constructor-only derived state; its private scratch RNG never enters
        // replay. Every canonical runtime field, including count, is overwritten.
        let mut generator = Generator::new::<RetailTrig>(
            descriptor,
            bank_id,
            0,
            &mut HsdRng::new(0),
            &mut DrawLog::default(),
        )
        .unwrap();
        generator.id = index as usize;
        generator_fields(
            &mut generator,
            &mut Fields {
                scope,
                source: Some(record),
                output: &mut output,
            },
            bank,
        );
        generator.texture_images = bank.textures[generator.descriptor.texture_group as usize]
            .as_ref()
            .unwrap()
            .images
            .clone();
        generators.push(generator);
    }
    let transforms = restore_appsrts(record);
    // Runtime AppSRT identity is the id of the generator that allocated it
    // (ParticleSystem::insert_generator); the dump's is first-encounter order.
    let mut owners = vec![None; transforms.len()];
    for generator in &mut generators {
        if let Some(index) = generator.appsrt_id {
            let owner = *owners[index].get_or_insert(generator.id);
            generator.appsrt_id = Some(owner);
            generator.application_transform = Some(std::sync::Arc::clone(&transforms[index]));
        }
    }
    let particles = std::array::from_fn(|link| {
        let prefix = format!("particles.link[{link}]");
        (0..uint(record, &format!("{prefix}.count")))
            .map(|index| {
                let scope = Scope::Particle {
                    link,
                    index: index as usize,
                };
                let prefix = format!("{prefix}.particle[{index}]");
                let id = uint(record, &format!("{prefix}.program_kind")) as u32;
                let bank_id = uint(record, &format!("{prefix}.bank")) as u8;
                let bank = banks.bank(bank_id);
                let mut particle =
                    Particle::new(bank.descriptor(id).unwrap(), bank_id, link as u8).unwrap();
                particle_fields(
                    &mut particle,
                    &mut Fields {
                        scope,
                        source: Some(record),
                        output: &mut output,
                    },
                    bank,
                );
                particle.texture_images = bank.textures[particle.texture_group as usize]
                    .as_ref()
                    .unwrap()
                    .images
                    .clone();
                if let Some(index) = particle.appsrt_id {
                    particle.appsrt_id = Some(owners[index].expect(
                        "AppSRT held only by particles: its generator identity is not imported",
                    ));
                    particle.application_transform =
                        Some(std::sync::Arc::clone(&transforms[index]));
                }
                particle
            })
            .collect()
    });
    assert_eq!(
        uint(record, "particles.pending_count"),
        0,
        "pending attachment execution unsupported"
    );
    let system = ParticleSystem::from_live_lists(
        generators,
        particles,
        uint(record, "particles.family_id_counter")
            .try_into()
            .unwrap(),
    );
    // This also rejects an unmapped field instead of silently dropping it.
    assert_state(
        record,
        &snapshot(
            &system,
            uint(record, "rng.seed") as u32,
            record.frame,
            banks,
        ),
    );
    system
}

/// `particles.appsrt[i]`, as appsrt_fields writes them.
fn restore_appsrts(
    record: &Record,
) -> Vec<std::sync::Arc<hsd_particle::appsrt::ApplicationTransform>> {
    use hsd_types::{Mtx, Vec3};
    (0..)
        .map_while(|index| {
            let prefix = format!("particles.appsrt[{index}]");
            record.state.get(&format!("{prefix}.id"))?;
            let float = |name: String| f32::decode(&record.state[&format!("{prefix}.{name}")]);
            let array = |name: &str, i: usize| float(format!("{name}[{i}]"));
            let vec3 = |name: &str| Vec3::new(array(name, 0), array(name, 1), array(name, 2));
            let mut model_view_matrix = Mtx([[0.0; 4]; 3]);
            model_view_matrix.0[0][0] = float("scale_x".into());
            model_view_matrix.0[0][1] = float("scale_y".into());
            let mut axis_scale = [0.0; 2];
            for i in 0..12 {
                let value = array("unknown_float", i);
                if i < 10 {
                    model_view_matrix.0[(i + 2) / 4][(i + 2) % 4] = value;
                } else {
                    axis_scale[i - 10] = value;
                }
            }
            Some(std::sync::Arc::new(
                hsd_particle::appsrt::ApplicationTransform {
                    translation: vec3("translation"),
                    rotation: vec3("rotation"),
                    rotation_w: array("rotation", 3),
                    scale: vec3("scale"),
                    status: i32::from(uint(record, &format!("{prefix}.status")) as u8),
                    frame_number: uint(record, &format!("{prefix}.frame_count")) as u8,
                    model_matrix: Mtx(std::array::from_fn(|r| {
                        std::array::from_fn(|c| array("matrix", r * 4 + c))
                    })),
                    model_view_matrix,
                    axis_scale,
                    family_id: uint(record, &format!("{prefix}.id")) as u16,
                    camera_facing: uint(record, &format!("{prefix}.unknown_byte")) as u8,
                },
            ))
        })
        .collect()
}

fn appsrt_fields<S: FieldSink>(
    value: &hsd_particle::appsrt::ApplicationTransform,
    mut count: usize,
    mut generator_index: Option<usize>,
    fields: &mut Fields<'_, S>,
) {
    let mut value = value.clone();
    let mut translation = [
        value.translation.x,
        value.translation.y,
        value.translation.z,
    ];
    fields.array("translation", &mut translation);
    fields.array(
        "rotation",
        &mut [
            value.rotation.x,
            value.rotation.y,
            value.rotation.z,
            value.rotation_w,
        ],
    );
    fields.array("scale", &mut [value.scale.x, value.scale.y, value.scale.z]);
    fields.scalar("status", &mut (value.status as u8));
    fields.scalar("frame_count", &mut (value.frame_number));
    fields.scalar("use_count", &mut (count));
    fields.array(
        "matrix",
        &mut std::array::from_fn::<_, 12, _>(|i| value.model_matrix.0[i / 4][i % 4]),
    );
    fields.scalar("scale_x", &mut (value.model_view_matrix.0[0][0]));
    fields.scalar("scale_y", &mut (value.model_view_matrix.0[0][1]));
    fields.array(
        "unknown_float",
        &mut std::array::from_fn::<_, 12, _>(|i| {
            if i < 10 {
                value.model_view_matrix.0[(i + 2) / 4][(i + 2) % 4]
            } else {
                value.axis_scale[i - 10]
            }
        }),
    );
    fields.scalar("id", &mut (value.family_id));
    fields.scalar("unknown_byte", &mut (value.camera_facing));
    fields.scalar("generator_index", &mut generator_index);
}

pub fn snapshot(system: &ParticleSystem, seed: u32, frame: u64, bank: &impl Banks) -> Record {
    let mut output = BTreeMap::new();
    write_fields(system, seed, bank, &mut output);
    Record {
        frame,
        phase: "particles".into(),
        state: output,
    }
}

/// Every field of the retail particle dump, from the port's particle system.
pub fn write_fields(
    system: &ParticleSystem,
    seed: u32,
    bank: &impl Banks,
    output: &mut impl FieldSink,
) {
    // Dump identities are first-encounter order, not allocator/generator IDs.
    let mut transforms = Vec::new();
    for (id, transform) in system
        .generators
        .iter()
        .map(|g| (g.appsrt_id, &g.application_transform))
        .chain(
            system
                .particles
                .iter()
                .flatten()
                .map(|p| (p.appsrt_id, &p.application_transform)),
        )
    {
        if let Some(id) = id {
            let transform = transform.as_ref().expect("owned AppSRT");
            if let Some((_, _, count)) = transforms
                .iter_mut()
                .find(|(existing, _, _)| *existing == id)
            {
                *count += 1usize;
            } else {
                transforms.push((id, transform.as_ref(), 1usize));
            }
        }
    }
    let normalized = |id: Option<usize>| {
        id.map(|id| {
            transforms
                .iter()
                .position(|(owned, _, _)| *owned == id)
                .unwrap()
        })
    };
    let appsrts = if output.wants_appsrts() {
        transforms.as_slice()
    } else {
        &[]
    };
    for (index, (id, transform, count)) in appsrts.iter().enumerate() {
        // hsd_8039F05C / eflib_create_generator_add_appsrt set the owner;
        // psRemoveGeneratorSRT clears it when the allocating generator dies.
        let owner = system.generators.iter().position(|generator| {
            generator.id == *id
                && (generator.descriptor.kind & 0x20000 != 0
                    || (generator.attachment_id.is_some() && generator.flags & 0x900 == 0x900))
        });
        appsrt_fields(
            transform,
            *count,
            owner,
            &mut Fields {
                scope: Scope::AppSrt(index),
                source: None,
                output,
            },
        );
    }
    let generator_index = |id: usize| system.generators.iter().position(|g| g.id == id);
    for (name, value) in [
        ("rng.seed", seed.encode()),
        (
            "particles.generator_count",
            system.generators.len().encode(),
        ),
        (
            "particles.family_id_counter",
            system.family_counter.encode(),
        ),
        (
            "particles.pending_count",
            system.pending_generators.len().encode(),
        ),
    ] {
        output.put(Scope::Global, name, None, value);
    }
    for (index, pending) in system.pending_generators.iter().enumerate() {
        // hsd_804D78F4: an entry whose generator died holds a null.
        output.put(
            Scope::Pending(index),
            "generator_index",
            None,
            pending.and_then(generator_index).encode(),
        );
    }
    for (index, generator) in system.generators.iter().enumerate() {
        let mut generator = generator.clone();
        generator.appsrt_id = normalized(generator.appsrt_id);
        generator_fields(
            &mut generator,
            &mut Fields {
                scope: Scope::Generator(index),
                source: None,
                output,
            },
            bank,
        );
    }
    for (link, particles) in system.particles.iter().enumerate() {
        output.put(Scope::Link(link), "count", None, particles.len().encode());
        for (index, particle) in particles.iter().enumerate() {
            let mut particle = particle.clone();
            particle.appsrt_id = normalized(particle.appsrt_id);
            particle.generator_id = particle
                .generator_id
                .map(|id| generator_index(id).expect("live generator association"));
            particle_fields(
                &mut particle,
                &mut Fields {
                    scope: Scope::Particle { link, index },
                    source: None,
                    output,
                },
                bank,
            );
        }
    }
}

pub fn assert_state(expected: &Record, actual: &Record) {
    assert_eq!(
        (expected.frame, &expected.phase),
        (actual.frame, &actual.phase)
    );
    for (field, expected_value) in &expected.state {
        assert_eq!(
            Some(expected_value),
            actual.state.get(field),
            "tick {} entity/field {field}: expected {expected_value:?}, actual {:?}",
            expected.frame,
            actual.state.get(field)
        );
    }
    assert_eq!(
        expected.state.len(),
        actual.state.len(),
        "unexpected output fields at tick {}",
        expected.frame
    );
}
