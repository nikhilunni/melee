mod common;

use common::{srt, RetailTrig};
use hsd_anim::aobj::{AObj, AOBJ_LOOP, AOBJ_NO_ANIM};
use hsd_anim::fobj::{HSD_A_FRAC_S16, HSD_A_OP_LIN};
use hsd_anim::jobj::{JObjId, JObjTree, JointSpec, JOBJ_CLASSICAL_SCALE, JOBJ_INSTANCE};
use hsd_anim::load::attach_anim_joint;
use hsd_archive::{desc, Archive, ArchiveHeader};
use melee_lb::anim::{
    animation_frames, attach_figatree, attach_joint_tracks, request_frame, AttachError,
};

/// Minimal version of hsd-anim/tests/load_common's relocated archive builder.
#[derive(Default)]
struct Builder {
    data: Vec<u8>,
    relocs: Vec<u32>,
}

impl Builder {
    fn bytes(&mut self, bytes: &[u8]) -> u32 {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(bytes);
        offset
    }
    fn word(&mut self, value: u32) -> u32 {
        self.bytes(&value.to_be_bytes())
    }
    fn pointer(&mut self, target: Option<u32>) {
        let slot = self.word(target.unwrap_or(0));
        if target.is_some() {
            self.relocs.push(slot);
        }
    }
    fn align(&mut self) {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
    }
    fn archive(self) -> Archive {
        let header = ArchiveHeader {
            file_size: (ArchiveHeader::SIZE + self.data.len() + 4 * self.relocs.len()) as u32,
            data_size: self.data.len() as u32,
            nb_reloc: self.relocs.len() as u32,
            nb_public: 0,
            nb_extern: 0,
            version: *b"001B",
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend(self.data);
        for slot in self.relocs {
            bytes.extend(slot.to_be_bytes());
        }
        Archive::parse(&bytes).unwrap()
    }
    fn anim_joint(&mut self, child: Option<u32>, next: Option<u32>, aobj: u32) -> u32 {
        let offset = self.data.len() as u32;
        self.pointer(child);
        self.pointer(next);
        self.pointer(Some(aobj));
        self.pointer(None);
        self.word(1);
        offset
    }
}

fn fixture() -> (Archive, desc::FigaTree, desc::AnimJoint) {
    let mut b = Builder::default();
    // Two LIN records, little-endian signed values / 2, separated by 4 frames.
    // Different track types and a delayed rotation catch incorrect field copies.
    let tracks = [
        (5, 0, 2i16, 10i16),
        (12, 0, 2, 2),
        (3, 1, 0, 4),
        (9, 0, 2, 6),
    ];
    let mut streams = Vec::new();
    for &(_, _, from, to) in &tracks {
        let mut stream = vec![0x10 | HSD_A_OP_LIN];
        stream.extend(from.to_le_bytes());
        stream.push(4);
        stream.extend(to.to_le_bytes());
        stream.push(4);
        streams.push((b.bytes(&stream), stream.len() as u16));
    }
    b.align();
    let track_array = b.data.len() as u32;
    for (&(ty, start, _, _), &(stream, length)) in tracks.iter().zip(&streams) {
        b.bytes(&length.to_be_bytes());
        b.bytes(&(start as u16).to_be_bytes());
        b.bytes(&[ty, HSD_A_FRAC_S16 | 1, HSD_A_FRAC_S16 | 3, 0]);
        b.pointer(Some(stream));
    }
    let nodes = b.bytes(&[2, 1, 1, 0xFF]);
    let figatree = b.word(1);
    b.word(AOBJ_LOOP);
    b.word(8.0f32.to_bits());
    b.pointer(Some(nodes));
    b.pointer(Some(track_array));

    let mut aobjs = Vec::new();
    for indices in [&[0, 1][..], &[2][..], &[3][..]] {
        let mut next = None;
        for &i in indices.iter().rev() {
            let (ty, start, _, _) = tracks[i];
            let (stream, length) = streams[i];
            let offset = b.data.len() as u32;
            b.pointer(next);
            b.word(u32::from(length));
            b.word((start as f32).to_bits());
            b.bytes(&[ty, HSD_A_FRAC_S16 | 1, HSD_A_FRAC_S16 | 3, 0]);
            b.pointer(Some(stream));
            next = Some(offset);
        }
        let aobj = b.word(AOBJ_LOOP);
        b.word(8.0f32.to_bits());
        b.pointer(next);
        b.word(0);
        aobjs.push(aobj);
    }
    let second = b.anim_joint(None, None, aobjs[2]);
    let first = b.anim_joint(None, Some(second), aobjs[1]);
    let anim = b.anim_joint(Some(first), None, aobjs[0]);
    let archive = b.archive();
    let figa = desc::FigaTree::read(&archive, figatree).unwrap();
    let anim = desc::AnimJoint::read(&archive, anim).unwrap();
    (archive, figa, anim)
}

fn skeleton() -> (JObjTree, JObjId, [JObjId; 3]) {
    let mut tree = JObjTree::new();
    // Arena order deliberately differs from child/next traversal order.
    let second = tree.load_joint(&JointSpec::new());
    let root = tree.load_joint(&JointSpec::new());
    let first = tree.load_joint(&JointSpec::new());
    tree.add_child(root, first);
    tree.add_child(root, second);
    (tree, root, [root, first, second])
}

#[test]
fn three_nodes_match_equivalent_anim_joint_state_and_frame_values() {
    let (archive, figa, anim) = fixture();
    let (mut actual, root, ids) = skeleton();
    let (mut expected, _, _) = skeleton();
    assert_eq!(attach_figatree(&mut actual, root, &figa).unwrap(), 3);
    attach_anim_joint(&mut expected, root, &anim, &archive).unwrap();
    for (id, types) in ids.into_iter().zip([vec![12, 5], vec![3], vec![9]]) {
        let aobj = actual.get(id).aobj.as_ref().unwrap();
        assert_eq!(aobj, expected.get(id).aobj.as_ref().unwrap());
        assert_eq!(
            aobj.fobj.iter().map(|f| f.obj_type).collect::<Vec<_>>(),
            types
        );
        assert_eq!(aobj.flags, AOBJ_NO_ANIM | AOBJ_LOOP);
        assert_eq!(aobj.end_frame.to_bits(), 8.0f32.to_bits());
        assert_eq!(aobj.rewind_frame.to_bits(), 0.0f32.to_bits());
        assert_eq!(aobj.framerate.to_bits(), 1.0f32.to_bits());
        assert_ne!(actual.get(id).flags & JOBJ_CLASSICAL_SCALE, 0);
    }
    for frame in [0.0, 1.0, 2.5] {
        request_frame(&mut actual, root, frame);
        expected.req_anim_all(root, frame);
        assert_eq!(
            actual.anim_all::<RetailTrig>(root),
            expected.anim_all::<RetailTrig>(root)
        );
        for id in ids {
            assert_eq!(
                srt(&actual, id),
                srt(&expected, id),
                "frame {frame}, joint {id:?}"
            );
            assert_eq!(actual.get(id).aobj, expected.get(id).aobj);
            assert_eq!(actual.get(id).flags, expected.get(id).flags);
        }
        assert_eq!(
            actual.get(root).translate.x.to_bits(),
            (1.0 + frame).to_bits()
        );
    }
}

#[test]
fn zero_tracks_preserve_old_animation_and_signed_startframe_is_a_bit_copy() {
    let (_, mut figa, _) = fixture();
    let (mut tree, root, _) = skeleton();
    tree.get_mut(root).aobj = Some(AObj::alloc());
    tree.set_flags(root, JOBJ_CLASSICAL_SCALE);
    let old = tree.get(root).aobj.clone();
    figa.type_ = 0;
    attach_joint_tracks(&mut tree, root, &figa, &[]);
    assert_eq!(tree.get(root).aobj, old);
    assert_ne!(tree.get(root).flags & JOBJ_CLASSICAL_SCALE, 0);
    figa.tracks[0].startframe = 0xFFFF;
    attach_joint_tracks(&mut tree, root, &figa, &figa.tracks[..1]);
    assert_eq!(tree.get(root).aobj.as_ref().unwrap().fobj[0].startframe, -1);
    assert_eq!(tree.get(root).flags & JOBJ_CLASSICAL_SCALE, 0);
    assert_eq!(animation_frames(Some(&figa)).to_bits(), 8.0f32.to_bits());
    assert_eq!(animation_frames(None).to_bits(), 0.0f32.to_bits());
}

#[test]
fn subtree_boundary_excludes_root_siblings_and_instance_children() {
    let (_, mut figa, _) = fixture();
    let (mut tree, root, ids) = skeleton();
    let outside = tree.load_joint(&JointSpec::new());
    tree.get_mut(root).next = Some(outside);
    attach_figatree(&mut tree, root, &figa).unwrap();
    assert!(tree.get(outside).aobj.is_none());
    tree.set_flags(root, JOBJ_INSTANCE);
    figa.nodes.truncate(1);
    figa.tracks.truncate(2);
    let old = tree.get(ids[1]).aobj.clone();
    assert_eq!(attach_figatree(&mut tree, root, &figa).unwrap(), 1);
    assert_eq!(tree.get(ids[1]).aobj, old);
}

#[test]
fn malformed_mapping_fails_before_changing_any_animation() {
    let (_, mut figa, _) = fixture();
    let (mut tree, root, ids) = skeleton();
    figa.nodes.pop();
    assert!(matches!(
        attach_figatree(&mut tree, root, &figa),
        Err(AttachError::NodeCount { .. })
    ));
    figa.nodes.push(-2);
    assert_eq!(
        attach_figatree(&mut tree, root, &figa),
        Err(AttachError::InvalidTracks)
    );
    figa.nodes[2] = 1;
    figa.tracks[0].length += 1;
    assert_eq!(
        attach_figatree(&mut tree, root, &figa),
        Err(AttachError::InvalidTracks)
    );
    tree.get_mut(ids[1]).next = Some(ids[1]);
    assert_eq!(
        attach_figatree(&mut tree, root, &figa),
        Err(AttachError::RepeatedJoint(ids[1]))
    );
    tree.get_mut(ids[1]).next = Some(JObjId(99));
    assert_eq!(
        attach_figatree(&mut tree, root, &figa),
        Err(AttachError::InvalidJoint(JObjId(99)))
    );
    assert!(ids.iter().all(|&id| tree.get(id).aobj.is_none()));
}
