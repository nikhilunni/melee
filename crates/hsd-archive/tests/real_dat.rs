//! Parse real archives from the extracted disc.
//!
//! These tests read `harness/roms/files/` (produced by
//! `harness/extract_fst.py`, gitignored) and skip with a note when it is
//! absent so the gate stays green on machines without the disc. Every
//! number asserted here was measured on the NTSC 1.02 disc and is
//! recorded in `docs/DISC.md`; if one changes, the parser changed, not
//! the disc.
//!
//! On-disc struct layouts that belong to Melee rather than HSD
//! (`ftData`, `Fighter_WaitAnimData`, `MapCollData`, `UnkStageDat`) are
//! spelled out as local offset constants with their decomp source, since
//! this crate is the one place allowed to know them.

use std::path::{Path, PathBuf};

use hsd_archive::desc::{read_public_figatree, read_public_jobj, JObjDesc, MatAnimJoint};
use hsd_archive::{Archive, ArchiveHeader, Reader};

/// `harness/roms/files/`, or `None` (after printing why) if the disc has
/// not been extracted on this machine.
fn disc_files() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if dir.is_dir() {
        Some(dir)
    } else {
        eprintln!(
            "skipping: {} not found; run `cd harness && uv run python extract_fst.py roms/GALE01.iso`",
            dir.display()
        );
        None
    }
}

/// Early-return from a test when the disc is absent.
macro_rules! require_disc {
    () => {
        match disc_files() {
            Some(dir) => dir,
            None => return,
        }
    };
}

fn read(dir: &Path, name: &str) -> Vec<u8> {
    std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("reading {name}: {e}"))
}

fn parse(dir: &Path, name: &str) -> Archive {
    let bytes = read(dir, name);
    let archive = Archive::parse(&bytes).unwrap_or_else(|e| panic!("parsing {name}: {e}"));
    assert_eq!(
        archive.header().file_size as usize,
        bytes.len(),
        "{name}: header file_size vs real length"
    );
    archive
}

