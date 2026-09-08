//! Stage collision map data types (decomp `src/melee/mp/types.h`,
//! `src/melee/mp/forward.h`, and the `CollData` family from
//! `src/melee/lb/types.h`).
//!
//! Plain data only. The C structs hold raw pointers into the stage archive
//! (`MapLine*`, `MapJoint*`, `HSD_JObj*`) and into the fighter (`CollData*`
//! callbacks); the Rust versions replace every pointer with an index into the
//! owning `melee_mp::CollMap` and leave callbacks to the logic crate. Field
//! names follow the header, including the `xNN` placeholders for fields the
//! decomp has not named, so the two can be diffed side by side.

use hsd_types::{Vec2, Vec3};

// ---------------------------------------------------------------------------
// Line kind bits and flag words (forward.h)
// ---------------------------------------------------------------------------

/// `CollLineKind` (`forward.h:46`). Bit flags stored in the low nibble of
/// [`CollLine::flags`] and of [`MapLine::hi_flags`]. Exactly one bit is set
/// for a static line; dynamic lines are reclassified every frame by
/// `mpJointUpdateDynamics`.
pub mod line_kind {
    /// `CollLine_Floor = 1 << 0`.
    pub const FLOOR: u32 = 1 << 0;
    /// `CollLine_Ceiling = 1 << 1`.
    pub const CEILING: u32 = 1 << 1;
    /// `CollLine_RightWall = 1 << 2`: a wall whose outward normal points
    /// right, i.e. the wall a fighter touches with their *left* side.
    pub const RIGHT_WALL: u32 = 1 << 2;
    /// `CollLine_LeftWall = 1 << 3`: a wall whose outward normal points left.
    pub const LEFT_WALL: u32 = 1 << 3;
    /// `LINE_FLAG_KIND` (`forward.h:53`): mask over the four kind bits.
    pub const KIND_MASK: u32 = 0xF;
}

/// `LINE_FLAG_*` (`forward.h:53-58`). These live in [`CollLine::flags`]
/// (the runtime word) and, for the low sixteen bits, in
/// [`MapLine::hi_flags`] / [`MapLine::lo_flags`] as the header comments
/// describe.
pub mod line_flag {
    /// `LINE_FLAG_EMPTY (1 << 7)`: set by `mpPruneEmptyLines` on a line
    /// whose two vertices coincide; the line is unlinked and skipped by
    /// every query.
    pub const EMPTY: u32 = 1 << 7;
    /// `LINE_FLAG_PLATFORM (1 << 8)`: a drop-through platform. Lives in
    /// `lo_flags` on disc and is mirrored into `CollLine::flags`.
    pub const PLATFORM: u32 = 1 << 8;
    /// `LINE_FLAG_LEDGE (1 << 9)`: this floor line's outer vertex is a
    /// grabbable ledge (`lo_flags`).
    pub const LEDGE: u32 = 1 << 9;
    /// Bit 10 of `lo_flags`: a dynamic line that becomes a platform when it
    /// is classified as a floor (`mpJointUpdateDynamics`, `mplib.c:4828`).
    /// The decomp writes the literal `0x400`.
    pub const DYNAMIC_PLATFORM: u32 = 1 << 10;
    /// `LINE_FLAG_ENABLED (1 << 16)`: cleared when the owning joint is
    /// removed from the joint list (`mpLib_80057BC0`).
    pub const ENABLED: u32 = 1 << 16;
    /// `LINE_FLAG_HIDDEN (1 << 18)`: set by `mpJointHide` while the joint's
    /// JObj is hidden.
    pub const HIDDEN: u32 = 1 << 18;
    /// Mask over the material byte of `lo_flags`. `mpLib_80054D68` writes
    /// exactly these eight bits; `mpLib_800569EC` indexes the terrain table
    /// with `(u8) flags`.
    pub const MATERIAL_MASK: u32 = 0xFF;
}

