//! Texture animation state. Pixels stay in immutable archive/presentation assets;
//! tracks change descriptors and table selections, never image ownership.
use crate::aobj::{AObj, AObjDesc, AObjEndCallback};
use hsd_archive::visual::TextureDescriptor;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct TexAnim {
    pub id: u32,
    pub animation: Option<AObjDesc>,
    pub images: Arc<[Option<u32>]>,
    pub palettes: Arc<[Option<u32>]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TObj {
    pub descriptor: TextureDescriptor,
    pub animation: Option<AObj>,
    pub lod_bias: f32,
    images: Arc<[Option<u32>]>,
    palettes: Arc<[Option<u32>]>,
}
impl TObj {
    pub fn load(descriptor: TextureDescriptor) -> Self {
        Self {
            descriptor,
            animation: None,
            lod_bias: 0.0,
            images: Arc::from([]),
            palettes: Arc::from([]),
        }
    }
    /// HSD_TObjAddAnim: the texture map ID selects its animation record.
    pub fn add_anim(&mut self, animations: &[TexAnim]) {
        if let Some(anim) = animations.iter().find(|a| a.id == self.descriptor.id) {
            self.animation = anim.animation.as_ref().map(AObj::load_desc);
            self.images = Arc::clone(&anim.images);
            self.palettes = Arc::clone(&anim.palettes);
        }
    }
    pub fn req_anim(&mut self, frame: f32) {
        if let Some(animation) = &mut self.animation {
            animation.req_anim(frame);
        }
    }
    pub fn anim(&mut self, cb: &mut AObjEndCallback) {
        if let Some(animation) = &mut self.animation {
            let descriptor = &mut self.descriptor;
            let lod_bias = &mut self.lod_bias;
            let images = &self.images;
            let palettes = &self.palettes;
            animation.interpret_anim(
                &mut |track, value| {
                    update(descriptor, lod_bias, images, palettes, track, value);
                },
                cb,
            );
        }
    }
}
/// TObjUpdateFunc (0x8035E860, tobj.c:135): assignments and fctiwz conversions.
/// Retail asm --fused: no fused instructions.
/// Constant colors share MObj's double multiply and byte-store conversion.
fn update(
    d: &mut TextureDescriptor,
    lod_bias: &mut f32,
    images: &[Option<u32>],
    palettes: &[Option<u32>],
    track: u8,
    value: f32,
) {
    match track {
        1 => {
            let index = gekko_math::msl::fctiwz(value) as usize;
            if let Some(image) = images[index] {
                d.image = image;
            }
        }
        2..=3 => d.translation[usize::from(track - 2)] = value,
        4..=5 => d.scale[usize::from(track - 4)] = value,
        6..=8 => d.rotation[usize::from(track - 6)] = value,
        9 | 24 => d.blending = value,
        10 => {
            if !palettes.is_empty() {
                let index = gekko_math::msl::fctiwz(value) as u8;
                if let Some(palette) = palettes[usize::from(index)] {
                    d.palette = Some(palette);
                }
            }
        }
        11 => *lod_bias = value,
        12..=23 => {
            if let Some(combiner) = &mut d.combiner {
                let channel = usize::from(track - 12);
                combiner.constants[channel / 4][channel % 4] = crate::mobj::scale_to_u8(value);
            }
        }
        _ => {}
    }
}