/// Every slot the relocation table names must itself hold a data offset,
/// i.e. every pointer must point inside the data section.
fn assert_relocs_point_inside_data(name: &str, archive: &Archive) {
    let data_size = archive.header().data_size;
    let reader = archive.reader();
    for &slot in archive.reloc_targets() {
        let target = reader.u32(slot).unwrap();
        assert!(
            target < data_size,
            "{name}: relocation slot {slot:#x} holds {target:#x}, past data_size {data_size:#x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Whole-file parsing
// ---------------------------------------------------------------------------

struct Expected {
    name: &'static str,
    file_size: u32,
    nb_reloc: u32,
    version: [u8; 4],
    publics: &'static [&'static str],
}

/// The four stand-alone archives. `PlFxAJ.dat` is not one; see
/// [`fox_animation_table_indexes_aj_sub_archives`].
const ARCHIVES: &[Expected] = &[
    Expected {
        name: "PlFxNr.dat",
        file_size: 362_978,
        nb_reloc: 1792,
        version: [0; 4],
        publics: &["PlyFox5K_Share_joint", "PlyFox5K_Share_matanim_joint"],
    },
    Expected {
        name: "PlFx.dat",
        file_size: 259_850,
        nb_reloc: 3710,
        version: *b"001B",
        publics: &["ftDataFox"],
    },
    Expected {
        name: "PlCo.dat",
        file_size: 149_101,
        nb_reloc: 805,
        version: *b"001B",
        publics: &["ftLoadCommonData"],
    },
    Expected {
        name: "GrNLa.dat",
        file_size: 611_125,
        nb_reloc: 7999,
        version: *b"001B",
        publics: &[
            "ALDYakuAll",
            "GrdLastCloud2_I8_image",
            "GrdLastGround2_CMPR_image",
            "GrdLastLine1_I4_image",
            "GrdLastLine2_I4_image",
            "GrdLastMilkyWay1_I8_image",
            "GrdLastMilkyWay2_I8_image",
            "GrdLastRipple1_CMPR_image",
            "GrdLastRipple3_I8_image",
            "GrdLastStar1_I4_image",
            "GrdLastStar2_I8_image",
            "GrdLastStar3_I8_image",
            "GrdLastStar4_I4_image",
            "GrdLastStarDust1_I4_image",
            "GrdLastTile2_CMPR_image",
            "GrdLastWall1_RGBA8_image",
            "GrdLastYuka0_I4_image",
            "GrdLastYuka1_I4_image",
            "coll_data",
            "grGroundParam",
            "itemdata",
            "map_head",
            "map_plit",
            "map_ptcl",
            "map_texg",
            "quake_model_set",
            "yakumono_param",
        ],
    },
];

#[test]
fn retail_archives_parse_with_expected_tables() {
    let dir = require_disc!();
    for exp in ARCHIVES {
        let archive = parse(&dir, exp.name);
        let header = archive.header();
        assert_eq!(header.file_size, exp.file_size, "{}: file_size", exp.name);
        assert_eq!(header.nb_reloc, exp.nb_reloc, "{}: nb_reloc", exp.name);
        assert_eq!(
            header.nb_extern, 0,
            "{}: retail files have no externs",
            exp.name
        );
        assert_eq!(header.version, exp.version, "{}: version bytes", exp.name);
        assert_relocs_point_inside_data(exp.name, &archive);

        let names: Vec<&str> = archive.publics().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, exp.publics, "{}: public symbol names", exp.name);
    }
}

/// A relocation slot that holds `0` is a real link to the struct at data
/// offset 0, not a null pointer. Retail files do this: `ftDataFox.x8`
/// (the `FtPartsDesc`) lives at offset 0 of `PlFx.dat`'s data section.
/// This is why [`Archive::link`] consults the relocation table instead of
/// the `0 == null` heuristic of [`Reader::offset`].
#[test]
fn relocated_zero_is_a_link_to_offset_zero() {
    let dir = require_disc!();
    let archive = parse(&dir, "PlFx.dat");
    let ft_data = archive.public("ftDataFox").unwrap();
    let parts_desc_slot = ft_data + ft_data_off::X8_PARTS_DESC;
    assert_eq!(archive.link(parts_desc_slot).unwrap(), Some(0));
    assert_eq!(archive.reader().offset(parts_desc_slot).unwrap(), None);

    // The count of such slots per file, for the record.
    let zero_links = |archive: &Archive| {
        archive
            .reloc_targets()
            .iter()
            .filter(|&&slot| archive.reader().u32(slot).unwrap() == 0)
            .count()
    };
    assert_eq!(zero_links(&archive), 1);
    assert_eq!(zero_links(&parse(&dir, "PlFxNr.dat")), 3);
    assert_eq!(zero_links(&parse(&dir, "PlCo.dat")), 4);
    assert_eq!(zero_links(&parse(&dir, "GrNLa.dat")), 1);
}

// ---------------------------------------------------------------------------
// PlFxNr.dat: Fox's skeleton
// ---------------------------------------------------------------------------

/// Root joint of Fox's neutral costume, `ftFx_Init_CostumeStrings[0]`
/// (`ftfox.c`). Every costume file `PlFx{Nr,Or,La,Gr}.dat` exports the
/// same two names.
const FOX_ROOT_JOINT: &str = "PlyFox5K_Share_joint";
const FOX_MATANIM_JOINT: &str = "PlyFox5K_Share_matanim_joint";

/// Deepest `child` nesting below `root`, with `root` at depth 0.
fn joint_depth(root: &JObjDesc) -> usize {
    root.children()
        .map(|c| 1 + joint_depth(c))
        .max()
        .unwrap_or(0)
}

/// Nodes in the sub-tree rooted at `node`, its own `next` siblings excluded.
fn matanim_node_count(node: &MatAnimJoint) -> usize {
    1 + node.children().map(matanim_node_count).sum::<usize>()
}

#[test]
fn fox_skeleton_has_73_joints_14_levels_deep() {
    let dir = require_disc!();
    let archive = parse(&dir, "PlFxNr.dat");
    assert_eq!(archive.public(FOX_ROOT_JOINT), Some(0x1c668));

    let root = read_public_jobj(&archive, FOX_ROOT_JOINT).unwrap();
    assert!(root.next.is_none(), "the root joint has no sibling");

    let joints = root.descendants();
    assert_eq!(joints.len(), 73, "joint count");
    assert_eq!(joint_depth(&root), 13, "child depth below the root");

    // What the skeleton uses and does not use.
    assert!(joints.iter().all(|j| j.class_name.is_none()));
    assert!(joints.iter().all(|j| !j.is_instance()));
    assert!(joints.iter().all(|j| j.robjdesc.is_none()));
    assert_eq!(
        joints.iter().filter(|j| j.mtx.is_some()).count(),
        65,
        "envelope matrices"
    );
    assert_eq!(
        joints.iter().filter(|j| j.u.dobj().is_some()).count(),
        4,
        "joints carrying geometry"
    );
    let mut flags: Vec<u32> = joints.iter().map(|j| j.flags).collect();
    flags.sort_unstable();
    flags.dedup();
    assert_eq!(
        flags,
        [0x8, 0x9, 0x4_0089, 0x1000_0009, 0x1005_0089, 0x1005_018e],
        "distinct JOBJ flag words"
    );

    // The material animation tree mirrors the skeleton one node per joint.
    let matanim = MatAnimJoint::read(&archive, archive.public(FOX_MATANIM_JOINT).unwrap()).unwrap();
    assert_eq!(matanim_node_count(&matanim), 73);
}

// ---------------------------------------------------------------------------
// PlFx.dat + PlFxAJ.dat: the animation table and the motion file it indexes
// ---------------------------------------------------------------------------

/// Fields of `struct ftData` (`ft/types.h:612`), the root exported as
/// `ftData<Name>` by every `Pl<XX>.dat`.
mod ft_data_off {
    /// `+0x08 struct ftData_x8* x8`: the `FtPartsDesc`, at data offset 0.
    pub const X8_PARTS_DESC: u32 = 0x08;
    /// `+0x0C struct Fighter_WaitAnimData* xC`: the animation table, one
    /// entry per `MotionState`, indexed by the motion-state id.
    pub const XC_ANIM_TABLE: u32 = 0x0C;
    /// `+0x14 struct Fighter_WaitAnimData* x14`: the demo animation table
    /// (win/lose/intro/ending poses), indexing the `Pl<XX>D*AJ.dat` files.
    pub const X14_DEMO_ANIM_TABLE: u32 = 0x14;
}

/// `struct Fighter_WaitAnimData` (`ft/types.h:885`), 0x18 bytes. How
/// `ftData_80085A14` / `ftData_80085E50` (ftdata.c) use it: the whole AJ
/// file is loaded flat, then per motion state `aj_size` bytes at
/// `aj_offset` are copied into a fighter-owned 0x8000-byte buffer, handed
/// to `HSD_ArchiveParse`, and `figatree_name` is looked up in the result.
mod anim_entry_off {
    /// `+0x00 char* x0`: public symbol of the `FigaTree` inside the
    /// sub-archive, e.g. `PlyFox5K_Share_ACTION_Wait1_figatree`.
    pub const FIGATREE_NAME: u32 = 0x00;
    /// `+0x04 s32 x4`: byte offset of the sub-archive in the AJ file.
    pub const AJ_OFFSET: u32 = 0x04;
    /// `+0x08 s32 x8`: byte length of the sub-archive; 0 for a motion
    /// state with no animation. Asserted `<= 0x8000` by retail.
    pub const AJ_SIZE: u32 = 0x08;
    /// `+0x0C union CmdUnion* xC`: the subaction script.
    pub const SCRIPT: u32 = 0x0C;
    /// `+0x10 s32 x10_animCurrFlags`.
    pub const FLAGS: u32 = 0x10;
    /// `+0x14 u32 x14`: filled at runtime with the RAM address of the
    /// sub-archive; always 0 on disc.
    pub const RUNTIME_PTR: u32 = 0x14;
    pub const SIZE: u32 = 0x18;
}

/// `ftData_Table_Unk0[FTKIND_FOX].count` (`ftdata.c:255`): the number of
/// entries in Fox's animation table. Not stored in the file.
const FOX_ANIM_COUNT: u32 = 327;

/// Retail's ceiling on one sub-archive, `HSD_ASSERTREPORT(0x9AF, ...)`
/// in `ftData_80085A14` and the size of `Fighter_x59C_t`.
const MAX_AJ_SUB_ARCHIVE: u32 = 0x8000;

/// Sub-archives in an AJ file start on 32-byte boundaries (DVD read
/// alignment; `OSRoundUp32B` in `ftData_80085CD8`).
const AJ_ALIGN: u32 = 32;

/// One row of the animation table as stored on disc.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AnimEntry {
    index: u32,
    figatree_name: Option<String>,
    aj_offset: u32,
    aj_size: u32,
    script: Option<u32>,
    flags: u32,
}

