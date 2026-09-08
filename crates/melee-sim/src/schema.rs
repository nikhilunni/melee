//! Schema loading and `Snapshot` coverage checking.
//!
//! `harness/gen_schema.py` extracts every offset-annotated field of the
//! decomp's `struct Fighter` and `struct Item` into
//! `harness/schema/*.generated.{yaml,json}`. The Dolphin decoder reads the
//! YAML; this module reads the JSON twin (no YAML parser in Rust) and checks
//! that a Rust `Snapshot` implementation emits every path the decoder can
//! produce, so the two sides of the oracle comparison never silently drift.
//!
//! # How a real `Fighter` will be checked
//!
//! Once `melee_ft::Fighter` exists and implements `Snapshot`, its test will
//! look like this:
//!
//! ```ignore
//! use melee_sim::schema::{Exclusion, Schema, SchemaCoverage};
//!
//! const EXCLUDED: &[Exclusion] = &[
//!     // Pointers are heap addresses: meaningless across emulator and port.
//!     // `ptr` is not a required type, so these need no entry. Only fields
//!     // of a *required* type that the port deliberately does not mirror go
//!     // here, each with its reason:
//!     Exclusion { path: "x18", reason: "scratch: written by ftAnim, never read" },
//!     Exclusion { path: "x40", reason: "camera-only; port has no renderer" },
//! ];
//!
//! #[test]
//! fn fighter_covers_schema() {
//!     let schema = Schema::fighter_generated();
//!     let cov = SchemaCoverage::new(&schema, EXCLUDED);
//!     cov.assert_covers(&Fighter::default());
//! }
//! ```
//!
//! The rules the check enforces:
//!
//! - **Required paths** are every schema field whose type is one of
//!   [`REQUIRED_TYPES`] (`u8 u16 u32 s8 s16 s32 f32 f64 vec3`, with `vec3`
//!   expanding to `.x .y .z`), minus the exclusions. `ptr` fields are not
//!   required (addresses differ by construction). `unknown` fields are not
//!   required either: the decoder cannot read them yet, so the port cannot be
//!   compared on them.
//! - **Exclusions must be live.** Every excluded path must exist in the
//!   schema (else the exclusion is stale and the check fails) and must not
//!   be emitted (else the exclusion is redundant and the check fails). The
//!   list therefore stays an honest record of what is knowingly not compared.
//! - **Extra paths are allowed.** The port may emit more than the schema
//!   (derived values, ported-but-not-yet-decoded fields). `melee-diff` only
//!   compares keys present in the expected trace. They are reported so a
//!   typo in a field name shows up as one missing plus one extra.
//!
//! When `gen_schema.py` learns to type a field that used to be `unknown`
//! (a bitfield, a nested aggregate that gets its own schema), that field
//! becomes required and the `Fighter` test fails until the port either emits
//! it or adds an exclusion with a reason. That is the intended ratchet.
//!
//! Until `Fighter` is ported, `tests/schema_coverage.rs` proves the machinery
//! on a stub against `fighter.hand.generated.json`, the JSON mirror of the
//! hand-written `fighter.yaml` that the decoder currently uses.

use melee_types::snapshot::{SnapValue, Snapshot};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt;

/// Schema field types whose paths a `Snapshot` impl must emit.
///
/// `ptr` is excluded because addresses cannot match across emulator and port;
/// `unknown` because the decoder cannot read it.
pub const REQUIRED_TYPES: &[&str] = &["u8", "u16", "u32", "s8", "s16", "s32", "f32", "f64", "vec3"];

/// One struct layout, as written by `harness/gen_schema.py`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Schema {
    /// C struct name, e.g. `Fighter`.
    #[serde(rename = "struct")]
    pub struct_name: String,
    /// `ASSERT_SIZE`, if the header has one.
    pub size: Option<u32>,
    /// `path:line` the layout was read from.
    pub source: Option<String>,
    /// Fields in header order.
    pub fields: Vec<Field>,
}

