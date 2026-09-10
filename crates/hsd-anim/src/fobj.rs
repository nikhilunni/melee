//! Keyframe tracks: `HSD_FObj` from `src/sysdolphin/baselib/fobj.c` and
//! `fobj.h`, plus `splGetHelmite` from `spline.c` (the only spline routine the
//! interpreter needs).
//!
//! An FObj is one animated scalar (a translate-X, a rotate-Z, a texture blend
//! factor, ...) driven by a packed byte stream of keyframes. A list of them is
//! owned by an [`crate::aobj::AObj`], which advances every track by the same
//! frame delta and pushes each produced value to an update callback keyed by
//! the track's `obj_type` (for joints, [`JObjTrack`]).
//!
//! # On-disc keyframe stream (`HSD_FObjDesc::ad`, `length` bytes)
//!
//! Pinned down from `parseOpCode`, `parsePackInfo`, `parseWait`, `parseFloat`
//! and `FObjLoadData` in `fobj.c`:
//!
//! * The stream is a sequence of *packs*. A pack starts with a header var-int
//!   whose first byte is `op | ((n - 1) & 7) << 4 | cont << 7`: bits 0-3 the
//!   opcode (`HSD_A_OP_*`), bits 4-6 the low three bits of `n - 1`, bit 7 the
//!   continuation flag. Continuation bytes carry seven bits each (bit 7 =
//!   more follow) and are shifted in from bit 3 upward, so
//!   `n = ((b0 >> 4) & 7) + 1 + (b1 & 0x7f) << 3 + (b2 & 0x7f) << 10 + ...`.
//!   `n` is the pack's key count (`nb_pack`).
//! * Each key in the pack is one *data* record followed by one *wait*
//!   var-int (plain LEB128: seven bits per byte, little-endian, bit 7 =
//!   continue). The wait is the frame count until the next key (`fterm`).
//!   The interpreter reads data and wait alternately: `FObjLoadData` then
//!   `FObjLoadWait`. The stream ends when the read cursor reaches `length`.
//! * The opcode that governs interpolation of a segment (`op_intrp`) is the
//!   one in force when the segment's *start* key was read: `FObjLoadData`
//!   copies `op` into `op_intrp` before parsing a new pack header, so the
//!   first segment after a pack boundary still uses the previous pack's
//!   opcode. Consequently a track needs at least two data records before
//!   `op_intrp` is meaningful (see [`FOBJ_UNINITIALISED_VALUE`]).
//! * A data record depends on the opcode: `CON`, `LIN`, `SPL0`, `KEY` read
//!   one value (`frac_value` encoding); `SPL` reads a value (`frac_value`)
//!   then a slope (`frac_slope`); `SLP` reads only a slope and, unlike the
//!   others, does *not* advance the state machine, so a following data
//!   record is read in the same step (a slope-only prefix for the next key).
//! * Value encoding (`frac`, one byte in the desc): bits 5-7 select the
//!   type (`HSD_A_FRAC_*`: `0` f32, `1` s16, `2` u16, `3` s8, `4` u8), bits
//!   0-4 the binary fraction shift. An integer value `v` decodes as
//!   `(f32)v / (f32)(1 << shift)`. A raw f32 is only taken when the whole
//!   frac byte is `0`; type `0` with a non-zero shift decodes as `0.0`.
//!   All multi-byte quantities are **little-endian** inside the stream (the
//!   f32 is assembled byte 0 into bits 0-7 ... byte 3 into bits 24-31).
//!
//! # Interpreter state (`HSD_FObj`)
//!
//! `flags` bits 0-3 are the load state (`FOBJ_LOAD_*` below), bit 5 marks a
//! `LIN` segment whose slope must be recomputed, bit 6 a `KEY` value waiting
//! to become the current one, bit 7 a `KEY` value ready to be emitted. `p0`,
//! `p1` are the previous and current key values, `d0`, `d1` their slopes,
//! `time` the frame position relative to the current segment start, `fterm`
//! the current segment length, `nb_pack` the keys left in the current pack.
//!
//! Every method here is a literal transcription. Float operation order is
//! preserved; interpolation uses explicit fused operations from the retail DOL.

use gekko_math::fma::fmadds;

