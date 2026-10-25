//! The C API's gamepads (`melee_app_pad_*`) and menu focus movement.
use super::{copy_out, string, Handle};
use crate::{
    focus,
    pads::{Family, Input, RawPad, Target, INPUT_COUNT},
};
use std::{
    ffi::{c_char, CString},
    sync::OnceLock,
};

#[repr(C)]
pub struct RawPadC {
    buttons: [f32; INPUT_COUNT],
    axes: [f32; 4],
}

#[repr(C)]
pub struct PadInfoC {
    family: u32,
    name: [c_char; 64],
    /// -1 without a port.
    port: i32,
    /// The `melee_pad_target_e` waited for, or -1.
    listening: i32,
    bindings: [u8; INPUT_COUNT],
    deadzone: f32,
    trigger_click: f32,
    buttons: u32,
    stick: [i8; 2],
    cstick: [i8; 2],
    triggers: [u8; 2],
}

/// Static NUL-terminated labels.
struct Labels {
    families: Vec<CString>,
    inputs: Vec<Vec<CString>>,
    positions: Vec<CString>,
    targets: Vec<CString>,
}
fn labels() -> &'static Labels {
    static LABELS: OnceLock<Labels> = OnceLock::new();
    LABELS.get_or_init(|| {
        let c = |text: &str| CString::new(text).expect("labels have no NUL");
        Labels {
            families: Family::ALL.iter().map(|f| c(f.name())).collect(),
            inputs: Family::ALL
                .iter()
                .map(|f| Input::ALL.iter().map(|i| c(f.label(*i))).collect())
                .collect(),
            positions: Input::ALL.iter().map(|i| c(i.position())).collect(),
            targets: (0..=Target::ALL.len() as u32)
                .map(|t| c(Target::from_index(t).unwrap_or_default().name()))
                .collect(),
        }
    })
}
fn family_index(family: Family) -> u32 {
    Family::ALL.iter().position(|f| *f == family).unwrap_or_default() as u32
}

/// The family's name (`melee_pad_family_e`), static.
#[no_mangle]
pub extern "C" fn melee_pad_family_name(family: u32) -> *const c_char {
    labels()
        .families
        .get(family as usize)
        .map_or(std::ptr::null(), |s| s.as_ptr())
}
/// What the family prints on a physical control, static.
#[no_mangle]
pub extern "C" fn melee_pad_input_label(family: u32, input: u32) -> *const c_char {
    labels()
        .inputs
        .get(family as usize)
        .and_then(|f| f.get(input as usize))
        .map_or(std::ptr::null(), |s| s.as_ptr())
}
/// Where a physical control sits ("bottom face button"), static.
#[no_mangle]
pub extern "C" fn melee_pad_input_position(input: u32) -> *const c_char {
    labels()
        .positions
        .get(input as usize)
        .map_or(std::ptr::null(), |s| s.as_ptr())
}
/// A GameCube control's name (`melee_pad_target_e`), static.
#[no_mangle]
pub extern "C" fn melee_pad_target_name(target: u32) -> *const c_char {
    labels()
        .targets
        .get(target as usize)
        .map_or(std::ptr::null(), |s| s.as_ptr())
}

