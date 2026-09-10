//! Item oracle keys, independent of the existing 49 fighter keys.
use anyhow::{ensure, Context, Result};
use melee_diff::{Record, Value};
use std::collections::BTreeMap;

/// Twelve per-item keys plus list cardinality. The same keys repeat in list order.
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

pub fn expected(json: &serde_json::Value, frame: u64) -> Result<Option<Record>> {
    let Some(items) = json.get("items") else {
        return Ok(None);
    };
    let items = items.as_array().context("items must be an array")?;
    let mut record = Record {
        frame,
        phase: "frame_end".into(),
        state: BTreeMap::new(),
    };
    record
        .state
        .insert("items.count".into(), Value::UInt(items.len() as u64));
    for (index, item) in items.iter().enumerate() {
        for key in KEYS {
            let value = if key == "owner" {
                match item.get("owner").context("missing item owner")? {
                    serde_json::Value::Null => Value::Null,
                    value => Value::UInt(
                        value
                            .as_u64()
                            .context("item owner must be a player slot or null")?,
                    ),
                }
            } else {
                serde_json::from_value(item["state"][key].clone())
                    .with_context(|| format!("missing item key {index}.{key}"))?
            };
            if key == "kind" {
                ensure!(
                    value == Value::Int(item["kind"].as_i64().context("item kind")?),
                    "item kind metadata mismatch"
                );
            }
            record.state.insert(format!("items.{index}.{key}"), value);
        }
    }
    Ok(Some(record))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use melee_diff::first_divergence;
    use serde_json::json;

    fn item(kind: i64, owner: u8) -> serde_json::Value {
        let mut state = serde_json::Map::new();
        for key in KEYS.into_iter().filter(|key| *key != "owner") {
            let value = match key {
                "kind" => Value::Int(kind),
                "motion_id" | "hitbox0.state" => Value::Int(0),
                _ => Value::f32(0.0),
            };
            state.insert(key.into(), serde_json::to_value(value).unwrap());
        }
        json!({"kind": kind, "owner": owner, "state": state})
    }

    #[test]
    fn item_oracle_rejects_reordering_missing_owners_and_one_ulp() {
        let blaster = item(74, 0);
        let laser = item(54, 0);
        let expected_row = json!({"items": [blaster, laser]});
        let expected = expected(&expected_row, 42).unwrap().unwrap();
        let reordered = json!({"items": [item(54, 0), item(74, 0)]});
        let reordered = super::expected(&reordered, 42).unwrap().unwrap();
        assert_eq!(
            first_divergence([&expected], [&reordered]).unwrap().path,
            "items.0.kind"
        );

        let mut one_ulp = expected_row.clone();
        one_ulp["items"][1]["state"]["pos.x"] =
            serde_json::to_value(Value::f32(f32::from_bits(1))).unwrap();
        let one_ulp = super::expected(&one_ulp, 42).unwrap().unwrap();
        assert_eq!(
            first_divergence([&expected], [&one_ulp]).unwrap().path,
            "items.1.pos.x"
        );

        let mut missing_owner = expected_row;
        missing_owner["items"][0]
            .as_object_mut()
            .unwrap()
            .remove("owner");
        assert!(super::expected(&missing_owner, 42).is_err());
        let empty = super::expected(&json!({"items": []}), 42).unwrap().unwrap();
        assert!(first_divergence([&expected], [&empty]).is_some());
        assert!(super::expected(&json!({}), 42).unwrap().is_none());
    }
}