fn read_anim_table(archive: &Archive, table: u32, count: u32) -> Vec<AnimEntry> {
    let reader = archive.reader();
    (0..count)
        .map(|index| {
            let base = table + index * anim_entry_off::SIZE;
            let name = archive
                .link(base + anim_entry_off::FIGATREE_NAME)
                .unwrap()
                .map(|off| reader.cstr(off).unwrap().to_owned());
            assert_eq!(reader.u32(base + anim_entry_off::RUNTIME_PTR).unwrap(), 0);
            AnimEntry {
                index,
                figatree_name: name,
                aj_offset: reader.u32(base + anim_entry_off::AJ_OFFSET).unwrap(),
                aj_size: reader.u32(base + anim_entry_off::AJ_SIZE).unwrap(),
                script: archive.link(base + anim_entry_off::SCRIPT).unwrap(),
                flags: reader.u32(base + anim_entry_off::FLAGS).unwrap(),
            }
        })
        .collect()
}

/// Walk an AJ file header by header: each sub-archive's `file_size` says
/// where it ends, and the next starts at the following 32-byte boundary.
/// Returns `(offset, file_size, public name)` per sub-archive.
fn scan_aj_file(aj: &[u8]) -> Vec<(u32, u32, String)> {
    let mut out = Vec::new();
    let mut cursor = 0u32;
    while (cursor as usize) < aj.len() {
        let header = ArchiveHeader::parse(&aj[cursor as usize..]).unwrap();
        let end = cursor + header.file_size;
        let sub = Archive::parse(&aj[cursor as usize..end as usize])
            .unwrap_or_else(|e| panic!("sub-archive at {cursor:#x}: {e}"));
        assert_eq!(
            sub.publics().len(),
            1,
            "sub-archive at {cursor:#x} exports one symbol"
        );
        assert_eq!(sub.externs().len(), 0);
        out.push((cursor, header.file_size, sub.publics()[0].name.clone()));
        cursor = end.next_multiple_of(AJ_ALIGN);
    }
    assert_eq!(
        cursor as usize,
        aj.len().next_multiple_of(AJ_ALIGN as usize)
    );
    out
}

