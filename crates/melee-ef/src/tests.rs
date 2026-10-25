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
        effect.tree.events.reserve_exact(initial.event_capacity);
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
fn a_reused_slot_carries_no_callback_or_constraint_from_its_last_model() {
    let Some(archive) = archive() else {
        return;
    };
    // efLib_Create clears `update` and loads a fresh JObj: a recycled slot
    // must equal a fresh one, whichever callback its last model ran.
    let initial = Effect::load(&archive, 0x28).unwrap();
    let mut effect = initial.clone();
    effect.oriented = true;
    effect.orient::<Trig>(hsd_anim::orientation::OrientationTarget::World(
        Mtx::IDENTITY,
    ));
    effect.callback_rotation_z = Some(0.5);
    effect.follow_bone_height = Some((1, 2, 3.0));
    effect.lifetime_end_spawn = Some(85);
    effect.reset(&initial);
    assert!(!effect.oriented);
    assert_eq!(effect.callback_rotation_z, None);
    assert_eq!(effect.follow_bone_height, None);
    assert_eq!(effect.lifetime_end_spawn, None);
    assert_eq!(effect.tree, initial.tree);
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

#[test]
fn issued_graphics_do_not_query_the_current_owner_matrix_again() {
    struct Owner {
        queue: request::EffectQueue,
        queries: usize,
    }
    impl EffectOwner for Owner {
        fn effect_queue(&mut self) -> &mut request::EffectQueue {
            &mut self.queue
        }
        fn effect_matrix(&mut self, bone: Option<usize>) -> Mtx {
            assert_eq!(bone, Some(61));
            self.queries += 1;
            Mtx::IDENTITY
        }
        fn effect_orientation_target(
            &mut self,
            _: usize,
        ) -> hsd_anim::orientation::OrientationTarget {
            panic!("0x41D is a positional model, not an orientation attachment");
        }
        fn effect_facing(&self) -> f32 {
            -1.0
        }
        fn effect_scale(&self) -> Vec3 {
            Vec3::new(1.0, 1.0, 1.0)
        }
    }
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let path = root.join("harness/roms/files/EfCoData.dat");
    if !melee_test_support::require_files([path.clone()]) {
        return;
    }
    let archive = Archive::parse(&std::fs::read(path).unwrap()).unwrap();
    let table = archive.public("effCommonDataTable").unwrap();
    let commands = archive.link(table).unwrap().unwrap() as usize;
    let textures = archive.link(table + 4).unwrap().unwrap() as usize;
    let bank = ParticleBank::from_bytes(
        &archive.data()[commands..textures],
        &archive.data()[textures..],
    )
    .unwrap();
    let request = EffectRequest::Graphics {
        id: 0x41D,
        bone: 61,
        offset: Vec3::ZERO,
        facing: -1.0,
        floor_angle: 0.0,
    };
    for issued in [true, false] {
        let mut effects = Effects::load(&archive).unwrap();
        let mut owner = Owner {
            queue: Default::default(),
            queries: 0,
        };
        if issued {
            owner.queue.push_graphics_at(request, Mtx::IDENTITY);
        } else {
            owner.queue.push_graphics(request);
        }
        let timing = if issued {
            EffectTiming::Immediate
        } else {
            EffectTiming::Deferred
        };
        effects
            .flush::<Trig>(
                timing,
                0,
                &mut owner,
                &bank,
                &bank,
                &mut ParticleSystem::default(),
                &mut HsdRng::new(1),
            )
            .unwrap();
        assert_eq!(owner.queries, usize::from(!issued), "issued={issued}");
        assert_eq!(effects.instances.len(), 1);
    }
}