/// One field with an offset comment.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Field {
    /// Dotted name, e.g. `cur_pos` or `cpu.buttons`.
    pub name: String,
    /// Absolute byte offset from the struct base.
    pub offset: u32,
    /// Schema type: `u8 u16 u32 s8 s16 s32 f32 f64 ptr vec3 unknown`.
    #[serde(rename = "type")]
    pub ty: String,
    /// The C type spelling, kept for `unknown` (and any non-scalar) fields.
    #[serde(default)]
    pub ctype: Option<String>,
    /// Bit offset within the byte, for bitfields.
    #[serde(default)]
    pub bit: Option<u32>,
    /// Bit width, for bitfields.
    #[serde(default)]
    pub width: Option<u32>,
}

impl Field {
    /// Whether a `Snapshot` impl must emit this field.
    pub fn is_required(&self) -> bool {
        REQUIRED_TYPES.contains(&self.ty.as_str())
    }

    /// The trace paths this field expands to: `.x .y .z` for `vec3`, the
    /// name itself otherwise.
    pub fn paths(&self) -> Vec<String> {
        if self.ty == "vec3" {
            ["x", "y", "z"]
                .iter()
                .map(|a| format!("{}.{a}", self.name))
                .collect()
        } else {
            vec![self.name.clone()]
        }
    }
}

impl Schema {
    /// Parse a `*.generated.json` document.
    pub fn from_json(text: &str) -> Result<Schema, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// `struct Fighter` as extracted from the decomp header (692 fields).
    pub fn fighter_generated() -> Schema {
        Schema::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/schema/fighter.generated.json"
        )))
        .expect("fighter.generated.json is well-formed; regenerate with `cd harness && uv run python gen_schema.py`")
    }

    /// `struct Item` as extracted from the decomp header.
    pub fn item_generated() -> Schema {
        Schema::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/schema/item.generated.json"
        )))
        .expect("item.generated.json is well-formed; regenerate with `cd harness && uv run python gen_schema.py`")
    }

    /// The hand-written `fighter.yaml` subset the decoder reads today, mirrored
    /// to JSON by `gen_schema.py`.
    pub fn fighter_hand() -> Schema {
        Schema::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/schema/fighter.hand.generated.json"
        )))
        .expect("fighter.hand.generated.json is well-formed; regenerate with `cd harness && uv run python gen_schema.py`")
    }

    /// Look up a field by dotted name.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Every path of every field, required or not.
    pub fn all_paths(&self) -> BTreeSet<String> {
        self.fields.iter().flat_map(Field::paths).collect()
    }

    /// Every path of every field of a [`REQUIRED_TYPES`] type.
    pub fn required_paths(&self) -> BTreeSet<String> {
        self.fields
            .iter()
            .filter(|f| f.is_required())
            .flat_map(Field::paths)
            .collect()
    }
}

/// A schema path the port knowingly does not emit, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exclusion {
    /// Dotted path exactly as it appears in the trace (for a `vec3`, the
    /// field name excludes all three components; a single component may be
    /// excluded as `name.x`).
    pub path: &'static str,
    pub reason: &'static str,
}

/// Compares the set of paths a `Snapshot` impl emits against a [`Schema`].
pub struct SchemaCoverage<'a> {
    schema: &'a Schema,
    excluded: &'a [Exclusion],
}

impl<'a> SchemaCoverage<'a> {
    pub fn new(schema: &'a Schema, excluded: &'a [Exclusion]) -> Self {
        SchemaCoverage { schema, excluded }
    }

    /// The paths a `Snapshot` impl must emit: required paths minus exclusions.
    pub fn required(&self) -> BTreeSet<String> {
        let mut req = self.schema.required_paths();
        for ex in self.excluded {
            for p in self.expand_exclusion(ex.path) {
                req.remove(&p);
            }
        }
        req
    }

    /// An exclusion names either a full field (expands like the field) or one
    /// already-expanded path.
    fn expand_exclusion(&self, path: &str) -> Vec<String> {
        match self.schema.field(path) {
            Some(f) => f.paths(),
            None => vec![path.to_owned()],
        }
    }