#[test]
fn fox_animation_table_indexes_aj_sub_archives() {
    let dir = require_disc!();
    let pl_fx = parse(&dir, "PlFx.dat");
    let aj = read(&dir, "PlFxAJ.dat");
    assert_eq!(aj.len(), 1_525_984);

    let ft_data = pl_fx.public("ftDataFox").unwrap();
    assert_eq!(ft_data, 0x98f4);
    let table = pl_fx
        .link(ft_data + ft_data_off::XC_ANIM_TABLE)
        .unwrap()
        .unwrap();
    assert_eq!(table, 0x771c);
    let entries = read_anim_table(&pl_fx, table, FOX_ANIM_COUNT);

    let with_anim: Vec<&AnimEntry> = entries.iter().filter(|e| e.aj_size != 0).collect();
    assert_eq!(with_anim.len(), 278, "motion states with an animation");
    assert!(entries
        .iter()
        .filter(|e| e.aj_size == 0)
        .all(|e| e.figatree_name.is_none() && e.aj_offset == 0));

    // Wait1 is motion state 2 and is the first sub-archive in the file. It
    // is also shared with motion state 6 (WaitItem): same offset and size.
    let wait1 = &entries[2];
    assert_eq!(
        wait1.figatree_name.as_deref(),
        Some("PlyFox5K_Share_ACTION_Wait1_figatree")
    );
    assert_eq!((wait1.aj_offset, wait1.aj_size), (0, 5077));
    assert_eq!((entries[6].aj_offset, entries[6].aj_size), (0, 5077));
    assert_eq!(entries[6].figatree_name, wait1.figatree_name);
    assert_ne!(
        entries[6].script, wait1.script,
        "shared animation, distinct scripts"
    );

    // Every referenced sub-archive is a complete .dat whose one public is
    // the entry's figatree, exactly as ftData_80085E50 consumes it.
    for e in &with_anim {
        assert!(
            e.aj_size <= MAX_AJ_SUB_ARCHIVE,
            "entry {}: {:#x}",
            e.index,
            e.aj_size
        );
        assert_eq!(e.aj_offset % AJ_ALIGN, 0, "entry {}: alignment", e.index);
        let bytes = &aj[e.aj_offset as usize..(e.aj_offset + e.aj_size) as usize];
        let sub = Archive::parse(bytes).unwrap_or_else(|err| panic!("entry {}: {err}", e.index));
        let name = e.figatree_name.as_deref().unwrap();
        assert_eq!(sub.publics().len(), 1);
        assert_eq!(sub.publics()[0].name, name);
        assert_eq!(sub.header().version, [0; 4]);
        assert_relocs_point_inside_data(name, &sub);

        let tree = read_public_figatree(&sub, name).unwrap();
        assert!(tree.frames >= 1.0, "{name}: frames {}", tree.frames);
        let expected_tracks: usize = tree.nodes.iter().map(|&n| n as usize).sum();
        assert_eq!(tree.tracks.len(), expected_tracks, "{name}: track count");
        assert!(tree.tracks.iter().all(|t| t.ad.len() == t.length as usize));
    }

    // Wait1 in detail.
    let wait1_arc = Archive::parse(&aj[..5077]).unwrap();
    let wait1_tree =
        read_public_figatree(&wait1_arc, wait1.figatree_name.as_deref().unwrap()).unwrap();
    assert_eq!(wait1_tree.frames, 120.0);
    assert_eq!(wait1_tree.nodes.len(), 73, "one node per skeleton joint");

    // The file is the referenced sub-archives packed on 32-byte
    // boundaries plus one orphan, WalkBrake, that no table entry names.
    let scanned = scan_aj_file(&aj);
    assert_eq!(scanned.len(), 221);
    let mut referenced: Vec<(u32, u32, String)> = with_anim
        .iter()
        .map(|e| (e.aj_offset, e.aj_size, e.figatree_name.clone().unwrap()))
        .collect();
    referenced.sort();
    referenced.dedup();
    assert_eq!(referenced.len(), 220, "distinct sub-archives referenced");
    let orphans: Vec<&(u32, u32, String)> =
        scanned.iter().filter(|s| !referenced.contains(s)).collect();
    assert_eq!(
        orphans,
        [&(
            0x5dc0,
            3601,
            "PlyFox5K_Share_ACTION_WalkBrake_figatree".to_owned()
        )]
    );
    assert!(referenced.iter().all(|r| scanned.contains(r)));
}

