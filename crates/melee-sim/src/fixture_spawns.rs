//! External particle inputs observed at their production callers. This module
//! never observes particle outputs or the interpreter's child spawn requests.
use hsd_particle::system::SpawnRequest;
use hsd_types::{Mtx, Vec3};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Concrete optional sink: no trait object, shared owner, or allocation on the
/// normal path. All JSON construction is behind the `Some` branch.
#[derive(Default)]
pub(crate) struct EventSink(Option<Recording>);

#[derive(Default)]
struct Recording {
    frame: u64,
    events: Vec<(u64, Value)>,
}

impl EventSink {
    pub(crate) fn enable(&mut self) {
        self.0 = Some(Recording::default());
    }

    #[inline]
    pub(crate) fn begin_tick(&mut self, frame: u64) {
        if let Some(recording) = &mut self.0 {
            recording.frame = frame;
        }
    }

    #[inline]
    pub(crate) fn spawn(&mut self, request: &SpawnRequest, detach: bool, after_particles: bool) {
        if let Some(recording) = &mut self.0 {
            // The existing fixture reader has no velocity override form. Fail
            // explicitly rather than silently producing an incomplete fixture.
            assert!(
                request.velocity.is_none(),
                "fixture velocity override unsupported"
            );
            let mut event = json!({
                "bank": request.bank, "spawn": request.kind, "link": request.link,
                "position": request.position.map(f32::to_bits),
                "joint": request.joint.map(|(id, m)| json!({"id": id, "matrix": matrix(m)})),
            });
            if let Some(t) = &request.application_transform {
                event["transform"] = json!({
                    "translation": vector(t.translation), "rotation": vector(t.rotation),
                    "scale": vector(t.scale), "status": t.status,
                });
            }
            if request.mirror {
                event["mirror"] = json!(true);
            }
            if detach {
                event["detach"] = json!(true);
            }
            if after_particles {
                event["after_particles"] = json!(true);
            }
            recording.events.push((recording.frame, event));
        }
    }

    #[inline]
    pub(crate) fn update_joint(&mut self, id: usize, m: Mtx) {
        if let Some(recording) = &mut self.0 {
            recording
                .events
                .push((recording.frame, json!({"update": id, "matrix": matrix(m)})));
        }
    }

    #[inline]
    pub(crate) fn expire_joint(&mut self, id: usize) {
        if let Some(recording) = &mut self.0 {
            recording
                .events
                .push((recording.frame, json!({"expire": id})));
        }
    }

    #[inline]
    pub(crate) fn flags(&mut self, joint: usize, clear: u16, set: u16) {
        if let Some(recording) = &mut self.0 {
            recording.events.push((
                recording.frame,
                json!({"flags_joint": joint, "clear": clear, "set": set}),
            ));
        }
    }

    #[inline]
    pub(crate) fn external_randf(&mut self, site: u32) {
        if let Some(recording) = &mut self.0 {
            recording
                .events
                .push((recording.frame, json!({"external_randf": site})));
        }
    }

    pub(crate) fn finish(&mut self) -> BTreeMap<u64, Vec<Value>> {
        let recording = self.0.take().expect("fixture recording enabled");
        let joints: BTreeSet<_> = recording
            .events
            .iter()
            .filter(|(_, e)| e["detach"] != true)
            .filter_map(|(_, e)| e["joint"]["id"].as_u64())
            .collect();
        let mut matrices = BTreeMap::new();
        let mut ticks = BTreeMap::<_, Vec<_>>::new();
        for (frame, event) in recording.events {
            if let Some(id) = event["update"].as_u64() {
                if !joints.contains(&id) || matrices.get(&id) == Some(&event["matrix"]) {
                    continue;
                }
                // A spawn matrix is local to that generator. Only update_joint
                // changes older generators which share its joint identity.
                matrices.insert(id, event["matrix"].clone());
            }
            if let Some(id) = event["expire"].as_u64() {
                if !joints.contains(&id) {
                    continue;
                }
                matrices.remove(&id);
            }
            ticks.entry(frame).or_default().push(event);
        }
        ticks
    }
}

fn matrix(m: Mtx) -> [[u32; 4]; 3] {
    m.0.map(|row| row.map(f32::to_bits))
}

fn vector(v: Vec3) -> [u32; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}
