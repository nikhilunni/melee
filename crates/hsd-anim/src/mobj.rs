//! Material objects: `HSD_MObj` from `src/sysdolphin/baselib/mobj.c` and
//! `mobj.h`, as a data holder plus its material-colour animation.
//!
//! What is ported: the `rendermode` word and its `RENDER_*` bit layout, the
//! `HSD_Material` colours, the optional `HSD_PEDesc`, the material `AObj`
//! and `MObjUpdateFunc` (`mobj.c:86`), `HSD_MObjAnim` (`mobj.c:143`), the
//! `ReqAnim`/`RemoveAnim`/`AddAnim` flag plumbing and the two colour
//! setters. Nothing here touches GX.
//!
//! Texture state and animation live in `tobj`; neither module decodes pixels
//! or retains GPU resources. Render-time TEV compilation, class allocation and
//! shadow/toon globals belong to presentation. RenderAnim remains unsupported.

use crate::aobj::{AObj, AObjDesc, AObjEndCallback};

/// `MOBJ_ANIM` (`mobj.h`): the `flags` bit that selects the material AObj
/// in the `*ByFlags` functions.
pub const MOBJ_ANIM: u32 = 0x4;
/// `TOBJ_ANIM`: selects texture animations.
pub const TOBJ_ANIM: u32 = 0x10;
/// `ALL_ANIM`: every animation kind.
pub const ALL_ANIM: u32 = 0x7FF;

/// `HSD_A_M_*` track ids (`mobj.h`) that `MObjUpdateFunc` switches on.
pub const HSD_A_M_AMBIENT_R: u8 = 1;
pub const HSD_A_M_AMBIENT_G: u8 = 2;
pub const HSD_A_M_AMBIENT_B: u8 = 3;
pub const HSD_A_M_DIFFUSE_R: u8 = 4;
pub const HSD_A_M_DIFFUSE_G: u8 = 5;
pub const HSD_A_M_DIFFUSE_B: u8 = 6;
pub const HSD_A_M_SPECULAR_R: u8 = 7;
pub const HSD_A_M_SPECULAR_G: u8 = 8;
pub const HSD_A_M_SPECULAR_B: u8 = 9;
pub const HSD_A_M_ALPHA: u8 = 10;
pub const HSD_A_M_PE_REF0: u8 = 11;
pub const HSD_A_M_PE_REF1: u8 = 12;
pub const HSD_A_M_PE_DSTALPHA: u8 = 13;

// `RENDER_*` bits of `HSD_MObj::rendermode` (`mobj.h`), transcribed for the
// callers that inspect them (`DObjLoad` reads the blending bits).
pub const RENDER_DIFFUSE_SHIFT: u32 = 0;
pub const RENDER_DIFFUSE_BITS: u32 = 3 << RENDER_DIFFUSE_SHIFT;
pub const RENDER_DIFFUSE_MAT0: u32 = 0 << RENDER_DIFFUSE_SHIFT;
pub const RENDER_DIFFUSE_MAT: u32 = 1 << RENDER_DIFFUSE_SHIFT;
pub const RENDER_DIFFUSE_VTX: u32 = 2 << RENDER_DIFFUSE_SHIFT;
pub const RENDER_DIFFUSE_BOTH: u32 = 3 << RENDER_DIFFUSE_SHIFT;
pub const RENDER_CONSTANT: u32 = 1 << 0;
pub const RENDER_VERTEX: u32 = 1 << 1;
pub const RENDER_DIFFUSE: u32 = 1 << 2;
pub const RENDER_SPECULAR: u32 = 1 << 3;
pub const CHANNEL_FIELD: u32 = RENDER_CONSTANT | RENDER_VERTEX | RENDER_DIFFUSE | RENDER_SPECULAR;
pub const RENDER_TEX0: u32 = 1 << 4;
pub const RENDER_TEX1: u32 = 1 << 5;
pub const RENDER_TEX2: u32 = 1 << 6;
pub const RENDER_TEX3: u32 = 1 << 7;
pub const RENDER_TEX4: u32 = 1 << 8;
pub const RENDER_TEX5: u32 = 1 << 9;
pub const RENDER_TEX6: u32 = 1 << 10;
pub const RENDER_TEX7: u32 = 1 << 11;
pub const RENDER_TEXTURES: u32 = RENDER_TEX0
    | RENDER_TEX1
    | RENDER_TEX2
    | RENDER_TEX3
    | RENDER_TEX4
    | RENDER_TEX5
    | RENDER_TEX6
    | RENDER_TEX7;
