//! `FigaTree` and `FigaTrack` (melee/lb/lbanim.h): the compact fighter
//! animation format used by `Pl*AJ.dat` motion files.
//!
//! ```text
//! FigaTree, 0x14 bytes                  FigaTrack, 0x0C bytes
//!   +0x00 s32        type                 +0x00 u16 length      bytes in ad_head
//!   +0x04 u32        flags                +0x02 u16 startframe
//!   +0x08 f32        frames               +0x04 u8  obj_type    HSD_A_J_* track id
//!   +0x0C s8*        nodes                +0x05 u8  frac_value
//!   +0x10 FigaTrack* tracks               +0x06 u8  frac_slope
//!                                         +0x07 u8  (pad)
//!                                         +0x08 u8* ad_head     key-frame byte stream
//! ```
//!
//! Semantics, from `lbAnim_8001E6D8` / `lbAnim_InitFrames` (lbanim.c) and
//! `ftAnim_SetAnimation`-family callers in ftanim.c:
//!
//! - `nodes` is an `s8` array terminated by `-1`. Entry `i` is the number
//!   of consecutive `tracks` belonging to the `i`-th animated joint, in
//!   skeleton order. `0` is legal (the joint gets no `AObj`). Which
//!   skeleton joints the entries map to is decided by the fighter's part
//!   table (`ftParts_8007506C`, `Fighter.parts[i].flags_b*`) on the Melee
//!   side, not by this struct; this reader only groups tracks per node.
//! - `tracks` is one flat array of `sum(nodes)` `FigaTrack`s.
//! - `frames` becomes the `AObj` end frame and `flags` its `AOBJ_*` flags.
//! - `type & 1` sets `JOBJ_CLASSICAL_SCALE` on the joint.
//! - A `FigaTrack` maps one-to-one onto the `HSD_FObj` fields
//!   `lbAnim_InitFrames` fills: `startframe` (widened from `u16` to the
//!   `s16` `HSD_FObj.startframe`), `obj_type`, `frac_value`, `frac_slope`,
//!   `ad_head`, `length`. The stream encoding is the same as
//!   `HSD_FObjDesc.ad`.
//!
//! `fn_8001E60C` (used by `lbAnim_8001E7E8`) skips tracks whose `obj_type`
//! is `HSD_A_J_TRAX` (5), `HSD_A_J_TRAY` (6) or `HSD_A_J_TRAZ` (7) when
//! building the `FObj` list. That filter is a load-time policy and is left
//! to the caller; every track is read here.

use super::{Ctx, DescError, Result};
use crate::archive::Archive;
use crate::reader::add_offset;

/// `sizeof(FigaTree)`.
pub const FIGATREE_SIZE: u32 = 0x14;
/// `sizeof(FigaTrack)`.
pub const FIGATRACK_SIZE: u32 = 0x0C;

mod tree_off {
    pub const TYPE: u32 = 0x00;
    pub const FLAGS: u32 = 0x04;
    pub const FRAMES: u32 = 0x08;
    pub const NODES: u32 = 0x0C;
    pub const TRACKS: u32 = 0x10;
}
const _: () = assert!(tree_off::TRACKS + 4 == FIGATREE_SIZE);

mod track_off {
    pub const LENGTH: u32 = 0x00;
    pub const STARTFRAME: u32 = 0x02;
    pub const OBJ_TYPE: u32 = 0x04;
    pub const FRAC_VALUE: u32 = 0x05;
    pub const FRAC_SLOPE: u32 = 0x06;
    pub const PAD: u32 = 0x07;
    pub const AD_HEAD: u32 = 0x08;
}
const _: () = assert!(track_off::PAD + 1 == track_off::AD_HEAD);
const _: () = assert!(track_off::AD_HEAD + 4 == FIGATRACK_SIZE);

/// `FigaTrack` (lbanim.h): one animation track of a fighter motion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FigaTrack {
    /// Data offset this struct was read from.
    pub offset: u32,
    /// Byte length of the key-frame stream.
    pub length: u16,
    pub startframe: u16,
    /// `HSD_A_J_*` track id.
    pub obj_type: u8,
    pub frac_value: u8,
    pub frac_slope: u8,
    /// The padding byte at `+0x07`, kept so a round trip is exact.
    pub pad: u8,
    /// Data offset of the key-frame stream, if any.
    pub ad_offset: Option<u32>,
    /// The `length` bytes of key-frame data at `ad_offset`. Empty when
    /// `ad_head` is null.
    pub ad: Vec<u8>,
}

