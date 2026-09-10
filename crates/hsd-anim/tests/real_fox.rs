//! Load the real Fox skeleton (`PlFxNr.dat` from the extracted disc) through
//! `hsd_anim::load`. Requires the disc unless explicitly opted out; the
//! numbers asserted come from `docs/DISC.md`.

use std::path::{Path, PathBuf};

use hsd_anim::jobj::{JObjId, JOBJ_MTX_DIRTY};
use hsd_anim::load::load_joint_tree;
use hsd_archive::desc::read_public_jobj;
use hsd_archive::Archive;

const FOX_ROOT_JOINT: &str = "PlyFox5K_Share_joint";
const FOX_JOINT_COUNT: usize = 73;

fn disc_files() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    melee_test_support::require_files(["PlFxNr.dat"].map(|name| dir.join(name))).then_some(dir)
}

#[test]
fn fox_skeleton_loads_into_a_runtime_tree_in_descriptor_order() {
    let Some(dir) = disc_files() else { return };
    let bytes = std::fs::read(dir.join("PlFxNr.dat")).unwrap();
    let archive = Archive::parse(&bytes).unwrap();
    let root_desc = read_public_jobj(&archive, FOX_ROOT_JOINT).unwrap();

    let (tree, root) = load_joint_tree(&archive, &root_desc).expect("Fox skeleton loads");

    assert_eq!(root, JObjId(0), "the root is allocated first");
    assert_eq!(tree.len(), FOX_JOINT_COUNT);
    let visited: Vec<JObjId> = tree.depth_first(root).collect();
    assert_eq!(
        visited.len(),
        FOX_JOINT_COUNT,
        "depth-first walk reaches every joint"
    );

    // Runtime joints keep the descriptor's flags and transforms bit for bit,
    // in the same depth-first order the descriptor tree enumerates.
    let descriptors = root_desc.descendants();
    for (id, desc) in visited.iter().zip(&descriptors) {
        let joint = tree.get(*id);
        // Loading marks every matrix dirty (`JObjLoad` -> `HSD_JObjSetMtxDirty`);
        // every other flag bit is the descriptor's.
        assert_ne!(joint.flags & JOBJ_MTX_DIRTY, 0);
        assert_eq!(joint.flags & !JOBJ_MTX_DIRTY, desc.flags);
        assert_eq!(joint.translate.x.to_bits(), desc.position.x.to_bits());
        assert_eq!(joint.rotate.z.to_bits(), desc.rotation.z.to_bits());
        assert_eq!(joint.scale.y.to_bits(), desc.scale.y.to_bits());
        assert_eq!(joint.envelopemtx.is_some(), desc.mtx.is_some());
    }
    // `ftparts` addresses bones by depth-first index; the last one must exist.
    assert!(tree.bone(root, FOX_JOINT_COUNT - 1).is_some());
    assert!(tree.bone(root, FOX_JOINT_COUNT).is_none());
}
