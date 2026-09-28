//! Stage collision map: floors, walls, ledges, line queries (decomp src/melee/mp)
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
//!
//! The decomp keeps the map in file-scope statics (`groundCollVtx`,
//! `groundCollLine`, `groundCollJoint`, `jointListStart`, `didCheckBounding`,
//! `mpLib_80458868`). Here all of that is owned by one [`CollMap`], built from
//! a [`melee_types::mp::MapCollData`] by [`CollMap::load`] (`mpLibLoad`).
//! Read stage archives with [`desc::read_public_coll_data`] or construct
//! synthetic maps with [`CollMapBuilder`].
//!
//! Every method's doc comment carries the retail address and decomp function
//! name so the assembly can be found. Float math is transcribed in the C's
//! order with `f32`/`f64` promotions preserved; audited multiply-add sites
//! cite their retail instructions (see `FUSION_AUDIT.md` in this crate).
//!
//! Not ported from `mplib.c` (see the crate report): the GX debug drawing
//! (`mpLib_SetupDraw` through `mpLib_DrawZones`), the per-stage terrain sound
//! tables (`mpLib_803BF248`, `mpLib_800569EC`..`mpLib_80056B34`), the JObj
//! tree walk `mpLib_800552B0` (callers pass the matrix to
//! [`CollMap::update_joint_transform`] instead), and the `mpisland.c` hooks,
//! which are recorded as no-ops on [`CollMap::island_update`].

mod builder;
pub mod desc;
mod geom;
mod map;
mod mpcoll;
mod query;
mod walk;

pub use builder::CollMapBuilder;
pub use geom::{line_intersection, line_intersection_h, line_intersection_v, remap_2d};
pub use map::{CollMap, JobjState, JointCallbacks, JointCollisionHandler};
pub use mpcoll::{
    air_flags, clear_floor_skip, coll_prev, copy_coll_data, interpolate_ecb, load_ecb,
    load_ecb_box, load_ecb_fixed, load_ecb_jobj, load_ecb_with_flags, mark_ecb_clear,
    sanitize_desired_ecb, set_ecb_angle, set_ecb_source_fixed, set_ecb_source_jobj, set_facing_dir,
    set_ledge_snap, set_position, squeeze_horizontal, squeeze_vertical, terrain_effects,
    terrain_footstep, terrain_speed_scale, update_floor_skip, BoneLookup, DynamicAttrHook,
    TerrainEffects, TerrainFootstep, TERRAIN_SPEED_SCALE,
};
pub use query::{FloorWalk, LedgeHit, LineFilter, LineHit, SurfaceProbe};

#[cfg(test)]
mod tests;
