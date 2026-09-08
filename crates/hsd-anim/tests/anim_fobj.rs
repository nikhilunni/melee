//! FObj keyframe interpreter: hand-built streams for every opcode, evaluated
//! at several frames against expected values computed here with the same f32
//! operation sequence the C uses; var-int and value-record decoding edge
//! cases; cached interpolation state.

mod anim_common;

use anim_common::{decoded, Stream};
use hsd_anim::fobj::*;

const FLOAT: u8 = HSD_A_FRAC_FLOAT;
const TRACK: u8 = JObjTrack::TraX as u8;

/// Reference Hermite: the `splGetHelmite` formula written out again, so the
/// library transcription is checked against an independent copy.
fn hermite(inv_t: f32, t: f32, p0: f32, p1: f32, d0: f32, d1: f32) -> f32 {
    let tt = t * t;
    let i2 = inv_t * inv_t;
    let a = tt * inv_t;
    let b = i2 * (tt * t);
    let c = 2.0 * b * inv_t;
    let d = 3.0 * tt * i2;
    (d1 * (b - a)) + ((d0 * (t + ((b - a) - a))) + ((p0 * (1.0 + (c - d))) + (p1 * (-c + d))))
}

fn inv(fterm: u16) -> f32 {
    (1.0f64 / fterm as f64) as f32
}

fn track(bytes: &[u8], frac_value: u8, frac_slope: u8) -> FObj {
    let mut f = FObj::new(bytes, 0.0, TRACK, frac_value, frac_slope);
    f.req_anim(0.0);
    f
}

/// Step at rate 1 (rate 0 for the first frame) and collect emitted values.
fn play(f: &mut FObj, frames: usize) -> Vec<Option<f32>> {
    (0..frames)
        .map(|i| f.step(if i == 0 { 0.0 } else { 1.0 }))
        .collect()
}

// ---------------------------------------------------------------------------
// var-int readers
// ---------------------------------------------------------------------------

#[test]
fn pack_info_single_byte_counts() {
    for n in 1..=8u32 {
        let mut s = Stream::new();
        s.pack(HSD_A_OP_LIN, n);
        assert_eq!(s.bytes.len(), 1);
        let mut pos = 0;
        assert_eq!(parse_op_code(&s.bytes, pos), HSD_A_OP_LIN);
        assert_eq!(parse_pack_info(&s.bytes, &mut pos), n);
        assert_eq!(pos, 1);
    }
}

#[test]
fn pack_info_continuation_bytes() {
    // 9 needs one continuation byte: low three bits of 8 are 0, rest is 1.
    let bytes = [0x80 | HSD_A_OP_CON, 0x01];
    let mut pos = 0;
    assert_eq!(parse_pack_info(&bytes, &mut pos), 9);
    assert_eq!(pos, 2);

    for n in [9u32, 16, 1000, 1024, 1025, 8 * 128 + 1, 70_000, 1 << 20] {
        let mut s = Stream::new();
        s.pack(HSD_A_OP_SPL, n);
        let mut pos = 0;
        assert_eq!(parse_op_code(&s.bytes, 0), HSD_A_OP_SPL);
        assert_eq!(parse_pack_info(&s.bytes, &mut pos), n, "n = {n}");
        assert_eq!(pos, s.bytes.len());
    }

    // Hand-checked: n-1 = 999 = 0b1111100111 -> low3 = 7, rest = 124.
    let bytes = [0x80 | (7 << 4) | HSD_A_OP_KEY, 124];
    let mut pos = 0;
    assert_eq!(parse_pack_info(&bytes, &mut pos), 1000);
}

#[test]
fn pack_info_shift_past_31_reads_as_zero() {
    // After the header byte, continuation shifts are 3, 10, 17, 24, 31, 38.
    // The sixth continuation byte lands at shift 38: PowerPC slw yields 0,
    // so it contributes nothing.
    let bytes = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7F];
    let mut pos = 0;
    assert_eq!(parse_pack_info(&bytes, &mut pos), 1);
    assert_eq!(pos, 7);
    // The fifth continuation byte at shift 31 keeps only its low bit.
    let bytes = [0x80, 0x80, 0x80, 0x80, 0x80, 0x03];
    let mut pos = 0;
    assert_eq!(parse_pack_info(&bytes, &mut pos), 1u32.wrapping_add(1 << 31));
    assert_eq!(pos, 6);
}