/// `CollJointFlags` (`forward.h:70`). Bits of [`CollJoint::flags`].
pub mod joint_flag {
    /// `CollJoint_B8 = 1 << 8`: the joint's JObj matrix has been applied
    /// this frame (`mpLib_80055E9C`); remap queries use the previous vertex
    /// positions.
    pub const B8: u32 = 1 << 8;
    /// `CollJoint_B9 = 1 << 9`: the JObj matrix has a rotation component.
    pub const B9: u32 = 1 << 9;
    /// `CollJoint_B10 = 1 << 10`: bounding box is not recomputed from the
    /// JObj and the joint is never marked too far (`mpJointSetB10`).
    pub const B10: u32 = 1 << 10;
    /// `CollJoint_B11 = 1 << 11`: island update disabled (`mpLib_80058044`).
    pub const B11: u32 = 1 << 11;
    /// `CollJoint_TooFar = 1 << 12`: transient, set by `mpBoundingCheck`
    /// for joints outside the query box and cleared by `mpUncheckBounding`.
    pub const TOO_FAR: u32 = 1 << 12;
    /// `CollJoint_Enabled = 1 << 16`: the joint is in the active list.
    pub const ENABLED: u32 = 1 << 16;
    /// `CollJoint_Hidden = 1 << 18`.
    pub const HIDDEN: u32 = 1 << 18;
    /// The three "remap" bits tested together by `mpCheckFloorRemap` and
    /// friends.
    pub const REMAP_MASK: u32 = B8 | B9 | B10;
}

/// `Collide_*` (`forward.h:81-108`): bits of [`CollData::env_flags`] set by
/// `mpcoll.c` as the ECB is resolved against the map.
pub mod collide {
    pub const LEFT_WALL_PUSH: u32 = 0x1;
    pub const LEFT_WALL_HUG: u32 = 0x20;
    pub const LEFT_WALL_MASK: u32 = 0x3F;
    pub const RIGHT_WALL_PUSH: u32 = 0x40;
    pub const RIGHT_WALL_HUG: u32 = 0x800;
    pub const RIGHT_WALL_MASK: u32 = 0xFC0;
    pub const WALL_MASK: u32 = LEFT_WALL_MASK | RIGHT_WALL_MASK;
    pub const CEILING_PUSH: u32 = 0x2000;
    pub const CEILING_HUG: u32 = 0x4000;
    pub const CEILING_MASK: u32 = CEILING_PUSH | CEILING_HUG;
    pub const FLOOR_PUSH: u32 = 0x8000;
    pub const FLOOR_HUG: u32 = 0x10000;
    pub const FLOOR_MASK: u32 = FLOOR_PUSH | FLOOR_HUG;
    pub const LEFT_EDGE: u32 = 0x100000;
    pub const RIGHT_EDGE: u32 = 0x200000;
    pub const EDGE: u32 = 0x800000;
    pub const LEFT_LEDGE_GRAB: u32 = 0x1000000;
    pub const RIGHT_LEDGE_GRAB: u32 = 0x2000000;
    pub const LEDGE_GRAB_MASK: u32 = LEFT_LEDGE_GRAB | RIGHT_LEDGE_GRAB;
    pub const LEFT_LEDGE_SLIP: u32 = 0x10000000;
    pub const RIGHT_LEDGE_SLIP: u32 = 0x20000000;
}

/// `CollDataX130Flags` (`forward.h:65`): bits of [`CollData::x130_flags`].
pub mod coll_data_x130 {
    /// `CollData_X130_Locked = 1 << 4`.
    pub const LOCKED: u32 = 1 << 4;
    /// `CollData_X130_Clear = 1 << 5`.
    pub const CLEAR: u32 = 1 << 5;
}

/// `MPCOLL_WALLID_MAX` (`forward.h:60`).
pub const MPCOLL_WALLID_MAX: i32 = 9;

/// Sentinel for "no line" / "no joint" / "no vertex", the C `-1`.
pub const NO_ID: i32 = -1;

