//! Canonical dump adapter. JSON and melee-diff remain test dependencies.
use crate::common::RetailTrig;
use gekko_math::rng::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    generator::{EmissionShape, Generator},
    particle::Particle,
    rng_sites::DrawLog,
    system::ParticleSystem,
};
use melee_diff::{Record, Value};
use std::{collections::BTreeMap, fs::File, io::BufReader, path::Path};

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

pub fn read(path: &Path) -> Vec<Record> {
    melee_diff::read_trace(BufReader::new(File::open(path).unwrap())).unwrap()
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
/// The same field map reads and writes. Round-trip equality detects unknown
/// capture fields; replay compares independently updated state with the oracle.
struct Fields<'a> {
    prefix: String,
    source: Option<&'a Record>,
    output: &'a mut BTreeMap<String, Value>,
}
impl Fields<'_> {
    fn scalar<T: Scalar>(&mut self, name: &str, value: &mut T) {
        let key = format!("{}.{name}", self.prefix);
        if let Some(source) = self.source {
            *value = T::decode(
                source
                    .state
                    .get(&key)
                    .unwrap_or_else(|| panic!("missing {key}")),
            );
        }
        self.output.insert(key, value.encode());
    }
    fn array<T: Scalar, const N: usize>(&mut self, name: &str, values: &mut [T; N]) {
        for (index, value) in values.iter_mut().enumerate() {
            self.scalar(&format!("{name}[{index}]"), value);
        }
    }
    fn program(&mut self, bank: &ParticleBank, bytes: &[u8]) {
        let id = bank.descriptors.iter().enumerate().find_map(|(index, d)| {
            d.as_ref()
                .filter(|d| d.program.as_ref() == bytes)
                .map(|_| bank.first_descriptor_id + index as u32)
        });
        let resolved = u8::from(id.is_some());
        let index = id.map(|n| n as usize);
        // These are derived from the actual program, never copied from expected.
        self.output
            .insert(format!("{}.program_kind", self.prefix), index.encode());
        self.output.insert(
            format!("{}.program_resolved", self.prefix),
            resolved.encode(),
        );
        if let Some(source) = self.source {
            assert_eq!(
                source.state[&format!("{}.program_kind", self.prefix)],
                index.encode()
            );
            assert_eq!(
                source.state[&format!("{}.program_resolved", self.prefix)],
                resolved.encode()
            );
        }
    }
}
fn generator_fields(value: &mut Generator, fields: &mut Fields<'_>, banks: &impl Banks) {
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
    fields.program(bank, &value.descriptor.program);
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
            for (name, value) in ["x", "y", "z"].into_iter().zip(dimensions.iter_mut()) {
                fields.scalar(&format!("aux.{name}"), value);
            }
            for (row, names) in [["xx", "xy", "xz"], ["yx", "yy", "yz"], ["zx", "zy", "zz"]]
                .into_iter()
                .enumerate()
            {
                for (col, name) in names.into_iter().enumerate() {
                    let mut value = if row == col { dimensions[row] } else { 0.0 };
                    fields.scalar(&format!("aux.{name}"), &mut value);
                }
            }
            fields.scalar("aux.flags", &mut 0u16);
        }
        other => panic!("live FD adapter requires sphere/disc auxiliary state, got {other:?}"),
    }
}
fn particle_fields(value: &mut Particle, fields: &mut Fields<'_>, banks: &impl Banks) {
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
    fields.program(bank, &value.program);
}

pub fn restore(record: &Record, banks: &impl Banks) -> ParticleSystem {
    let mut output = BTreeMap::new();
    let mut generators = Vec::new();
    for index in 0..uint(record, "particles.generator_count") {
        let prefix = format!("particles.generator[{index}]");
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
                prefix,
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
        assert!(
            generator.appsrt_id.is_none(),
            "AppSRT execution unsupported"
        );
        generators.push(generator);
    }
    let particles = std::array::from_fn(|link| {
        let prefix = format!("particles.link[{link}]");
        (0..uint(record, &format!("{prefix}.count")))
            .map(|index| {
                let prefix = format!("{prefix}.particle[{index}]");
                let id = uint(record, &format!("{prefix}.program_kind")) as u32;
                let bank_id = uint(record, &format!("{prefix}.bank")) as u8;
                let bank = banks.bank(bank_id);
                let mut particle =
                    Particle::new(bank.descriptor(id).unwrap(), bank_id, link as u8).unwrap();
                particle_fields(
                    &mut particle,
                    &mut Fields {
                        prefix,
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
                assert!(particle.appsrt_id.is_none(), "AppSRT execution unsupported");
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

fn appsrt_fields(
    value: &hsd_particle::appsrt::ApplicationTransform,
    mut count: usize,
    mut generator_index: Option<usize>,
    fields: &mut Fields<'_>,
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
    for (index, (id, transform, count)) in transforms.iter().enumerate() {
        // hsd_8039F05C sets gp for kind 0x20000 (8039F698);
        // eflib_create_generator_add_appsrt also sets gp for attached effects.
        // Standalone effect overrides leave gp null. The owned
        // AppSRT identity is its allocating generator ID, retained by children.
        // psRemoveGeneratorSRT clears gp when that owner leaves the live list
        // (803A4428), so normalize only the still-live allocating generator.
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
                prefix: format!("particles.appsrt[{index}]"),
                source: None,
                output: &mut output,
            },
        );
    }
    output.insert("rng.seed".into(), seed.encode());
    output.insert(
        "particles.generator_count".into(),
        system.generators.len().encode(),
    );
    output.insert(
        "particles.family_id_counter".into(),
        system.family_counter.encode(),
    );
    output.insert(
        "particles.pending_count".into(),
        system.pending_generators.len().encode(),
    );
    for (index, generator) in system.generators.iter().enumerate() {
        let mut generator = generator.clone();
        generator.appsrt_id = normalized(generator.appsrt_id);
        generator_fields(
            &mut generator,
            &mut Fields {
                prefix: format!("particles.generator[{index}]"),
                source: None,
                output: &mut output,
            },
            bank,
        );
    }
    for (link, particles) in system.particles.iter().enumerate() {
        let prefix = format!("particles.link[{link}]");
        output.insert(format!("{prefix}.count"), particles.len().encode());
        for (index, particle) in particles.iter().enumerate() {
            let mut particle = particle.clone();
            particle.appsrt_id = normalized(particle.appsrt_id);
            particle.generator_id = particle.generator_id.map(|id| {
                system
                    .generators
                    .iter()
                    .position(|g| g.id == id)
                    .expect("live generator association")
            });
            particle_fields(
                &mut particle,
                &mut Fields {
                    prefix: format!("{prefix}.particle[{index}]"),
                    source: None,
                    output: &mut output,
                },
                bank,
            );
        }
    }
    Record {
        frame,
        phase: "particles".into(),
        state: output,
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
