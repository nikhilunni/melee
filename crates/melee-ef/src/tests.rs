use super::pool::ASYNC_CAPACITY;
use super::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
};

struct Allocator;
thread_local! {
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
}
fn allocation() {
    let _ = COUNT.try_with(|count| {
        if let Some(n) = count.get() {
            count.set(Some(n + 1));
        }
    });
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn count_allocations(run: impl FnOnce()) -> usize {
    COUNT.with(|count| count.set(Some(0)));
    run();
    COUNT.with(|count| count.replace(None).unwrap())
}
struct Trig;
impl InverseTrig for Trig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}
fn archive() -> Option<Archive> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/EfCoData.dat");
    if !path.is_file() {
        eprintln!("skipping effect assets: {} absent", path.display());
        return None;
    }
    Some(Archive::parse(&std::fs::read(path).unwrap()).unwrap())
}

#[test]
fn outgoing_pose_batches_preserve_order_without_allocating() {
    use request::{EffectQueue, REQUEST_CAPACITY};
    let mut queue = EffectQueue::default();
    let first = EffectRequest::CaptureFlash { bone: 1 };
    let second = EffectRequest::CaptureFlash { bone: 2 };
    let mut matrix = Mtx::IDENTITY;
    matrix.0[0][3] = 17.0;
    let allocations = count_allocations(|| {
        queue.push(first);
        queue.push(EffectRequest::DestroyOwned);
        queue.push(second);
        queue.resolve_pending(|_| matrix);
        queue.push(first);
        let mut newer = matrix;
        newer.0[0][3] = 23.0;
        queue.resolve_pending(|_| newer);
        let immediate = queue.drain(EffectTiming::Immediate);
        let mut actual = immediate.iter();
        assert_eq!(actual.next().unwrap().request, EffectRequest::DestroyOwned);
        for (request, pose) in [(second, matrix), (first, matrix), (first, newer)] {
            let entry = actual.next().unwrap();
            assert_eq!(entry.request, request);
            assert_eq!(entry.matrix, Some(pose));
        }
        assert!(actual.next().is_none());
        for bone in 0..REQUEST_CAPACITY {
            queue.push(EffectRequest::CaptureFlash { bone });
        }
        let pending = queue.drain(EffectTiming::Deferred);
        for (bone, entry) in pending.iter().rev().enumerate() {
            assert_eq!(entry.request, EffectRequest::CaptureFlash { bone });
            assert!(entry.matrix.is_none());
        }
        assert!(queue.is_empty());
    });
    assert_eq!(allocations, 0);
}

#[test]
fn entry_request_follows_only_the_graphics_issued_before_it() {
    // Deferred flushes pop newest first; the queue holds issue order.
    let flash = EffectRequest::CaptureFlash { bone: 71 };
    let graphic = |bone| EffectRequest::Attached { id: 0x412, bone };
    let cases: [(usize, &[EffectRequest]); 3] = [
        (0, &[flash, graphic(1), graphic(2)]),
        (1, &[graphic(1), flash, graphic(2)]),
        (2, &[graphic(1), graphic(2), flash]),
    ];
    for (issued_before, expected) in cases {
        let mut queue = request::EffectQueue::default();
        queue.push_after_graphics(flash, issued_before);
        queue.push_graphics(graphic(1));
        queue.push_graphics(graphic(2));
        queue.finish_graphics();
        assert!(queue.iter().eq(expected.iter()), "{issued_before} issued before");
    }
}

#[test]
#[should_panic(expected = "effect storage capacity 64 exhausted")]
fn request_overflow_is_explicit() {
    let mut queue = request::EffectQueue::default();
    for _ in 0..=request::REQUEST_CAPACITY {
        queue.push(EffectRequest::DestroyOwned);
    }
}

#[test]
fn every_model_restarts_exactly_without_allocation() {
    let Some(archive) = archive() else {
        return;
    };
    // Exercise every supported descriptor past ordinary lifetimes, then restore
    // and compare its complete tree/cursors against a freshly loaded model.
    for id in [
        4, 5, 8, 9, 10, 0xB, 0xC, 0xD, 0xF, 0x12, 0x13, 0x18, 0x19, 0x1E, 0x1F, 0x24,
    ] {
        let initial = Effect::load(&archive, id).unwrap();
        let mut effect = initial.clone();
        effect
            .tree
            .events
            .reserve_exact(initial.tree.events.capacity());
        let allocations = count_allocations(|| {
            for _ in 0..3 {
                effect
                    .tree
                    .set_scale(effect.root, &Vec3::new(2.0, 3.0, 4.0));
                effect
                    .tree
                    .set_translate(effect.root, &Vec3::new(11.0, 12.0, 13.0));
                for _ in 0..100 {
                    for &joint in effect.joints.iter() {
                        effect.tree.anim::<Trig>(joint, &mut Default::default());
                        effect.tree.events.clear();
                        effect.tree.setup_matrix(joint);
                    }
                }
                effect.reset(&initial);
            }
        });
        assert_eq!(allocations, 0, "model {id:#x}");
        assert_eq!(effect.tree, initial.tree, "model {id:#x}");
    }
}

#[test]
fn full_pool_retires_oldest_and_reuses_slots_without_allocation() {
    let Some(archive) = archive() else {
        return;
    };
    let mut effects = Effects::load(&archive).unwrap();
    let mut particles = ParticleSystem::default();
    let allocations = count_allocations(|| {
        for generation in 0..3 * ASYNC_CAPACITY {
            let mut effect = effects.acquire(5, &mut particles);
            effect.joint_base = generation;
            effects.instances.push(effect);
        }
        assert_eq!(effects.instances.len(), ASYNC_CAPACITY);
        assert_eq!(
            effects.instances.iter().next().unwrap().joint_base,
            2 * ASYNC_CAPACITY
        );
        effects.recycle_where(|_| true);
        assert!(effects.instances.is_empty());
    });
    assert_eq!(allocations, 0);
}

#[test]
fn async_pressure_preserves_sync_models_and_retires_link_11_before_death() {
    let Some(archive) = archive() else {
        return;
    };
    let mut effects = Effects::load(&archive).unwrap();
    let mut particles = ParticleSystem::default();
    for id in [0xB, 0x24, 0x1E, 0x19] {
        let effect = effects.acquire(id, &mut particles);
        effects.instances.push(effect);
    }
    for generation in 0..ASYNC_CAPACITY {
        let mut effect = effects.acquire(5, &mut particles);
        effect.joint_base = generation;
        effects.instances.push(effect);
    }
    assert_eq!(effects.instances.len(), ASYNC_CAPACITY + 3);
    let descriptors: Vec<_> = effects
        .instances
        .iter()
        .take(4)
        .map(|e| e.descriptor)
        .collect();
    assert_eq!(descriptors, [0xB, 0x24, 0x1E, 0x19]);
    assert_eq!(effects.instances.iter().nth(4).unwrap().joint_base, 1);
}
