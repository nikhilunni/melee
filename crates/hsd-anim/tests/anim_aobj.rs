//! AObj frame counter, loop and end-frame behaviour, flag handling and the
//! end-callback counters.

mod anim_common;

use anim_common::Stream;
use hsd_anim::aobj::*;
use hsd_anim::fobj::*;

const FLOAT: u8 = HSD_A_FRAC_FLOAT;

fn lin_desc(track: u8, v0: f32, wait: u32, v1: f32) -> FObjDesc {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_LIN, 2).f32(v0).wait(wait).f32(v1);
    FObjDesc {
        length: s.bytes.len() as u32,
        startframe: 0.0,
        obj_type: track,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: s.finish(),
    }
}

fn con_desc(track: u8, keys: &[(f32, u32)]) -> FObjDesc {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_CON, keys.len() as u32);
    for (v, w) in keys {
        s.f32(*v).wait(*w);
    }
    FObjDesc {
        length: s.bytes.len() as u32,
        startframe: 0.0,
        obj_type: track,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: s.finish(),
    }
}

fn key_desc(track: u8, v0: f32, wait: u32, v1: f32) -> FObjDesc {
    let mut s = Stream::new();
    s.pack(HSD_A_OP_KEY, 2).f32(v0).wait(wait).f32(v1);
    FObjDesc {
        length: s.bytes.len() as u32,
        startframe: 0.0,
        obj_type: track,
        frac_value: FLOAT,
        frac_slope: FLOAT,
        ad: s.finish(),
    }
}

/// One `HSD_AObjInterpretAnim` call; returns the `(track, value)` pairs.
fn step(aobj: &mut AObj, cb: &mut AObjEndCallback) -> Vec<(u8, f32)> {
    let mut got = Vec::new();
    aobj.interpret_anim(&mut |t, v| got.push((t, v)), cb);
    got
}

#[test]
fn alloc_defaults() {
    let a = AObj::alloc();
    assert_eq!(a.flags, AOBJ_NO_ANIM);
    assert_eq!(a.framerate, 1.0);
    assert_eq!(a.curr_frame, 0.0);
    assert_eq!(a.end_frame, 0.0);
    assert_eq!(a.rewind_frame, 0.0);
    assert!(a.fobj.is_empty());
    assert_eq!(AObj::default(), a);
}

#[test]
fn load_desc_masks_flags() {
    let desc = AObjDesc {
        flags: 0xFFFF_FFFF,
        end_frame: 12.5,
        fobjdesc: vec![lin_desc(5, 0.0, 4, 1.0), lin_desc(6, 0.0, 4, 1.0)],
        obj_id: 0,
    };
    let a = AObj::load_desc(&desc);
    assert_eq!(a.flags, AOBJ_NO_ANIM | AOBJ_LOOP | AOBJ_NO_UPDATE);
    assert_eq!(a.end_frame, 12.5);
    assert_eq!(a.end_frame(), 12.5);
    assert_eq!(a.rewind_frame, 0.0);
    assert_eq!(a.fobj.len(), 2);
    assert_eq!(a.fobj[1].obj_type, 6);
    assert_eq!(a.fobj[0].state(), 0);

    let mut a = a;
    a.clear_flags(0xFFFF_FFFF);
    assert_eq!(a.flags, AOBJ_NO_ANIM);
    a.set_flags(AOBJ_LOOP | AOBJ_FIRST_PLAY | AOBJ_REWINDED);
    assert_eq!(a.flags(), AOBJ_NO_ANIM | AOBJ_LOOP);
}

