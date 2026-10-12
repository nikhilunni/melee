//! The two digests of a particle record, from a dump's keys or from the
//! port's snapshot fields.
//!
//! Both are sums of per-field hashes, so either side adds its fields in
//! whatever order it holds them and the port never spells a key.
use crate::frame::Simulation;
use anyhow::{Context, Result};
use melee_diff::{Record, Value};
use melee_lib::diagnostics::{ParticleFieldSink, ParticleScope};
use std::{collections::HashMap, sync::Arc};

/// Keys the digests leave out: the AppSRT display caches, which need the
/// camera and the render schedule.
pub(super) const APPSRT_PREFIX: &str = "particles.appsrt[";

/// A particle record reduced to two numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Digests {
    /// Every compared field under its own key: generators in list order,
    /// each particle at its place in its list.
    pub ordered: u64,
    /// The same, except that a particle's place in its list does not count:
    /// each of the sixteen lists is a bag of particles.
    pub unordered: u64,
}

/// splitmix64's finalizer.
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// FNV-1a.
fn bytes_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

fn scope_hash(scope: ParticleScope) -> u64 {
    let (kind, a, b) = match scope {
        ParticleScope::Global => (0, 0, 0),
        ParticleScope::Generator(index) => (1, index, 0),
        ParticleScope::Link(link) => (2, link, 0),
        ParticleScope::Particle { link, index } => (3, link, index),
        ParticleScope::AppSrt(index) => (4, index, 0),
        ParticleScope::Pending(index) => (5, index, 0),
    };
    mix(kind ^ (a as u64) << 8 ^ (b as u64) << 36)
}

/// A field's name, array component and value. Floats enter by bit pattern,
/// as the trace compares them.
fn value_hash(name: &str, component: Option<usize>, value: &Value) -> u64 {
    let (tag, payload) = match value {
        Value::Int(v) => (0, *v as u64),
        Value::UInt(v) => (1, *v),
        Value::F32 { bits, .. } => (2, u64::from(*bits)),
        Value::F64 { bits, .. } => (3, *bits),
        Value::Str(text) => (4, bytes_hash(text.as_bytes())),
        Value::Null => (5, 0),
    };
    let hash = mix(bytes_hash(name.as_bytes()) ^ component.map_or(0, |c| c as u64 + 1) ^ tag << 32);
    mix(hash ^ payload)
}

/// One field's term of a digest: which object, and [`value_hash`].
fn field_term(scope: ParticleScope, field: u64) -> u64 {
    mix(scope_hash(scope).wrapping_add(field))
}

/// A dump key split into the parts the port's snapshot names a field by.
pub(super) fn parse_key(key: &str) -> Option<(ParticleScope, &str, Option<usize>)> {
    fn indexed<'a>(text: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
        let rest = text.strip_prefix(prefix)?;
        let (index, rest) = rest.split_once("].")?;
        Some((index.parse().ok()?, rest))
    }
    let (scope, field) = if let Some((index, field)) = indexed(key, "particles.generator[") {
        (ParticleScope::Generator(index), field)
    } else if let Some((link, field)) = indexed(key, "particles.link[") {
        match indexed(field, "particle[") {
            Some((index, field)) => (ParticleScope::Particle { link, index }, field),
            None => (ParticleScope::Link(link), field),
        }
    } else if let Some((index, field)) = indexed(key, APPSRT_PREFIX) {
        (ParticleScope::AppSrt(index), field)
    } else if let Some((index, field)) = indexed(key, "particles.pending[") {
        (ParticleScope::Pending(index), field)
    } else {
        return Some((ParticleScope::Global, key, None));
    };
    match field.strip_suffix(']').and_then(|f| f.split_once('[')) {
        Some((name, component)) => Some((scope, name, Some(component.parse().ok()?))),
        None => Some((scope, field, None)),
    }
}

/// Both digests of one record, a field at a time.
#[derive(Default)]
struct Accumulator {
    ordered: u64,
    /// The fields that are not a particle's.
    others: u64,
    /// Each particle's fields under its list alone.
    particles: HashMap<(usize, usize), u64>,
    /// The particle whose fields are arriving (a snapshot and a dump line
    /// both hold a particle's fields together), not yet in `particles`.
    current: Option<((usize, usize), u64)>,
}

impl Accumulator {
    fn add(&mut self, scope: ParticleScope, name: &str, component: Option<usize>, value: &Value) {
        let field = value_hash(name, component, value);
        let term = field_term(scope, field);
        match scope {
            ParticleScope::AppSrt(_) => return,
            ParticleScope::Particle { link, index } => {
                let in_list = field_term(ParticleScope::Particle { link, index: 0 }, field);
                match &mut self.current {
                    Some((particle, sum)) if *particle == (link, index) => {
                        *sum = sum.wrapping_add(in_list);
                    }
                    _ => {
                        self.keep_current();
                        self.current = Some(((link, index), in_list));
                    }
                }
            }
            _ => self.others = self.others.wrapping_add(term),
        }
        self.ordered = self.ordered.wrapping_add(term);
    }