/// `HSD_A_OP_NONE` (`fobj.h`).
pub const HSD_A_OP_NONE: u8 = 0;
/// `HSD_A_OP_CON`: step to the new value when the segment ends.
pub const HSD_A_OP_CON: u8 = 1;
/// `HSD_A_OP_LIN`: linear interpolation to the new value.
pub const HSD_A_OP_LIN: u8 = 2;
/// `HSD_A_OP_SPL0`: Hermite interpolation, incoming slope `0`.
pub const HSD_A_OP_SPL0: u8 = 3;
/// `HSD_A_OP_SPL`: Hermite interpolation with an explicit slope.
pub const HSD_A_OP_SPL: u8 = 4;
/// `HSD_A_OP_SLP`: slope-only record modifying the next key's slopes.
pub const HSD_A_OP_SLP: u8 = 5;
/// `HSD_A_OP_KEY`: discrete key, emitted once when its segment starts.
pub const HSD_A_OP_KEY: u8 = 6;

/// `HSD_A_FRAC_FLOAT` (`fobj.h`): raw little-endian f32, only when the whole frac byte is 0.
pub const HSD_A_FRAC_FLOAT: u8 = 0 << 5;
/// `HSD_A_FRAC_S16`.
pub const HSD_A_FRAC_S16: u8 = 1 << 5;
/// `HSD_A_FRAC_U16`.
pub const HSD_A_FRAC_U16: u8 = 2 << 5;
/// `HSD_A_FRAC_S8`.
pub const HSD_A_FRAC_S8: u8 = 3 << 5;
/// `HSD_A_FRAC_U8`.
pub const HSD_A_FRAC_U8: u8 = 4 << 5;

/// `FOBJ_LOAD_DATA0` (`fobj.h`): about to read the first data record.
pub const FOBJ_LOAD_DATA0: u32 = 1;
/// `FOBJ_LOAD_DATA`: about to read a data record.
pub const FOBJ_LOAD_DATA: u32 = 2;
/// `FOBJ_LOAD_WAIT`: about to read a wait var-int.
pub const FOBJ_LOAD_WAIT: u32 = 3;
/// State 4 (unnamed in `fobj.h`): inside a segment, evaluate at `time`.
pub const FOBJ_INTERPOLATE: u32 = 4;
/// State 5 (unnamed): a value was emitted this step; resume at 4 next step.
pub const FOBJ_EMITTED: u32 = 5;
/// State 6 (unnamed): stream exhausted, hold the final value.
pub const FOBJ_END: u32 = 6;

/// `flags & 0x20`: a `LIN` segment was entered and `d0` must be recomputed.
pub const FOBJ_FLAG_LIN_SLOPE_DIRTY: u8 = 0x20;
/// `flags & 0x40`: a `KEY` value is loaded in `p1` and waits to become current.
pub const FOBJ_FLAG_KEY_PENDING: u8 = 0x40;
/// `flags & 0x80`: a `KEY` value is in `p0` and must be emitted.
pub const FOBJ_FLAG_KEY_READY: u8 = 0x80;

/// Stand-in for the value `FObjUpdateAnim` passes when `op_intrp` is
/// `HSD_A_OP_NONE`. In the C the `HSD_ObjData` local is uninitialised there,
/// so retail forwards whatever the stack slot held. Reachable when a stream
/// runs out before its second data record has been loaded (a track with a
/// single key). See the module notes and `update_anim`.
pub const FOBJ_UNINITIALISED_VALUE: f32 = 0.0;

/// Keyframe opcode, the low nibble of a pack header byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    None = HSD_A_OP_NONE,
    Con = HSD_A_OP_CON,
    Lin = HSD_A_OP_LIN,
    Spl0 = HSD_A_OP_SPL0,
    Spl = HSD_A_OP_SPL,
    Slp = HSD_A_OP_SLP,
    Key = HSD_A_OP_KEY,
}

impl Op {
    /// Decode the low nibble of a pack header; `None` for 7..=15.
    pub fn from_u8(v: u8) -> Option<Op> {
        Some(match v {
            HSD_A_OP_NONE => Op::None,
            HSD_A_OP_CON => Op::Con,
            HSD_A_OP_LIN => Op::Lin,
            HSD_A_OP_SPL0 => Op::Spl0,
            HSD_A_OP_SPL => Op::Spl,
            HSD_A_OP_SLP => Op::Slp,
            HSD_A_OP_KEY => Op::Key,
            _ => return None,
        })
    }
}

/// Value type of a frac byte (its bits 5-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FracType {
    Float = HSD_A_FRAC_FLOAT,
    S16 = HSD_A_FRAC_S16,
    U16 = HSD_A_FRAC_U16,
    S8 = HSD_A_FRAC_S8,
    U8 = HSD_A_FRAC_U8,
}

