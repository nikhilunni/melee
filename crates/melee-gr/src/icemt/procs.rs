//! grIm_StageData (gricemt.c): the maps, their collision joints and their
//! gobj procs.
use super::Controller;
use crate::{desc::JointMapping, last::procs::ProcRegistration};

/// The base (spawn and respawn points, camera and blast zone markers).
pub const BASE_MAP: u8 = 0;
/// The ice block model the segments hang on their block bones
/// (grIceMt_801F8CDC: `unk8[7].unk0`).
pub const BLOCK_MODEL: u8 = 7;
/// The scrolling background (stage_callbacks[8], flags 0xC0000000).
pub const BACKGROUND_MAP: u8 = 8;
/// The Versus scroll controller.
pub const CONTROLLER_MAP: u8 = 9;
/// The field segments.
pub const SEGMENT_MAPS: [u8; 6] = [1, 2, 3, 4, 5, 6];

/// stageGObj0_OnInit: the respawn points (bones 18..21, grIm_803B8220[1..4])
/// as their translations, before the map scale.
pub const RESPAWN_POINTS: [(usize, [f32; 2]); 4] = [
    (18, [-8.0, 15.0]),
    (19, [8.0, 15.0]),
    (20, [-24.0, 15.0]),
    (21, [24.0, 15.0]),
];

/// Each segment's spawn point bones (`icemt1.x4..x10`, handed to
/// Ground_801C2D0C(0..3) by grIceMt_801F98A8), its block bones
/// (grIceMt_801F8CDC's lists) and its gobj proc.
pub struct Segment {
    pub map: u8,
    pub spawn_bones: [usize; 4],
    pub block_bones: &'static [usize],
    pub proc_name: &'static str,
    pub proc_address: u32,
}

pub const SEGMENTS: [Segment; 6] = [
    // stageGObj1_OnInit (0x801F7D94): grIm_804DB588.
    Segment {
        map: 1,
        spawn_bones: [3, 4, 5, 6],
        block_bones: &[1, 2],
        proc_name: "stageGObj1_GObjProc",
        proc_address: 0x801F_7EE8,
    },
    // stageGObj2_OnInit (0x801F7F70): grIm_804DB58C.
    Segment {
        map: 2,
        spawn_bones: [6, 7, 8, 9],
        block_bones: &[1, 2],
        proc_name: "stageGObj2_GObjProc",
        proc_address: 0x801F_815C,
    },
    // stageGObj3_OnInit (0x801F8208): grIm_804DB590, grIm_804DB594.
    Segment {
        map: 3,
        spawn_bones: [5, 6, 7, 8],
        block_bones: &[1, 2, 3, 4],
        proc_name: "stageGObj3_GObjProc",
        proc_address: 0x801F_8364,
    },
    // stageGObj4_OnInit (0x801F83EC): grIm_804DB598, grIm_804DB59C.
    Segment {
        map: 4,
        spawn_bones: [8, 9, 10, 11],
        block_bones: &[1, 2, 3, 4],
        proc_name: "stageGObj4_GObjProc",
        proc_address: 0x801F_85C4,
    },
    // stageGObj5_OnInit (0x801F865C): grIm_803B825C.
    Segment {
        map: 5,
        spawn_bones: [6, 7, 8, 9],
        block_bones: &[1, 2, 3, 4, 5],
        proc_name: "stageGObj5_GObjProc",
        proc_address: 0x801F_87C8,
    },
    // stageGObj6_OnInit (0x801F8850): grIm_804DB5A0, grIm_804DB5A4.
    Segment {
        map: 6,
        spawn_bones: [5, 6, 7, 8],
        block_bones: &[1, 2, 3, 4],
        proc_name: "stageGObj6_GObjProc",
        proc_address: 0x801F_89AC,
    },
];

pub fn segment(map: u8) -> &'static Segment {
    &SEGMENTS[usize::from(map) - 1]
}

/// A platform that gives way under a fighter (grIceMt_801F91EC): the
/// bones whose animation shakes and drops it, its collision joint, and
/// the frames at which the joint leaves and returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapseSite {
    pub bones: (usize, Option<usize>),
    pub joint: i32,
    pub remove_at: i16,
    pub add_at: i16,
}