#[test]
fn wait_leb128() {
    for w in [0u32, 1, 127, 128, 129, 255, 16383, 16384, 65535, 65536, 1 << 27] {
        let mut s = Stream::new();
        s.wait(w);
        let mut pos = 0;
        assert_eq!(parse_wait(&s.bytes, &mut pos), w as i32, "w = {w}");
        assert_eq!(pos, s.bytes.len());
    }
    let bytes = [0x7F];
    let mut pos = 0;
    assert_eq!(parse_wait(&bytes, &mut pos), 127);
    let bytes = [0x80, 0x01];
    let mut pos = 0;
    assert_eq!(parse_wait(&bytes, &mut pos), 128);
    assert_eq!(pos, 2);
}

#[test]
fn wait_shift_past_31_reads_as_zero() {
    // Shifts 0, 7, 14, 21, 28, 35. Byte five keeps four bits; byte six is lost.
    let bytes = [0x80, 0x80, 0x80, 0x80, 0x8F, 0x01];
    let mut pos = 0;
    assert_eq!(parse_wait(&bytes, &mut pos), 0xF000_0000u32 as i32);
    assert_eq!(pos, 6);
}

// ---------------------------------------------------------------------------
// value records
// ---------------------------------------------------------------------------

#[test]
fn parse_float_raw_f32_is_little_endian() {
    let v = 1.5f32;
    let mut bytes = v.to_bits().to_le_bytes().to_vec();
    bytes.extend_from_slice(&(-2.25f32).to_bits().to_le_bytes());
    let mut pos = 0;
    assert_eq!(parse_float(&bytes, &mut pos, FLOAT).to_bits(), v.to_bits());
    assert_eq!(pos, 4);
    assert_eq!(parse_float(&bytes, &mut pos, FLOAT), -2.25);
    assert_eq!(pos, 8);
    // Byte 3 is the sign/exponent byte, so a big-endian reading is wrong.
    let be = v.to_bits().to_be_bytes();
    let mut pos = 0;
    assert_ne!(parse_float(&be, &mut pos, FLOAT).to_bits(), v.to_bits());
    // NaN payloads pass through untouched.
    let bytes = 0x7FC0_1234u32.to_le_bytes();
    let mut pos = 0;
    assert_eq!(parse_float(&bytes, &mut pos, FLOAT).to_bits(), 0x7FC0_1234);
}

#[test]
fn parse_float_integer_encodings() {
    // S8 with shift 4: -0x30 / 16 = -3.0.
    let f = frac(FracType::S8, 4);
    let mut pos = 0;
    assert_eq!(parse_float(&[0xD0], &mut pos, f), -3.0);
    assert_eq!(pos, 1);
    // U8 with shift 4: 0xD0 / 16 = 13.0.
    let f = frac(FracType::U8, 4);
    let mut pos = 0;
    assert_eq!(parse_float(&[0xD0], &mut pos, f), 13.0);
    // S16 little-endian: bytes [0x34, 0xF2] = 0xF234 -> -3532, shift 8.
    let f = frac(FracType::S16, 8);
    let mut pos = 0;
    assert_eq!(parse_float(&[0x34, 0xF2], &mut pos, f), -3532.0f32 / 256.0);
    assert_eq!(pos, 2);
    // U16 little-endian: 0xF234 = 62004, shift 8.
    let f = frac(FracType::U16, 8);
    let mut pos = 0;
    assert_eq!(parse_float(&[0x34, 0xF2], &mut pos, f), 62004.0f32 / 256.0);
    // Shift 0: integers straight through.
    let f = frac(FracType::S16, 0);
    let mut pos = 0;
    assert_eq!(parse_float(&[0x00, 0x80], &mut pos, f), -32768.0);
    // Sign comes from byte 1 only: [0xFF, 0x00] is +255.
    let mut pos = 0;
    assert_eq!(parse_float(&[0xFF, 0x00], &mut pos, f), 255.0);
    // Inexact quotients follow f32 division.
    let f = frac(FracType::S16, 5);
    let mut pos = 0;
    assert_eq!(
        parse_float(&[0x07, 0x00], &mut pos, f).to_bits(),
        decoded(f, 7).to_bits()
    );
}