pub const RENDER_TOON: u32 = 1 << 12;
pub const RENDER_ALPHA_SHIFT: u32 = 13;
pub const RENDER_ALPHA_BITS: u32 = 3 << RENDER_ALPHA_SHIFT;
pub const RENDER_ALPHA_COMPAT: u32 = 0 << RENDER_ALPHA_SHIFT;
pub const RENDER_ALPHA_MAT: u32 = 1 << RENDER_ALPHA_SHIFT;
pub const RENDER_ALPHA_VTX: u32 = 2 << RENDER_ALPHA_SHIFT;
pub const RENDER_ALPHA_BOTH: u32 = 3 << RENDER_ALPHA_SHIFT;
pub const RENDER_SHADOW: u32 = 1 << 26;
pub const RENDER_ZMODE_ALWAYS: u32 = 1 << 27;
pub const RENDER_NO_ZUPDATE: u32 = 1 << 29;
pub const RENDER_XLU: u32 = 1 << 30;
pub const RENDER_BLENDING: u32 = RENDER_XLU | RENDER_NO_ZUPDATE;

/// `GXColor` (`dolphin/gx/GXStruct.h`): four `u8` channels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GxColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl GxColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> GxColor {
        GxColor { r, g, b, a }
    }
}

/// `HSD_Material` (`mobj.h`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub ambient: GxColor,
    pub diffuse: GxColor,
    pub specular: GxColor,
    pub alpha: f32,
    pub shininess: f32,
}

impl Default for Material {
    /// `HSD_MaterialAlloc` hands back zeroed memory; `MObjLoad` then copies
    /// the desc over it, so a default material is all zeros.
    fn default() -> Material {
        Material {
            ambient: GxColor::default(),
            diffuse: GxColor::default(),
            specular: GxColor::default(),
            alpha: 0.0,
            shininess: 0.0,
        }
    }
}

/// `HSD_PEDesc` (`mobj.h`): pixel-engine state. Stored verbatim; only
/// `ref0`, `ref1` and `dst_alpha` are animated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeDesc {
    pub flags: u8,
    pub ref0: u8,
    pub ref1: u8,
    pub dst_alpha: u8,
    pub ty: u8,
    pub src_factor: u8,
    pub dst_factor: u8,
    pub logic_op: u8,
    pub z_comp: u8,
    pub alpha_comp0: u8,
    pub alpha_op: u8,
    pub alpha_comp1: u8,
}

/// Prepared material and texture animation descriptors.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatAnim {
    pub aobjdesc: Option<AObjDesc>,
    pub textures: Vec<crate::tobj::TexAnim>,
}

/// `HSD_MObj` (`mobj.h`) without class headers or GPU expression objects.
#[derive(Debug, Clone, PartialEq)]
pub struct MObj {
    pub textures: Vec<crate::tobj::TObj>,
    /// `rendermode`: the `RENDER_*` bits.
    pub rendermode: u32,
    /// `mat`: the material colours. Always present after `MObjLoad`.
    pub mat: Material,
    /// `pe`: present when the desc carried a `pedesc`.
    pub pe: Option<PeDesc>,
    /// `aobj`: the material animation.
    pub aobj: Option<AObj>,
}