c_enum! {
    /// `mp_Terrain` (`forward.h:20`): the material byte of a line's
    /// `lo_flags`, used for footstep sounds and dust effects.
    pub enum MpTerrain: i32 {
        /// `mp_Terrain_Basic`
        Basic = 0,
        /// `mp_Terrain_Rock`
        Rock = 1,
        /// `mp_Terrain_Grass`
        Grass = 2,
        /// `mp_Terrain_Dirt`
        Dirt = 3,
        /// `mp_Terrain_Wood`
        Wood = 4,
        /// `mp_Terrain_LightMetal`
        LightMetal = 5,
        /// `mp_Terrain_HeavyMetal`
        HeavyMetal = 6,
        /// `mp_Terrain_Paper`
        Paper = 7,
        /// `mp_Terrain_Goop`
        Goop = 8,
        /// `mp_Terrain_Birdo` (GrI2)
        Birdo = 9,
        /// `mp_Terrain_Water`
        Water = 10,
        /// `mp_Terrain_Unk11` (GrTe)
        Unk11 = 11,
        /// `mp_Terrain_UFO`
        Ufo = 12,
        /// `mp_Terrain_Turtle`
        Turtle = 13,
        /// `mp_Terrain_Snow`
        Snow = 14,
        /// `mp_Terrain_Ice`
        Ice = 15,
        /// `mp_Terrain_GnW`
        GnW = 16,
        /// `mp_Terrain_Unk17` (GrTe)
        Unk17 = 17,
        /// `mp_Terrain_Checkered`
        Checkered = 18,
        /// `mp_Terrain_Unk19`
        Unk19 = 19,
    }
}

c_enum! {
    /// `mpLib_GroundEnum` (`forward.h:43`): the `ground_kind` argument of a
    /// joint collision callback. `mpLib_8005811C` passes the literal `3`,
    /// which the header does not name; see [`MpLibGroundEnum::LEDGE`].
    pub enum MpLibGroundEnum: i32 {
        /// `mpLib_GroundEnum_Unk0`
        Unk0 = 0,
        /// `mpLib_GroundEnum_Unk1`
        Unk1 = 1,
        /// `mpLib_GroundEnum_Unk2`
        Unk2 = 2,
    }
}

impl MpLibGroundEnum {
    /// The unnamed `3` passed by `mpLib_8005811C` when a ledge is grabbed.
    pub const LEDGE: i32 = 3;
}

c_enum! {
    /// `ECBSourceKind` (`lb/forward.h:106`).
    pub enum EcbSourceKind: i32 {
        /// `ECBSource_None`
        None = 0,
        /// `ECBSource_JObj`
        JObj = 1,
        /// `ECBSource_Fixed`
        Fixed = 2,
    }
}

// ---------------------------------------------------------------------------
// On-disc map data (types.h)
// ---------------------------------------------------------------------------

/// `MapLine` (`types.h:57`, 16 bytes). One collision segment from `v0` to
/// `v1`. For a floor, `v0` is the left end and `v1` the right; for a left
/// wall `v0` is the bottom, for a right wall `v0` is the top (see the
/// walkers in `mplib.c:1097-1395`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapLine {
    /// `+0 u16 v0_idx`
    pub v0_idx: u16,
    /// `+2 u16 v1_idx`
    pub v1_idx: u16,
    /// `+4 s16 prev_id0`: previous line in the same chain, or -1.
    pub prev_id0: i16,
    /// `+6 s16 next_id0`
    pub next_id0: i16,
    /// `+8 s16 prev_id1`: alternate previous line, preferred by
    /// `mpLineGetPrev` when its end lies within 2 units of ours. Rewritten
    /// at runtime by `mpLib_800581DC`.
    pub prev_id1: i16,
    /// `+A s16 next_id1`
    pub next_id1: i16,
    /// `+C u16 hi_flags`: line kind bits ([`line_kind`]) plus
    /// [`line_flag::EMPTY`]. Copied into `CollLine::flags` at load.
    pub hi_flags: u16,
    /// `+E u16 lo_flags`: material byte, [`line_flag::PLATFORM`],
    /// [`line_flag::LEDGE`], [`line_flag::DYNAMIC_PLATFORM`]. This is what
    /// `mpLineGetFlags` and every `flags_out` return.
    pub lo_flags: u16,
}