#[test]
fn parse_float_shift_31_denominator_is_int_min() {
    // 1 << 31 as s32 is INT_MIN, so the value flips sign.
    let f = frac(FracType::U8, 31);
    let mut pos = 0;
    let v = parse_float(&[0x80], &mut pos, f);
    assert_eq!(v, 128.0f32 / -2147483648.0f32);
    assert!(v < 0.0);
}

#[test]
fn parse_float_float_type_with_shift_is_zero_and_does_not_advance() {
    // Type bits 0 but shift non-zero: not the raw-f32 path, and no integer
    // case matches, so the C returns 0.0 without touching the cursor.
    let f = frac(FracType::Float, 3);
    let mut pos = 0;
    assert_eq!(parse_float(&[0xFF, 0xFF, 0xFF, 0xFF], &mut pos, f).to_bits(), 0);
    assert_eq!(pos, 0);
    // Type bits 5..7 (undefined) likewise.
    let mut pos = 0;
    assert_eq!(parse_float(&[0xFF], &mut pos, 5 << 5).to_bits(), 0);
    assert_eq!(parse_float(&[0xFF], &mut pos, 7 << 5).to_bits(), 0);
    assert_eq!(pos, 0);
}

#[test]
fn enums_round_trip() {
    for op in 0..=6u8 {
        assert_eq!(Op::from_u8(op).unwrap() as u8, op);
    }
    assert_eq!(Op::from_u8(7), None);
    assert_eq!(Op::from_u8(15), None);
    for id in 1..=39u8 {
        match JObjTrack::from_u8(id) {
            Some(t) => assert_eq!(t as u8, id),
            None => assert!((13..=19).contains(&id), "id {id}"),
        }
    }
    assert_eq!(JObjTrack::from_u8(0), None);
    assert_eq!(JObjTrack::from_u8(40), None);
    assert_eq!(JObjTrack::TraX as u8, 5);
    assert_eq!(JObjTrack::ScaZ as u8, 10);
    assert_eq!(frac(FracType::S16, 12), 0x2C);
    assert_eq!(frac(FracType::U8, 40), 0x88);
}

// ---------------------------------------------------------------------------
// opcodes
// ---------------------------------------------------------------------------

#[test]
fn load_desc_truncates_startframe_like_fctiwz() {
    let d = |sf: f32| FObjDesc {
        length: 0,
        startframe: sf,
        obj_type: TRACK,
        frac_value: 0,
        frac_slope: 0,
        ad: vec![],
    };
    assert_eq!(FObj::load_desc(&d(2.7)).startframe, 2);
    assert_eq!(FObj::load_desc(&d(-2.7)).startframe, -2);
    assert_eq!(FObj::load_desc(&d(-0.5)).startframe, 0);
    assert_eq!(FObj::load_desc(&d(40000.0)).startframe, 40000i32 as i16);
    assert_eq!(FObj::load_desc(&d(1e10)).startframe, i32::MAX as i16);
    let f = FObj::load_desc(&d(3.0));
    assert_eq!(f.state(), 0);
    assert_eq!(f.flags, 0);
}