/// `(u8) (255.0 * val->fv)` as MWCC compiles it: the product is a double,
/// `fctiwz` truncates it to a saturated `s32` (NaN to the most negative
/// integer), and the byte store keeps the low eight bits.
pub(crate) fn scale_to_u8(fv: f32) -> u8 {
    let d: f64 = 255.0 * f64::from(fv);
    let i: i32 = if d.is_nan() { i32::MIN } else { d as i32 };
    i as u8
}

/// `MObjUpdateFunc` (`mobj.c:86`) on the fields it writes. Split from
/// [`MObj`] so the AObj can be borrowed while the callback runs.
fn material_update(mat: &mut Material, pe: Option<&mut PeDesc>, ty: u8, fv: f32) {
    match ty {
        HSD_A_M_AMBIENT_R => mat.ambient.r = scale_to_u8(fv),
        HSD_A_M_AMBIENT_G => mat.ambient.g = scale_to_u8(fv),
        HSD_A_M_AMBIENT_B => mat.ambient.b = scale_to_u8(fv),
        HSD_A_M_DIFFUSE_R => mat.diffuse.r = scale_to_u8(fv),
        HSD_A_M_DIFFUSE_G => mat.diffuse.g = scale_to_u8(fv),
        HSD_A_M_DIFFUSE_B => mat.diffuse.b = scale_to_u8(fv),
        HSD_A_M_ALPHA => mat.alpha = 1.0 - fv,
        HSD_A_M_SPECULAR_R => mat.specular.r = scale_to_u8(fv),
        HSD_A_M_SPECULAR_G => mat.specular.g = scale_to_u8(fv),
        HSD_A_M_SPECULAR_B => mat.specular.b = scale_to_u8(fv),
        HSD_A_M_PE_REF0 => {
            if let Some(pe) = pe {
                pe.ref0 = scale_to_u8(fv);
            }
        }
        HSD_A_M_PE_REF1 => {
            if let Some(pe) = pe {
                pe.ref1 = scale_to_u8(fv);
            }
        }
        HSD_A_M_PE_DSTALPHA => {
            if let Some(pe) = pe {
                pe.dst_alpha = scale_to_u8(fv);
            }
        }
        _ => {}
    }
}

impl MObj {
    /// Restore values and clocks for the same prepared material definition.
    pub fn restore_playback(&mut self, source: &Self) {
        self.rendermode = source.rendermode;
        self.mat = source.mat;
        self.pe = source.pe;
        if let (Some(target), Some(source)) = (&mut self.aobj, &source.aobj) {
            target.restore_playback(source);
        }
        assert_eq!(self.textures.len(), source.textures.len());
        for (target, source) in self.textures.iter_mut().zip(&source.textures) {
            target.restore_playback(source);
        }
    }

    /// Stop playback while retaining prepared track capacity for replacement.
    pub fn clear_prepared_animation(&mut self) {
        for clock in self.aobj.iter_mut().chain(
            self.textures
                .iter_mut()
                .filter_map(|t| t.animation.as_mut()),
        ) {
            clock.flags |= crate::aobj::AOBJ_NO_ANIM;
            clock.fobj.clear();
        }
    }
    /// `MObjLoad` (`mobj.c:152`) without the TObj and TEV steps: copies the
    /// desc's `rendermode` and material, ORs in `RENDER_TOON`, copies the
    /// optional `pedesc`, and starts with no animation.
    pub fn load(rendermode: u32, mat: Material, pe: Option<PeDesc>) -> MObj {
        MObj {
            textures: Vec::new(),
            rendermode: rendermode | RENDER_TOON,
            mat,
            pe,
            aobj: None,
        }
    }

    /// `HSD_MObjSetFlags` (`mobj.c:26`): ORs into `rendermode`.
    pub fn set_flags(&mut self, flags: u32) {
        self.rendermode |= flags;
    }

    /// `HSD_MObjClearFlags` (`mobj.c:33`).
    pub fn clear_flags(&mut self, flags: u32) {
        self.rendermode &= !flags;
    }