/// Build a frac byte: type in bits 5-7, binary fraction shift in bits 0-4.
pub const fn frac(ty: FracType, shift: u8) -> u8 {
    (ty as u8) | (shift & 0x1F)
}

/// Joint track ids, `HSD_A_J_*` in `jobj.h`. These are the `obj_type` values
/// an FObj carries when its AObj drives a JObj; `JObjUpdateFunc` (`jobj.c`)
/// switches on them to set translate/rotate/scale by axis. Other object
/// kinds (TObj, MObj, LObj, ...) have their own id spaces and are not
/// listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JObjTrack {
    RotX = 1,
    RotY = 2,
    RotZ = 3,
    Path = 4,
    TraX = 5,
    TraY = 6,
    TraZ = 7,
    ScaX = 8,
    ScaY = 9,
    ScaZ = 10,
    Node = 11,
    Branch = 12,
    SetByte0 = 20,
    SetByte1 = 21,
    SetByte2 = 22,
    SetByte3 = 23,
    SetByte4 = 24,
    SetByte5 = 25,
    SetByte6 = 26,
    SetByte7 = 27,
    SetByte8 = 28,
    SetByte9 = 29,
    SetFloat0 = 30,
    SetFloat1 = 31,
    SetFloat2 = 32,
    SetFloat3 = 33,
    SetFloat4 = 34,
    SetFloat5 = 35,
    SetFloat6 = 36,
    SetFloat7 = 37,
    SetFloat8 = 38,
    SetFloat9 = 39,
}

impl JObjTrack {
    /// Decode an `obj_type`; `None` for ids `JObjUpdateFunc` has no named
    /// case for (it also handles the unnamed `0x28`..`0x2A` and `0x32`..`0x34`).
    pub fn from_u8(v: u8) -> Option<JObjTrack> {
        Some(match v {
            1 => JObjTrack::RotX,
            2 => JObjTrack::RotY,
            3 => JObjTrack::RotZ,
            4 => JObjTrack::Path,
            5 => JObjTrack::TraX,
            6 => JObjTrack::TraY,
            7 => JObjTrack::TraZ,
            8 => JObjTrack::ScaX,
            9 => JObjTrack::ScaY,
            10 => JObjTrack::ScaZ,
            11 => JObjTrack::Node,
            12 => JObjTrack::Branch,
            20 => JObjTrack::SetByte0,
            21 => JObjTrack::SetByte1,
            22 => JObjTrack::SetByte2,
            23 => JObjTrack::SetByte3,
            24 => JObjTrack::SetByte4,
            25 => JObjTrack::SetByte5,
            26 => JObjTrack::SetByte6,
            27 => JObjTrack::SetByte7,
            28 => JObjTrack::SetByte8,
            29 => JObjTrack::SetByte9,
            30 => JObjTrack::SetFloat0,
            31 => JObjTrack::SetFloat1,
            32 => JObjTrack::SetFloat2,
            33 => JObjTrack::SetFloat3,
            34 => JObjTrack::SetFloat4,
            35 => JObjTrack::SetFloat5,
            36 => JObjTrack::SetFloat6,
            37 => JObjTrack::SetFloat7,
            38 => JObjTrack::SetFloat8,
            39 => JObjTrack::SetFloat9,
            _ => return None,
        })
    }
}

/// `HSD_ObjUpdateFunc` (`forward.h`): `void (*)(void* obj, enum_t type,
/// HSD_ObjData* fval)`. The `obj` pointer is the closure's captured
/// environment. `type` is the track's `obj_type`. `fval` is always written
/// through its `fv` member by `FObjUpdateAnim`; consumers that read `iv`
/// (`HSD_A_J_SETBYTE*`) take `value.to_bits() as i32`.
pub type ObjUpdateFunc<'a> = dyn FnMut(u8, f32) + 'a;

/// `HSD_FObjDesc` (`fobj.h`) without its `next` link: the on-disc track
/// description that `hsd-archive` will produce.
#[derive(Debug, Clone, PartialEq)]
pub struct FObjDesc {
    /// Byte length of the keyframe stream the interpreter may consume.
    pub length: u32,
    /// Track start offset in frames (stored as `s16` in the FObj).
    pub startframe: f32,
    /// Track id (`obj_type`), e.g. a [`JObjTrack`] for joints.
    pub obj_type: u8,
    /// Encoding of value records.
    pub frac_value: u8,
    /// Encoding of slope records.
    pub frac_slope: u8,
    /// The keyframe stream. May be longer than `length`.
    pub ad: Vec<u8>,
}

