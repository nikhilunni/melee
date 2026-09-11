//! Round-trip tests for the descriptor readers in `hsd_archive::desc`
//! against synthetic archives.
//!
//! The `Builder` is a copy of the one in `synthetic.rs`, extended with
//! struct emitters. Structs are laid out bottom-up (leaves first) so every
//! link points backwards and no patching is needed, except where a test
//! deliberately builds a cycle.

use hsd_archive::desc::{
    read_public_animjoint, read_public_figatree, read_public_jobj, AObjDesc, AnimJoint, DObjDesc,
    DescError, FObjDesc, FigaTree, GxColor, JObjDesc, JObjUnion, MObjDesc, MatAnimJoint,
    ShapeAnimJoint, Vec3, ANIM_JOINT_SIZE, AOBJ_DESC_SIZE, DOBJ_DESC_SIZE, FIGATRACK_SIZE,
    FIGATREE_SIZE, FOBJ_DESC_SIZE, JOBJ_DESC_SIZE, MATERIAL_SIZE, MAT_ANIM_JOINT_SIZE, MAX_DEPTH,
    MOBJ_DESC_SIZE, SHAPE_ANIM_JOINT_SIZE,
};
use hsd_archive::{Archive, ArchiveHeader, Error};

/// Assembles a `.dat` file from its parts.
#[derive(Default)]
struct Builder {
    data: Vec<u8>,
    relocs: Vec<u32>,
    publics: Vec<(u32, u32)>,
    strings: Vec<u8>,
}

impl Builder {
    fn new() -> Self {
        Self::default()
    }

    fn here(&self) -> u32 {
        self.data.len() as u32
    }

    fn push_u8(&mut self, v: u8) -> u32 {
        let at = self.here();
        self.data.push(v);
        at
    }