#[test]
fn constant_track_steps_at_segment_ends() {
    let (v0, v1, v2) = (0.25f32, -7.5f32, 3.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 3)
        .f32(v0)
        .wait(3)
        .f32(v1)
        .wait(2)
        .f32(v2)
        .wait(0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let got = play(&mut f, 8);
    let want = [v0, v0, v0, v1, v1, v2, v2, v2];
    assert_eq!(got, want.iter().map(|v| Some(*v)).collect::<Vec<_>>());
    // Final state: exhausted, holding p1 = v2.
    assert_eq!(f.p1, v2);
    assert_eq!(f.p0, v1);
    assert_eq!(f.op_intrp, HSD_A_OP_CON);

    // Without the trailing wait the sequence is the same.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 3).f32(v0).wait(3).f32(v1).wait(2).f32(v2);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    assert_eq!(play(&mut f, 8), want.iter().map(|v| Some(*v)).collect::<Vec<_>>());
}

#[test]
fn constant_track_state_trace() {
    // Walk the state machine step by step and check the cached fields.
    let (v0, v1) = (1.0f32, 2.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 2).f32(v0).wait(2).f32(v1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    assert_eq!(f.state(), FOBJ_LOAD_DATA0);
    assert_eq!(f.time, 0.0);

    assert_eq!(f.step(0.0), Some(v0));
    // Loaded v0, wait 2, v1: p0 = v0, p1 = v1, fterm = 2, pack used up.
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (v0, v1, 0.0, 0.0));
    assert_eq!(f.fterm, 2);
    assert_eq!(f.nb_pack, 0);
    assert_eq!(f.op, HSD_A_OP_CON);
    assert_eq!(f.op_intrp, HSD_A_OP_CON);
    assert_eq!(f.state(), FOBJ_EMITTED);
    assert_eq!(f.pos, 1 + 4 + 1 + 4);
    assert_eq!(f.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, FOBJ_FLAG_LIN_SLOPE_DIRTY);

    assert_eq!(f.step(1.0), Some(v0));
    assert_eq!(f.time, 1.0);
    assert_eq!(f.state(), FOBJ_EMITTED);

    // time reaches fterm: segment passed, stream exhausted, time restored.
    assert_eq!(f.step(1.0), Some(v1));
    assert_eq!(f.time, 2.0);
    assert_eq!(f.state(), FOBJ_LOAD_WAIT);

    assert_eq!(f.step(1.0), Some(v1));
    assert_eq!(f.time, 3.0);
}

#[test]
fn linear_track_interpolates_with_cached_slope() {
    let (v0, v1) = (0.3f32, 1.7f32);
    let fterm = 4u16;
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).f32(v0).wait(fterm as u32).f32(v1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);

    let d0 = (v1 - v0) / fterm as f32;
    let expect = |t: f32| d0 * t + v0;
    let got = play(&mut f, 7);
    for (i, g) in got.iter().enumerate() {
        // Past the end, time keeps growing and the line is extrapolated.
        assert_eq!(g.unwrap().to_bits(), expect(i as f32).to_bits(), "frame {i}");
    }
    assert_eq!(f.d0.to_bits(), d0.to_bits());
    assert_eq!(f.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, 0);
}

#[test]
fn linear_track_with_integer_encoding() {
    let fv = frac(FracType::S16, 6);
    let (r0, r1) = (-1234i32, 4567i32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).raw(fv, r0).wait(7).raw(fv, r1);
    let mut f = track(&s.finish(), fv, fv);
    let (v0, v1) = (decoded(fv, r0), decoded(fv, r1));
    let d0 = (v1 - v0) / 7.0f32;
    for i in 0..7 {
        let got = f.step(if i == 0 { 0.0 } else { 1.0 }).unwrap();
        assert_eq!(got.to_bits(), (d0 * i as f32 + v0).to_bits(), "frame {i}");
    }
}

#[test]
fn linear_zero_length_segment_snaps_to_end() {
    let (v0, v1, v2) = (1.0f32, 5.0f32, 9.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 3).f32(v0).wait(0).f32(v1).wait(2).f32(v2);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    // Frame 0: v0 loaded, wait 0, v1 loaded -> state 4 with fterm 0 <= time 0,
    // pass straight through: wait 2, v2 loaded, fterm 2. LIN slope dirty:
    // d0 = (v2 - v1) / 2, value = d0 * 0 + v1.
    let d0 = (v2 - v1) / 2.0;
    assert_eq!(f.step(0.0), Some(v1));
    assert_eq!(f.d0, d0);
    assert_eq!(f.step(1.0), Some(d0 + v1));
    assert_eq!(f.step(1.0), Some(d0 * 2.0 + v1));
    assert_eq!(f.step(1.0), Some(d0 * 3.0 + v1));
}

#[test]
fn spline_track_hermite_with_explicit_slopes() {
    let (v0, s0, v1, s1) = (-1.0f32, 0.75f32, 2.5f32, -0.125f32);
    let fterm = 5u16;
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2)
        .f32(v0)
        .f32(s0)
        .wait(fterm as u32)
        .f32(v1)
        .f32(s1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let got = play(&mut f, 6);
    for (i, g) in got.iter().enumerate() {
        let want = hermite(inv(fterm), i as f32, v0, v1, s0, s1);
        assert_eq!(g.unwrap().to_bits(), want.to_bits(), "frame {i}");
    }
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (v0, v1, s0, s1));
    // The start lands on the key value; the end does not in general, since
    // 1/5 is inexact in f32 (2.5000005 here). Retail has the same wobble.
    assert_eq!(got[0], Some(v0));
    assert_ne!(got[5], Some(v1));
    assert!((got[5].unwrap() - v1).abs() < 1e-5);
}