/// `HSD_FObj` (`fobj.h`): one keyframe track and its interpreter state.
///
/// Fields are public so tests and the future JObj port can inspect the
/// cached interpolation state; mutate them only through the methods.
#[derive(Debug, Clone, PartialEq)]
pub struct FObj {
    /// `ad_head`: the keyframe stream.
    ad: Box<[u8]>,
    /// `ad - ad_head`: read cursor into `ad`.
    pub pos: usize,
    /// `length`.
    pub length: u32,
    /// `flags`: low nibble is the load state, high bits the `FOBJ_FLAG_*`.
    pub flags: u8,
    /// `op`: opcode of the pack being read.
    pub op: u8,
    /// `op_intrp`: opcode governing interpolation of the current segment.
    pub op_intrp: u8,
    /// `obj_type`: track id.
    pub obj_type: u8,
    /// `frac_value`.
    pub frac_value: u8,
    /// `frac_slope`.
    pub frac_slope: u8,
    /// `nb_pack`: keys remaining in the current pack.
    pub nb_pack: u16,
    /// `startframe`.
    pub startframe: i16,
    /// `fterm`: length of the current segment in frames.
    pub fterm: u16,
    /// `time`: frames into the current segment.
    pub time: f32,
    /// `p0`: previous key value.
    pub p0: f32,
    /// `p1`: current key value.
    pub p1: f32,
    /// `d0`: slope at `p0`.
    pub d0: f32,
    /// `d1`: slope at `p1`.
    pub d1: f32,
}
impl FObj {
    /// Restore a preloaded track's playback state without copying its immutable
    /// byte stream. The caller must supply the same track/stream (fixed pools).
    pub fn restore_playback(&mut self, initial: &Self) {
        debug_assert_eq!(self.ad, initial.ad);
        self.pos = initial.pos;
        self.length = initial.length;
        self.flags = initial.flags;
        self.op = initial.op;
        self.op_intrp = initial.op_intrp;
        self.obj_type = initial.obj_type;
        self.frac_value = initial.frac_value;
        self.frac_slope = initial.frac_slope;
        self.nb_pack = initial.nb_pack;
        self.startframe = initial.startframe;
        self.fterm = initial.fterm;
        self.time = initial.time;
        self.p0 = initial.p0;
        self.p1 = initial.p1;
        self.d0 = initial.d0;
        self.d1 = initial.d1;
    }
}

/// PowerPC `slw`: a 32-bit shift left that yields 0 when bit 5 of the shift
/// amount is set. The var-int readers shift by `7 * k`, which the C leaves
/// undefined once it passes 31; this is what the hardware does.
#[inline]
fn slw(x: u32, sh: i32) -> u32 {
    if sh & 0x20 != 0 {
        0
    } else {
        x << (sh & 0x1F)
    }
}

/// `parseFloat` (`fobj.c`, retail `0x8036AC10`): read one value record in
/// the `frac` encoding at `*pos` and advance the cursor.
///
/// Panics if the stream is truncated (the C would read past the buffer).
pub fn parse_float(ad: &[u8], pos: &mut usize, frac: u8) -> f32 {
    if frac == HSD_A_FRAC_FLOAT {
        let mut d: u32 = ad[*pos] as i32 as u32;
        *pos += 1;
        d |= (ad[*pos] as u32) << 8;
        *pos += 1;
        d |= (ad[*pos] as u32) << 16;
        *pos += 1;
        d |= (ad[*pos] as u32) << 24;
        *pos += 1;
        return f32::from_bits(d);
    }

    // `1 << 31` is `INT_MIN` on the hardware (the C leaves it undefined).
    let denom: i32 = 1i32.wrapping_shl((frac & 0x1F) as u32);
    let numer: f32;
    match frac & 0xE0 {
        HSD_A_FRAC_S8 => {
            numer = ad[*pos] as i8 as f32;
            *pos += 1;
        }
        HSD_A_FRAC_U8 => {
            numer = ad[*pos] as f32;
            *pos += 1;
        }
        HSD_A_FRAC_S16 => {
            numer = (((ad[*pos + 1] as i8 as i32) << 8) | ad[*pos] as i32) as f32;
            *pos += 2;
        }
        HSD_A_FRAC_U16 => {
            numer = (((ad[*pos + 1] as i32) << 8) | ad[*pos] as i32) as f32;
            *pos += 2;
        }
        _ => return 0.0,
    }
    numer / denom as f32
}

