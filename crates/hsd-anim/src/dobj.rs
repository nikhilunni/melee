//! Display objects: `HSD_DObj` from `src/sysdolphin/baselib/dobj.c` and
//! `dobj.h`, as a data holder that a joint can own.
//!
//! In the C a joint holds a singly linked list of DObjs (`HSD_DObj::next`).
//! Here [`crate::jobj::JObj::dobj`] is a `Vec<DObj>` in that list order
//! (index 0 is the head), so `DObj` carries no `next` field; the `*All`
//! functions that walk the list are the slice functions below.
//!
//! What is *not* ported: `pobj` (polygon objects, vertex data, envelopes),
//! `HSD_DObjDisp` and every render-time routine, the class system, the
//! allocator and `HSD_DObjResolveRefs` (a PObj concern). `DObj::aobj` is
//! kept because `HSD_DObjRemoveAnimByFlags` clears it, but nothing in the
//! retail library ever loads or steps a DObj-level AObj (`HSD_DObjAnim`
//! only forwards to the PObj and MObj lists).

use crate::aobj::{AObj, AObjEndCallback};
use crate::mobj::{MObj, MatAnim};

/// `DOBJ_HIDDEN` (`dobj.h`).
pub const DOBJ_HIDDEN: u32 = 0x1;

/// `HSD_DObj` (`dobj.h`) minus `parent` (class header), `next` (see the
/// module notes) and `pobj`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DObj {
    /// `mobj`: the material.
    pub mobj: Option<MObj>,
    /// `aobj`: never populated by the retail library; see the module notes.
    pub aobj: Option<AObj>,
    /// `flags`: `DOBJ_HIDDEN` plus the blending class bits `DObjLoad` sets.
    pub flags: u32,
}

impl DObj {
    /// `DObjLoad` (`dobj.c:178`) without the PObj: classifies the material's
    /// blending mode into flag bits `2` (opaque), `8` (`RENDER_XLU`) or `4`
    /// (`RENDER_XLU | RENDER_NO_ZUPDATE`) under mask `0xE`.
    ///
    /// Panics on any other combination, where the C calls `HSD_Panic`
    /// ("mobj has unexpected blending flags").
    pub fn load(mobj: Option<MObj>) -> DObj {
        let mut dobj = DObj {
            mobj,
            aobj: None,
            flags: 0,
        };
        if let Some(mobj) = dobj.mobj.as_ref() {
            match mobj.rendermode & 0x6000_0000 {
                0 => dobj.modify_flags(2, 0xE),
                0x4000_0000 => dobj.modify_flags(8, 0xE),
                0x6000_0000 => dobj.modify_flags(4, 0xE),
                other => panic!("mobj has unexpected blending flags ({other:#x})."),
            }
        }
        dobj
    }

    /// `HSD_DObjGetFlags` (`dobj.c:24`).
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// `HSD_DObjSetFlags` (`dobj.c:32`).
    pub fn set_flags(&mut self, flags: u32) {
        self.flags |= flags;
    }

    /// `HSD_DObjClearFlags` (`dobj.c:39`).
    pub fn clear_flags(&mut self, flags: u32) {
        self.flags &= !flags;
    }

    /// `HSD_DObjModifyFlags` (`dobj.c:46`).
    pub fn modify_flags(&mut self, flags: u32, mask: u32) {
        self.flags = (self.flags & !mask) | (flags & mask);
    }

    /// `HSD_DObjRemoveAnimByFlags` (`dobj.c:55`). PObj skipped.
    pub fn remove_anim_by_flags(&mut self, flags: u32) {
        if flags & 2 != 0 {
            self.aobj = None;
        }
        if let Some(mobj) = self.mobj.as_mut() {
            mobj.remove_anim_by_flags(flags);
        }
    }

    /// `HSD_DObjRemoveAnimAllByFlags` (`dobj.c:69`) over the list.
    pub fn remove_anim_all_by_flags(list: &mut [DObj], flags: u32) {
        for dp in list {
            dp.remove_anim_by_flags(flags);
        }
    }

    /// `HSD_DObjAddAnim` (`dobj.c:82`). Shape animations (PObj) skipped.
    pub fn add_anim(&mut self, mat_anim: Option<&MatAnim>) {
        if let Some(mobj) = self.mobj.as_mut() {
            mobj.add_anim(mat_anim);
        }
    }

    /// `HSD_DObjAddAnimAll` (`dobj.c:101`): walks the DObj list and the
    /// matanim list in parallel; once the matanim list runs out the rest
    /// of the DObjs get `None` (`next_p` of a null pointer).
    pub fn add_anim_all(list: &mut [DObj], matanim: &[MatAnim]) {
        let mut ma = matanim.iter();
        for dp in list {
            dp.add_anim(ma.next());
        }
    }

    /// `HSD_DObjReqAnimByFlags` (`dobj.c:119`). PObj skipped.
    pub fn req_anim_by_flags(&mut self, startframe: f32, flags: u32) {
        if let Some(mobj) = self.mobj.as_mut() {
            mobj.req_anim_by_flags(startframe, flags);
        }
    }

    /// `HSD_DObjReqAnimAllByFlags` (`dobj.c:129`).
    pub fn req_anim_all_by_flags(list: &mut [DObj], startframe: f32, flags: u32) {
        for dp in list {
            dp.req_anim_by_flags(startframe, flags);
        }
    }

    /// `HSD_DObjReqAnimAll` (`dobj.c:142`): all animation kinds (`0x7FF`).
    pub fn req_anim_all(list: &mut [DObj], startframe: f32) {
        for dp in list {
            dp.req_anim_by_flags(startframe, 0x7FF);
        }
    }

    /// `HSD_DObjAnim` (`dobj.c:155`): PObj skipped, then `HSD_MObjAnim`.
    pub fn anim(&mut self, cb: &mut AObjEndCallback) {
        if let Some(mobj) = self.mobj.as_mut() {
            mobj.anim(cb);
        }
    }

    /// `HSD_DObjAnimAll` (`dobj.c:165`).
    pub fn anim_all(list: &mut [DObj], cb: &mut AObjEndCallback) {
        for dp in list {
            dp.anim(cb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobj::{Material, RENDER_NO_ZUPDATE, RENDER_XLU};

    #[test]
    fn load_classifies_blending() {
        let m = |rm| Some(MObj::load(rm, Material::default(), None));
        assert_eq!(DObj::load(m(0)).flags, 2);
        assert_eq!(DObj::load(m(RENDER_XLU)).flags, 8);
        assert_eq!(DObj::load(m(RENDER_XLU | RENDER_NO_ZUPDATE)).flags, 4);
        assert_eq!(DObj::load(None).flags, 0);
    }

    #[test]
    #[should_panic(expected = "unexpected blending flags")]
    fn load_panics_on_no_zupdate_alone() {
        let _ = DObj::load(Some(MObj::load(
            RENDER_NO_ZUPDATE,
            Material::default(),
            None,
        )));
    }

    #[test]
    fn modify_flags_masks() {
        let mut d = DObj {
            flags: 0xF1,
            ..DObj::default()
        };
        d.modify_flags(0x0A, 0x0E);
        assert_eq!(d.flags, 0xFB);
    }
}