#[test]
fn spline_track_fractional_rate() {
    let (v0, s0, v1, s1) = (10.0f32, -3.0f32, -4.0f32, 1.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2).f32(v0).f32(s0).wait(3).f32(v1).f32(s1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let mut t = 0.0f32;
    assert_eq!(f.step(0.0), Some(v0));
    for _ in 0..4 {
        t += 0.7;
        let want = hermite(inv(3), t, v0, v1, s0, s1);
        assert_eq!(f.step(0.7).unwrap().to_bits(), want.to_bits(), "t = {t}");
    }
    // t = 3.5 passes the segment: the stream is exhausted, time is restored
    // to 3.5 and the same Hermite is extrapolated.
    t += 0.7;
    let want = hermite(inv(3), t, v0, v1, s0, s1);
    assert_eq!(f.step(0.7).unwrap().to_bits(), want.to_bits());
    assert_eq!(f.time, t);
}

#[test]
fn spline_slope_encoding_uses_frac_slope() {
    let fv = frac(FracType::S16, 8);
    let fs = frac(FracType::S8, 4);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2)
        .raw(fv, 256)
        .raw(fs, -16)
        .wait(4)
        .raw(fv, -512)
        .raw(fs, 8);
    let mut f = track(&s.finish(), fv, fs);
    f.step(0.0);
    assert_eq!((f.p0, f.p1), (1.0, -2.0));
    assert_eq!((f.d0, f.d1), (-1.0, 0.5));
    assert_eq!(f.step(1.0).unwrap().to_bits(), hermite(inv(4), 1.0, 1.0, -2.0, -1.0, 0.5).to_bits());
}

#[test]
fn spl0_track_has_zero_slopes() {
    let (v0, v1) = (2.0f32, 6.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL0, 2).f32(v0).wait(4).f32(v1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let got = play(&mut f, 5);
    for (i, g) in got.iter().enumerate() {
        let want = hermite(inv(4), i as f32, v0, v1, 0.0, 0.0);
        assert_eq!(g.unwrap().to_bits(), want.to_bits(), "frame {i}");
    }
    assert_eq!((f.d0, f.d1), (0.0, 0.0));
    // Midpoint of a zero-slope Hermite is the average.
    assert_eq!(got[2], Some(4.0));
}

#[test]
fn spl0_after_spl_keeps_incoming_slope() {
    // SPL key (v0, s0) then an SPL0 key v1: d0 = s0, d1 = 0.
    let (v0, s0, v1) = (0.0f32, 2.0f32, 1.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 1).f32(v0).f32(s0).wait(2);
    s.pack(HSD_A_OP_SPL0, 1).f32(v1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.step(0.0);
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (v0, v1, s0, 0.0));
    // op_intrp is copied from op *before* the SPL0 pack header is parsed, so
    // the segment starting at the SPL key is governed by SPL; both take the
    // Hermite branch.
    assert_eq!(f.op, HSD_A_OP_SPL0);
    assert_eq!(f.op_intrp, HSD_A_OP_SPL);
    let want = hermite(inv(2), 1.0, v0, v1, s0, 0.0);
    assert_eq!(f.step(1.0).unwrap().to_bits(), want.to_bits());
}

#[test]
fn slope_only_record_turns_next_linear_key_into_hermite() {
    // LIN v0, wait, SLP s, LIN v1: the SLP leaves op_intrp == SLP for the
    // segment, so FObjUpdateAnim takes the Hermite branch with d0 = 0,
    // d1 = s, and FObjAnimLinear skips its slope shift.
    let (v0, sl, v1) = (1.0f32, 4.0f32, 3.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 1).f32(v0).wait(5);
    s.pack(HSD_A_OP_SLP, 1).f32(sl);
    s.pack(HSD_A_OP_LIN, 1).f32(v1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    assert_eq!(f.step(0.0), Some(v0));
    assert_eq!(f.op, HSD_A_OP_LIN);
    assert_eq!(f.op_intrp, HSD_A_OP_SLP);
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (v0, v1, 0.0, sl));
    for i in 1..=5 {
        let want = hermite(inv(5), i as f32, v0, v1, 0.0, sl);
        assert_eq!(f.step(1.0).unwrap().to_bits(), want.to_bits(), "frame {i}");
    }
}

#[test]
fn slope_only_record_before_spl_key() {
    // SPL (v0,s0) wait, SLP s, SPL (v1,s1): SLP sets d1 = s, then SPL's
    // d0 = d1 = s and d1 = s1. The SPL's own slope shift is unconditional.
    let (v0, s0, sl, v1, s1) = (0.0f32, 1.0f32, 2.0f32, 3.0f32, 4.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 1).f32(v0).f32(s0).wait(3);
    s.pack(HSD_A_OP_SLP, 1).f32(sl);
    s.pack(HSD_A_OP_SPL, 1).f32(v1).f32(s1);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.step(0.0);
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (v0, v1, sl, s1));
    assert_eq!(f.op_intrp, HSD_A_OP_SLP);
    let want = hermite(inv(3), 2.0, v0, v1, sl, s1);
    f.step(1.0);
    assert_eq!(f.step(1.0).unwrap().to_bits(), want.to_bits());
}