    fn push_u16(&mut self, v: u16) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(&v.to_be_bytes());
        at
    }

    fn push_u32(&mut self, v: u32) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(&v.to_be_bytes());
        at
    }

    fn push_f32(&mut self, v: f32) -> u32 {
        self.push_u32(v.to_bits())
    }

    fn push_bytes(&mut self, v: &[u8]) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(v);
        at
    }

    /// Write a pointer field: `Some(target)` is relocated, `None` is a
    /// plain zero.
    fn push_ptr(&mut self, target: Option<u32>) -> u32 {
        match target {
            Some(t) => {
                let at = self.push_u32(t);
                self.relocs.push(at);
                at
            }
            None => self.push_u32(0),
        }
    }

    /// Overwrite the `u32` at `at` with `target` and make it a link.
    fn patch_link(&mut self, at: u32, target: u32) {
        let at = at as usize;
        self.data[at..at + 4].copy_from_slice(&target.to_be_bytes());
        self.relocs.push(at as u32);
    }

    fn align(&mut self, n: usize) {
        while !self.data.len().is_multiple_of(n) {
            self.data.push(0);
        }
    }

    /// A NUL-terminated string in the *data* section (class names live
    /// there, not in the string table).
    fn data_cstr(&mut self, s: &str) -> u32 {
        let at = self.push_bytes(s.as_bytes());
        self.push_u8(0);
        self.align(4);
        at
    }

    fn string(&mut self, s: &str) -> u32 {
        let at = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0);
        at
    }

    fn public(&mut self, data_offset: u32, name: &str) {
        let sym = self.string(name);
        self.publics.push((data_offset, sym));
    }

    fn build(&self) -> Vec<u8> {
        let body_len =
            self.data.len() + self.relocs.len() * 4 + self.publics.len() * 8 + self.strings.len();
        let file_size = (ArchiveHeader::SIZE + body_len) as u32;
        let header = ArchiveHeader {
            file_size,
            data_size: self.data.len() as u32,
            nb_reloc: self.relocs.len() as u32,
            nb_public: self.publics.len() as u32,
            nb_extern: 0,
            version: *b"001B",
        };
        let mut out = Vec::with_capacity(file_size as usize);
        out.extend_from_slice(&header.to_bytes());
        out.extend_from_slice(&self.data);
        for r in &self.relocs {
            out.extend_from_slice(&r.to_be_bytes());
        }
        for (off, sym) in &self.publics {
            out.extend_from_slice(&off.to_be_bytes());
            out.extend_from_slice(&sym.to_be_bytes());
        }
        out.extend_from_slice(&self.strings);
        out
    }

    fn archive(&self) -> Archive {
        Archive::parse(&self.build()).expect("parse")
    }

    // ---- struct emitters --------------------------------------------

    fn vec3(&mut self, v: [f32; 3]) {
        for c in v {
            self.push_f32(c);
        }
    }

    /// `HSD_Joint`, 0x40 bytes.
    fn joint(&mut self, j: &Joint) -> u32 {
        let at = self.here();
        self.push_ptr(j.class_name);
        self.push_u32(j.flags);
        self.push_ptr(j.child);
        self.push_ptr(j.next);
        self.push_ptr(j.u);
        self.vec3(j.rotation);
        self.vec3(j.scale);
        self.vec3(j.position);
        self.push_ptr(j.mtx);
        self.push_ptr(j.robjdesc);
        assert_eq!(self.here() - at, JOBJ_DESC_SIZE);
        at
    }

    /// `HSD_DObjDesc`, 0x10 bytes.
    fn dobj(
        &mut self,
        class_name: Option<u32>,
        next: Option<u32>,
        mobj: Option<u32>,
        pobj: Option<u32>,
    ) -> u32 {
        let at = self.here();
        self.push_ptr(class_name);
        self.push_ptr(next);
        self.push_ptr(mobj);
        self.push_ptr(pobj);
        assert_eq!(self.here() - at, DOBJ_DESC_SIZE);
        at
    }

    /// `HSD_MObjDesc`, 0x18 bytes.
    fn mobj(&mut self, m: &MObj) -> u32 {
        let at = self.here();
        self.push_ptr(m.class_name);
        self.push_u32(m.rendermode);
        self.push_ptr(m.texdesc);
        self.push_ptr(m.mat);
        self.push_ptr(m.renderdesc);
        self.push_ptr(m.pedesc);
        assert_eq!(self.here() - at, MOBJ_DESC_SIZE);
        at
    }

    /// `HSD_Material`, 0x14 bytes.
    fn material(&mut self, colors: [[u8; 4]; 3], alpha: f32, shininess: f32) -> u32 {
        let at = self.here();
        for c in colors {
            self.push_bytes(&c);
        }
        self.push_f32(alpha);
        self.push_f32(shininess);
        assert_eq!(self.here() - at, MATERIAL_SIZE);
        at
    }

    /// `HSD_FObjDesc`, 0x14 bytes.
    fn fobj(&mut self, f: &FObj) -> u32 {
        let at = self.here();
        self.push_ptr(f.next);
        self.push_u32(f.length);
        self.push_f32(f.startframe);
        self.push_u8(f.type_);
        self.push_u8(f.frac_value);
        self.push_u8(f.frac_slope);
        self.push_u8(f.dummy0);
        self.push_ptr(f.ad);
        assert_eq!(self.here() - at, FOBJ_DESC_SIZE);
        at
    }

    /// `HSD_AObjDesc`, 0x10 bytes. `obj_id` is written as a link when
    /// `obj_id_link`, else as a plain u32.
    fn aobj(
        &mut self,
        flags: u32,
        end_frame: f32,
        fobj: Option<u32>,
        obj_id: u32,
        obj_id_link: bool,
    ) -> u32 {
        let at = self.here();
        self.push_u32(flags);
        self.push_f32(end_frame);
        self.push_ptr(fobj);
        if obj_id_link {
            self.push_ptr(Some(obj_id));
        } else {
            self.push_u32(obj_id);
        }
        assert_eq!(self.here() - at, AOBJ_DESC_SIZE);
        at
    }

    /// `HSD_AnimJoint`, 0x14 bytes.
    fn animjoint(
        &mut self,
        child: Option<u32>,
        next: Option<u32>,
        aobj: Option<u32>,
        robj_anim: Option<u32>,
        flags: u32,
    ) -> u32 {
        let at = self.here();
        self.push_ptr(child);
        self.push_ptr(next);
        self.push_ptr(aobj);
        self.push_ptr(robj_anim);
        self.push_u32(flags);
        assert_eq!(self.here() - at, ANIM_JOINT_SIZE);
        at
    }

    /// `HSD_MatAnimJoint` / `HSD_ShapeAnimJoint`, 0x0C bytes.
    fn tree3(&mut self, child: Option<u32>, next: Option<u32>, payload: Option<u32>) -> u32 {
        let at = self.here();
        self.push_ptr(child);
        self.push_ptr(next);
        self.push_ptr(payload);
        assert_eq!(self.here() - at, MAT_ANIM_JOINT_SIZE);
        assert_eq!(self.here() - at, SHAPE_ANIM_JOINT_SIZE);
        at
    }

    /// `FigaTrack`, 0x0C bytes.
    fn figatrack(
        &mut self,
        length: u16,
        startframe: u16,
        obj_type: u8,
        frac_value: u8,
        frac_slope: u8,
        ad: Option<u32>,
    ) -> u32 {
        let at = self.here();
        self.push_u16(length);
        self.push_u16(startframe);
        self.push_u8(obj_type);
        self.push_u8(frac_value);
        self.push_u8(frac_slope);
        self.push_u8(0);
        self.push_ptr(ad);
        assert_eq!(self.here() - at, FIGATRACK_SIZE);
        at
    }

    /// `FigaTree`, 0x14 bytes.
    fn figatree(
        &mut self,
        type_: i32,
        flags: u32,
        frames: f32,
        nodes: Option<u32>,
        tracks: Option<u32>,
    ) -> u32 {
        let at = self.here();
        self.push_u32(type_ as u32);
        self.push_u32(flags);
        self.push_f32(frames);
        self.push_ptr(nodes);
        self.push_ptr(tracks);
        assert_eq!(self.here() - at, FIGATREE_SIZE);
        at
    }
}

#[derive(Default, Clone, Copy)]
struct Joint {
    class_name: Option<u32>,
    flags: u32,
    child: Option<u32>,
    next: Option<u32>,
    u: Option<u32>,
    rotation: [f32; 3],
    scale: [f32; 3],
    position: [f32; 3],
    mtx: Option<u32>,
    robjdesc: Option<u32>,
}

#[derive(Default, Clone, Copy)]
struct MObj {
    class_name: Option<u32>,
    rendermode: u32,
    texdesc: Option<u32>,
    mat: Option<u32>,
    renderdesc: Option<u32>,
    pedesc: Option<u32>,
}

