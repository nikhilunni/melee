//! Proves the schema-coverage ratchet on a stand-in for `Fighter`.
//!
//! `melee_ft::Fighter` is not ported yet. `FighterStub` here implements
//! `Snapshot` for exactly the 18 fields in the hand-written
//! `harness/schema/fighter.yaml` (the subset `decode.py` reads today), and the
//! tests show that the coverage check passes for a complete emitter and fails,
//! naming the path, as soon as one field is dropped. See
//! `melee_sim::schema` for how the real `Fighter` will be held to
//! `fighter.generated.json`.

use hsd_types::Vec3;
use melee_sim::schema::{emitted_paths, Exclusion, Schema, SchemaCoverage};
use melee_types::snapshot::{PrefixSink, SnapValue, Snapshot, SnapshotSink};

/// `struct CpuFighter` subset (types.h, `fp+1A88`).
#[derive(Default)]
struct CpuStub {
    buttons: u32,
    lstick_x: i8,
    lstick_y: i8,
    ty: i32,
    level: i32,
    behavior: i32,
    timer: i32,
}

impl Snapshot for CpuStub {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("buttons", &self.buttons);
        sink.field("lstick_x", &self.lstick_x);
        sink.field("lstick_y", &self.lstick_y);
        sink.field("type", &self.ty);
        sink.field("level", &self.level);
        sink.field("behavior", &self.behavior);
        sink.field("timer", &self.timer);
    }
}

/// `struct Fighter` subset matching `harness/schema/fighter.yaml`.
#[derive(Default)]
struct FighterStub {
    kind: i32,
    player_id: u8,
    motion_id: i32,
    facing_dir: f32,
    self_vel: Vec3,
    kb_vel: Vec3,
    cur_pos: Vec3,
    ground_or_air: i32,
    cur_anim_frame: f32,
    percent: f32,
    jumps_used: u8,
    cpu: CpuStub,
}

impl Snapshot for FighterStub {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("kind", &self.kind);
        sink.field("player_id", &self.player_id);
        sink.field("motion_id", &self.motion_id);
        sink.field("facing_dir", &self.facing_dir);
        sink.field("self_vel", &self.self_vel);
        sink.field("kb_vel", &self.kb_vel);
        sink.field("cur_pos", &self.cur_pos);
        sink.field("ground_or_air", &self.ground_or_air);
        sink.field("cur_anim_frame", &self.cur_anim_frame);
        sink.field("percent", &self.percent);
        sink.field("jumps_used", &self.jumps_used);
        sink.field("cpu", &self.cpu);
    }
}

/// The same struct with one field forgotten, as a porting mistake would be.
struct MissingTimer<'a>(&'a FighterStub);

impl Snapshot for MissingTimer<'_> {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        let f = self.0;
        sink.field("kind", &f.kind);
        sink.field("player_id", &f.player_id);
        sink.field("motion_id", &f.motion_id);
        sink.field("facing_dir", &f.facing_dir);
        sink.field("self_vel", &f.self_vel);
        sink.field("kb_vel", &f.kb_vel);
        sink.field("cur_pos", &f.cur_pos);
        sink.field("ground_or_air", &f.ground_or_air);
        sink.field("cur_anim_frame", &f.cur_anim_frame);
        sink.field("percent", &f.percent);
        sink.field("jumps_used", &f.jumps_used);
        // cpu.timer intentionally not emitted.
        sink.field("cpu.buttons", &f.cpu.buttons);
        sink.field("cpu.lstick_x", &f.cpu.lstick_x);
        sink.field("cpu.lstick_y", &f.cpu.lstick_y);
        sink.field("cpu.type", &f.cpu.ty);
        sink.field("cpu.level", &f.cpu.level);
        sink.field("cpu.behavior", &f.cpu.behavior);
    }
}

#[test]
fn stub_covers_hand_schema() {
    let schema = Schema::fighter_hand();
    let cov = SchemaCoverage::new(&schema, &[]);
    let report = cov.check(&FighterStub::default());
    assert!(report.is_ok(), "{report}");
    assert_eq!((report.covered, report.required), (24, 24));
    assert!(report.extra.is_empty(), "{report}");
    cov.assert_covers(&FighterStub::default());
}