#[test]
fn slope_only_at_stream_start_is_shifted_out() {
    // SLP before the first key: the first key's load moves the slope into
    // d0, the second key's load moves it out again.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SLP, 1).f32(9.0);
    s.pack(HSD_A_OP_SPL0, 2).f32(1.0).wait(2).f32(2.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.step(0.0);
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (1.0, 2.0, 0.0, 0.0));
    assert_eq!(f.op_intrp, HSD_A_OP_SPL0);
}

#[test]
fn key_track_emits_once_per_key() {
    let (v0, v1, v2) = (5.0f32, 6.0f32, 7.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 3).f32(v0).wait(2).f32(v1).wait(3).f32(v2);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let got = play(&mut f, 8);
    assert_eq!(
        got,
        vec![Some(v0), None, Some(v1), None, None, Some(v2), None, None]
    );
    assert_eq!(f.flags & (FOBJ_FLAG_KEY_PENDING | FOBJ_FLAG_KEY_READY), 0);
    assert_eq!(f.p0, v2);
}

#[test]
fn key_track_skipping_segments_emits_every_key() {
    // A large rate crosses two key boundaries in one step; both keys are
    // emitted in that step (one in state 3, one in state 4).
    let (v0, v1, v2, v3) = (1.0f32, 2.0f32, 3.0f32, 4.0f32);
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 4)
        .f32(v0)
        .wait(1)
        .f32(v1)
        .wait(1)
        .f32(v2)
        .wait(5)
        .f32(v3);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let mut emitted = Vec::new();
    f.interpret_anim(Some(&mut |t, v| emitted.push((t, v))), 0.0);
    assert_eq!(emitted, vec![(TRACK, v0)]);
    emitted.clear();
    f.interpret_anim(Some(&mut |t, v| emitted.push((t, v))), 2.5);
    assert_eq!(emitted, vec![(TRACK, v1), (TRACK, v2)]);
    assert_eq!(f.time, 0.5);
    assert_eq!(f.fterm, 5);
}

#[test]
fn key_ready_flag_survives_a_step_without_callback() {
    // With no update function FObjUpdateAnim returns before clearing 0x80,
    // so the value is emitted on the next step that has a callback.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 2).f32(1.0).wait(3).f32(2.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.interpret_anim(None, 0.0);
    assert_eq!(f.flags & FOBJ_FLAG_KEY_READY, FOBJ_FLAG_KEY_READY);
    assert_eq!(f.state(), FOBJ_EMITTED);
    assert_eq!(f.step(1.0), Some(1.0));
    assert_eq!(f.flags & FOBJ_FLAG_KEY_READY, 0);
}

#[test]
fn linear_dirty_flag_survives_a_step_without_callback() {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).f32(0.0).wait(4).f32(8.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.interpret_anim(None, 0.0);
    assert_eq!(f.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, FOBJ_FLAG_LIN_SLOPE_DIRTY);
    assert_eq!(f.d0, 0.0);
    assert_eq!(f.step(1.0), Some(2.0));
    assert_eq!(f.d0, 2.0);
}