/// `parseOpCode` (`fobj.c`, inlined): the low nibble of the byte at `pos`.
/// Does not advance; `parse_pack_info` consumes the same byte.
pub fn parse_op_code(ad: &[u8], pos: usize) -> u8 {
    ad[pos] & 0xF
}

/// `parsePackInfo` (`fobj.c`, retail `0x8036ADDC`): the pack header var-int.
/// Bits 4-6 of the first byte are the low three bits of `count - 1`; bit 7
/// says a continuation byte follows; continuation bytes add seven bits each
/// starting at bit 3 of the count.
pub fn parse_pack_info(ad: &[u8], pos: &mut usize) -> u32 {
    let mut d = ad[*pos];
    *pos += 1;
    let mut nb_pack: u32 = (((d >> 4) & 7) + 1) as u32;
    let mut shift: i32 = 3;
    if d & 0x80 == 0 {
        return nb_pack;
    }
    loop {
        d = ad[*pos];
        *pos += 1;
        nb_pack = nb_pack.wrapping_add(slw((d & 0x7F) as u32, shift));
        shift += 7;
        if d & 0x80 == 0 {
            break;
        }
    }
    nb_pack
}

/// `parseWait` (`fobj.c`, inlined): LEB128 unsigned var-int, seven bits per
/// byte, bit 7 = continue. Returned as the C's `s32`.
pub fn parse_wait(ad: &[u8], pos: &mut usize) -> i32 {
    let mut wait: i32 = 0;
    let mut shift: i32 = 0;
    loop {
        let d = ad[*pos];
        *pos += 1;
        wait |= slw((d & 0x7F) as u32, shift) as i32;
        shift += 7;
        if d & 0x80 == 0 {
            break;
        }
    }
    wait
}

/// `splGetHelmite` (`spline.c`, retail `0x80378A34`): cubic Hermite
/// evaluation. `fterm` is the reciprocal of the segment length (the caller
/// passes `1.0 / fobj->fterm`), `time` the position within the segment,
/// `p0`/`p1` the end values and `d0`/`d1` the end slopes per frame.
///
/// The three weighted additions fuse in retail; basis arithmetic stays separate.
#[allow(clippy::just_underscores_and_digits)]
pub fn spl_get_helmite(fterm: f32, time: f32, p0: f32, p1: f32, d0: f32, d1: f32) -> f32 {
    let _1_t2: f32 = time * time;
    let t2: f32 = fterm * fterm;
    let t2_t: f32 = _1_t2 * fterm;
    let t3_t2: f32 = t2 * (_1_t2 * time);
    let _2t3_t3: f32 = 2.0 * t3_t2 * fterm;
    let _3t2_t2: f32 = 3.0 * _1_t2 * t2;

    // retail 0x80378A84: fmadds, with the p1 product rounded first.
    let inner: f32 = fmadds(p0, 1.0 + (_2t3_t3 - _3t2_t2), p1 * (-_2t3_t3 + _3t2_t2));
    // retail 0x80378A88: fmadds.
    let mid: f32 = fmadds(d0, time + ((t3_t2 - t2_t) - t2_t), inner);
    // retail 0x80378A8C: fmadds.
    fmadds(d1, t3_t2 - t2_t, mid)
}

impl FObj {
    /// `HSD_FObjLoadDesc` (`fobj.c`, retail `0x8036B73C`) for one desc:
    /// `startframe` is converted `f32 -> s16` as MWCC does (`fctiwz`, then a
    /// halfword store keeps the low 16 bits). `flags` starts at 0 (state 0,
    /// not animating) until `req_anim`.
    pub fn load_desc(desc: &FObjDesc) -> FObj {
        FObj {
            ad: desc.ad.clone().into_boxed_slice(),
            pos: 0,
            length: desc.length,
            flags: 0,
            op: 0,
            op_intrp: 0,
            obj_type: desc.obj_type,
            frac_value: desc.frac_value,
            frac_slope: desc.frac_slope,
            nb_pack: 0,
            startframe: gekko_math::msl::fctiwz(desc.startframe) as i16,
            fterm: 0,
            time: 0.0,
            p0: 0.0,
            p1: 0.0,
            d0: 0.0,
            d1: 0.0,
        }
    }

