//! libmelee: the application core every host shares. It owns the disc,
//! the menu flow ([`app::App`]), the match session and the renderer; native
//! hosts (the macOS app through the C API in `include/melee_platform.h`, the
//! web page through `melee-web`) draw menus and forward user intent.
pub mod app;
pub mod art;
pub mod catalog;
pub mod disc;
#[cfg(not(target_arch = "wasm32"))]
mod ffi;
mod lighting;
pub mod preview;
pub mod renderer;
pub mod session;
pub mod surface;

mod camera;
pub mod material;

mod sprites;

mod srgb;

/// The message of a caught panic.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "panicked with a non-string payload".into())
}