#[test]
fn negative_time_yields_nothing() {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 2).f32(1.0).wait(2).f32(2.0);
    let mut f = FObj::new(&s.finish(), -2.0, TRACK, FLOAT, FLOAT);
    f.req_anim(0.0);
    assert_eq!(f.time, -2.0);
    assert_eq!(f.step(0.0), None);
    assert_eq!(f.state(), FOBJ_LOAD_DATA0);
    assert_eq!(f.step(1.0), None);
    assert_eq!(f.time, -1.0);
    assert_eq!(f.step(1.0), Some(1.0));
    assert_eq!(f.time, 0.0);
    // Positive request offset shifts the other way.
    let mut f = FObj::new(&s.finish(), 0.0, TRACK, FLOAT, FLOAT);
    f.req_anim(2.0);
    assert_eq!(f.step(0.0), Some(2.0));
}

#[test]
fn stopped_track_ignores_steps() {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 2).f32(1.0).wait(2).f32(2.0);
    let mut f = FObj::new(&s.finish(), 0.0, TRACK, FLOAT, FLOAT);
    assert_eq!(f.state(), 0);
    assert_eq!(f.step(1.0), None);
    assert_eq!(f.time, 0.0);
    f.req_anim(0.0);
    assert_eq!(f.step(0.0), Some(1.0));
    f.stop_anim(None, 1.0);
    assert_eq!(f.state(), 0);
    assert_eq!(f.step(1.0), None);
}

#[test]
fn stop_anim_flushes_pending_key() {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 2).f32(1.0).wait(3).f32(2.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    assert_eq!(f.step(0.0), Some(1.0));
    assert_eq!(f.step(1.0), None);
    let mut emitted = Vec::new();
    f.stop_anim(Some(&mut |_, v| emitted.push(v)), 2.0);
    // The flush advances time by the rate (1 + 2 = 3 >= fterm) so the
    // second key is launched and emitted before the track stops.
    assert_eq!(emitted, vec![2.0]);
    assert_eq!(f.state(), 0);
    // A CON track has nothing to flush.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 2).f32(1.0).wait(3).f32(2.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.step(0.0);
    let mut emitted = Vec::new();
    f.stop_anim(Some(&mut |_, v| emitted.push(v)), 5.0);
    assert!(emitted.is_empty());
    assert_eq!(f.time, 0.0);
}

#[test]
fn req_anim_rewinds_everything_but_keeps_key_ready_bit() {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_SPL, 2).f32(1.0).f32(2.0).wait(3).f32(4.0).f32(5.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    f.step(0.0);
    f.step(1.0);
    f.flags |= FOBJ_FLAG_KEY_PENDING | FOBJ_FLAG_KEY_READY | FOBJ_FLAG_LIN_SLOPE_DIRTY;
    f.req_anim(1.5);
    assert_eq!(f.pos, 0);
    assert_eq!(f.time, 1.5);
    assert_eq!((f.op, f.op_intrp, f.nb_pack, f.fterm), (0, 0, 0, 0));
    assert_eq!((f.p0, f.p1, f.d0, f.d1), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(f.state(), FOBJ_LOAD_DATA0);
    // Only 0x40 is cleared by HSD_FObjReqAnim.
    assert_eq!(f.flags & FOBJ_FLAG_KEY_PENDING, 0);
    assert_eq!(f.flags & FOBJ_FLAG_KEY_READY, FOBJ_FLAG_KEY_READY);
    assert_eq!(f.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, FOBJ_FLAG_LIN_SLOPE_DIRTY);
}

#[test]
fn multiple_packs_and_large_pack_counts() {
    // 10 keys in one pack (needs a continuation byte in the header), then a
    // second pack with a different opcode.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 10);
    for i in 0..10 {
        s.f32(i as f32).wait(1);
    }
    s.pack(HSD_A_OP_LIN, 1).f32(100.0);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    let got = play(&mut f, 12);
    // CON: frame i shows key i. Frame 9 loads the LIN key, but the segment
    // 9 -> 100 is governed by the opcode of the pack its *start* key came
    // from (op_intrp is copied from op before the new header is parsed), so
    // it is still a CON step: 9 until frame 10, then 100.
    for (i, g) in got.iter().enumerate().take(10) {
        assert_eq!(*g, Some(i as f32), "frame {i}");
    }
    assert_eq!(f.op, HSD_A_OP_LIN);
    assert_eq!(f.op_intrp, HSD_A_OP_CON);
    assert_eq!(got[10], Some(100.0));
    assert_eq!(got[11], Some(100.0));
    // The LIN slope was never computed.
    assert_eq!(f.flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, FOBJ_FLAG_LIN_SLOPE_DIRTY);
    assert_eq!(f.d0, 0.0);
}

#[test]
fn length_limits_the_readable_stream() {
    // Bytes past `length` are never read: the track ends after v1.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 3).f32(1.0).wait(1).f32(2.0).wait(1).f32(3.0);
    let bytes = s.finish();
    let desc = FObjDesc {
        length: (1 + 4 + 1 + 4) as u32,
        startframe: 0.0,
        obj_type: TRACK,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: bytes,
    };
    let mut f = FObj::load_desc(&desc);
    f.req_anim(0.0);
    assert_eq!(play(&mut f, 4), vec![Some(1.0), Some(2.0), Some(2.0), Some(2.0)]);
    assert_eq!(f.pos, 10);
}

#[test]
fn unknown_opcode_stalls_the_track() {
    let bytes = [0x07u8, 0, 0, 0, 0];
    let mut f = track(&bytes, FLOAT, FLOAT);
    assert_eq!(f.step(0.0), None);
    assert_eq!(f.state(), FOBJ_LOAD_DATA0);
    assert_eq!(f.op, 7);
    assert_eq!(f.nb_pack, 0);
    assert_eq!(f.pos, 1);
    assert_eq!(f.step(1.0), None);
    assert_eq!(f.time, 1.0);
}

#[test]
fn single_key_stream_hits_uninitialised_path() {
    // One CON key and its wait: the stream runs out before op_intrp is set,
    // so the C forwards an uninitialised value. The port uses the documented
    // stand-in.
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, 1).f32(42.0).wait(3);
    let mut f = track(&s.finish(), FLOAT, FLOAT);
    assert_eq!(f.step(0.0), Some(FOBJ_UNINITIALISED_VALUE));
    assert_eq!(f.op_intrp, HSD_A_OP_NONE);
    assert_eq!(f.p1, 42.0);
}