#[test]
fn dropped_field_is_named() {
    let schema = Schema::fighter_hand();
    let cov = SchemaCoverage::new(&schema, &[]);
    let stub = FighterStub::default();
    let report = cov.check(&MissingTimer(&stub));
    assert!(!report.is_ok());
    assert_eq!(report.missing, ["cpu.timer"]);
    assert_eq!((report.covered, report.required), (23, 24));
    assert!(report.to_string().contains("missing (1):\n    cpu.timer"));
}

#[test]
fn dropped_vec3_component_is_named() {
    let schema = Schema::fighter_hand();
    let cov = SchemaCoverage::new(&schema, &[]);
    let mut paths = emitted_paths(&FighterStub::default());
    paths.retain(|p| p != "kb_vel.z");
    let report = cov.check_paths(paths);
    assert_eq!(report.missing, ["kb_vel.z"]);
}

#[test]
fn exclusion_with_reason_accepts_the_gap() {
    let schema = Schema::fighter_hand();
    let excluded = [Exclusion {
        path: "cpu.timer",
        reason: "CPU AI not ported until milestone 5",
    }];
    let cov = SchemaCoverage::new(&schema, &excluded);
    let stub = FighterStub::default();
    let report = cov.check(&MissingTimer(&stub));
    assert!(report.is_ok(), "{report}");
    assert_eq!(report.required, 23);

    // Once the field is emitted the exclusion is stale and must be removed.
    let report = cov.check(&stub);
    assert!(!report.is_ok());
    assert_eq!(report.stale_exclusions.len(), 1);
    assert!(report.stale_exclusions[0].starts_with("cpu.timer: excluded but emitted"));
}

#[test]
fn misspelled_field_shows_as_missing_plus_extra() {
    let schema = Schema::fighter_hand();
    let cov = SchemaCoverage::new(&schema, &[]);
    let paths: Vec<String> = emitted_paths(&FighterStub::default())
        .into_iter()
        .map(|p| {
            if p == "jumps_used" {
                "jump_used".to_owned()
            } else {
                p
            }
        })
        .collect();
    let report = cov.check_paths(paths);
    assert_eq!(report.missing, ["jumps_used"]);
    assert_eq!(report.extra, ["jump_used"]);
}

#[test]
fn assert_covers_panics_with_report() {
    let schema = Schema::fighter_hand();
    let cov = SchemaCoverage::new(&schema, &[]);
    let stub = FighterStub::default();
    let err = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cov.assert_covers(&MissingTimer(&stub));
    }))
    .expect_err("must panic");
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(msg.contains("cpu.timer"), "{msg}");
}

/// The stub's value kinds match what `decode.py` produces for each schema
/// type, and the whole thing lands in a `melee-diff` record under a player
/// prefix exactly as the oracle trace would carry it.
#[test]
fn stub_values_match_schema_types_and_build_a_record() {
    let schema = Schema::fighter_hand();
    let stub = FighterStub {
        kind: 1,
        player_id: 0,
        motion_id: 14,
        facing_dir: -1.0,
        cur_pos: Vec3::new(0.0, 0.0001, 0.0),
        percent: 42.5,
        jumps_used: 1,
        cpu: CpuStub {
            level: 9,
            lstick_x: -80,
            ..CpuStub::default()
        },
        ..FighterStub::default()
    };
    let mut entries: Vec<(String, SnapValue)> = Vec::new();
    stub.snapshot(&mut entries);
    for (path, v) in &entries {
        let field = schema
            .fields
            .iter()
            .find(|f| f.paths().iter().any(|p| p == path))
            .unwrap_or_else(|| panic!("{path} not in schema"));
        let ok = matches!(
            (field.ty.as_str(), v),
            ("s8" | "s16" | "s32", SnapValue::I64(_))
                | ("u8" | "u16" | "u32", SnapValue::U64(_))
                | ("f32" | "vec3", SnapValue::F32(_))
        );
        assert!(ok, "{path}: schema type {} but emitted {v:?}", field.ty);
    }

    let mut sink = melee_diff::RecordSink::new(12, "frame_end");
    stub.snapshot(&mut PrefixSink::new(&mut sink, "p0"));
    let rec = sink.finish();
    assert_eq!(rec.state.len(), 24);
    assert_eq!(rec.state["p0.facing_dir"], melee_diff::Value::f32(-1.0));
    assert_eq!(rec.state["p0.cpu.lstick_x"], melee_diff::Value::Int(-80));
    assert_eq!(rec.state["p0.jumps_used"], melee_diff::Value::UInt(1));
    assert!(rec.state.contains_key("p0.cur_pos.y"));
}