#[test]
fn plays_to_end_frame_and_stops() {
    let (v0, v1) = (0.3f32, 1.7f32);
    let desc = AObjDesc {
        flags: 0,
        end_frame: 4.0,
        fobjdesc: vec![lin_desc(JObjTrack::TraX as u8, v0, 4, v1)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();

    // Not requested yet: nothing happens, no counters.
    assert!(step(&mut a, &mut cb).is_empty());
    assert_eq!(cb, AObjEndCallback::default());

    a.req_anim(0.0);
    assert_eq!(a.flags & AOBJ_NO_ANIM, 0);
    assert_eq!(a.flags & AOBJ_FIRST_PLAY, AOBJ_FIRST_PLAY);
    assert_eq!(a.fobj[0].state(), FOBJ_LOAD_DATA0);

    let d0 = (v1 - v0) / 4.0f32;
    // First play: rate 0, frame stays 0.
    assert_eq!(step(&mut a, &mut cb), vec![(5, d0 * 0.0 + v0)]);
    assert_eq!(a.curr_frame(), 0.0);
    assert_eq!(a.flags & AOBJ_FIRST_PLAY, 0);
    assert_eq!(cb, AObjEndCallback { ended: 0, running: 1 });

    for i in 1..4 {
        assert_eq!(step(&mut a, &mut cb), vec![(5, d0 * i as f32 + v0)], "frame {i}");
        assert_eq!(a.curr_frame, i as f32);
    }
    assert_eq!(cb, AObjEndCallback { ended: 0, running: 4 });

    // Frame 4 reaches end_frame: the value is still pushed, then the tracks
    // are stopped and NO_ANIM set; the pass counts it as ended.
    assert_eq!(step(&mut a, &mut cb), vec![(5, d0 * 4.0 + v0)]);
    assert_eq!(a.curr_frame, 4.0);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
    assert_eq!(a.fobj[0].state(), 0);
    assert_eq!(cb, AObjEndCallback { ended: 1, running: 4 });
    assert!(!cb.should_invoke());

    // Stopped: no update, no counters, frame frozen.
    assert!(step(&mut a, &mut cb).is_empty());
    assert_eq!(a.curr_frame, 4.0);
    assert_eq!(cb, AObjEndCallback { ended: 1, running: 4 });

    cb.init();
    assert!(step(&mut a, &mut cb).is_empty());
    assert_eq!(cb, AObjEndCallback::default());
    assert!(!cb.should_invoke());
    cb.ended = 1;
    assert!(cb.should_invoke());
}

#[test]
fn framerate_scales_the_step() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 10.0,
        fobjdesc: vec![lin_desc(5, 0.0, 10, 10.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    a.set_rate(2.5);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 0.0)]);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 2.5)]);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 5.0)]);
    assert_eq!(a.curr_frame, 5.0);
    assert_eq!(a.fobj[0].time, 5.0);
    // Overshoot: 7.5 then 10.0 == end_frame stops.
    step(&mut a, &mut cb);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 10.0)]);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
}

#[test]
fn req_anim_at_a_later_frame() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 10.0,
        fobjdesc: vec![lin_desc(5, 0.0, 10, 10.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(3.0);
    assert_eq!(a.curr_frame, 3.0);
    assert_eq!(a.fobj[0].time, 3.0);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 3.0)]);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 4.0)]);
}

#[test]
fn end_flush_emits_pending_key() {
    // KEY v0 at 0, v1 at 3; end_frame 2. Frame 2 does not reach the key,
    // but the stop flush advances the track by another framerate and the
    // key at 3 is launched and emitted before the track stops.
    let desc = AObjDesc {
        flags: 0,
        end_frame: 2.0,
        fobjdesc: vec![key_desc(11, 1.0, 3, 2.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    assert_eq!(step(&mut a, &mut cb), vec![(11, 1.0)]);
    assert!(step(&mut a, &mut cb).is_empty());
    assert_eq!(step(&mut a, &mut cb), vec![(11, 2.0)]);
    assert_eq!(a.fobj[0].time, 3.0);
    assert_eq!(a.fobj[0].state(), 0);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
}

#[test]
fn stop_anim_flushes_and_sets_no_anim() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 10.0,
        fobjdesc: vec![key_desc(11, 1.0, 2, 2.0), con_desc(5, &[(4.0, 2), (5.0, 0)])],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    assert_eq!(step(&mut a, &mut cb), vec![(11, 1.0), (5, 4.0)]);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 4.0)]);
    let mut got = Vec::new();
    a.stop_anim(Some(&mut |t, v| got.push((t, v))));
    assert_eq!(got, vec![(11, 2.0)]);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
    assert!(a.fobj.iter().all(|f| f.state() == 0));
    // stop with no callback still stops.
    a.req_anim(0.0);
    a.stop_anim(None);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
}