/// `MapJoint` (`types.h:87`, 40 bytes). One collision group: index ranges
/// into the line table for each kind, an axis-aligned bound in map units,
/// and the vertex range the group owns.
///
/// `mpLib_800581DC` treats the five `(start, count)` pairs as an array
/// `struct pair[5]`; [`MapJoint::section`] exposes that view.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MapJoint {
    /// `+0 s16 floor_start`
    pub floor_start: i16,
    /// `+2 s16 floor_count`
    pub floor_count: i16,
    /// `+4 s16 ceiling_start`
    pub ceiling_start: i16,
    /// `+6 s16 ceiling_count`
    pub ceiling_count: i16,
    /// `+8 s16 right_wall_start`
    pub right_wall_start: i16,
    /// `+A s16 right_wall_count`
    pub right_wall_count: i16,
    /// `+C s16 left_wall_start`
    pub left_wall_start: i16,
    /// `+E s16 left_wall_count`
    pub left_wall_count: i16,
    /// `+10 s16 dynamic_start`
    pub dynamic_start: i16,
    /// `+12 s16 dynamic_count`
    pub dynamic_count: i16,
    /// `+14 float left_bound`
    pub left_bound: f32,
    /// `+18 float bottom_bound`
    pub bottom_bound: f32,
    /// `+1C float right_bound`
    pub right_bound: f32,
    /// `+20 float top_bound`
    pub top_bound: f32,
    /// `+24 s16 vtx_start`
    pub vtx_start: i16,
    /// `+26 s16 vtx_count`
    pub vtx_count: i16,
}

/// Index of a `(start, count)` pair in [`MapJoint`] / [`MapCollData`], in
/// header order. `mpLib_800581DC` iterates `0..5` over exactly this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum LineSection {
    Floor = 0,
    Ceiling = 1,
    RightWall = 2,
    LeftWall = 3,
    Dynamic = 4,
}

impl LineSection {
    /// All five sections in header order.
    pub const ALL: [LineSection; 5] = [
        LineSection::Floor,
        LineSection::Ceiling,
        LineSection::RightWall,
        LineSection::LeftWall,
        LineSection::Dynamic,
    ];
}

impl MapJoint {
    /// `(start, count)` of one section, the `struct pair` view used by
    /// `mpLib_800581DC` (`mplib.c:5670`).
    pub fn section(&self, s: LineSection) -> (i16, i16) {
        match s {
            LineSection::Floor => (self.floor_start, self.floor_count),
            LineSection::Ceiling => (self.ceiling_start, self.ceiling_count),
            LineSection::RightWall => (self.right_wall_start, self.right_wall_count),
            LineSection::LeftWall => (self.left_wall_start, self.left_wall_count),
            LineSection::Dynamic => (self.dynamic_start, self.dynamic_count),
        }
    }
}

/// `MapCollData` (`types.h:129`): the `coll_data` node of a stage archive.
/// Pointers become owned vectors; `x2C` is unnamed in the decomp.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapCollData {
    /// `+0 Vec2* verts`, `+4 int vert_count`
    pub verts: Vec<Vec2>,
    /// `+8 MapLine* lines`, `+C int line_count`
    pub lines: Vec<MapLine>,
    /// `+10 s16 floor_start`
    pub floor_start: i16,
    /// `+12 s16 floor_count`
    pub floor_count: i16,
    /// `+14 s16 ceiling_start`
    pub ceiling_start: i16,
    /// `+16 s16 ceiling_count`
    pub ceiling_count: i16,
    /// `+18 s16 right_wall_start`
    pub right_wall_start: i16,
    /// `+1A s16 right_wall_count`
    pub right_wall_count: i16,
    /// `+1C s16 left_wall_start`
    pub left_wall_start: i16,
    /// `+1E s16 left_wall_count`
    pub left_wall_count: i16,
    /// `+20 s16 dynamic_start`
    pub dynamic_start: i16,
    /// `+22 s16 dynamic_count`
    pub dynamic_count: i16,
    /// `+24 MapJoint* joints`, `+28 int joint_count`
    pub joints: Vec<MapJoint>,
    /// `+2C int x2C` (inferred by the decomp).
    pub x2c: i32,
}

