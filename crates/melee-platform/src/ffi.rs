//! Small C adapter around the shared Rust session. Calls for one handle must be
//! serialized by its host. No Rust-owned references or collections cross the ABI.
use crate::{
    session::{Action, Session},
    surface::WindowRenderer,
};
use std::{
    ffi::{c_char, CStr},
    panic::{catch_unwind, AssertUnwindSafe},
    time::Instant,
};
pub struct Handle {
    window: Option<WindowRenderer>,
    session: Session,
    last: Instant,
    error: String,
}
#[repr(C)]
pub struct Status {
    tick: u64,
    damage: [f32; 2],
    position: [[f32; 2]; 2],
    stocks: [u8; 2],
    running: u8,
    paused: u8,
    needs_frame: u8,
}
unsafe fn message(out: *mut c_char, capacity: usize, text: &str) {
    if !out.is_null() && capacity > 0 {
        let bytes = text.as_bytes();
        let n = bytes.len().min(capacity - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast(), n);
            *out.add(n) = 0;
        }
    }
}
fn perform(handle: &mut Handle, action: impl FnOnce(&mut Handle) -> Result<(), String>) -> bool {
    match catch_unwind(AssertUnwindSafe(|| action(handle))) {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            handle.error = e;
            false
        }
        Err(_) => {
            handle.error = "native application operation panicked".into();
            false
        }
    }
}
/// # Safety
/// `directory` must be a terminated UTF-8 C string. `out` must be writable for
/// `capacity` bytes, or null. The returned handle must be destroyed exactly once.
#[no_mangle]
pub unsafe extern "C" fn melee_session_create(
    directory: *const c_char,
    out: *mut c_char,
    capacity: usize,
) -> *mut Handle {
    let result = catch_unwind(|| {
        if directory.is_null() {
            return Err("missing asset directory".into());
        }
        let path = unsafe { CStr::from_ptr(directory) }
            .to_str()
            .map_err(|e| e.to_string())?;
        Session::new(path)
    });
    match result {
        Ok(Ok(session)) => Box::into_raw(Box::new(Handle {
            window: None,
            session,
            last: Instant::now(),
            error: String::new(),
        })),
        other => {
            let text = match other {
                Ok(Err(e)) => e,
                _ => "application initialization panicked".into(),
            };
            unsafe { message(out, capacity, &text) };
            std::ptr::null_mut()
        }
    }
}
/// # Safety
/// Handle is null or a live handle from create, exclusively owned by this call.
/// Any attached native layer must remain alive until this function returns.
#[no_mangle]
pub unsafe extern "C" fn melee_session_destroy(handle: *mut Handle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}
/// # Safety
/// Handle must be live and exclusively borrowed for this call.
#[no_mangle]
pub unsafe extern "C" fn melee_session_action(
    handle: *mut Handle,
    player: u32,
    action: u32,
    down: bool,
) {
    if let (Some(h), Ok(action)) = (unsafe { handle.as_mut() }, Action::try_from(action)) {
        h.session.set_action(player as usize, action, down);
    }
}
/// # Safety
/// Handle must be live and exclusively borrowed for this call.
#[no_mangle]
pub unsafe extern "C" fn melee_session_pause(handle: *mut Handle, paused: bool) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.session.pause(paused);
        h.last = Instant::now();
    }
}
/// # Safety
/// Handle must be live and exclusively borrowed for this call.
#[no_mangle]
pub unsafe extern "C" fn melee_session_reset(handle: *mut Handle) -> bool {
    unsafe { handle.as_mut() }.is_some_and(|h| {
        perform(h, |h| {
            h.last = Instant::now();
            h.session.reset()
        })
    })
}
/// # Safety
/// Handle must be live and exclusively borrowed. Native surface remains alive.
#[no_mangle]
pub unsafe extern "C" fn melee_session_frame(handle: *mut Handle, width: u32, height: u32) -> bool {
    unsafe { handle.as_mut() }.is_some_and(|h| {
        perform(h, |h| {
            let now = Instant::now();
            let elapsed = now.duration_since(h.last);
            h.last = now;
            h.session.advance(elapsed)?;
            if width > 0 && height > 0 {
                if let Some(window) = &mut h.window {
                    window.resize([width, height]);
                    window.draw(&h.session)?;
                }
            }
            Ok(())
        })
    })
}
/// # Safety
/// Handle must be live. Output must be writable for one Status and not alias it.
#[no_mangle]
pub unsafe extern "C" fn melee_session_status(handle: *const Handle, out: *mut Status) -> bool {
    let (Some(h), Some(out)) = (unsafe { handle.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Ok(view) = h.session.game().observe() else {
        return false;
    };
    out.tick = view.tick.0;
    out.running = u8::from(view.status.is_running());
    out.paused = u8::from(h.session.is_paused());
    out.needs_frame = u8::from(h.session.needs_frame());
    for (i, f) in view.fighters().enumerate() {
        out.damage[i] = f.percent();
        let position = f.position();
        out.position[i] = [position.x, position.y];
        out.stocks[i] = f.stocks();
    }
    true
}
/// # Safety
/// Handle must be live. Output must be writable for capacity bytes or null.
#[no_mangle]
pub unsafe extern "C" fn melee_session_error(
    handle: *const Handle,
    out: *mut c_char,
    capacity: usize,
) {
    if let Some(h) = unsafe { handle.as_ref() } {
        unsafe { message(out, capacity, &h.error) };
    }
}
/// # Safety
/// Handle must be live and exclusively borrowed. Layer must be a CAMetalLayer
/// owned by the main thread, and must outlive the handle or its replacement.
#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn melee_session_attach_macos(
    handle: *mut Handle,
    layer: *mut std::ffi::c_void,
    width: u32,
    height: u32,
) -> bool {
    if layer.is_null() {
        return false;
    }
    unsafe { handle.as_mut() }.is_some_and(|h| {
        perform(h, |h| {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::METAL,
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let surface = unsafe {
                instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))
            }
            .map_err(|e| e.to_string())?;
            h.window = Some(pollster::block_on(WindowRenderer::new(
                &instance,
                surface,
                &h.session,
                [width, height],
            ))?);
            h.last = Instant::now();
            Ok(())
        })
    })
}

/// # Safety
/// Handle must be live and exclusively borrowed for this call.
#[no_mangle]
pub unsafe extern "C" fn melee_session_focus(handle: *mut Handle, focused: bool) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.session.set_focused(focused);
        h.last = Instant::now();
    }
}
/// # Safety
/// Handle must be live and exclusively borrowed for this call.
#[no_mangle]
pub unsafe extern "C" fn melee_session_toggle_pause(handle: *mut Handle) {
    if let Some(h) = unsafe { handle.as_mut() } {
        h.session.toggle_pause();
        h.last = Instant::now();
    }
}