    fn add_key(&mut self, key: &str, value: &Value) -> Result<()> {
        let (scope, name, component) =
            parse_key(key).with_context(|| format!("unreadable particle key {key}"))?;
        self.add(scope, name, component, value);
        Ok(())
    }

    fn keep_current(&mut self) {
        if let Some((particle, sum)) = self.current.take() {
            let kept = self.particles.entry(particle).or_default();
            *kept = kept.wrapping_add(sum);
        }
    }

    /// The record's digests; the accumulator is ready for the next record.
    fn finish(&mut self) -> Digests {
        self.keep_current();
        let bags = self
            .particles
            .drain()
            .fold(0_u64, |sum, (_, particle)| sum.wrapping_add(mix(particle)));
        let digests = Digests {
            ordered: self.ordered,
            unordered: self.others.wrapping_add(bags),
        };
        (self.ordered, self.others) = (0, 0);
        digests
    }
}

/// The digests of a whole record (a dump line, or the port's snapshot).
pub fn record_digests(record: &Record) -> Result<Digests> {
    let mut sum = Accumulator::default();
    for (key, value) in &record.state {
        sum.add_key(key, value)?;
    }
    Ok(sum.finish())
}

/// The port side: the snapshot's fields summed as they are visited.
#[derive(Default)]
pub struct PortDigest {
    sum: Accumulator,
    /// The bank descriptor of each program allocation seen, by bank and
    /// address; checked against the bank on every use.
    programs: HashMap<(u8, usize), u32>,
}

impl ParticleFieldSink for PortDigest {
    fn put(&mut self, scope: ParticleScope, name: &str, component: Option<usize>, value: Value) {
        self.sum.add(scope, name, component, &value);
    }
    fn program_id(
        &mut self,
        bank_id: u8,
        bank: &hsd_particle::bank::ParticleBank,
        program: &Arc<[u8]>,
    ) -> Option<u32> {
        let key = (bank_id, Arc::as_ptr(program).cast::<u8>() as usize);
        if let Some(&id) = self.programs.get(&key) {
            let same = bank
                .descriptor(id)
                .is_some_and(|d| Arc::ptr_eq(&d.program, program));
            if same {
                return Some(id);
            }
        }
        let id = melee_lib::diagnostics::particle_program_id(bank, program)?;
        self.programs.insert(key, id);
        Some(id)
    }
    fn wants_appsrts(&self) -> bool {
        false
    }
}

impl PortDigest {
    /// The digests of the simulation's particle system as it stands.
    pub fn of(&mut self, simulation: &Simulation) -> Digests {
        simulation.write_particle_fields(self);
        self.sum.finish()
    }
}

/// A dump line's `state`, digested while it is parsed: no map is built.
pub(super) struct StateDigest {
    pub digests: Digests,
    pub seed: Option<u32>,
}

