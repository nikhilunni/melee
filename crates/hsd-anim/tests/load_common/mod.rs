//! Synthetic archive builder copied from hsd-archive/tests/desc.rs (itself
//! based on synthetic.rs), restricted to joint/material/animation fixtures.

use hsd_archive::desc::{
    ANIM_JOINT_SIZE, AOBJ_DESC_SIZE, DOBJ_DESC_SIZE, FOBJ_DESC_SIZE, JOBJ_DESC_SIZE, MATERIAL_SIZE,
    MOBJ_DESC_SIZE,
};
use hsd_archive::{Archive, ArchiveHeader};

/// Assembles a `.dat` file from its parts.
#[derive(Default)]
pub struct Builder {
    pub data: Vec<u8>,
    pub relocs: Vec<u32>,
    pub publics: Vec<(u32, u32)>,
    pub strings: Vec<u8>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn here(&self) -> u32 {
        self.data.len() as u32
    }

    pub fn push_u8(&mut self, v: u8) -> u32 {
        let at = self.here();
        self.data.push(v);
        at
    }

    pub fn push_u32(&mut self, v: u32) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(&v.to_be_bytes());
        at
    }

    pub fn push_f32(&mut self, v: f32) -> u32 {
        self.push_u32(v.to_bits())
    }

    pub fn push_bytes(&mut self, v: &[u8]) -> u32 {
        let at = self.here();
        self.data.extend_from_slice(v);
        at
    }

    /// Write a pointer field: `Some(target)` is relocated, `None` is a
    /// plain zero.
    pub fn push_ptr(&mut self, target: Option<u32>) -> u32 {
        match target {
            Some(t) => {
                let at = self.push_u32(t);
                self.relocs.push(at);
                at
            }
            None => self.push_u32(0),
        }
    }

    pub fn align(&mut self, n: usize) {
        while !self.data.len().is_multiple_of(n) {
            self.data.push(0);
        }
    }

    /// A NUL-terminated string in the *data* section (class names live
    /// there, not in the string table).
    pub fn data_cstr(&mut self, s: &str) -> u32 {
        let at = self.push_bytes(s.as_bytes());
        self.push_u8(0);
        self.align(4);
        at
    }

    pub fn string(&mut self, s: &str) -> u32 {
        let at = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0);
        at
    }

    pub fn public(&mut self, data_offset: u32, name: &str) {
        let sym = self.string(name);
        self.publics.push((data_offset, sym));
    }

    pub fn build(&self) -> Vec<u8> {
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

    pub fn archive(&self) -> Archive {
        Archive::parse(&self.build()).expect("parse")
    }

    // ---- struct emitters --------------------------------------------

    pub fn vec3(&mut self, v: [f32; 3]) {
        for c in v {
            self.push_f32(c);
        }
    }

    /// `HSD_Joint`, 0x40 bytes.
    pub fn joint(&mut self, j: &Joint) -> u32 {
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
    pub fn dobj(
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
    pub fn mobj(&mut self, m: &MObj) -> u32 {
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
    pub fn material(&mut self, colors: [[u8; 4]; 3], alpha: f32, shininess: f32) -> u32 {
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
    pub fn fobj(&mut self, f: &FObj) -> u32 {
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
    pub fn aobj(
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
    pub fn animjoint(
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
}

#[derive(Default, Clone, Copy)]
pub struct Joint {
    pub class_name: Option<u32>,
    pub flags: u32,
    pub child: Option<u32>,
    pub next: Option<u32>,
    pub u: Option<u32>,
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub position: [f32; 3],
    pub mtx: Option<u32>,
    pub robjdesc: Option<u32>,
}

#[derive(Default, Clone, Copy)]
pub struct MObj {
    pub class_name: Option<u32>,
    pub rendermode: u32,
    pub texdesc: Option<u32>,
    pub mat: Option<u32>,
    pub renderdesc: Option<u32>,
    pub pedesc: Option<u32>,
}

#[derive(Default, Clone, Copy)]
pub struct FObj {
    pub next: Option<u32>,
    pub length: u32,
    pub startframe: f32,
    pub type_: u8,
    pub frac_value: u8,
    pub frac_slope: u8,
    pub dummy0: u8,
    pub ad: Option<u32>,
}