#[derive(Default, Clone, Copy)]
struct FObj {
    next: Option<u32>,
    length: u32,
    startframe: f32,
    type_: u8,
    frac_value: u8,
    frac_slope: u8,
    dummy0: u8,
    ad: Option<u32>,
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

/// Offsets of everything in the reference scene, for assertions.
struct Scene {
    b: Builder,
    root: u32,
    joint_a: u32,
    joint_b: u32,
    dobj: u32,
    mobj: u32,
    mat: u32,
    pobj_raw: u32,
    robj_raw: u32,
    aj_root: u32,
    aj_a: u32,
    aj_b: u32,
    aobj: u32,
    fobj1: u32,
    fobj2: u32,
    ad1: u32,
    ad2: u32,
}

const AD1: &[u8] = &[0x0A, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
const AD2: &[u8] = &[0xF0, 0xE1, 0xD2];
const MTX: [[f32; 4]; 3] = [
    [1.0, 2.0, 3.0, 4.0],
    [5.0, 6.0, 7.0, 8.0],
    [-1.0, -2.0, -3.0, -4.0],
];

/// A 3-joint skeleton (root -> A, A.next = B), one DObj/MObj with a
/// material on A, an envelope matrix on B, and a parallel AnimJoint tree
/// with an AObjDesc of two FObjDescs on A.
///
/// ```text
/// root "jobj_root"        aj_root
///   child: A                child: aj_a (aobj: fobj1 -> fobj2)
///     next: B                 next: aj_b
/// ```
fn scene() -> Scene {
    let mut b = Builder::new();

    // Leaves first: strings, byte streams, raw-offset stand-ins.
    let root_name = b.data_cstr("jobj_root");
    let dobj_name = b.data_cstr("dobj_class");
    let mobj_name = b.data_cstr("mobj_class");
    let ad1 = b.push_bytes(AD1);
    b.align(4);
    let ad2 = b.push_bytes(AD2);
    b.align(4);
    let pobj_raw = b.push_u32(0xDEAD_BEEF); // stands in for an HSD_PObjDesc
    let robj_raw = b.push_u32(0xFEED_FACE); // stands in for an HSD_RObjDesc
    let mtx = b.here();
    for row in MTX {
        for c in row {
            b.push_f32(c);
        }
    }

    // Geometry on joint A.
    let mat = b.material(
        [[1, 2, 3, 4], [10, 20, 30, 40], [100, 150, 200, 250]],
        0.75,
        50.0,
    );
    let mobj = b.mobj(&MObj {
        class_name: Some(mobj_name),
        rendermode: 0x1234_5678,
        texdesc: None,
        mat: Some(mat),
        renderdesc: None,
        pedesc: None,
    });
    let dobj = b.dobj(Some(dobj_name), None, Some(mobj), Some(pobj_raw));

    // Joints, leaves first so links point backwards.
    let joint_b = b.joint(&Joint {
        flags: 0x40,
        rotation: [0.1, 0.2, 0.3],
        scale: [1.0, 1.0, 1.0],
        position: [7.0, 8.0, 9.0],
        mtx: Some(mtx),
        ..Joint::default()
    });
    let joint_a = b.joint(&Joint {
        flags: 0x41,
        next: Some(joint_b),
        u: Some(dobj),
        rotation: [-0.5, 0.0, 0.5],
        scale: [2.0, 2.0, 2.0],
        position: [1.0, 2.0, 3.0],
        robjdesc: Some(robj_raw),
        ..Joint::default()
    });
    let root = b.joint(&Joint {
        class_name: Some(root_name),
        flags: 0x1000_0043,
        child: Some(joint_a),
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        position: [0.0, 10.0, 0.0],
        ..Joint::default()
    });
    b.public(root, "skeleton_joint");

    // Animation: two tracks on A.
    let fobj2 = b.fobj(&FObj {
        length: AD2.len() as u32,
        startframe: 5.0,
        type_: 6,
        frac_value: 0x21,
        frac_slope: 0x22,
        dummy0: 0x99,
        ad: Some(ad2),
        ..FObj::default()
    });
    let fobj1 = b.fobj(&FObj {
        next: Some(fobj2),
        length: AD1.len() as u32,
        startframe: 0.0,
        type_: 1,
        frac_value: 0x03,
        frac_slope: 0x04,
        ad: Some(ad1),
        ..FObj::default()
    });
    let aobj = b.aobj(0x2000_0000, 60.0, Some(fobj1), joint_a, true);
    let aj_b = b.animjoint(None, None, None, None, 2);
    let aj_a = b.animjoint(None, Some(aj_b), Some(aobj), Some(robj_raw), 1);
    let aj_root = b.animjoint(Some(aj_a), None, None, None, 0);
    b.public(aj_root, "skeleton_animjoint");

    Scene {
        b,
        root,
        joint_a,
        joint_b,
        dobj,
        mobj,
        mat,
        pobj_raw,
        robj_raw,
        aj_root,
        aj_a,
        aj_b,
        aobj,
        fobj1,
        fobj2,
        ad1,
        ad2,
    }
}

#[test]
fn reads_three_joint_skeleton() {
    let s = scene();
    let a = s.b.archive();
    let root = read_public_jobj(&a, "skeleton_joint").expect("read");
    assert_eq!(JObjDesc::read(&a, s.root).unwrap(), root);

    // Root.
    assert_eq!(root.offset, s.root);
    assert_eq!(root.class_name.as_deref(), Some("jobj_root"));
    assert_eq!(root.flags, 0x1000_0043);
    assert!(root.next.is_none());
    assert_eq!(root.instance_of, None);
    assert_eq!(root.u, JObjUnion::Dobj(None));
    assert_eq!(root.rotation, v(0.0, 0.0, 0.0));
    assert_eq!(root.scale, v(1.0, 1.0, 1.0));
    assert_eq!(root.position, v(0.0, 10.0, 0.0));
    assert_eq!(root.mtx, None);
    assert_eq!(root.robjdesc, None);
    assert!(!root.is_instance());

    // A: first child, carries the geometry.
    let ja = root.child.as_deref().expect("child A");
    assert_eq!(ja.offset, s.joint_a);
    assert_eq!(ja.class_name, None);
    assert_eq!(ja.flags, 0x41);
    assert!(ja.child.is_none());
    assert_eq!(ja.rotation, v(-0.5, 0.0, 0.5));
    assert_eq!(ja.scale, v(2.0, 2.0, 2.0));
    assert_eq!(ja.position, v(1.0, 2.0, 3.0));
    assert_eq!(ja.mtx, None);
    assert_eq!(ja.robjdesc, Some(s.robj_raw));

    let d = ja.u.dobj().expect("dobj");
    assert_eq!(d.offset, s.dobj);
    assert_eq!(d.class_name.as_deref(), Some("dobj_class"));
    assert!(d.next.is_none());
    assert_eq!(d.pobjdesc, Some(s.pobj_raw));
    let m = d.mobj.as_deref().expect("mobj");
    assert_eq!(m.offset, s.mobj);
    assert_eq!(m.class_name.as_deref(), Some("mobj_class"));
    assert_eq!(m.rendermode, 0x1234_5678);
    assert_eq!(m.texdesc, None);
    assert_eq!(m.renderdesc, None);
    assert_eq!(m.pedesc, None);
    let mat = m.mat.expect("material");
    assert_eq!(
        mat.ambient,
        GxColor {
            r: 1,
            g: 2,
            b: 3,
            a: 4
        }
    );
    assert_eq!(
        mat.diffuse,
        GxColor {
            r: 10,
            g: 20,
            b: 30,
            a: 40
        }
    );
    assert_eq!(
        mat.specular,
        GxColor {
            r: 100,
            g: 150,
            b: 200,
            a: 250
        }
    );
    assert_eq!(mat.alpha.to_bits(), 0.75f32.to_bits());
    assert_eq!(mat.shininess.to_bits(), 50.0f32.to_bits());
    assert_eq!(hsd_archive::desc::Material::read(&a, s.mat).unwrap(), mat);
    assert_eq!(MObjDesc::read(&a, s.mobj).unwrap(), *m);
    assert_eq!(DObjDesc::read(&a, s.dobj).unwrap(), *d);

    // B: A's sibling, carries the envelope matrix.
    let jb = ja.next.as_deref().expect("sibling B");
    assert_eq!(jb.offset, s.joint_b);
    assert_eq!(jb.flags, 0x40);
    assert!(jb.child.is_none());
    assert!(jb.next.is_none());
    assert_eq!(jb.u, JObjUnion::Dobj(None));
    assert_eq!(jb.rotation, v(0.1, 0.2, 0.3));
    assert_eq!(jb.scale, v(1.0, 1.0, 1.0));
    assert_eq!(jb.position, v(7.0, 8.0, 9.0));
    assert_eq!(jb.mtx, Some(MTX));

    // Traversal helpers.
    let kids: Vec<u32> = root.children().map(|j| j.offset).collect();
    assert_eq!(kids, vec![s.joint_a, s.joint_b]);
    let all: Vec<u32> = root.descendants().iter().map(|j| j.offset).collect();
    assert_eq!(all, vec![s.root, s.joint_a, s.joint_b]);
    let sibs: Vec<u32> = ja.siblings().map(|j| j.offset).collect();
    assert_eq!(sibs, vec![s.joint_a, s.joint_b]);
}

#[test]
fn reads_animjoint_tree_with_aobj_and_two_fobjs() {
    let s = scene();
    let a = s.b.archive();
    let aj = read_public_animjoint(&a, "skeleton_animjoint").expect("read");
    assert_eq!(AnimJoint::read(&a, s.aj_root).unwrap(), aj);

    assert_eq!(aj.offset, s.aj_root);
    assert_eq!(aj.flags, 0);
    assert!(aj.next.is_none());
    assert!(aj.aobjdesc.is_none());
    assert_eq!(aj.robj_anim, None);

    let aj_a = aj.child.as_deref().expect("aj A");
    assert_eq!(aj_a.offset, s.aj_a);
    assert_eq!(aj_a.flags, 1);
    assert!(aj_a.child.is_none());
    assert_eq!(aj_a.robj_anim, Some(s.robj_raw));

    let aobj = aj_a.aobjdesc.as_deref().expect("aobj");
    assert_eq!(aobj.offset, s.aobj);
    assert_eq!(aobj.flags, 0x2000_0000);
    assert_eq!(aobj.end_frame.to_bits(), 60.0f32.to_bits());
    assert_eq!(aobj.obj_id, s.joint_a);
    assert!(aobj.obj_id_is_link);
    assert_eq!(AObjDesc::read(&a, s.aobj).unwrap(), *aobj);

    let f1 = aobj.fobj.as_deref().expect("fobj1");
    assert_eq!(f1.offset, s.fobj1);
    assert_eq!(f1.length, AD1.len() as u32);
    assert_eq!(f1.startframe.to_bits(), 0.0f32.to_bits());
    assert_eq!(f1.type_, 1);
    assert_eq!(f1.frac_value, 0x03);
    assert_eq!(f1.frac_slope, 0x04);
    assert_eq!(f1.dummy0, 0);
    assert_eq!(f1.ad_offset, Some(s.ad1));
    assert_eq!(f1.ad, AD1);

    let f2 = f1.next.as_deref().expect("fobj2");
    assert_eq!(f2.offset, s.fobj2);
    assert!(f2.next.is_none());
    assert_eq!(f2.length, AD2.len() as u32);
    assert_eq!(f2.startframe.to_bits(), 5.0f32.to_bits());
    assert_eq!(f2.type_, 6);
    assert_eq!(f2.frac_value, 0x21);
    assert_eq!(f2.frac_slope, 0x22);
    assert_eq!(f2.dummy0, 0x99);
    assert_eq!(f2.ad_offset, Some(s.ad2));
    assert_eq!(f2.ad, AD2);
    assert_eq!(FObjDesc::read(&a, s.fobj1).unwrap(), *f1);

    let tracks: Vec<u8> = aobj.tracks().map(|f| f.type_).collect();
    assert_eq!(tracks, vec![1, 6]);

    let aj_b = aj_a.next.as_deref().expect("aj B");
    assert_eq!(aj_b.offset, s.aj_b);
    assert_eq!(aj_b.flags, 2);
    assert!(aj_b.child.is_none());
    assert!(aj_b.next.is_none());
    assert!(aj_b.aobjdesc.is_none());

    // The anim tree parallels the joint tree node for node, as
    // HSD_JObjAddAnimAll requires.
    let root = read_public_jobj(&a, "skeleton_joint").unwrap();
    let jk: Vec<u32> = root.children().map(|j| j.offset).collect();
    let ak: Vec<u32> = aj.children().map(|j| j.offset).collect();
    assert_eq!(jk.len(), ak.len());
    assert_eq!(ak, vec![s.aj_a, s.aj_b]);
}

#[test]
fn aobj_obj_id_plain_value_is_kept_raw() {
    let mut b = Builder::new();
    let aobj = b.aobj(0, 1.0, None, 0x77, false);
    let a = b.archive();
    let d = AObjDesc::read(&a, aobj).unwrap();
    assert_eq!(d.obj_id, 0x77);
    assert!(!d.obj_id_is_link);
    assert!(d.fobj.is_none());
}

#[test]
fn null_ad_gives_empty_stream() {
    let mut b = Builder::new();
    let f = b.fobj(&FObj {
        length: 12,
        ..FObj::default()
    });
    let a = b.archive();
    let d = FObjDesc::read(&a, f).unwrap();
    assert_eq!(d.length, 12);
    assert_eq!(d.ad_offset, None);
    assert!(d.ad.is_empty());
}

#[test]
fn truncated_ad_stream_is_an_error() {
    // The stream starts inside the data section but its declared length
    // runs past the end.
    let mut b = Builder::new();
    let ad = b.push_bytes(&[1, 2, 3, 4]);
    let fobj = b.fobj(&FObj {
        length: 0x1000,
        ad: Some(ad),
        ..FObj::default()
    });
    let a = b.archive();
    assert_eq!(
        FObjDesc::read(&a, fobj),
        Err(DescError::TruncatedStream {
            field: "HSD_FObjDesc.ad",
            offset: ad,
            len: 0x1000,
            available: a.data().len(),
        })
    );

    // The same fault surfaces through an AObjDesc and an AnimJoint.
    let aobj = {
        let mut b = Builder::new();
        let ad = b.push_bytes(&[1, 2, 3, 4]);
        let fobj = b.fobj(&FObj {
            length: 0x1000,
            ad: Some(ad),
            ..FObj::default()
        });
        let aobj = b.aobj(0, 1.0, Some(fobj), 0, false);
        let aj = b.animjoint(None, None, Some(aobj), None, 0);
        let a = b.archive();
        let err = AObjDesc::read(&a, aobj).unwrap_err();
        assert!(
            matches!(err, DescError::TruncatedStream { len: 0x1000, .. }),
            "{err:?}"
        );
        assert_eq!(AnimJoint::read(&a, aj).unwrap_err(), err);
        err
    };
    assert!(aobj.to_string().contains("HSD_FObjDesc.ad"));

    // A stream that starts exactly at the end of the data section.
    let mut b = Builder::new();
    let fobj_at = b.here();
    b.fobj(&FObj {
        length: 1,
        ad: Some(FOBJ_DESC_SIZE), // == data_size once the fobj is written
        ..FObj::default()
    });
    let a = b.archive();
    assert_eq!(
        FObjDesc::read(&a, fobj_at),
        Err(DescError::TruncatedStream {
            field: "HSD_FObjDesc.ad",
            offset: FOBJ_DESC_SIZE,
            len: 1,
            available: FOBJ_DESC_SIZE as usize,
        })
    );
}

#[test]
fn child_cycle_is_detected() {
    // root.child = A, A.child = root.
    let mut b = Builder::new();
    let ja = b.joint(&Joint::default());
    let root = b.joint(&Joint {
        child: Some(ja),
        ..Joint::default()
    });
    b.patch_link(ja + 0x08, root);
    let a = b.archive();
    assert_eq!(
        JObjDesc::read(&a, root),
        Err(DescError::Cycle {
            what: "HSD_Joint",
            offset: root
        })
    );
}

#[test]
fn next_cycle_is_detected() {
    // A.next = B, B.next = A.
    let mut b = Builder::new();
    let ja = b.joint(&Joint::default());
    let jb = b.joint(&Joint {
        next: Some(ja),
        ..Joint::default()
    });
    b.patch_link(ja + 0x0C, jb);
    let a = b.archive();
    assert_eq!(
        JObjDesc::read(&a, ja),
        Err(DescError::Cycle {
            what: "HSD_Joint",
            offset: ja
        })
    );
    // A child pointing at a later sibling of its parent is *not* a cycle
    // (retail would load that joint twice and terminate); it is read as a
    // shared sub-tree.
    let mut b = Builder::new();
    let kid = b.joint(&Joint::default());
    let jb = b.joint(&Joint::default());
    let ja = b.joint(&Joint {
        next: Some(jb),
        child: Some(kid),
        ..Joint::default()
    });
    b.patch_link(kid + 0x08, jb);
    let a = b.archive();
    let root = JObjDesc::read(&a, ja).unwrap();
    assert_eq!(
        root.child.as_ref().unwrap().child.as_ref().unwrap().offset,
        jb
    );
    assert_eq!(root.next.as_ref().unwrap().offset, jb);
}

#[test]
fn shared_subtree_is_not_a_cycle() {
    // Two joints point at the same DObjDesc; two FObjs at the same stream.
    let mut b = Builder::new();
    let mat = b.material([[0; 4]; 3], 1.0, 0.0);
    let mobj = b.mobj(&MObj {
        mat: Some(mat),
        ..MObj::default()
    });
    let dobj = b.dobj(None, None, Some(mobj), None);
    let jb = b.joint(&Joint {
        u: Some(dobj),
        ..Joint::default()
    });
    let ja = b.joint(&Joint {
        next: Some(jb),
        u: Some(dobj),
        ..Joint::default()
    });
    let a = b.archive();
    let root = JObjDesc::read(&a, ja).unwrap();
    assert_eq!(root.u.dobj().unwrap().offset, dobj);
    assert_eq!(root.next.as_ref().unwrap().u.dobj().unwrap().offset, dobj);
}

#[test]
fn fobj_cycle_is_detected() {
    let mut b = Builder::new();
    let f = b.fobj(&FObj::default());
    b.patch_link(f, f); // next = self
    let a = b.archive();
    assert_eq!(
        FObjDesc::read(&a, f),
        Err(DescError::Cycle {
            what: "HSD_FObjDesc",
            offset: f
        })
    );
}

#[test]
fn depth_limit_is_enforced() {
    // A chain of MAX_DEPTH + 2 nested children: too deep by one.
    let mut b = Builder::new();
    let mut child = None;
    for _ in 0..=MAX_DEPTH + 1 {
        child = Some(b.joint(&Joint {
            child,
            ..Joint::default()
        }));
    }
    let root = child.unwrap();
    let a = b.archive();
    assert!(matches!(
        JObjDesc::read(&a, root),
        Err(DescError::DepthExceeded {
            what: "HSD_Joint",
            ..
        })
    ));

    // Exactly MAX_DEPTH levels below the root is fine.
    let mut b = Builder::new();
    let mut child = None;
    for _ in 0..=MAX_DEPTH {
        child = Some(b.joint(&Joint {
            child,
            ..Joint::default()
        }));
    }
    let a = b.archive();
    let root = JObjDesc::read(&a, child.unwrap()).unwrap();
    assert_eq!(root.descendants().len(), MAX_DEPTH + 1);
}

#[test]
fn long_next_chain_reads_and_drops_without_recursion() {
    // 100k siblings: deep enough that a recursive drop would overflow the
    // 2 MiB test-thread stack.
    const N: usize = 100_000;
    let mut b = Builder::new();
    let mut next = None;
    for _ in 0..N {
        next = Some(b.joint(&Joint {
            next,
            ..Joint::default()
        }));
    }
    let a = b.archive();
    let head = JObjDesc::read(&a, next.unwrap()).unwrap();
    assert_eq!(head.siblings().count(), N);
    drop(head);
}

#[test]
fn instance_joint_keeps_child_as_id() {
    let mut b = Builder::new();
    let target = b.joint(&Joint::default());
    let inst = b.joint(&Joint {
        flags: hsd_archive::desc::jobj::JOBJ_INSTANCE,
        child: Some(target),
        ..Joint::default()
    });
    let a = b.archive();
    let j = JObjDesc::read(&a, inst).unwrap();
    assert!(j.is_instance());
    assert!(j.child.is_none());
    assert_eq!(j.instance_of, Some(target));
}

#[test]
fn spline_and_ptcl_unions_are_raw() {
    let mut b = Builder::new();
    let payload = b.push_u32(0);
    let spline = b.joint(&Joint {
        flags: hsd_archive::desc::jobj::JOBJ_SPLINE | hsd_archive::desc::jobj::JOBJ_PTCL,
        u: Some(payload),
        ..Joint::default()
    });
    let ptcl = b.joint(&Joint {
        flags: hsd_archive::desc::jobj::JOBJ_PTCL,
        u: Some(payload),
        ..Joint::default()
    });
    let a = b.archive();
    // Spline wins when both bits are set, as in JObjLoad.
    assert_eq!(
        JObjDesc::read(&a, spline).unwrap().u,
        JObjUnion::Spline(Some(payload))
    );
    assert_eq!(
        JObjDesc::read(&a, ptcl).unwrap().u,
        JObjUnion::Ptcl(Some(payload))
    );
}

#[test]
fn unrelocated_nonzero_pointer_is_an_error() {
    let mut b = Builder::new();
    let j = b.joint(&Joint::default());
    // Poke a non-zero value into `child` without relocating it.
    let at = (j + 0x08) as usize;
    b.data[at..at + 4].copy_from_slice(&0x40u32.to_be_bytes());
    let a = b.archive();
    assert_eq!(
        JObjDesc::read(&a, j),
        Err(DescError::UnrelocatedPointer {
            field: "HSD_Joint.child",
            at: j + 0x08,
            value: 0x40
        })
    );
}

#[test]
fn link_to_offset_zero_is_followed() {
    // The DObjDesc sits at offset 0; a relocated slot holding 0 links to it.
    let mut b = Builder::new();
    let dobj = b.dobj(None, None, None, None);
    assert_eq!(dobj, 0);
    let j = b.joint(&Joint {
        u: Some(dobj),
        ..Joint::default()
    });
    let a = b.archive();
    let j = JObjDesc::read(&a, j).unwrap();
    assert_eq!(j.u.dobj().unwrap().offset, 0);
}

#[test]
fn missing_symbol_and_out_of_bounds() {
    let s = scene();
    let a = s.b.archive();
    assert_eq!(
        read_public_jobj(&a, "nope"),
        Err(DescError::MissingSymbol {
            name: "nope".into()
        })
    );
    let end = a.data().len() as u32;
    assert!(matches!(
        JObjDesc::read(&a, end - 4),
        Err(DescError::Archive(Error::OutOfBounds { .. }))
    ));
    assert!(matches!(
        JObjDesc::read(&a, u32::MAX - 2),
        Err(DescError::Archive(Error::OffsetOverflow { .. }))
    ));
}

#[test]
fn mat_and_shape_anim_trees() {
    let mut b = Builder::new();
    let payload = b.push_u32(0);
    let leaf_b = b.tree3(None, None, None);
    let leaf_a = b.tree3(None, Some(leaf_b), Some(payload));
    let root = b.tree3(Some(leaf_a), None, None);
    let a = b.archive();

    let m = MatAnimJoint::read(&a, root).unwrap();
    assert_eq!(m.offset, root);
    assert_eq!(m.matanim, None);
    let kids: Vec<&MatAnimJoint> = m.children().collect();
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].offset, leaf_a);
    assert_eq!(kids[0].matanim, Some(payload));
    assert_eq!(kids[1].offset, leaf_b);
    assert_eq!(kids[1].matanim, None);
    assert_eq!(kids[0].siblings().count(), 2);