#[test]
fn loop_rewinds_with_fmodf() {
    let keys = [(0.0f32, 2u32), (1.0, 2), (2.0, 0)];
    let desc = AObjDesc {
        flags: AOBJ_LOOP,
        end_frame: 4.0,
        fobjdesc: vec![con_desc(5, &keys)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    let mut seq = Vec::new();
    for _ in 0..9 {
        let got = step(&mut a, &mut cb);
        assert_eq!(got.len(), 1);
        seq.push((a.curr_frame, got[0].1, a.flags & AOBJ_REWINDED != 0));
    }
    assert_eq!(
        seq,
        vec![
            (0.0, 0.0, false),
            (1.0, 0.0, false),
            (2.0, 1.0, false),
            (3.0, 1.0, false),
            // Frame 4 == end_frame: fmodf(4, 4) = 0, rewound, evaluated at 0.
            (0.0, 0.0, true),
            (1.0, 0.0, false),
            (2.0, 1.0, false),
            (3.0, 1.0, false),
            (0.0, 0.0, true),
        ]
    );
    // Looping never ends.
    assert_eq!(cb, AObjEndCallback { ended: 0, running: 9 });
    assert_eq!(a.flags & AOBJ_NO_ANIM, 0);
}

#[test]
fn loop_rewind_keeps_fractional_remainder() {
    let desc = AObjDesc {
        flags: AOBJ_LOOP,
        end_frame: 4.0,
        fobjdesc: vec![lin_desc(5, 0.0, 4, 4.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    a.set_rate(1.5);
    a.set_rewind_frame(1.0);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    // 0, 1.5, 3.0, then 4.5 >= 4: x = 3.5, y = 3, fmodf = 0.5, + 1 = 1.5.
    step(&mut a, &mut cb);
    step(&mut a, &mut cb);
    step(&mut a, &mut cb);
    let got = step(&mut a, &mut cb);
    let want = gekko_math::msl::fmodf(4.5 - 1.0, 4.0 - 1.0) + 1.0;
    assert_eq!(a.curr_frame, want);
    assert_eq!(a.curr_frame, 1.5);
    assert_eq!(a.fobj[0].time, 1.5);
    assert_eq!(a.flags & AOBJ_REWINDED, AOBJ_REWINDED);
    // The track was re-requested at 1.5 and evaluated with rate 0.
    assert_eq!(got, vec![(5, 1.5)]);
    // Next step advances again and clears REWINDED.
    assert_eq!(step(&mut a, &mut cb), vec![(5, 3.0)]);
    assert_eq!(a.flags & AOBJ_REWINDED, 0);
}

#[test]
fn loop_with_rewind_frame_past_end_clamps() {
    let desc = AObjDesc {
        flags: AOBJ_LOOP,
        end_frame: 2.0,
        fobjdesc: vec![lin_desc(5, 0.0, 2, 2.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    a.set_rewind_frame(2.0);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    step(&mut a, &mut cb);
    step(&mut a, &mut cb);
    // curr_frame 2 >= end 2 but rewind 2 is not < end: clamp, rate 0.
    let got = step(&mut a, &mut cb);
    assert_eq!(a.curr_frame, 2.0);
    assert_eq!(a.flags & AOBJ_REWINDED, AOBJ_REWINDED);
    // The track is neither re-requested nor advanced (rate 0), so it stays
    // frozen at the previous frame's time and value: curr_frame and the
    // tracks disagree from here on.
    assert_eq!(a.fobj[0].time, 1.0);
    assert_eq!(got, vec![(5, 1.0)]);
    let got = step(&mut a, &mut cb);
    // curr_frame advanced to 3 then clamped back to 2; track time stays 1.
    assert_eq!(a.curr_frame, 2.0);
    assert_eq!(a.fobj[0].time, 1.0);
    assert_eq!(got, vec![(5, 1.0)]);
    assert_eq!(a.flags & AOBJ_NO_ANIM, 0);
}

#[test]
fn loop_rewind_flushes_keys_with_the_step_rate() {
    // KEY v0 at 0, v1 at 3; end 3, loop. Frame 3: stop flush with rate 1
    // moves the track to 3 and emits v1, then rewind emits v0 at frame 0.
    let desc = AObjDesc {
        flags: AOBJ_LOOP,
        end_frame: 3.0,
        fobjdesc: vec![key_desc(11, 1.0, 3, 2.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    assert_eq!(step(&mut a, &mut cb), vec![(11, 1.0)]);
    assert!(step(&mut a, &mut cb).is_empty());
    assert!(step(&mut a, &mut cb).is_empty());
    assert_eq!(step(&mut a, &mut cb), vec![(11, 2.0), (11, 1.0)]);
    assert_eq!(a.curr_frame, 0.0);
}

#[test]
fn no_update_advances_without_callbacks() {
    let desc = AObjDesc {
        flags: AOBJ_NO_UPDATE,
        end_frame: 3.0,
        fobjdesc: vec![lin_desc(5, 0.0, 4, 4.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    for _ in 0..3 {
        assert!(step(&mut a, &mut cb).is_empty());
    }
    assert_eq!(a.curr_frame, 2.0);
    assert_eq!(a.fobj[0].time, 2.0);
    // The LIN slope was never computed because the callback was null.
    assert_eq!(a.fobj[0].flags & FOBJ_FLAG_LIN_SLOPE_DIRTY, FOBJ_FLAG_LIN_SLOPE_DIRTY);
    assert_eq!(cb, AObjEndCallback { ended: 0, running: 3 });
    // Clearing NO_UPDATE resumes the callbacks; the dirty slope is computed
    // now, at frame 3, and the end stop happens in the same step.
    a.clear_flags(AOBJ_NO_UPDATE);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 3.0)]);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
    assert_eq!(cb, AObjEndCallback { ended: 1, running: 3 });
}

#[test]
fn no_update_end_flush_still_uses_the_callback() {
    // The end-of-animation HSD_FObjStopAnimAll passes update_func even when
    // AOBJ_NO_UPDATE is set, so a pending KEY is delivered.
    let desc = AObjDesc {
        flags: AOBJ_NO_UPDATE,
        end_frame: 1.0,
        fobjdesc: vec![key_desc(11, 1.0, 2, 2.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    assert!(step(&mut a, &mut cb).is_empty());
    // Frame 1 == end: flush interprets with rate 1 -> time 2 >= wait 2, key
    // launched. The first key's 0x80 was never cleared (null callback), so
    // it is emitted in state 3 and the second key in state 6.
    assert_eq!(step(&mut a, &mut cb), vec![(11, 1.0), (11, 2.0)]);
}

#[test]
fn set_current_frame_seeks_a_running_animation() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 10.0,
        fobjdesc: vec![lin_desc(5, 0.0, 10, 10.0)],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    // Stopped: ignored.
    a.set_current_frame(5.0);
    assert_eq!(a.curr_frame, 0.0);
    assert_eq!(a.fobj[0].state(), 0);

    a.req_anim(0.0);
    step(&mut a, &mut cb);
    step(&mut a, &mut cb);
    a.set_current_frame(7.0);
    assert_eq!(a.curr_frame, 7.0);
    assert_eq!(a.flags & AOBJ_FIRST_PLAY, AOBJ_FIRST_PLAY);
    assert_eq!(a.fobj[0].time, 7.0);
    assert_eq!(a.fobj[0].state(), FOBJ_LOAD_DATA0);
    // FIRST_PLAY: evaluated at 7 without advancing.
    assert_eq!(step(&mut a, &mut cb), vec![(5, 7.0)]);
    assert_eq!(a.curr_frame, 7.0);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 8.0)]);
}

#[test]
fn req_anim_restarts_a_stopped_animation() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 1.0,
        fobjdesc: vec![con_desc(5, &[(1.0, 1), (2.0, 0)])],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    step(&mut a, &mut cb);
    step(&mut a, &mut cb);
    assert_eq!(a.flags & AOBJ_NO_ANIM, AOBJ_NO_ANIM);
    a.req_anim(0.0);
    assert_eq!(a.flags & AOBJ_NO_ANIM, 0);
    assert_eq!(step(&mut a, &mut cb), vec![(5, 1.0)]);
}

#[test]
fn tracks_are_updated_in_list_order_with_their_ids() {
    let desc = AObjDesc {
        flags: 0,
        end_frame: 5.0,
        fobjdesc: vec![
            con_desc(JObjTrack::TraX as u8, &[(1.0, 5), (2.0, 0)]),
            con_desc(JObjTrack::RotY as u8, &[(3.0, 5), (4.0, 0)]),
            con_desc(JObjTrack::ScaZ as u8, &[(5.0, 5), (6.0, 0)]),
        ],
        obj_id: 0,
    };
    let mut a = AObj::load_desc(&desc);
    let mut cb = AObjEndCallback::default();
    a.req_anim(0.0);
    let got = step(&mut a, &mut cb);
    assert_eq!(got, vec![(5, 1.0), (2, 3.0), (10, 5.0)]);
    let ids: Vec<_> = got.iter().map(|(t, _)| JObjTrack::from_u8(*t).unwrap()).collect();
    assert_eq!(ids, vec![JObjTrack::TraX, JObjTrack::RotY, JObjTrack::ScaZ]);
}