    /// Check an explicit set of emitted paths (schema-relative: `cur_pos.x`,
    /// not `p0.cur_pos.x`).
    pub fn check_paths<I, S>(&self, emitted: I) -> CoverageReport
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let emitted: BTreeSet<String> = emitted.into_iter().map(Into::into).collect();
        let all = self.schema.all_paths();
        let required = self.required();

        let missing = required.difference(&emitted).cloned().collect();
        let extra = emitted.difference(&all).cloned().collect();

        let mut stale_exclusions = Vec::new();
        for ex in self.excluded {
            let paths = self.expand_exclusion(ex.path);
            let in_schema = paths.iter().all(|p| all.contains(p));
            if !in_schema {
                stale_exclusions.push(format!("{}: not in schema ({})", ex.path, ex.reason));
                continue;
            }
            if let Some(p) = paths.iter().find(|p| emitted.contains(*p)) {
                stale_exclusions.push(format!("{p}: excluded but emitted ({})", ex.reason));
            }
        }

        CoverageReport {
            struct_name: self.schema.struct_name.clone(),
            required: required.len(),
            covered: required.intersection(&emitted).count(),
            missing,
            extra,
            stale_exclusions,
        }
    }

    /// Walk `value` with a collecting sink and check the paths it emitted.
    pub fn check(&self, value: &dyn Snapshot) -> CoverageReport {
        self.check_paths(emitted_paths(value))
    }

    /// Panic with the full report if `value` does not cover the schema.
    pub fn assert_covers(&self, value: &dyn Snapshot) {
        let report = self.check(value);
        assert!(report.is_ok(), "{report}");
    }
}

/// Run a `Snapshot` walk and return the paths it produced, in emission order.
pub fn emitted_paths(value: &dyn Snapshot) -> Vec<String> {
    let mut sink: Vec<(String, SnapValue)> = Vec::new();
    value.snapshot(&mut sink);
    sink.into_iter().map(|(p, _)| p).collect()
}

/// Outcome of a coverage check. `is_ok()` iff `missing` and
/// `stale_exclusions` are both empty; `extra` is informational.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub struct_name: String,
    /// Number of required paths after exclusions.
    pub required: usize,
    /// Number of required paths that were emitted.
    pub covered: usize,
    /// Required paths not emitted. Sorted.
    pub missing: Vec<String>,
    /// Emitted paths the schema does not know. Sorted.
    pub extra: Vec<String>,
    /// Exclusions that name a path absent from the schema, or one that is
    /// emitted anyway.
    pub stale_exclusions: Vec<String>,
}

impl CoverageReport {
    pub fn is_ok(&self) -> bool {
        self.missing.is_empty() && self.stale_exclusions.is_empty()
    }
}

