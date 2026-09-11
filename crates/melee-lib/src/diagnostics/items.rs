use melee_diff::{Record, Value};
use std::collections::BTreeMap;
pub fn actual(items: &melee_it::ItemPool, frame: u64) -> Record {
    let mut record = Record {
        frame,
        phase: "frame_end".into(),
        state: BTreeMap::new(),
    };
    record
        .state
        .insert("items.count".into(), Value::UInt(items.len() as u64));
    for (index, item) in items.iter().enumerate() {
        use melee_coll::hitbox::CapsulePhase;
        let phase = match item.hitboxes[0].as_ref().map(|hit| hit.phase) {
            None => 0,
            Some(CapsulePhase::Enabled) => 1,
            Some(CapsulePhase::FirstPosition) => 2,
            Some(CapsulePhase::Sweeping) => 3,
        };
        let values = [
            Value::Int(i32::from(item.kind) as i64),
            item.owner
                .map_or(Value::Null, |p| Value::UInt(u64::from(p))),
            Value::f32(item.position.x),
            Value::f32(item.position.y),
            Value::f32(item.position.z),
            Value::f32(item.velocity.x),
            Value::f32(item.velocity.y),
            Value::f32(item.velocity.z),
            Value::f32(item.facing),
            Value::Int(i64::from(item.motion)),
            Value::f32(item.life_timer),
            Value::Int(phase),
        ];
        for (key, value) in KEYS.into_iter().zip(values) {
            record.state.insert(format!("items.{index}.{key}"), value);
        }
    }
    record
}
pub const KEYS: [&str; 12] = [
    "kind",
    "owner",
    "pos.x",
    "pos.y",
    "pos.z",
    "vel.x",
    "vel.y",
    "vel.z",
    "facing_dir",
    "motion_id",
    "life_timer",
    "hitbox0.state",
];
