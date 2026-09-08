//! Bit-exact float primitives for porting Melee off the GameCube.
//!
//! Everything in `melee-core` and `hsd-core` must do its arithmetic through
//! this crate rather than through `std` or `libm`. The retail game was
//! compiled by Metrowerks CodeWarrior for the Gekko CPU, and three things
//! about that toolchain leak into observable game state:
//!
//! 1. **Fused multiply-add.** The compiler emitted `fmadds`/`fmsubs`/`fnmsubs`
//!    wherever the C had `a * b + c` shapes. A fused op rounds once; separate
//!    mul and add round twice. The C source does *not* show where fusion
//!    happened. The retail assembly does. See [`fma`].
//! 2. **Estimate instructions.** `frsqrte` and `fres` return hardware
//!    approximations from lookup tables, not IEEE results. MSL's `sqrtf` is
//!    built on `frsqrte` plus Newton steps. See [`estimate`] and [`msl`].
//! 3. **MSL's own libm.** `sinf`, `cosf`, `tanf`, `logf` are Metrowerks
//!    polynomial implementations, not the platform libm. See [`msl`].
//!
//! The decomp sources these are ported from live in the `melee-decomp`
//! submodule under `src/MSL/` and `src/sysdolphin/baselib/random.c`.

pub mod estimate;
pub mod fma;
pub mod msl;
pub mod rng;

pub use rng::HsdRng;