/// stageGObj2_OnInit's two (fn_801F9338, fn_801F9448: joints 0x25 and 38
/// on bones 4 and 5) and stageGObj4_OnInit's one (fn_801F9558: joint 117
/// on bones 7 and 5); each gives way at 265 frames and returns at 638.
pub const COLLAPSES: [(u8, CollapseSite); 3] = [
    (
        2,
        CollapseSite {
            bones: (4, None),
            joint: 0x25,
            remove_at: 265,
            add_at: 638,
        },
    ),
    (
        2,
        CollapseSite {
            bones: (5, None),
            joint: 38,
            remove_at: 265,
            add_at: 638,
        },
    ),
    (
        4,
        CollapseSite {
            bones: (7, Some(5)),
            joint: 117,
            remove_at: 265,
            add_at: 638,
        },
    ),
];

const fn joint(joint_index: i16, target_index: i16, extra: i16) -> JointMapping {
    JointMapping {
        joint_index,
        target_index,
        extra,
    }
}

/// grIm_803E40B0 (grIm_StageData.joints): {joint, map, bone}.
pub const STAGE_JOINTS: [JointMapping; 195] = {
    // Bone 0 unless listed: joint 36 (map 2) on bone 3, joints 113..115
    // (map 4) on bones 2..4.
    const RANGES: [(i16, i16, i16); 13] = [
        (0, 7, 1),
        (10, 31, 1),
        (32, 33, 2),
        (36, 36, 2),
        (39, 66, 2),
        (67, 69, 3),
        (74, 101, 3),
        (102, 111, 4),
        (113, 115, 4),
        (118, 142, 4),
        (143, 146, 5),
        (152, 179, 5),
        (180, 184, 6),
    ];
    let mut table = [joint(0, 0, 0); 195];
    let mut n = 0;
    let mut r = 0;
    while r < RANGES.len() {
        let (first, last, map) = RANGES[r];
        let mut j = first;
        while j <= last {
            let bone = match j {
                36 => 3,
                113 => 2,
                114 => 3,
                115 => 4,
                _ => 0,
            };
            table[n] = joint(j, map, bone);
            n += 1;
            j += 1;
        }
        r += 1;
    }
    let mut j = 189;
    while j <= 216 {
        table[n] = joint(j, 6, 0);
        n += 1;
        j += 1;
    }
    assert!(n == 195);
    table
};

/// The stage_callbacks gobj procs (s_link 4).
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        BASE_MAP => ("stageGObj0_GObjProc", 0x801F_75E4),
        BACKGROUND_MAP => ("stageGObj8_GObjProc", 0x801F_8B10),
        CONTROLLER_MAP => ("stageGObj9_GObjProc", 0x801F_7728),
        1..=6 => {
            let s = segment(map);
            (s.proc_name, s.proc_address)
        }
        _ => unreachable!("Icicle Mountain map {map}"),
    }
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct IceMtStage {
    pub parameters: super::Parameters,
    pub controller: Controller,
    /// Each live segment (`icemt1` vars), by map.
    pub segments: [Option<SegmentState>; 6],
    /// The GObj order as created at setup: 0, 8, 9, then the two segments.
    pub setup_order: [u8; 5],
    /// Ground_804D6950: a segment's joints Ground_801C4A08 disabled.
    pub released: [bool; 7],
    /// grIceMt_801F9668's speed for the background (`icemt_bg.x0`).
    pub background_speed: f32,
    pub lights: Vec<crate::battle::lights::Light>,
}

/// A live field segment (`grIceMt_GObj1_GroundVars`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SegmentState {
    /// x0_b0: the segment holds the view.
    pub current: bool,
    /// x0_b1: its spawn points are due to Ground_801C2D0C.
    pub markers_due: bool,
    /// x34[]: its blocks' material items.
    pub blocks: [Option<u32>; 5],
    /// The platforms that give way (map 2's two, map 4's one).
    pub collapses: [Collapse; 2],
}

/// A platform's countdown (grIceMt_801F91EC's seven halfwords).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Collapse {
    pub active: bool,
    pub timer: i16,
}

impl IceMtStage {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        let row = |s_link, p_link, map_id, callback, address| ProcRegistration {
            s_link,
            p_link,
            p_priority: 0,
            map_id,
            callback,
            address,
        };
        let mut rows = vec![
            row(0, 3, None, "Ground_801C461C", 0x801C461C),
            row(0, 4, None, "fn_801CADBC", 0x801CADBC),
        ];
        for map in self.setup_order {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        for map in self.setup_order {
            let (name, address) = map_callback(map);
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
