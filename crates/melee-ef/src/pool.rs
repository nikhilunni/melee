//! efLib_Create/efLib_RemoveLast model storage, prepared from assets at load.
use super::*;

/// efLib_Create (8005BE88), eflib.c:442-447 caps async models at 64.
pub const ASYNC_CAPACITY: usize = 64;
/// Port bound for sync models. Retail excludes these from EffectCount and uses
/// HSD_ObjAlloc without a numeric cap (eflib.c:442-464, efsync.c:84).
const SYNC_CAPACITY: usize = 64;
pub const INSTANCE_CAPACITY: usize = ASYNC_CAPACITY + SYNC_CAPACITY;
// Either descriptor class can occupy all 64 slots in its own pool.
const SLOTS_PER_MODEL: usize = 64;
// Common descriptors reached by the supported efAsync/efSync dispatch rows.
static MODEL_IDS: [u32; 15] = [
    4, 5, 8, 9, 10, 0xB, 0xC, 0xD, 0xF, 0x12, 0x13, 0x18, 0x19, 0x1E, 0x1F,
];
const WARP_MODEL: u32 = 0x24;

pub(super) struct ModelPool {
    models: Vec<ModelSlots>,
}
struct ModelSlots {
    initial: Effect,
    free: FixedVec<Effect, SLOTS_PER_MODEL>,
}
impl ModelPool {
    fn load(archive: &Archive) -> Result<Self> {
        let mut models = Vec::with_capacity(MODEL_IDS.len() + 1);
        for id in MODEL_IDS.into_iter().chain([WARP_MODEL]) {
            let initial = Effect::load(archive, id)?;
            let mut free = FixedVec::default();
            for _ in 0..SLOTS_PER_MODEL {
                let mut effect = initial.clone();
                effect
                    .tree
                    .events
                    .reserve_exact(initial.tree.events.capacity());
                free.push(effect);
            }
            models.push(ModelSlots { initial, free });
        }
        Ok(Self { models })
    }
    fn take(&mut self, descriptor: u32) -> Effect {
        self.models
            .iter_mut()
            .find(|model| model.initial.descriptor == descriptor)
            .expect("effect descriptor was not loaded")
            .free
            .pop()
            .expect("effect model pool exhausted")
    }
    fn release(&mut self, mut effect: Effect) {
        let model = self
            .models
            .iter_mut()
            .find(|model| model.initial.descriptor == effect.descriptor)
            .unwrap();
        effect.reset(&model.initial);
        model.free.push(effect);
    }
}
impl Effects {
    /// Decode the supported common models and prepare every reusable slot.
    /// Archives, trees and animation byte streams are never cloned in the tick.
    pub fn load(archive: &Archive) -> Result<Self> {
        Ok(Self {
            events: Default::default(),
            camera_quakes: Default::default(),
            draws: DrawLog(Vec::with_capacity(DRAW_CAPACITY)),
            instances: Default::default(),
            models: ModelPool::load(archive)?,
            next_joint: 0,
            fighter_joints: [false; 2 * FIGHTER_JOINT_STRIDE],
        })
    }
    pub(super) fn recycle_where(&mut self, remove: impl Fn(&Effect) -> bool) {
        let mut index = 0;
        while index < self.instances.len() {
            if remove(self.instances.iter().nth(index).unwrap()) {
                self.models.release(self.instances.remove(index));
            } else {
                index += 1;
            }
        }
    }
    pub(super) fn acquire(&mut self, descriptor: u32, particles: &mut ParticleSystem) -> Effect {
        self.make_room(descriptor, 1, particles);
        self.models.take(descriptor)
    }
    pub(super) fn make_room(
        &mut self,
        descriptor: u32,
        count: usize,
        particles: &mut ParticleSystem,
    ) {
        if is_sync(descriptor) {
            assert!(
                self.instances
                    .iter()
                    .filter(|e| is_sync(e.descriptor))
                    .count()
                    + count
                    <= SYNC_CAPACITY,
                "synchronous effect pool capacity exhausted"
            );
            return;
        }
        assert!(count <= ASYNC_CAPACITY);
        while self
            .instances
            .iter()
            .filter(|e| !is_sync(e.descriptor))
            .count()
            + count
            > ASYNC_CAPACITY
        {
            // efLib_RemoveLast (8005BBB4): p_link 11 before p_link 12,
            // oldest first in each, and efLib_Destroy skips sync models.
            let index = self
                .instances
                .iter()
                .position(|e| !is_sync(e.descriptor) && e.descriptor != 0x19)
                .or_else(|| self.instances.iter().position(|e| !is_sync(e.descriptor)))
                .unwrap();
            let effect = self.instances.remove(index);
            for &joint in &effect.joints {
                self.events.expire_joint(effect.joint_base + joint.0);
                particles.expire_joint(effect.joint_base + joint.0);
            }
            self.models.release(effect);
        }
    }
    /// Pending camera requests, in call order. Camera rendering owns consumption;
    /// the headless update discards any remaining requests at link 15.
    pub fn drain_camera_quakes(&mut self, mut quake: impl FnMut(u16, Vec3)) {
        while !self.camera_quakes.is_empty() {
            let (kind, position) = self.camera_quakes.remove(0);
            quake(kind, position);
        }
    }
}
impl Effect {
    pub(super) fn reset(&mut self, initial: &Self) {
        self.velocity = None;
        self.attachment = None;
        self.owner = None;
        self.lifetime = initial.lifetime;
        self.indefinite = initial.indefinite;
        self.shield_bone = None;
        self.joint_base = 0;
        self.tree.events.clear();
        // The headless model has immutable topology, no DObjs/constraints and
        // fixed animation tracks. Restore only mutable pose and playback state.
        for &id in &self.joints {
            let source = initial.tree.get(id);
            let target = self.tree.get_mut(id);
            target.flags = source.flags;
            target.rotate = source.rotate;
            target.scale = source.scale;
            target.translate = source.translate;
            target.mtx = source.mtx;
            target.scl = source.scl;
            if let (Some(target), Some(source)) = (&mut target.aobj, &source.aobj) {
                target.flags = source.flags;
                target.curr_frame = source.curr_frame;
                target.rewind_frame = source.rewind_frame;
                target.end_frame = source.end_frame;
                target.framerate = source.framerate;
                for (target, source) in target.fobj.iter_mut().zip(&source.fobj) {
                    target.restore_playback(source);
                }
            }
        }
    }
}

/// Supported sync-load rows: shields/entry (efasync.c:407,429,453,751)
/// and egg shells (efsync.c:84,228-292). All other modeled rows use async load.
fn is_sync(descriptor: u32) -> bool {
    matches!(descriptor, 0xB | 0xC | 0xD | 0x1E | 0x1F | 0x24)
}