#[test]
fn fox_demo_animation_table_has_14_entries() {
    let dir = require_disc!();
    let pl_fx = parse(&dir, "PlFx.dat");
    let ft_data = pl_fx.public("ftDataFox").unwrap();
    let table = pl_fx
        .link(ft_data + ft_data_off::X14_DEMO_ANIM_TABLE)
        .unwrap()
        .unwrap();
    assert_eq!(table, 0x95c4);
    // `ftData_UnkIntPairs[FTKIND_FOX].count` (ftdata.c:1505).
    let entries = read_anim_table(&pl_fx, table, 14);
    let names: Vec<&str> = entries
        .iter()
        .map(|e| e.figatree_name.as_deref().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "PlyFox5K_Share_ACTION_Win1_figatree",
            "PlyFox5K_Share_ACTION_Win1Wait_figatree",
            "PlyFox5K_Share_ACTION_Win2_figatree",
            "PlyFox5K_Share_ACTION_Win2Wait1_figatree",
            "PlyFox5K_Share_ACTION_Win2Wait2_figatree",
            "PlyFox5K_Share_ACTION_Win3_figatree",
            "PlyFox5K_Share_ACTION_Win3Wait_figatree",
            "PlyFox5K_Share_ACTION_Selected_figatree",
            "PlyFox5K_Share_ACTION_SelectedWait_figatree",
            "PlyFox5K_Share_ACTION_Lose_figatree",
            "PlyFox5K_Share_ACTION_IntroL_figatree",
            "PlyFox5K_Share_ACTION_IntroR_figatree",
            "PlyFox5K_Share_ACTION_Ending_figatree",
            "PlyFox5K_Share_ACTION_Wait1_figatree",
        ]
    );
    // The last entry indexes PlFxDViWaitAJ.dat, whose size the offset and
    // length must fit; the file is 5154 bytes and the entry is 5077 at 0.
    assert_eq!((entries[13].aj_offset, entries[13].aj_size), (0, 5077));
}