/// # Safety
/// `app` is null or live; `name` and `category` are NUL-terminated or null.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_connect(
    app: *mut Handle,
    handle: u32,
    name: *const c_char,
    category: *const c_char,
    vendor: u16,
    product: u16,
) {
    let h = handle!(app);
    let name = unsafe { string(name) }.unwrap_or_default();
    let category = unsafe { string(category) }.unwrap_or_default();
    h.app.pad_connect(handle, &name, &category, vendor, product);
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_disconnect(app: *mut Handle, handle: u32) {
    handle!(app).app.pad_disconnect(handle);
}
/// # Safety
/// `app` is null or live; `raw` is null or readable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_update(app: *mut Handle, handle: u32, raw: *const RawPadC, now_ms: f64) -> u32 {
    let h = handle!(app);
    let Some(raw) = (unsafe { raw.as_ref() }) else {
        return 0;
    };
    let raw = RawPad {
        buttons: raw.buttons,
        axes: raw.axes,
        direct: None,
    };
    h.app.pad_update(handle, &raw, now_ms).0
}
/// # Safety
/// `app` is null or live; `report` is null or `len` readable bytes;
/// `events` is null or four writable words.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_adapter_report(
    app: *mut Handle,
    base: u32,
    report: *const u8,
    len: usize,
    now_ms: f64,
    events: *mut u32,
) -> u32 {
    let h = handle!(app);
    if report.is_null() {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(report, len) };
    let ports = h.app.pad_adapter_report(base, bytes, now_ms);
    let mut connected = 0;
    for (index, port) in ports.iter().enumerate() {
        if port.is_some() {
            connected |= 1 << index;
        }
        if !events.is_null() {
            unsafe { *events.add(index) = port.map_or(0, |e| e.0) };
        }
    }
    connected
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_count(app: *const Handle) -> u32 {
    unsafe { app.as_ref() }.map_or(0, |h| h.app.pads().handles().len() as u32)
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_handle(app: *const Handle, index: u32) -> u32 {
    unsafe { app.as_ref() }
        .and_then(|h| h.app.pads().handles().get(index as usize).copied())
        .unwrap_or(u32::MAX)
}
/// # Safety
/// `app` is null or live; `out` is null or writable.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_info(app: *const Handle, handle: u32, out: *mut PadInfoC) -> bool {
    let (Some(h), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Some(info) = h.app.pads().info(handle) else {
        return false;
    };
    *out = PadInfoC {
        family: family_index(info.family),
        name: super::fill(&info.name),
        port: info.port.map_or(-1, |p| p as i32),
        listening: info.listening.map_or(-1, |t| t.index() as i32),
        bindings: info.settings.bindings.map(|t| t.index() as u8),
        deadzone: info.settings.deadzone,
        trigger_click: info.settings.trigger_click,
        buttons: info.state.buttons,
        stick: info.state.stick,
        cstick: info.state.cstick,
        triggers: info.state.triggers,
    };
    true
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_bind(app: *mut Handle, handle: u32, input: u32, target: u32) {
    let h = handle!(app);
    let (Some(input), Some(target)) = (Input::from_index(input as usize), Target::from_index(target)) else {
        return;
    };
    if let Some(mut settings) = h.app.pads().settings(handle) {
        settings.bind(input, target);
        h.app.pads_mut().set_settings(handle, settings);
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_swap_ab(app: *mut Handle, handle: u32) {
    let h = handle!(app);
    if let Some(mut settings) = h.app.pads().settings(handle) {
        settings.swap_ab();
        h.app.pads_mut().set_settings(handle, settings);
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_set_deadzone(app: *mut Handle, handle: u32, deadzone: f32) {
    let h = handle!(app);
    if let Some(mut settings) = h.app.pads().settings(handle) {
        settings.set_deadzone(deadzone);
        h.app.pads_mut().set_settings(handle, settings);
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_set_trigger_click(app: *mut Handle, handle: u32, click: f32) {
    let h = handle!(app);
    if let Some(mut settings) = h.app.pads().settings(handle) {
        settings.set_trigger_click(click);
        h.app.pads_mut().set_settings(handle, settings);
    }
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_reset(app: *mut Handle, handle: u32) {
    handle!(app).app.pads_mut().reset(handle);
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_listen(app: *mut Handle, handle: u32, target: i32) {
    let target = u32::try_from(target).ok().and_then(Target::from_index);
    handle!(app).app.pads_mut().listen(handle, target);
}
/// # Safety
/// `app` is null or live.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_assign(app: *mut Handle, handle: u32, port: i32) {
    handle!(app).app.assign_pad(handle, usize::try_from(port).ok());
}
/// # Safety
/// `app` is null or live; `out` is null or `capacity` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_prefs(app: *const Handle, out: *mut c_char, capacity: usize) -> usize {
    let Some(h) = (unsafe { app.as_ref() }) else {
        return 0;
    };
    unsafe { copy_out(&h.app.pads().prefs(), out, capacity) }
}
/// # Safety
/// `app` is null or live; `prefs` is NUL-terminated or null.
#[no_mangle]
pub unsafe extern "C" fn melee_app_pad_load_prefs(app: *mut Handle, prefs: *const c_char) {
    let h = handle!(app);
    if let Ok(text) = unsafe { string(prefs) } {
        h.app.pads_mut().load_prefs(&text);
    }
}

/// # Safety
/// `frames` is null or `4 * count` readable floats (x, y, width, height).
#[no_mangle]
pub unsafe extern "C" fn melee_focus_next(frames: *const f32, count: u32, from: u32, direction: u32) -> i32 {
    let Some(direction) = focus::Direction::from_index(direction) else {
        return -1;
    };
    if frames.is_null() {
        return -1;
    }
    let flat = unsafe { std::slice::from_raw_parts(frames, count as usize * 4) };
    let frames: Vec<focus::Frame> = flat.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
    focus::next(&frames, from as usize, direction).map_or(-1, |i| i as i32)
}
