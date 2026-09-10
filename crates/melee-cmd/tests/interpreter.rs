use melee_cmd::{decode::decode, Command, ScriptState};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
fn count() {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        COUNT.with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[test]
fn loops_calls_and_fractional_timers_emit_in_order_without_allocating() {
    let code = [
        Command::BeginLoop(2),
        Command::Call {
            target: 7,
            continuation: 2,
        },
        Command::Wait(1.5),
        Command::EndLoop,
        Command::AtFrame(5.0),
        Command::AllowInterrupt,
        Command::End,
        Command::SetVariable { index: 0, value: 9 },
        Command::Return,
    ];
    let mut state = ScriptState::default();
    let mut output = [(0usize, 0usize); 3];
    COUNT.with(|n| n.set(0));
    ENABLED.with(|n| n.set(true));
    state.restart(0);
    let mut cursor = 0;
    for tick in 0..=5 {
        state.begin_frame(tick as f32, 1.0);
        while let Some(command) = state.next(&code, 1.0) {
            output[cursor] = (
                tick,
                match command {
                    Command::SetVariable { .. } => 9,
                    Command::AllowInterrupt => 1,
                    _ => panic!("unexpected command"),
                },
            );
            cursor += 1;
        }
    }
    ENABLED.with(|n| n.set(false));
    assert_eq!(COUNT.with(Cell::get), 0);
    assert_eq!(output, [(0, 9), (1, 9), (5, 1)]);
    assert!(state.instruction.is_none());
    assert!(state.loops.is_empty() && state.return_stack.is_empty());
}
#[test]
fn animation_wrap_wait_yields_then_resumes_only_after_wrap() {
    let code = [
        Command::WaitAnimationLoop,
        Command::AllowInterrupt,
        Command::End,
    ];
    let mut state = ScriptState::default();
    state.restart(0);
    state.begin_frame(0.0, 1.0);
    assert!(state.next(&code, 1.0).is_none());
    assert!(state.next(&code, 1.0).is_none());
    state.begin_frame(9.0, 1.0);
    assert!(state.next(&code, 1.0).is_none());
    state.begin_frame(0.25, 1.0);
    assert!(matches!(
        state.next(&code, 1.0),
        Some(Command::AllowInterrupt)
    ));
    assert_eq!(state.timer.to_bits(), (-0.25f32).to_bits());
}
#[test]
fn restart_clears_control_flow_and_stopped_script_still_samples_frame() {
    let mut state = ScriptState::default();
    state.restart(0);
    state.begin_frame(7.0, 0.0);
    assert!(state.next(&[Command::End], 0.0).is_none());
    state.begin_frame(8.0, 1.0);
    assert_eq!(state.frame, 8.0);
    assert_eq!(state.timer, 0.0);
    state.restart(2);
    assert_eq!(state.instruction, Some(2));
}
#[test]
fn decoding_preserves_signed_fields_and_relocated_control_targets() {
    let command = decode(&[(31 << 26) | (127 << 19) | 0x7ffff], None, 0).unwrap();
    assert!(matches!(
        command,
        Command::ModelSelection {
            group: -1,
            variant: -1
        }
    ));
    assert!(matches!(
        decode(&[5 << 26, 0], Some(15), 23).unwrap(),
        Command::Call {
            target: 15,
            continuation: 23
        }
    ));
    assert!(decode(&[5 << 26, 0], None, 0).is_err());
    assert!(decode(&[11 << 26], None, 0).is_err());
    assert!(decode(&[63 << 26], None, 0).is_err());
}
#[test]
fn graphics_and_capsules_use_retail_literal_and_signed_halfwords() {
    let Command::Graphics(graphics) = decode(
        &[10 << 26, 0x1234_0007, 0xff00_0100, 0x0000_0200, 0x0300_0400],
        None,
        0,
    )
    .unwrap() else {
        panic!("graphics");
    };
    assert_eq!(graphics.id, 0x1234);
    assert_eq!(graphics.offset.x.to_bits(), (-0.999936f32).to_bits());
    assert_eq!(graphics.offset.y.to_bits(), 0.999936f32.to_bits());
    assert_eq!(graphics.range.z.to_bits(), (0.003906f32 * 1024.0).to_bits());
    let Command::SpawnHitbox { descriptor, .. } =
        decode(&[(11 << 26) | 7, 0x0100_ff00, 0, 0, 3], None, 0).unwrap()
    else {
        panic!("hitbox");
    };
    assert_eq!(descriptor.damage, 7.0);
    assert_eq!(descriptor.radius.to_bits(), 0.999936f32.to_bits());
    assert_eq!(descriptor.offset.x.to_bits(), (-0.999936f32).to_bits());
    assert!(descriptor.hit_ground && descriptor.hit_air);
}

#[test]
fn graphics_payload_is_shared_with_color_overlay_opcode() {
    let mut words = [10 << 26 | 7 << 18, 0x1234_0007, 0xff00_0100, 0, 0];
    let ordinary = melee_cmd::decode::graphics(&words);
    words[0] = (words[0] & 0x03ff_ffff) | 21 << 26;
    let overlay = melee_cmd::decode::graphics(&words);
    assert_eq!(ordinary.bone, overlay.bone);
    assert_eq!(ordinary.id, overlay.id);
    assert_eq!(ordinary.offset.x.to_bits(), overlay.offset.x.to_bits());
}