impl MapCollData {
    /// `(start, count)` of one whole-map section (`mpLibLoad`,
    /// `mplib.c:941-986`).
    pub fn section(&self, s: LineSection) -> (i16, i16) {
        match s {
            LineSection::Floor => (self.floor_start, self.floor_count),
            LineSection::Ceiling => (self.ceiling_start, self.ceiling_count),
            LineSection::RightWall => (self.right_wall_start, self.right_wall_count),
            LineSection::LeftWall => (self.left_wall_start, self.left_wall_count),
            LineSection::Dynamic => (self.dynamic_start, self.dynamic_count),
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime collision tables (types.h)
// ---------------------------------------------------------------------------

/// `CollVtx` (`types.h:76`, 0x18 bytes). A vertex of the runtime map.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollVtx {
    /// `+0 f32 x0`: unscaled x from the archive.
    pub x0: f32,
    /// `+4 f32 x4`: unscaled y from the archive.
    pub x4: f32,
    /// `+8 Vec2 pos`: current world position (archive position times the
    /// stage scale, then moved by the owning joint's JObj each frame).
    pub pos: Vec2,
    /// `+10 float x10`: `pos.x` as of the previous frame.
    pub x10: f32,
    /// `+14 float x14`: `pos.y` as of the previous frame.
    pub x14: f32,
}

/// `CollLine` (`types.h:62`). The runtime word for line `i`; the `MapLine*`
/// it carries in C always points at `coll_data->lines[i]`, so it is elided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollLine {
    /// `+4 u32 flags`: [`line_kind`] bits, [`line_flag::EMPTY`],
    /// [`line_flag::PLATFORM`], [`line_flag::ENABLED`],
    /// [`line_flag::HIDDEN`].
    pub flags: u32,
}

/// `CollJoint` (`types.h:104`, 0x34 bytes) minus its pointers. `next`
/// (the active-list link) becomes the order of `melee_mp::CollMap`'s joint
/// list; `inner` is the `MapJoint` at the same index; `x20` (the JObj) and
/// the two callback slots are held by the logic crate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollJoint {
    /// `+8 u32 flags`: [`joint_flag`] bits.
    pub flags: u32,
    /// `+C s16 xC` (unused by mplib).
    pub xc: i16,
    /// `+E u8 xE : 1`: "dirty" bit read and cleared by `mpLib_80058614_Floor`
    /// to decide whether the floor bound needs recomputing.
    pub xe: bool,
    /// `+10 Vec2 bounding_min`: world-space box, scaled from
    /// `MapJoint::{left,bottom}_bound` at load and padded by 30 units after
    /// a JObj update.
    pub bounding_min: Vec2,
    /// `+18 Vec2 bounding_max`
    pub bounding_max: Vec2,
}

/// `mpCollisionBox` (`types.h:141`): `mpLib_80458868[2]`. Slot 0 is the
/// bound of every vertex at load; slot 1 the bound of every enabled floor
/// line, refreshed by `mpLib_80058614_Floor`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MpCollisionBox {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

// ---------------------------------------------------------------------------
// Fighter/item side: CollData and the ECB (lb/types.h)
// ---------------------------------------------------------------------------

/// `ECBFlagStruct` (`lb/types.h:154`): one byte of bitfields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EcbFlags {
    /// `u8 b0 : 1`
    pub b0: bool,
    /// `u8 b1234 : 4` (used as `s32` by callers).
    pub b1234: u8,
    /// `u8 b5 : 1`
    pub b5: bool,
    /// `u8 b6 : 1`
    pub b6: bool,
    /// `u8 b7 : 1`
    pub b7: bool,
}

/// `SurfaceData` (`lb/types.h:162`): the line a fighter is touching.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceData {
    /// `int index`: line id, or -1.
    pub index: i32,
    /// `u32 flags`: that line's `lo_flags`.
    pub flags: u32,
    /// `Vec3 normal`
    pub normal: Vec3,
}

/// `itECB` (`lb/types.h:168`): the item ECB, four extents.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItEcb {
    pub top: f32,
    pub bottom: f32,
    pub right: f32,
    pub left: f32,
}

/// `ftECB` (`lb/types.h:175`): the fighter ECB diamond, four points
/// relative to the fighter position.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FtEcb {
    pub top: Vec2,
    pub bottom: Vec2,
    pub right: Vec2,
    pub left: Vec2,
}