impl<'de> serde::Deserialize<'de> for StateDigest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Fields;
        impl<'de> serde::de::Visitor<'de> for Fields {
            type Value = StateDigest;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a particle record's state map")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sum = Accumulator::default();
                let mut seed = None;
                while let Some(key) = map.next_key::<std::borrow::Cow<'de, str>>()? {
                    let value: Value = map.next_value()?;
                    if key == "rng.seed" {
                        if let Value::UInt(value) = value {
                            seed = u32::try_from(value).ok();
                        }
                    }
                    sum.add_key(&key, &value)
                        .map_err(serde::de::Error::custom)?;
                }
                Ok(StateDigest {
                    digests: sum.finish(),
                    seed,
                })
            }
        }
        deserializer.deserialize_map(Fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_lib::diagnostics::particle_key;

    fn record(fields: &[(ParticleScope, &str, Option<usize>, Value)]) -> Record {
        Record {
            frame: 0,
            phase: "particles".into(),
            state: fields
                .iter()
                .map(|(scope, name, component, value)| {
                    (particle_key(*scope, name, *component), value.clone())
                })
                .collect(),
        }
    }

    fn particle(link: usize, index: usize) -> ParticleScope {
        ParticleScope::Particle { link, index }
    }

    #[test]
    fn dump_keys_parse_back_to_the_parts_the_port_names_them_by() {
        let scopes = [
            ParticleScope::Generator(0),
            ParticleScope::Generator(12),
            ParticleScope::Link(3),
            particle(3, 0),
            particle(15, 240),
            ParticleScope::Pending(1),
            ParticleScope::AppSrt(2),
        ];
        for scope in scopes {
            for (name, component) in [
                ("kind", None),
                ("position", Some(2)),
                ("aux.minimum_angle", None),
                ("primary_color", Some(0)),
            ] {
                let key = particle_key(scope, name, component);
                assert_eq!(parse_key(&key), Some((scope, name, component)), "{key}");
            }
        }
        for key in ["rng.seed", "particles.generator_count"] {
            assert_eq!(parse_key(key), Some((ParticleScope::Global, key, None)));
        }
    }

    #[test]
    fn the_port_sink_and_a_record_of_the_same_fields_digest_alike() {
        let fields = [
            (ParticleScope::Global, "rng.seed", None, Value::UInt(7)),
            (ParticleScope::Generator(1), "position", Some(0), Value::f32(1.5)),
            (ParticleScope::Generator(1), "position", Some(1), Value::f32(1.5)),
            (particle(2, 9), "generator_index", None, Value::Null),
            (particle(2, 9), "life", None, Value::UInt(4)),
            (ParticleScope::Link(2), "count", None, Value::UInt(10)),
        ];
        let mut port = PortDigest::default();
        for (scope, name, component, value) in fields.iter().rev() {
            port.put(*scope, name, *component, value.clone());
        }
        assert_eq!(record_digests(&record(&fields)).unwrap(), port.sum.finish());
    }

    #[test]
    fn one_ulp_one_index_and_one_type_each_change_both_digests() {
        let base = [(ParticleScope::Generator(1), "position", Some(0), Value::f32(1.5))];
        let base = record_digests(&record(&base)).unwrap();
        let ulp = Value::f32(f32::from_bits(1.5_f32.to_bits() + 1));
        let bits = Value::UInt(u64::from(1.5_f32.to_bits()));
        for other in [
            (ParticleScope::Generator(1), "position", Some(0), ulp),
            (ParticleScope::Generator(2), "position", Some(0), Value::f32(1.5)),
            (ParticleScope::Generator(1), "position", Some(1), Value::f32(1.5)),
            (ParticleScope::Generator(1), "velocity", Some(0), Value::f32(1.5)),
            (ParticleScope::Generator(1), "position", Some(0), bits),
            (particle(1, 0), "position", Some(0), Value::f32(1.5)),
        ] {
            let other = record_digests(&record(&[other])).unwrap();
            assert_ne!(base.ordered, other.ordered);
            assert_ne!(base.unordered, other.unordered);
        }
    }

    #[test]
    fn two_particles_changing_places_change_only_the_ordered_digest() {
        let list = |first: usize, second: usize| {
            record_digests(&record(&[
                (particle(0, first), "life", None, Value::UInt(4)),
                (particle(0, first), "size", None, Value::f32(1.0)),
                (particle(0, second), "life", None, Value::UInt(19)),
                (particle(0, second), "size", None, Value::f32(2.0)),
            ]))
            .unwrap()
        };
        let (kept, swapped) = (list(0, 1), list(1, 0));
        assert_ne!(kept.ordered, swapped.ordered);
        assert_eq!(kept.unordered, swapped.unordered);
        // Two particles trading one field are two other particles; so is a
        // particle in another list.
        let traded = record_digests(&record(&[
            (particle(0, 0), "life", None, Value::UInt(19)),
            (particle(0, 0), "size", None, Value::f32(1.0)),
            (particle(0, 1), "life", None, Value::UInt(4)),
            (particle(0, 1), "size", None, Value::f32(2.0)),
        ]))
        .unwrap();
        assert_ne!(kept.unordered, traded.unordered);
        let moved = record_digests(&record(&[
            (particle(1, 0), "life", None, Value::UInt(4)),
            (particle(1, 0), "size", None, Value::f32(1.0)),
            (particle(0, 1), "life", None, Value::UInt(19)),
            (particle(0, 1), "size", None, Value::f32(2.0)),
        ]))
        .unwrap();
        assert_ne!(kept.unordered, moved.unordered);
    }

    #[test]
    fn the_appsrt_display_caches_are_outside_the_digests() {
        let seed = (ParticleScope::Global, "rng.seed", None, Value::UInt(1));
        let cache = (ParticleScope::AppSrt(0), "matrix", Some(3), Value::f32(2.0));
        assert_eq!(
            record_digests(&record(std::slice::from_ref(&seed))).unwrap(),
            record_digests(&record(&[seed, cache])).unwrap()
        );
    }

    #[test]
    fn a_dump_line_digests_without_building_its_map() {
        let line = r#"{"rng.seed": {"t": "u", "v": 9},
            "particles.generator[0].random": {"t": "f32", "v": {"bits": 1092616192, "approx": 10.0}},
            "particles.link[0].particle[1].generator_index": {"t": "null"},
            "particles.appsrt[0].id": {"t": "u", "v": 3}}"#;
        let streamed: StateDigest = serde_json::from_str(line).unwrap();
        let state = serde_json::from_str(line).unwrap();
        let record = Record {
            frame: 4,
            phase: "particles".into(),
            state,
        };
        assert_eq!(streamed.seed, Some(9));
        assert_eq!(streamed.digests, record_digests(&record).unwrap());
    }
}
