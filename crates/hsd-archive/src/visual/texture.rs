//! GX tiled base-level texture decoding from HSD_ImageDesc / HSD_TlutDesc.
//! Original bytes remain immutable; decoded RGBA is suitable for any GPU API.
use super::color::{rgb565, rgb5a3};
use super::{invalid, unsupported, Result};
use crate::{add_offset, Archive, Reader};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct TextureLayer {
    pub flags: u32,
    pub repeat: [u8; 2],
    pub blending: f32,
    pub nearest: bool,
    pub combiner: Option<super::TextureCombiner>,
    pub wrap_s: u32,
    pub wrap_t: u32,
    pub scale: [f32; 3],
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub image: Arc<Texture>,
}
/// Archive-local decoding cache. Repeated material references share immutable
/// images; each archive has its own identity space and cache. No global state.
pub type DecodedImages = BTreeMap<(u32, Option<u32>), Arc<Texture>>;
pub struct TextureDecoder<'a> {
    archive: &'a Archive,
    images: BTreeMap<(u32, Option<u32>), Arc<Texture>>,
}
impl<'a> TextureDecoder<'a> {
    pub fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            images: BTreeMap::new(),
        }
    }
    pub fn into_images(self) -> DecodedImages {
        self.images
    }
    pub fn with_images(archive: &'a Archive, images: DecodedImages) -> Self {
        Self { archive, images }
    }
    pub fn image(&mut self, offset: u32, palette: Option<u32>) -> Result<Arc<Texture>> {
        let key = (offset, palette);
        if let Some(image) = self.images.get(&key) {
            return Ok(Arc::clone(image));
        }
        let image = Arc::new(read_image(self.archive, offset, palette)?);
        self.images.insert(key, Arc::clone(&image));
        Ok(image)
    }
    /// Decode TObj pixels through the same descriptor reader used by animation.
    pub fn read_chain(&mut self, next: Option<u32>) -> Result<Vec<TextureLayer>> {
        TextureDescriptor::read_chain(self.archive, next)?
            .into_iter()
            .map(|d| {
                Ok(TextureLayer {
                    flags: d.flags,
                    repeat: d.repeat,
                    blending: d.blending,
                    nearest: d.nearest,
                    combiner: d.combiner,
                    wrap_s: d.wrap_s,
                    wrap_t: d.wrap_t,
                    rotation: d.rotation,
                    scale: d.scale,
                    translation: d.translation,
                    image: self.image(d.image, d.palette)?,
                })
            })
            .collect()
    }
}
/// Immutable TObj descriptor without decoded pixels. Simulation and renderer
/// share this reader while owning their mutable state separately.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureDescriptor {
    pub id: u32,

    pub flags: u32,
    pub repeat: [u8; 2],
    pub blending: f32,
    pub nearest: bool,
    pub combiner: Option<super::TextureCombiner>,
    pub wrap_s: u32,
    pub wrap_t: u32,
    pub scale: [f32; 3],
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub image: u32,
    pub palette: Option<u32>,
}
impl TextureDescriptor {
    pub fn read_chain(archive: &Archive, mut next: Option<u32>) -> Result<Vec<Self>> {
        let r = Reader::new(archive.data());
        let mut seen = std::collections::BTreeSet::new();
        let mut result = Vec::new();
        while let Some(offset) = next {
            if !seen.insert(offset) {
                return Err(invalid(offset, "texture chain cycle"));
            }
            r.slice(offset, 92)?;
            let vec3 =
                |at| -> Result<[f32; 3]> { Ok([r.f32(at)?, r.f32(at + 4)?, r.f32(at + 8)?]) };
            let image = archive
                .link(offset + 76)?
                .ok_or_else(|| invalid(offset, "missing texture image"))?;
            result.push(Self {
                id: r.u32(offset + 8)?,
                flags: r.u32(offset + 64)?,
                repeat: [r.u8(offset + 60)?, r.u8(offset + 61)?],
                blending: r.f32(offset + 68)?,
                nearest: r.u32(offset + 72)? == 0,
                combiner: archive
                    .link(offset + 88)?
                    .map(|at| super::TextureCombiner::read(archive, at))
                    .transpose()?,
                wrap_s: r.u32(offset + 52)?,
                wrap_t: r.u32(offset + 56)?,
                rotation: vec3(offset + 16)?,
                scale: vec3(offset + 28)?,
                translation: vec3(offset + 40)?,
                image,
                palette: archive.link(offset + 80)?,
            });
            next = archive.link(offset + 4)?;
        }
        Ok(result)
    }
}
/// Decode the base level; mip generation and filtering are renderer policy.
pub fn read_image(archive: &Archive, offset: u32, palette: Option<u32>) -> Result<Texture> {
    let r = Reader::new(archive.data());
    r.slice(offset, 24)?;
    let data = archive
        .link(offset)?
        .ok_or_else(|| invalid(offset, "missing image bytes"))?;
    let width = r.u16(offset + 4)?;
    let height = r.u16(offset + 6)?;
    let format = r.u32(offset + 8)?;
    let palette = if matches!(format, 8..=10) {
        let p = palette.ok_or_else(|| invalid(offset, "indexed image lacks palette"))?;
        r.slice(p, 14)?;
        let base = archive
            .link(p)?
            .ok_or_else(|| invalid(p, "missing palette data"))?;
        let count = r.u16(p + 12)?;
        Some((r.slice(base, u32::from(count) * 2)?, r.u32(p + 4)?))
    } else {
        None
    };
    decode_pixels(
        archive
            .data()
            .get(data as usize..)
            .ok_or_else(|| invalid(data, "image out of bounds"))?,
        width,
        height,
        format,
        palette,
    )
}
/// One GX decoder serves archive ImageDesc and particle texture-bank images.
pub(super) fn decode_pixels(
    data: &[u8],
    width: u16,
    height: u16,
    format: u32,
    palette: Option<(&[u8], u32)>,
) -> Result<Texture> {
    let offset = 0;
    if width == 0 || height == 0 {
        return Err(invalid(offset, "empty image"));
    }
    let (bw, bh, bytes) = match format {
        0 | 8 => (8u32, 8u32, 32u32),
        1 | 2 | 9 => (8, 4, 32),
        3 | 4 | 5 | 10 => (4, 4, 32),
        6 => (4, 4, 64),
        14 => (8, 8, 32),
        _ => return Err(unsupported(offset, "GX texture format", format)),
    };
    let nx = u32::from(width).div_ceil(bw);
    let ny = u32::from(height).div_ceil(bh);
    let size = nx
        .checked_mul(ny)
        .and_then(|n| n.checked_mul(bytes))
        .ok_or_else(|| invalid(offset, "image size overflow"))?;
    // Validate source length before allocating decoded pixels.
    let source = Reader::new(Reader::new(data).slice(0, size)?);
    if matches!(format, 8..=10) && palette.is_none() {
        return Err(invalid(0, "indexed image lacks palette"));
    }
    let palette = palette.map(|(bytes, format)| (Reader::new(bytes), format));
    let mut rgba = vec![0; usize::from(width) * usize::from(height) * 4];
    for by in 0..ny {
        for bx in 0..nx {
            let tile = (by * nx + bx) * bytes;
            for y in 0..bh {
                for x in 0..bw {
                    let px = bx * bw + x;
                    let py = by * bh + y;
                    if px >= u32::from(width) || py >= u32::from(height) {
                        continue;
                    }
                    let i = y * bw + x;
                    let color = match format {
                        0 => {
                            let v = nibble(source.u8(tile + i / 2)?, i) * 17;
                            [v, v, v, v]
                        }
                        1 => {
                            let v = source.u8(tile + i)?;
                            [v, v, v, v]
                        }
                        2 => {
                            let v = source.u8(tile + i)?;
                            let a = (v >> 4) * 17;
                            let v = (v & 15) * 17;
                            [v, v, v, a]
                        }
                        3 => {
                            let a = source.u8(tile + i * 2)?;
                            let v = source.u8(tile + i * 2 + 1)?;
                            [v, v, v, a]
                        }
                        4 => rgb565(source.u16(tile + i * 2)?),
                        5 => rgb5a3(source.u16(tile + i * 2)?),
                        6 => [
                            source.u8(tile + i * 2 + 1)?,
                            source.u8(tile + 32 + i * 2)?,
                            source.u8(tile + 33 + i * 2)?,
                            source.u8(tile + i * 2)?,
                        ],
                        8..=10 => {
                            let index = match format {
                                8 => u32::from(nibble(source.u8(tile + i / 2)?, i)),
                                9 => u32::from(source.u8(tile + i)?),
                                _ => u32::from(source.u16(tile + i * 2)? & 0x3fff),
                            };
                            let (pal, ty) = palette.as_ref().unwrap();
                            let value = pal.u16(index * 2)?;
                            match ty {
                                0 => {
                                    let v = value as u8;
                                    [v, v, v, (value >> 8) as u8]
                                }
                                1 => rgb565(value),
                                2 => rgb5a3(value),
                                _ => return Err(unsupported(offset, "GX palette format", *ty)),
                            }
                        }
                        14 => cmpr(
                            source,
                            add_offset(tile, ((y / 4) * 2 + x / 4) * 8)?,
                            x % 4,
                            y % 4,
                        )?,
                        _ => unreachable!(),
                    };
                    let dest = (py as usize * usize::from(width) + px as usize) * 4;
                    rgba[dest..dest + 4].copy_from_slice(&color);
                }
            }
        }
    }
    Ok(Texture {
        width,
        height,
        rgba,
    })
}
fn nibble(byte: u8, index: u32) -> u8 {
    if index.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 15
    }
}
fn cmpr(r: Reader<'_>, offset: u32, x: u32, y: u32) -> Result<[u8; 4]> {
    let a = r.u16(offset)?;
    let b = r.u16(offset + 2)?;
    let mut colors = [rgb565(a), rgb565(b), [0; 4], [0; 4]];
    for (c, (first, second)) in colors[0].into_iter().zip(colors[1]).take(3).enumerate() {
        let first = u16::from(first);
        let second = u16::from(second);
        if a > b {
            colors[2][c] = ((2 * first + second) / 3) as u8;
            colors[3][c] = ((first + 2 * second) / 3) as u8;
        } else {
            colors[2][c] = ((first + second) / 2) as u8;
            colors[3][c] = colors[2][c];
        }
    }
    colors[2][3] = 255;
    colors[3][3] = if a > b { 255 } else { 0 };
    let index = (r.u8(offset + 4 + y)? >> (6 - x * 2)) & 3;
    Ok(colors[usize::from(index)])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(width: u16, height: u16, format: u32, data: &[u8]) -> Archive {
        let size = 24 + data.len();
        let mut bytes = vec![0u8; 32 + size + 4];
        let total = bytes.len() as u32;
        bytes[0..4].copy_from_slice(&total.to_be_bytes());
        bytes[4..8].copy_from_slice(&(size as u32).to_be_bytes());
        bytes[8..12].copy_from_slice(&1u32.to_be_bytes());
        bytes[32..36].copy_from_slice(&24u32.to_be_bytes());
        bytes[36..38].copy_from_slice(&width.to_be_bytes());
        bytes[38..40].copy_from_slice(&height.to_be_bytes());
        bytes[40..44].copy_from_slice(&format.to_be_bytes());
        bytes[56..56 + data.len()].copy_from_slice(data);
        Archive::parse(&bytes).unwrap()
    }
    #[test]
    fn rgba8_reassembles_split_planes_and_crops_padded_tile() {
        let mut tile = [0u8; 64];
        for i in 0..16 {
            tile[i * 2] = 128;
            tile[i * 2 + 1] = i as u8;
            tile[32 + i * 2] = 64;
            tile[33 + i * 2] = 255;
        }
        let decoded = read_image(&image(2, 2, 6, &tile), 0, None).unwrap();
        assert_eq!(
            decoded.rgba,
            [0, 64, 255, 128, 1, 64, 255, 128, 4, 64, 255, 128, 5, 64, 255, 128]
        );
    }
    #[test]
    fn intensity_tiles_preserve_nibble_order_across_tile_boundaries() {
        let mut data = [0x12u8; 64];
        data[32..].fill(0x34);
        let decoded = read_image(&image(10, 1, 0, &data), 0, None).unwrap();
        let red: Vec<_> = decoded.rgba.chunks_exact(4).map(|p| p[0]).collect();
        assert_eq!(red, [17, 34, 17, 34, 17, 34, 17, 34, 51, 68]);
    }
    #[test]
    fn cmpr_uses_big_endian_selector_order_and_transparent_palette_entry() {
        let block = [0xf8, 0, 0x07, 0xe0, 0x1b, 0, 0, 0];
        let r = Reader::new(&block);
        assert_eq!(cmpr(r, 0, 0, 0).unwrap(), [255, 0, 0, 255]);
        assert_eq!(cmpr(r, 0, 1, 0).unwrap(), [0, 255, 0, 255]);
        assert_eq!(cmpr(r, 0, 2, 0).unwrap(), [170, 85, 0, 255]);
        assert_eq!(cmpr(r, 0, 3, 0).unwrap(), [85, 170, 0, 255]);
        let transparent = [0, 0, 255, 255, 255, 0, 0, 0];
        assert_eq!(
            cmpr(Reader::new(&transparent), 0, 0, 0).unwrap(),
            [127, 127, 127, 0]
        );
    }
    #[test]
    fn repeated_image_references_share_only_within_their_archive() {
        let first = image(4, 4, 6, &[0; 64]);
        let second = image(4, 4, 6, &[255; 64]);
        let mut a = TextureDecoder::new(&first);
        let x = a.image(0, None).unwrap();
        assert!(Arc::ptr_eq(&x, &a.image(0, None).unwrap()));
        let y = TextureDecoder::new(&second).image(0, None).unwrap();
        assert!(!Arc::ptr_eq(&x, &y));
        assert_ne!(x.rgba, y.rgba);
    }
    #[test]
    fn truncated_images_and_missing_palettes_are_rejected() {
        assert!(read_image(&image(4, 4, 6, &[0; 32]), 0, None).is_err());
        assert!(read_image(&image(8, 8, 8, &[0; 32]), 0, None).is_err());
        assert!(read_image(&image(1, 1, 99, &[]), 0, None).is_err());
    }
}