/// `ftCollisionBox` (`ft/kinds/ftCommon/types.h:16`): a fixed ECB some
/// fighter states hand to `mpColl_80042C58` in place of the bone-derived
/// one. Lives here because `mpcoll.c` is its consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FtCollisionBox {
    /// `+0 float top`
    pub top: f32,
    /// `+4 float bottom`
    pub bottom: f32,
    /// `+8 Vec2 left`
    pub left: Vec2,
    /// `+10 Vec2 right`
    pub right: Vec2,
}

/// `ECBSource` (`lb/types.h:182`, at `fp+7F4`). The C union is split into
/// [`EcbSourceParams`]; the JObj pointers become slot indices owned by the
/// fighter's skeleton.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EcbSource {
    /// `fp+7F4 ECBSourceKind kind`
    pub kind: i32,
    /// `fp+7F8` union
    pub params: EcbSourceParams,
    /// `fp+814 float x124`
    pub x124: f32,
    /// `fp+818 float x128`
    pub x128: f32,
    /// `fp+81C float x12C`
    pub x12c: f32,
}

/// The union body of [`EcbSource`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EcbSourceParams {
    /// `ECBSource_JObj`: `fp+7F8 HSD_JObj* x108_joint`, `fp+7FC HSD_JObj*
    /// x10C_joint[6]`. Stored as bone indices into the fighter skeleton;
    /// `None` is a NULL pointer.
    JObj {
        x108_joint: Option<u32>,
        x10c_joint: [Option<u32>; 6],
    },
    /// `ECBSource_Fixed`: `up, down, front, back, angle` at `fp+7F8..808`.
    Fixed {
        up: f32,
        down: f32,
        front: f32,
        back: f32,
        angle: f32,
    },
}

impl Default for EcbSourceParams {
    fn default() -> Self {
        EcbSourceParams::JObj {
            x108_joint: None,
            x10c_joint: [None; 6],
        }
    }
}