impl FigaTrack {
    /// Read the `FigaTrack` at data offset `off`.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        Self::read_in(&Ctx::new(archive), off)
    }

    fn read_in(ctx: &Ctx<'_>, off: u32) -> Result<Self> {
        let r = ctx.reader();
        let length = r.u16(add_offset(off, track_off::LENGTH)?)?;
        let ad_offset = ctx.link("FigaTrack.ad_head", off, track_off::AD_HEAD)?;
        let ad = match ad_offset {
            Some(a) => ctx.stream("FigaTrack.ad_head", a, u32::from(length))?,
            None => Vec::new(),
        };
        Ok(Self {
            offset: off,
            length,
            startframe: r.u16(add_offset(off, track_off::STARTFRAME)?)?,
            obj_type: r.u8(add_offset(off, track_off::OBJ_TYPE)?)?,
            frac_value: r.u8(add_offset(off, track_off::FRAC_VALUE)?)?,
            frac_slope: r.u8(add_offset(off, track_off::FRAC_SLOPE)?)?,
            pad: r.u8(add_offset(off, track_off::PAD)?)?,
            ad_offset,
            ad,
        })
    }
}

/// `FigaTree` (lbanim.h): a fighter motion, tracks grouped per joint.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FigaTree {
    /// Data offset this struct was read from.
    pub offset: u32,
    /// Bit 0 selects `JOBJ_CLASSICAL_SCALE` on every animated joint.
    pub type_: i32,
    /// `AOBJ_*` flags applied to every `AObj` built from this tree.
    pub flags: u32,
    /// End frame of the animation.
    pub frames: f32,
    /// Per-joint track counts, in skeleton order, without the `-1`
    /// terminator. Every entry is `>= 0`.
    pub nodes: Vec<i8>,
    /// All tracks, flat, `sum(nodes)` long; entry `nodes[i]` tracks belong
    /// to joint `i`. See [`FigaTree::tracks_by_node`].
    pub tracks: Vec<FigaTrack>,
}

impl FigaTree {
    /// Read the `FigaTree` at data offset `off`, its node list and every
    /// track with its key-frame bytes.
    ///
    /// A null `nodes` pointer yields an empty tree. A negative node other
    /// than the `-1` terminator is [`DescError::BadFigaTreeNode`]; a
    /// `tracks` pointer that is null while `sum(nodes) > 0` is
    /// [`DescError::NullPointer`]; a node list or track array running off
    /// the data section is reported through the underlying bounds error.
    pub fn read(archive: &Archive, off: u32) -> Result<Self> {
        let ctx = Ctx::new(archive);
        let r = ctx.reader();

        let type_ = r.s32(add_offset(off, tree_off::TYPE)?)?;
        let flags = r.u32(add_offset(off, tree_off::FLAGS)?)?;
        let frames = r.f32(add_offset(off, tree_off::FRAMES)?)?;
        let nodes_off = ctx.link("FigaTree.nodes", off, tree_off::NODES)?;
        let tracks_off = ctx.link("FigaTree.tracks", off, tree_off::TRACKS)?;

        let mut nodes = Vec::new();
        if let Some(mut at) = nodes_off {
            loop {
                let v = r.s8(at)?;
                if v == -1 {
                    break;
                }
                if v < 0 {
                    return Err(DescError::BadFigaTreeNode {
                        offset: at,
                        value: v,
                    });
                }
                nodes.push(v);
                at = add_offset(at, 1)?;
            }
        }

        let total: u64 = nodes.iter().map(|&n| n as u64).sum();
        let mut tracks = Vec::new();
        if total > 0 {
            // Retail indexes `tracks[k]` unconditionally, so a null
            // pointer here is a genuine fault.
            let base = tracks_off.ok_or(DescError::NullPointer {
                field: "FigaTree.tracks",
                at: add_offset(off, tree_off::TRACKS)?,
            })?;
            // Bounds-check the whole array before allocating for it, so a
            // hostile node list cannot demand a huge allocation.
            let bytes = u32::try_from(total * u64::from(FIGATRACK_SIZE)).map_err(|_| {
                crate::error::Error::OffsetOverflow {
                    offset: base,
                    add: u32::MAX,
                }
            })?;
            r.slice(base, bytes)?;
            tracks.reserve_exact(total as usize);
            for k in 0..total as u32 {
                tracks.push(FigaTrack::read_in(&ctx, base + k * FIGATRACK_SIZE)?);
            }
        }

        Ok(Self {
            offset: off,
            type_,
            flags,
            frames,
            nodes,
            tracks,
        })
    }

    /// Tracks grouped per node, in node order: the `i`-th slice has
    /// `nodes[i]` entries and is what `lbAnim_8001E6D8` receives as
    /// `(track, frames)` for the `i`-th animated joint.
    pub fn tracks_by_node(&self) -> Vec<&[FigaTrack]> {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut cursor = 0usize;
        for &n in &self.nodes {
            let n = n as usize;
            out.push(self.tracks.get(cursor..cursor + n).unwrap_or(&[]));
            cursor += n;
        }
        out
    }
}