impl fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Snapshot coverage of {}: {}/{} required paths emitted",
            self.struct_name, self.covered, self.required
        )?;
        if !self.missing.is_empty() {
            writeln!(f, "  missing ({}):", self.missing.len())?;
            for p in &self.missing {
                writeln!(f, "    {p}")?;
            }
        }
        if !self.stale_exclusions.is_empty() {
            writeln!(f, "  stale exclusions ({}):", self.stale_exclusions.len())?;
            for p in &self.stale_exclusions {
                writeln!(f, "    {p}")?;
            }
        }
        if !self.extra.is_empty() {
            writeln!(f, "  extra, not in schema ({}):", self.extra.len())?;
            for p in &self.extra {
                writeln!(f, "    {p}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Schema {
        Schema::from_json(
            r#"{"struct":"T","size":32,"source":null,"fields":[
                {"name":"gobj","offset":0,"type":"ptr"},
                {"name":"kind","offset":4,"type":"s32"},
                {"name":"pos","offset":8,"type":"vec3"},
                {"name":"mtx","offset":20,"type":"unknown","ctype":"Mtx"},
                {"name":"flag","offset":24,"type":"unknown","ctype":"u8:1","bit":0,"width":1}
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_optional_attrs() {
        let s = tiny();
        assert_eq!(s.struct_name, "T");
        assert_eq!(s.size, Some(32));
        assert_eq!(s.fields.len(), 5);
        assert_eq!(s.field("mtx").unwrap().ctype.as_deref(), Some("Mtx"));
        assert_eq!(
            (s.field("flag").unwrap().bit, s.field("flag").unwrap().width),
            (Some(0), Some(1))
        );
        assert_eq!(s.field("kind").unwrap().ctype, None);
    }

    #[test]
    fn required_excludes_ptr_and_unknown_and_expands_vec3() {
        let s = tiny();
        let req: Vec<_> = s.required_paths().into_iter().collect();
        assert_eq!(req, ["kind", "pos.x", "pos.y", "pos.z"]);
        assert_eq!(s.all_paths().len(), 7);
    }

    #[test]
    fn exclusion_of_whole_vec3_removes_all_components() {
        let s = tiny();
        let ex = [Exclusion {
            path: "pos",
            reason: "test",
        }];
        let cov = SchemaCoverage::new(&s, &ex);
        let req: Vec<_> = cov.required().into_iter().collect();
        assert_eq!(req, ["kind"]);
        assert!(cov.check_paths(["kind"]).is_ok());
    }

    #[test]
    fn stale_exclusions_fail() {
        let s = tiny();
        let ex = [
            Exclusion {
                path: "nope",
                reason: "gone",
            },
            Exclusion {
                path: "kind",
                reason: "but emitted",
            },
        ];
        let cov = SchemaCoverage::new(&s, &ex);
        let r = cov.check_paths(["kind", "pos.x", "pos.y", "pos.z"]);
        assert!(r.missing.is_empty());
        assert_eq!(r.stale_exclusions.len(), 2);
        assert!(!r.is_ok());
        let text = r.to_string();
        assert!(text.contains("nope: not in schema"));
        assert!(text.contains("kind: excluded but emitted"));
    }

    #[test]
    fn extra_paths_are_reported_not_failed() {
        let s = tiny();
        let cov = SchemaCoverage::new(&s, &[]);
        let r = cov.check_paths(["kind", "pos.x", "pos.y", "pos.z", "derived"]);
        assert!(r.is_ok());
        assert_eq!(r.extra, ["derived"]);
        assert_eq!((r.covered, r.required), (4, 4));
    }

    #[test]
    fn generated_schemas_load() {
        let f = Schema::fighter_generated();
        assert_eq!(f.struct_name, "Fighter");
        assert_eq!(f.size, Some(0x23EC));
        assert_eq!(f.fields.len(), 692);
        assert!(f.fields.windows(2).all(|w| w[0].offset <= w[1].offset
            || w[0].name.contains('.')
            || w[1].name.contains('.')));
        assert_eq!(
            f.field("cur_pos").map(|x| (x.offset, x.ty.as_str())),
            Some((0xB0, "vec3"))
        );
        assert_eq!(f.field("cpu.buttons").map(|x| x.offset), Some(0x1A88));
        // Every required path is a scalar or vec3 component; nothing typed
        // `unknown` or `ptr` sneaks in.
        let req = f.required_paths();
        assert!(req.len() > 300, "{}", req.len());
        assert!(!req.contains("gobj"));
        assert!(!req.contains("x44_mtx"));

        let i = Schema::item_generated();
        assert_eq!(i.struct_name, "Item");
        assert_eq!(i.size, Some(0xFCC));
        assert_eq!(i.fields.len(), 35);

        let h = Schema::fighter_hand();
        assert_eq!(h.struct_name, "Fighter");
        assert_eq!(h.fields.len(), 18);
        // 18 fields, three of them vec3: 15 + 3*3 paths, all required.
        assert_eq!(h.required_paths().len(), 24);
        assert_eq!(h.all_paths(), h.required_paths());
    }
}