    let s = ShapeAnimJoint::read(&a, root).unwrap();
    assert_eq!(
        s.children().map(|n| n.offset).collect::<Vec<_>>(),
        vec![leaf_a, leaf_b]
    );
    assert_eq!(s.children().next().unwrap().shapeanimdobj, Some(payload));

    // Cycle through child.
    b.patch_link(leaf_b, root);
    let a = b.archive();
    assert_eq!(
        MatAnimJoint::read(&a, root),
        Err(DescError::Cycle {
            what: "HSD_MatAnimJoint",
            offset: root
        })
    );
}

#[test]
fn reads_figatree() {
    let mut b = Builder::new();
    let s0 = b.push_bytes(&[1, 2, 3, 4, 5]);
    let s1 = b.push_bytes(&[9, 8]);
    let s2 = b.push_bytes(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    b.align(4);
    // nodes: joint 0 has 2 tracks, joint 1 none, joint 2 one track.
    let nodes = b.push_bytes(&[2, 0, 1, 0xFF]);
    let tracks = b.here();
    b.figatrack(5, 0, 1, 0x03, 0x00, Some(s0));
    b.figatrack(2, 3, 5, 0x21, 0x22, Some(s1));
    b.figatrack(6, 7, 9, 0x40, 0x41, Some(s2));
    let tree = b.figatree(1, 0x2000_0000, 42.0, Some(nodes), Some(tracks));
    b.public(tree, "PlyFox5K_Share_ACTION_Wait1_figatree");
    let a = b.archive();

    let t = read_public_figatree(&a, "PlyFox5K_Share_ACTION_Wait1_figatree").unwrap();
    assert_eq!(FigaTree::read(&a, tree).unwrap(), t);
    assert_eq!(t.offset, tree);
    assert_eq!(t.type_, 1);
    assert_eq!(t.flags, 0x2000_0000);
    assert_eq!(t.frames.to_bits(), 42.0f32.to_bits());
    assert_eq!(t.nodes, vec![2, 0, 1]);
    assert_eq!(t.tracks.len(), 3);

    let tr = &t.tracks[0];
    assert_eq!(tr.offset, tracks);
    assert_eq!(tr.length, 5);
    assert_eq!(tr.startframe, 0);
    assert_eq!(tr.obj_type, 1);
    assert_eq!(tr.frac_value, 0x03);
    assert_eq!(tr.frac_slope, 0x00);
    assert_eq!(tr.pad, 0);
    assert_eq!(tr.ad_offset, Some(s0));
    assert_eq!(tr.ad.as_ref(), &[1, 2, 3, 4, 5]);
    let tr = &t.tracks[1];
    assert_eq!(tr.offset, tracks + FIGATRACK_SIZE);
    assert_eq!((tr.length, tr.startframe, tr.obj_type), (2, 3, 5));
    assert_eq!((tr.frac_value, tr.frac_slope), (0x21, 0x22));
    assert_eq!(tr.ad.as_ref(), &[9, 8]);
    let tr = &t.tracks[2];
    assert_eq!(tr.offset, tracks + 2 * FIGATRACK_SIZE);
    assert_eq!((tr.length, tr.startframe, tr.obj_type), (6, 7, 9));
    assert_eq!(tr.ad.as_ref(), &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let by_node = t.tracks_by_node();
    assert_eq!(by_node.len(), 3);
    assert_eq!(by_node[0].len(), 2);
    assert_eq!(by_node[1].len(), 0);
    assert_eq!(by_node[2].len(), 1);
    assert_eq!(by_node[2][0].obj_type, 9);
}

#[test]
fn figatree_errors() {
    // Bad node value.
    let mut b = Builder::new();
    let nodes = b.push_bytes(&[1, 0xFE, 0xFF, 0]);
    let tree = b.figatree(0, 0, 1.0, Some(nodes), None);
    let a = b.archive();
    assert_eq!(
        FigaTree::read(&a, tree),
        Err(DescError::BadFigaTreeNode {
            offset: nodes + 1,
            value: -2
        })
    );

    // Null tracks with non-empty nodes.
    let mut b = Builder::new();
    let nodes = b.push_bytes(&[1, 0xFF, 0, 0]);
    let tree = b.figatree(0, 0, 1.0, Some(nodes), None);
    let a = b.archive();
    assert_eq!(
        FigaTree::read(&a, tree),
        Err(DescError::NullPointer {
            field: "FigaTree.tracks",
            at: tree + 0x10
        })
    );

    // Track array running off the end.
    let mut b = Builder::new();
    let nodes = b.push_bytes(&[3, 0xFF, 0, 0]);
    let tracks = b.here();
    b.figatrack(0, 0, 1, 0, 0, None);
    let tree = b.figatree(0, 0, 1.0, Some(nodes), Some(tracks));
    let a = b.archive();
    assert!(matches!(
        FigaTree::read(&a, tree),
        Err(DescError::Archive(Error::OutOfBounds { .. }))
    ));

    // Truncated track stream.
    let mut b = Builder::new();
    let nodes = b.push_bytes(&[1, 0xFF, 0, 0]);
    let tracks = b.here();
    b.figatrack(0x100, 0, 1, 0, 0, Some(nodes));
    let tree = b.figatree(0, 0, 1.0, Some(nodes), Some(tracks));
    let a = b.archive();
    assert!(matches!(
        FigaTree::read(&a, tree),
        Err(DescError::TruncatedStream {
            field: "FigaTrack.ad_head",
            len: 0x100,
            ..
        })
    ));

    // Null nodes: empty tree, no error.
    let mut b = Builder::new();
    let tree = b.figatree(0, 0, 0.0, None, None);
    let a = b.archive();
    let t = FigaTree::read(&a, tree).unwrap();
    assert!(t.nodes.is_empty());
    assert!(t.tracks.is_empty());
    assert!(t.tracks_by_node().is_empty());
}

#[test]
fn errors_display_and_source() {
    let e = DescError::Cycle {
        what: "HSD_Joint",
        offset: 0x40,
    };
    assert!(e.to_string().contains("0x40"));
    let e: Box<dyn std::error::Error> = Box::new(DescError::Archive(Error::OutOfBounds {
        offset: 1,
        len: 2,
        available: 3,
    }));
    assert!(e.source().is_some());
    let e = DescError::from(Error::OffsetOverflow { offset: 1, add: 2 });
    assert!(matches!(e, DescError::Archive(_)));
}

#[test]
fn material_animation_tables_keep_null_entries_and_reject_cycles() {
    use hsd_archive::desc::material_animation::MaterialAnimation;
    let mut b = Builder::new();
    let image = b.push_u32(0);
    let images = b.push_ptr(Some(image));
    b.push_ptr(None);
    let texture = b.push_ptr(None);
    b.push_u32(3);
    b.push_ptr(None);
    b.push_ptr(Some(images));
    b.push_ptr(None);
    b.push_u16(2);
    b.push_u16(0);
    let material = b.push_ptr(None);
    b.push_ptr(None);
    b.push_ptr(Some(texture));
    b.push_ptr(None);
    let parsed = MaterialAnimation::read_chain(&b.archive(), Some(material)).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].textures[0].id, 3);
    assert_eq!(parsed[0].textures[0].images, [Some(image), None]);
    b.patch_link(material, material);
    assert!(matches!(
        MaterialAnimation::read_chain(&b.archive(), Some(material)),
        Err(DescError::Cycle {
            what: "HSD_MatAnim",
            ..
        })
    ));
}