    /// `HSD_FObjLoadDesc` over a whole `next`-linked list.
    pub fn load_desc_list(descs: &[FObjDesc]) -> Vec<FObj> {
        descs.iter().map(FObj::load_desc).collect()
    }

    /// Convenience constructor: a track over `ad` with `length = ad.len()`.
    pub fn new(ad: &[u8], startframe: f32, obj_type: u8, frac_value: u8, frac_slope: u8) -> FObj {
        FObj::load_desc(&FObjDesc {
            length: ad.len() as u32,
            startframe,
            obj_type,
            frac_value,
            frac_slope,
            ad: ad.to_vec(),
        })
    }

    /// `ad_head`: the keyframe stream.
    pub fn stream(&self) -> &[u8] {
        &self.ad
    }

    /// `HSD_FObjSetState` (retail `0x8036AA44`): store the low nibble of
    /// `state` in the flags; returns `state` unchanged.
    pub fn set_state(&mut self, state: u32) -> u32 {
        self.flags = ((state & 0xF) as u8) | (self.flags & 0xF0);
        state
    }

    /// `HSD_FObjGetState` (retail `0x8036AA64`).
    pub fn state(&self) -> u32 {
        (self.flags & 0xF) as u32
    }

    /// `HSD_FObjReqAnim` (`fobj.c`, inlined into `HSD_FObjReqAnimAll`,
    /// retail `0x8036AA80`): rewind to the start of the stream and position
    /// at `startframe` frames in.
    pub fn req_anim(&mut self, startframe: f32) {
        self.pos = 0;
        self.time = self.startframe as f32 + startframe;
        self.op = 0;
        self.op_intrp = 0;
        self.flags &= !FOBJ_FLAG_KEY_PENDING;
        self.nb_pack = 0;
        self.fterm = 0;
        self.p0 = 0.0;
        self.p1 = 0.0;
        self.d0 = 0.0;
        self.d1 = 0.0;
        self.set_state(FOBJ_LOAD_DATA0);
    }

    /// `HSD_FObjReqAnimAll` (retail `0x8036AA80`).
    pub fn req_anim_all(fobjs: &mut [FObj], startframe: f32) {
        for fp in fobjs.iter_mut() {
            fp.req_anim(startframe);
        }
    }

    /// `FObj_FlushKeyData` (`fobj.c`, inlined): a `KEY` track gets one more
    /// interpretation step so a pending key is emitted before stopping.
    fn flush_key_data(&mut self, obj_update: Option<&mut ObjUpdateFunc<'_>>, rate: f32) {
        if self.op_intrp == HSD_A_OP_KEY {
            self.interpret_anim(obj_update, rate);
        }
    }

    /// `HSD_FObjStopAnim` (retail `0x8036AB24`).
    pub fn stop_anim(&mut self, obj_update: Option<&mut ObjUpdateFunc<'_>>, rate: f32) {
        self.flush_key_data(obj_update, rate);
        self.set_state(0);
    }

    /// `HSD_FObjStopAnimAll` (retail `0x8036AB78`).
    pub fn stop_anim_all(
        fobjs: &mut [FObj],
        mut obj_update: Option<&mut ObjUpdateFunc<'_>>,
        rate: f32,
    ) {
        for fobj in fobjs.iter_mut() {
            fobj.stop_anim(obj_update.as_deref_mut(), rate);
        }
    }

    /// `FObjLaunchKeyData` (retail `0x8036AE38`): promote a pending `KEY`
    /// value to the emit slot.
    fn launch_key_data(&mut self) {
        if self.flags & FOBJ_FLAG_KEY_PENDING != 0 {
            self.op_intrp = self.op;
            self.flags &= !FOBJ_FLAG_KEY_PENDING;
            self.flags |= FOBJ_FLAG_KEY_READY;
            self.p0 = self.p1;
        }
    }

    /// `FObjLoadWait` (`fobj.c`, inlined): read the next wait var-int.
    /// Returns 6 without touching the state when the stream is exhausted.
    fn load_wait(&mut self) -> u32 {
        let st = self.state();
        assert!(st == FOBJ_LOAD_WAIT, "fobj.c:0x16C: st == FOBJ_LOAD_WAIT");

        if self.pos as u32 >= self.length {
            FOBJ_END
        } else {
            self.fterm = parse_wait(&self.ad, &mut self.pos) as u16;
            self.flags |= FOBJ_FLAG_LIN_SLOPE_DIRTY;
            self.set_state(FOBJ_LOAD_DATA)
        }
    }

