//! HSD particle generators and particles (`sysdolphin/baselib/generator.c`,
//! `particle.c`), ported for simulation fidelity only: every idle Melee tick
//! draws RNG from these (one draw per generator update plus six per live
//! particle), so `rng.seed` parity needs their lifetime and emission logic.
//! Rendering (`psdisp.c`) is out of scope.