// ---------------------------------------------------------------------------
// GrNLa.dat: Final Destination's collision map
// ---------------------------------------------------------------------------

/// `struct MapCollData` (`mp/types.h:119`), the `coll_data` public of a
/// stage archive. `ground.c:505` hands it straight to `mpLibLoad`.
mod coll_data_off {
    pub const VERTS: u32 = 0x00;
    pub const VERT_COUNT: u32 = 0x04;
    pub const LINES: u32 = 0x08;
    pub const LINE_COUNT: u32 = 0x0C;
    /// Five `(s16 start, s16 count)` pairs: floor, ceiling, right wall,
    /// left wall, dynamic.
    pub const SECTIONS: u32 = 0x10;
    pub const JOINTS: u32 = 0x24;
    pub const JOINT_COUNT: u32 = 0x28;
    pub const X2C: u32 = 0x2C;
}

/// `struct MapLine` (`mp/types.h:57`), 16 bytes.
mod map_line_off {
    pub const V0: u32 = 0x0;
    pub const V1: u32 = 0x2;
    pub const HI_FLAGS: u32 = 0xC;
    pub const LO_FLAGS: u32 = 0xE;
    pub const SIZE: u32 = 0x10;
}

/// `struct MapJoint` (`mp/types.h:87`), 40 bytes.
mod map_joint_off {
    pub const LEFT_BOUND: u32 = 0x14;
    pub const BOTTOM_BOUND: u32 = 0x18;
    pub const RIGHT_BOUND: u32 = 0x1C;
    pub const TOP_BOUND: u32 = 0x20;
    pub const VTX_START: u32 = 0x24;
    pub const VTX_COUNT: u32 = 0x26;
}

/// `CollLine_Floor` etc. (`mp/forward.h:46`) in `MapLine.hi_flags`.
const LINE_KIND_FLOOR: u16 = 1 << 0;
const LINE_KIND_CEILING: u16 = 1 << 1;
const LINE_KIND_RIGHT_WALL: u16 = 1 << 2;
const LINE_KIND_LEFT_WALL: u16 = 1 << 3;
/// `LINE_FLAG_LEDGE` (`mp/forward.h`) in `MapLine.lo_flags`.
const LINE_FLAG_LEDGE: u16 = 1 << 9;

fn section(reader: &Reader<'_>, coll_data: u32, index: u32) -> (i16, i16) {
    let at = coll_data + coll_data_off::SECTIONS + 4 * index;
    (reader.s16(at).unwrap(), reader.s16(at + 2).unwrap())
}