    /// `FObjAnimCON` (`fobj.c`, inlined).
    fn anim_con(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x17F: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.p0 = self.p1;
        self.p1 = parse_float(&self.ad, &mut self.pos, self.frac_value);
        if self.op_intrp != HSD_A_OP_SLP {
            self.d0 = self.d1;
            self.d1 = 0.0;
        }

        self.set_state(if st == FOBJ_LOAD_DATA0 { 3 } else { 4 })
    }

    /// `FObjAnimLinear` (`fobj.c`, inlined). Identical body to `anim_con`.
    fn anim_linear(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x193: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.p0 = self.p1;
        self.p1 = parse_float(&self.ad, &mut self.pos, self.frac_value);
        if self.op_intrp != HSD_A_OP_SLP {
            self.d0 = self.d1;
            self.d1 = 0.0;
        }

        self.set_state(if st == FOBJ_LOAD_DATA0 { 3 } else { 4 })
    }

    /// `FObjAnimSPL0` (`fobj.c`, inlined).
    fn anim_spl0(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x1A7: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.p0 = self.p1;
        self.d0 = self.d1;
        self.p1 = parse_float(&self.ad, &mut self.pos, self.frac_value);
        self.d1 = 0.0;

        self.set_state(if st == FOBJ_LOAD_DATA0 { 3 } else { 4 })
    }

    /// `FObjAnimSPL` (`fobj.c`, inlined).
    fn anim_spl(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x1B9: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.p0 = self.p1;
        self.p1 = parse_float(&self.ad, &mut self.pos, self.frac_value);
        self.d0 = self.d1;
        self.d1 = parse_float(&self.ad, &mut self.pos, self.frac_slope);

        self.set_state(if st == FOBJ_LOAD_DATA0 { 3 } else { 4 })
    }

    /// `FObjAnimSLP` (`fobj.c`, inlined): slope only; state unchanged, so
    /// the interpreter loads the next data record in the same step.
    fn anim_slp(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x1CC: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.d0 = self.d1;
        self.d1 = parse_float(&self.ad, &mut self.pos, self.frac_slope);

        self.state()
    }

    /// `FObjAnimKey` (`fobj.c`, inlined).
    fn anim_key(&mut self) -> u32 {
        let st = self.state();
        assert!(
            st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA,
            "fobj.c:0x1E9: st == FOBJ_LOAD_DATA0 || st == FOBJ_LOAD_DATA"
        );

        self.launch_key_data();
        self.p1 = parse_float(&self.ad, &mut self.pos, self.frac_value);
        self.flags |= FOBJ_FLAG_KEY_PENDING;

        self.set_state(if st == FOBJ_LOAD_DATA0 { 3 } else { 4 })
    }

    /// `FObjLoadData` (`fobj.c`, inlined): read the next data record,
    /// starting a new pack when the current one is used up. Returns 6
    /// without touching `op_intrp` or the state when the stream is
    /// exhausted; returns 0 (state untouched) for an unknown opcode.
    fn load_data(&mut self) -> u32 {
        if self.pos as u32 >= self.length {
            FOBJ_END
        } else {
            self.op_intrp = self.op;
            if self.nb_pack == 0 {
                self.op = parse_op_code(&self.ad, self.pos);
                self.nb_pack = parse_pack_info(&self.ad, &mut self.pos) as u16;
            }

            self.nb_pack = self.nb_pack.wrapping_sub(1);

            match self.op {
                HSD_A_OP_CON => self.anim_con(),
                HSD_A_OP_LIN => self.anim_linear(),
                HSD_A_OP_SPL0 => self.anim_spl0(),
                HSD_A_OP_SPL => self.anim_spl(),
                HSD_A_OP_SLP => self.anim_slp(),
                HSD_A_OP_KEY => self.anim_key(),
                _ => 0,
            }
        }
    }