#[test]
fn hermite_matches_reference_formula() {
    let cases = [
        (0.2f32, 1.0f32, -1.0f32, 2.5f32, 0.75f32, -0.125f32),
        (1.0 / 3.0, 2.0, 10.0, -4.0, -3.0, 1.0),
        (0.125, 3.3, 0.1, 0.2, 0.3, 0.4),
        (1.0, 0.5, 1e-3, 1e3, -1e2, 1e-2),
    ];
    for (a, t, p0, p1, d0, d1) in cases {
        assert_eq!(
            spl_get_helmite(a, t, p0, p1, d0, d1).to_bits(),
            hermite(a, t, p0, p1, d0, d1).to_bits()
        );
    }
    // Endpoints.
    assert_eq!(spl_get_helmite(0.25, 0.0, 3.0, 7.0, 1.0, -1.0), 3.0);
    assert_eq!(spl_get_helmite(0.25, 4.0, 3.0, 7.0, 1.0, -1.0), 7.0);
}

#[test]
fn interpret_anim_all_shares_the_callback() {
    let mut s0 = Stream::new();
    s0.pack(HSD_A_OP_CON, 2).f32(1.0).wait(1).f32(2.0);
    let mut s1 = Stream::new();
    s1.pack(HSD_A_OP_CON, 2).f32(10.0).wait(1).f32(20.0);
    let mut tracks = vec![
        FObj::new(&s0.finish(), 0.0, JObjTrack::TraX as u8, FLOAT, FLOAT),
        FObj::new(&s1.finish(), 0.0, JObjTrack::RotZ as u8, FLOAT, FLOAT),
    ];
    FObj::req_anim_all(&mut tracks, 0.0);
    let mut got = Vec::new();
    FObj::interpret_anim_all(&mut tracks, Some(&mut |t, v| got.push((t, v))), 0.0);
    assert_eq!(got, vec![(5, 1.0), (3, 10.0)]);
    got.clear();
    FObj::interpret_anim_all(&mut tracks, Some(&mut |t, v| got.push((t, v))), 1.0);
    assert_eq!(got, vec![(5, 2.0), (3, 20.0)]);
    FObj::stop_anim_all(&mut tracks, None, 1.0);
    assert!(tracks.iter().all(|t| t.state() == 0));
}