/// `CollData` (`lb/types.h:202`), embedded in `Fighter` at `fp+6F0` and in
/// `Item` at `it+378`. The `x0_gobj` back-pointer is dropped: the owner is
/// implicit in Rust.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CollData {
    /// `fp+6F4 Vec3 cur_pos`
    pub cur_pos: Vec3,
    /// `fp+700 Vec3 prev_pos`: position on the previous step of collision.
    pub prev_pos: Vec3,
    /// `fp+70C Vec3 last_pos`: position before the collision routine
    /// started.
    pub last_pos: Vec3,
    /// `fp+718 Vec3 x28_vec`
    pub x28_vec: Vec3,
    /// `fp+724 ECBFlagStruct x34_flags`
    pub x34_flags: EcbFlags,
    /// `fp+725 ECBFlagStruct x35_flags`
    pub x35_flags: EcbFlags,
    /// `fp+726 s16 facing_dir`
    pub facing_dir: i16,
    /// `fp+728 int x38`
    pub x38: i32,
    /// `fp+72C int floor_skip`: line id ignored by floor queries, or -1.
    pub floor_skip: i32,
    /// `fp+730 int ledge_id_right`
    pub ledge_id_right: i32,
    /// `fp+734 int ledge_id_left`
    pub ledge_id_left: i32,
    /// `fp+738 int joint_id_skip`
    pub joint_id_skip: i32,
    /// `fp+73C int joint_id_only`
    pub joint_id_only: i32,
    /// `fp+740 float x50`
    pub x50: f32,
    /// `fp+744 float ledge_snap_x`
    pub ledge_snap_x: f32,
    /// `fp+748 float ledge_snap_y`
    pub ledge_snap_y: f32,
    /// `fp+74C float ledge_snap_height`
    pub ledge_snap_height: f32,
    /// `fp+750 float lstick_x`
    pub lstick_x: f32,
    /// `fp+754 ftECB x64_ecb`
    pub x64_ecb: FtEcb,
    /// `fp+774 ftECB desired_ecb`
    pub desired_ecb: FtEcb,
    /// `fp+794 ftECB ecb`
    pub ecb: FtEcb,
    /// `fp+7B4 ftECB prev_ecb`: ECB on the previous step of collision.
    pub prev_ecb: FtEcb,
    /// `fp+7D4 ftECB xE4_ecb`
    pub xe4_ecb: FtEcb,
    /// `fp+7F4 ECBSource ecb_source`
    pub ecb_source: EcbSource,
    /// `fp+820 u32 x130_flags`: [`coll_data_x130`] bits.
    pub x130_flags: u32,
    /// `fp+824 s32 env_flags`: [`collide`] bits for this frame.
    pub env_flags: i32,
    /// `fp+828 s32 prev_env_flags`
    pub prev_env_flags: i32,
    /// `fp+82C s32 x13C`
    pub x13c: i32,
    /// `fp+830 Vec3 contact`
    pub contact: Vec3,
    /// `fp+83C SurfaceData floor`
    pub floor: SurfaceData,
    /// `fp+850 SurfaceData left_facing_wall`
    pub left_facing_wall: SurfaceData,
    /// `fp+864 SurfaceData right_facing_wall`
    pub right_facing_wall: SurfaceData,
    /// `fp+878 SurfaceData ceiling`
    pub ceiling: SurfaceData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_values_match_forward_h() {
        assert_eq!(line_kind::FLOOR, 1);
        assert_eq!(line_kind::CEILING, 2);
        assert_eq!(line_kind::RIGHT_WALL, 4);
        assert_eq!(line_kind::LEFT_WALL, 8);
        assert_eq!(line_kind::KIND_MASK, 0xF);
        assert_eq!(line_flag::EMPTY, 0x80);
        assert_eq!(line_flag::PLATFORM, 0x100);
        assert_eq!(line_flag::LEDGE, 0x200);
        assert_eq!(line_flag::DYNAMIC_PLATFORM, 0x400);
        assert_eq!(line_flag::ENABLED, 0x10000);
        assert_eq!(line_flag::HIDDEN, 0x40000);
        assert_eq!(joint_flag::TOO_FAR, 0x1000);
        assert_eq!(joint_flag::ENABLED, 0x10000);
        assert_eq!(joint_flag::HIDDEN, 0x40000);
        assert_eq!(collide::WALL_MASK, 0xFFF);
        assert_eq!(collide::FLOOR_MASK, 0x18000);
        assert_eq!(collide::LEDGE_GRAB_MASK, 0x3000000);
        assert_eq!(coll_data_x130::LOCKED, 0x10);
        assert_eq!(coll_data_x130::CLEAR, 0x20);
    }

    #[test]
    fn terrain_enum() {
        assert_eq!(i32::from(MpTerrain::Basic), 0);
        assert_eq!(i32::from(MpTerrain::Birdo), 9);
        assert_eq!(i32::from(MpTerrain::Unk19), 19);
        assert_eq!(MpTerrain::ALL.len(), 20);
        assert!(MpTerrain::try_from(20).is_err());
        assert_eq!(MpTerrain::try_from(15), Ok(MpTerrain::Ice));
    }

    #[test]
    fn ecb_source_kind() {
        assert_eq!(i32::from(EcbSourceKind::None), 0);
        assert_eq!(i32::from(EcbSourceKind::JObj), 1);
        assert_eq!(i32::from(EcbSourceKind::Fixed), 2);
        assert!(EcbSourceKind::try_from(3).is_err());
        assert_eq!(MpLibGroundEnum::LEDGE, 3);
        assert!(MpLibGroundEnum::try_from(3).is_err());
    }

    #[test]
    fn joint_sections_in_header_order() {
        let j = MapJoint {
            floor_start: 1,
            floor_count: 2,
            ceiling_start: 3,
            ceiling_count: 4,
            right_wall_start: 5,
            right_wall_count: 6,
            left_wall_start: 7,
            left_wall_count: 8,
            dynamic_start: 9,
            dynamic_count: 10,
            ..Default::default()
        };
        let pairs: Vec<_> = LineSection::ALL.iter().map(|&s| j.section(s)).collect();
        assert_eq!(pairs, [(1, 2), (3, 4), (5, 6), (7, 8), (9, 10)]);
        assert_eq!(LineSection::Dynamic as usize, 4);
    }
}