    /// `HSD_MObjRemoveAnimByFlags` (`mobj.c:40`).
    pub fn remove_anim_by_flags(&mut self, flags: u32) {
        if flags & TOBJ_ANIM != 0 {
            for texture in &mut self.textures {
                texture.animation = None;
            }
        }
        if flags & MOBJ_ANIM != 0 {
            self.aobj = None;
        }
    }

    /// `HSD_MObjAddAnim` (`mobj.c:55`): a `Some` matanim replaces the
    /// current material and matching texture AObjs; `None` leaves it alone.
    pub fn add_anim(&mut self, matanim: Option<&MatAnim>) {
        if let Some(matanim) = matanim {
            self.aobj = matanim.aobjdesc.as_ref().map(AObj::load_desc);
            for texture in &mut self.textures {
                texture.add_anim(&matanim.textures);
            }
        }
    }

    /// `HSD_MObjReqAnimByFlags` (`mobj.c:70`).
    pub fn req_anim_by_flags(&mut self, startframe: f32, flags: u32) {
        if flags & MOBJ_ANIM != 0 {
            if let Some(aobj) = self.aobj.as_mut() {
                aobj.req_anim(startframe);
            }
        }
        if flags & TOBJ_ANIM != 0 {
            for texture in &mut self.textures {
                texture.req_anim(startframe);
            }
        }
    }

    /// `HSD_MObjReqAnim` (`mobj.c:81`).
    pub fn req_anim(&mut self, startframe: f32) {
        self.req_anim_by_flags(startframe, ALL_ANIM);
    }

    /// `MObjUpdateFunc` (`mobj.c:86`) applied directly.
    pub fn update_func(&mut self, ty: u8, fv: f32) {
        material_update(&mut self.mat, self.pe.as_mut(), ty, fv);
    }

    /// `HSD_MObjAnim` (`mobj.c:143`): one `HSD_AObjInterpretAnim` step
    /// through `MObjUpdateFunc`. `cb` is the shared end-callback counter
    /// pass, followed by `HSD_TObjAnimAll`.
    pub fn anim(&mut self, cb: &mut AObjEndCallback) {
        let MObj { mat, pe, aobj, .. } = self;
        if let Some(aobj) = aobj {
            let mut pe = pe.as_mut();
            aobj.interpret_anim(
                &mut |ty, fv| material_update(mat, pe.as_deref_mut(), ty, fv),
                cb,
            );
        }
        for texture in &mut self.textures {
            texture.anim(cb);
        }
    }

    /// `HSD_MObjSetDiffuseColor` (`mobj.c:460`).
    pub fn set_diffuse_color(&mut self, r: u8, g: u8, b: u8) {
        self.mat.diffuse.r = r;
        self.mat.diffuse.g = g;
        self.mat.diffuse.b = b;
    }

    /// `HSD_MObjSetAlpha` (`mobj.c:467`).
    pub fn set_alpha(&mut self, alpha: f32) {
        self.mat.alpha = alpha;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_to_u8_matches_fctiwz_then_byte_store() {
        assert_eq!(scale_to_u8(0.0), 0);
        assert_eq!(scale_to_u8(1.0), 255);
        assert_eq!(scale_to_u8(0.5), 127);
        // 2.0 * 255 = 510 = 0x1FE -> low byte 0xFE.
        assert_eq!(scale_to_u8(2.0), 0xFE);
        assert_eq!(scale_to_u8(-1.0), 1); // -255 -> 0xFFFFFF01
        assert_eq!(scale_to_u8(f32::NAN), 0); // i32::MIN low byte
    }

    #[test]
    fn load_ors_toon() {
        let m = MObj::load(RENDER_XLU, Material::default(), None);
        assert_eq!(m.rendermode, RENDER_XLU | RENDER_TOON);
        assert!(m.aobj.is_none());
    }
}
