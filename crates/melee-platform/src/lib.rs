//! Shared native application session and renderer. Native shells provide window
//! surfaces and logical input; all application behavior lives in this crate.
mod ffi;
pub mod renderer;
pub mod session;
pub mod surface;

mod camera;
pub mod material;