    /// `FObjUpdateAnim` (`fobj.c`, retail `0x8036AE70`): evaluate the current
    /// segment at `time` and hand the value to `obj_update`. With no
    /// callback nothing happens, not even the flag updates.
    fn update_anim(&mut self, obj_update: Option<&mut ObjUpdateFunc<'_>>) {
        let Some(obj_update) = obj_update else {
            return;
        };
        let fv: f32;
        match self.op_intrp {
            HSD_A_OP_KEY => {
                if self.flags & FOBJ_FLAG_KEY_READY != 0 {
                    fv = self.p0;
                    self.flags &= !FOBJ_FLAG_KEY_READY;
                } else {
                    return;
                }
            }
            HSD_A_OP_CON => {
                fv = if self.time >= self.fterm as f32 {
                    self.p1
                } else {
                    self.p0
                };
            }
            HSD_A_OP_LIN => {
                if self.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY != 0 {
                    self.flags &= !FOBJ_FLAG_LIN_SLOPE_DIRTY;
                    if self.fterm != 0 {
                        self.d0 = (self.p1 - self.p0) / self.fterm as f32;
                    } else {
                        self.d0 = 0.0;
                        self.p0 = self.p1;
                    }
                }
                // retail 0x8036AF98: fmadds.
                fv = fmadds(self.d0, self.time, self.p0);
            }
            HSD_A_OP_SPL0 | HSD_A_OP_SPL | HSD_A_OP_SLP => {
                fv = if self.fterm != 0 {
                    // `1.0 / fobj->fterm` is a double divide (the literal is
                    // a double); the quotient is rounded to single once when
                    // passed as the f32 parameter.
                    spl_get_helmite(
                        (1.0f64 / self.fterm as f64) as f32,
                        self.time,
                        self.p0,
                        self.p1,
                        self.d0,
                        self.d1,
                    )
                } else {
                    self.p1
                };
            }
            _ => {
                // The C falls through with `fobjdata` uninitialised and
                // still calls the callback. See FOBJ_UNINITIALISED_VALUE.
                fv = FOBJ_UNINITIALISED_VALUE;
            }
        }
        obj_update(self.obj_type, fv);
    }

    /// `HSD_FObjInterpretAnim` (`fobj.c`, retail `0x8036B030`): advance the
    /// track by `rate` frames and emit at most one value through
    /// `obj_update`. Does nothing when the track is stopped (state 0) or
    /// when the advanced `time` is negative.
    ///
    /// The C's `switch` has no default, so an out-of-range state (7..=15,
    /// only reachable by corrupting `flags`) spins forever there; here it
    /// panics.
    pub fn interpret_anim(&mut self, mut obj_update: Option<&mut ObjUpdateFunc<'_>>, rate: f32) {
        let mut fterm: f32 = 0.0;
        let mut state = self.state();
        if state == 0 {
            return;
        }
        self.time += rate;
        if self.time < 0.0 {
            return;
        }
        loop {
            match state {
                FOBJ_END => {
                    self.time += fterm;
                    self.launch_key_data();
                    self.update_anim(obj_update);
                    return;
                }
                FOBJ_LOAD_DATA0 | FOBJ_LOAD_DATA => {
                    state = self.load_data();
                }
                FOBJ_LOAD_WAIT => {
                    if self.flags & FOBJ_FLAG_KEY_READY != 0 {
                        self.update_anim(obj_update.as_deref_mut());
                    }
                    state = self.load_wait();
                }
                FOBJ_INTERPOLATE => {
                    if self.fterm as f32 <= self.time {
                        state = FOBJ_LOAD_WAIT;
                        fterm = self.fterm as f32;
                        self.time -= self.fterm as f32;
                        self.set_state(state);
                        continue;
                    }
                    self.update_anim(obj_update.as_deref_mut());
                    state = FOBJ_EMITTED;
                    self.set_state(state);
                    return;
                }
                FOBJ_EMITTED => {
                    state = FOBJ_INTERPOLATE;
                    self.set_state(state);
                }
                0 => return,
                _ => panic!(
                    "HSD_FObjInterpretAnim: invalid state {state} (the C loops forever here)"
                ),
            }
        }
    }

    /// `HSD_FObjInterpretAnimAll` (retail `0x8036B6CC`).
    pub fn interpret_anim_all(
        fobjs: &mut [FObj],
        mut obj_update: Option<&mut ObjUpdateFunc<'_>>,
        rate: f32,
    ) {
        for fobj in fobjs.iter_mut() {
            fobj.interpret_anim(obj_update.as_deref_mut(), rate);
        }
    }

    /// Convenience over [`FObj::interpret_anim`]: advance by `rate` and return
    /// the emitted value, if any. Equivalent to passing a non-null callback.
    pub fn step(&mut self, rate: f32) -> Option<f32> {
        let mut out = None;
        self.interpret_anim(Some(&mut |_t, v| out = Some(v)), rate);
        out
    }
}