#[test]
fn final_destination_coll_data_is_one_joint_of_16_lines() {
    let dir = require_disc!();
    let archive = parse(&dir, "GrNLa.dat");
    let reader = archive.reader();

    let coll_data = archive.public("coll_data").unwrap();
    assert_eq!(coll_data, 0x4e0f0);
    let verts = archive
        .link(coll_data + coll_data_off::VERTS)
        .unwrap()
        .unwrap();
    let lines = archive
        .link(coll_data + coll_data_off::LINES)
        .unwrap()
        .unwrap();
    let joints = archive
        .link(coll_data + coll_data_off::JOINTS)
        .unwrap()
        .unwrap();
    assert_eq!((verts, lines, joints), (0x4df48, 0x4dfc8, 0x4e0c8));

    let vert_count = reader.s32(coll_data + coll_data_off::VERT_COUNT).unwrap();
    let line_count = reader.s32(coll_data + coll_data_off::LINE_COUNT).unwrap();
    let joint_count = reader.s32(coll_data + coll_data_off::JOINT_COUNT).unwrap();
    assert_eq!((vert_count, line_count, joint_count), (16, 16, 1));
    assert_eq!(reader.s32(coll_data + coll_data_off::X2C).unwrap(), 0);

    // Section ranges partition the 16 lines: 3 floors, 3 ceilings, 5
    // right walls, 5 left walls, no dynamic lines.
    let sections: Vec<(i16, i16)> = (0..5).map(|i| section(&reader, coll_data, i)).collect();
    assert_eq!(sections, [(0, 3), (3, 3), (6, 5), (11, 5), (0, 0)]);
    let kinds = [
        LINE_KIND_FLOOR,
        LINE_KIND_CEILING,
        LINE_KIND_RIGHT_WALL,
        LINE_KIND_LEFT_WALL,
    ];
    for (&(start, count), &kind) in sections.iter().zip(kinds.iter()) {
        for id in start..start + count {
            let line = lines + id as u32 * map_line_off::SIZE;
            let hi = reader.u16(line + map_line_off::HI_FLAGS).unwrap();
            assert_eq!(hi, kind, "line {id} kind bits");
        }
    }

    // The stage is the flat floor y = 0 from x = -85.5657 to 85.5657 in
    // three segments; the outer two carry the ledge flag.
    let vertex = |id: u16| {
        let at = verts + u32::from(id) * 8;
        (reader.f32(at).unwrap(), reader.f32(at + 4).unwrap())
    };
    let floor_edges: Vec<(u16, u16, u16)> = (0..3)
        .map(|id| {
            let line = lines + id * map_line_off::SIZE;
            (
                reader.u16(line + map_line_off::V0).unwrap(),
                reader.u16(line + map_line_off::V1).unwrap(),
                reader.u16(line + map_line_off::LO_FLAGS).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        floor_edges,
        [
            (4, 15, LINE_FLAG_LEDGE),
            (15, 6, 0),
            (6, 5, LINE_FLAG_LEDGE)
        ]
    );
    assert_eq!(vertex(4), (-85.5657, 0.0));
    assert_eq!(vertex(15), (-75.0, 0.0));
    assert_eq!(vertex(6), (75.0, 0.0));
    assert_eq!(vertex(5), (85.5657, 0.0));

    // The single joint owns every vertex and bounds them all.
    let joint = joints;
    let bounds = [
        reader.f32(joint + map_joint_off::LEFT_BOUND).unwrap(),
        reader.f32(joint + map_joint_off::BOTTOM_BOUND).unwrap(),
        reader.f32(joint + map_joint_off::RIGHT_BOUND).unwrap(),
        reader.f32(joint + map_joint_off::TOP_BOUND).unwrap(),
    ];
    assert_eq!(bounds, [-93.5657, -63.3882, 93.5657, 8.0]);
    assert_eq!(
        (
            reader.s16(joint + map_joint_off::VTX_START).unwrap(),
            reader.s16(joint + map_joint_off::VTX_COUNT).unwrap()
        ),
        (0, 16)
    );
    for id in 0..vert_count as u16 {
        let (x, y) = vertex(id);
        assert!(bounds[0] <= x && x <= bounds[2] && bounds[1] <= y && y <= bounds[3]);
    }

    // `Ground_801C0498` scales the map by `grGroundParam->y`; Final
    // Destination is unscaled.
    let param = archive.public("grGroundParam").unwrap();
    assert_eq!(reader.f32(param).unwrap(), 1.0);
}

/// `struct UnkStageDat` (`gr/types.h:2055`), the `map_head` public: six
/// `(pointer, count)` pairs followed by a zero word.
#[test]
fn final_destination_map_head_is_six_counted_arrays() {
    let dir = require_disc!();
    let archive = parse(&dir, "GrNLa.dat");
    let reader = archive.reader();
    let map_head = archive.public("map_head").unwrap();
    assert_eq!(map_head, 0x358);

    let pairs: Vec<(u32, i32)> = (0..6)
        .map(|i| {
            let at = map_head + 8 * i;
            let ptr = archive
                .link(at)
                .unwrap()
                .expect("array pointer is relocated");
            assert!(
                !archive.is_relocated_offset(at + 4),
                "count is a plain integer"
            );
            (ptr, reader.s32(at + 4).unwrap())
        })
        .collect();
    assert_eq!(
        pairs,
        [
            (0x054, 1),  // unk0 / unk4
            (0x0ac, 10), // unk8: UnkStageDat_x8_t model sets (0x34 bytes each)
            (0x2b4, 2),  // unk10: HSD_Spline*
            (0x2bc, 32), // unk18
            (0x33c, 3),  // unk20: GroundShadowEntry
            (0x354, 1),  // unk28: UnkStageDatInternal*
        ]
    );
    assert_eq!(reader.u32(map_head + 0x30).unwrap(), 0);
    // The model-set array is contiguous: 10 entries of 0x34 bytes end where
    // the next array begins.
    assert_eq!(0x0ac + 10 * 0x34, 0x2b4);
}
