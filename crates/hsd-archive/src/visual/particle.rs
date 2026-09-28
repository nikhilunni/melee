//! HSD_PSTexGroup: offsets are relative to the particle texture bank, not DAT
//! relocations. Pixel decoding shares the model-texture implementation.
use super::{invalid, texture::decode_pixels, Result, Texture};
use crate::{Archive, Reader};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone)]
pub struct ParticleTexture {
    pub group: u16,
    pub image: u16,
    pub palette: u16,
    pub indexed: bool,
    pub texture: Arc<Texture>,
}
pub fn read_particle_textures(archive: &Archive, offset: u32) -> Result<Vec<ParticleTexture>> {
    let data = archive
        .data()
        .get(offset as usize..)
        .ok_or_else(|| invalid(offset, "particle bank out of bounds"))?;
    read_bank(data)
}
fn read_bank(data: &[u8]) -> Result<Vec<ParticleTexture>> {
    let r = Reader::new(data);
    let count = r.u32(0)?;
    r.slice(
        4,
        count
            .checked_mul(4)
            .ok_or_else(|| invalid(0, "particle group overflow"))?,
    )?;
    let mut textures = Vec::new();
    let mut cache = BTreeMap::new();
    for group in 0..count {
        let base = r.u32(4 + group * 4)?;
        if base == 0 {
            continue;
        }
        let header = r.slice(base, 24)?;
        let h = Reader::new(header);
        let images = h.u32(0)?;
        let format = h.u32(4)?;
        // psdisp.c:2116: the TLUT format is `(u8) tex_group->tlutfmt`; some
        // banks (EfCaData, EfYsData) set bits above the low byte.
        let palette_format = h.u32(8)? & 0xFF;
        let width =
            u16::try_from(h.u32(12)?).map_err(|_| invalid(base, "particle image too wide"))?;
        let height =
            u16::try_from(h.u32(16)?).map_err(|_| invalid(base, "particle image too tall"))?;
        let palettes = if matches!(format, 8..=10) {
            if h.u16(22)? & 1 != 0 {
                1
            } else {
                let n = u32::from(h.u16(20)?);
                if n == 0 {
                    images
                } else {
                    n
                }
            }
        } else {
            0
        };
        let pointers = images
            .checked_add(palettes)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| invalid(base, "particle pointer overflow"))?;
        let table = Reader::new(r.slice(crate::add_offset(base, 24)?, pointers)?);
        for image in 0..images {
            let pixels = table.u32(image * 4)?;
            if pixels == 0 {
                continue;
            }
            for palette in 0..palettes.max(1) {
                let colors = if palettes == 0 {
                    0
                } else {
                    table.u32((images + palette) * 4)?
                };
                if palettes != 0 && colors == 0 {
                    continue;
                }
                let key = (pixels, colors, format, width, height, palette_format);
                let texture = if let Some(texture) = cache.get(&key) {
                    Arc::clone(texture)
                } else {
                    let palette_data = if palettes == 0 {
                        None
                    } else {
                        let entries = match format {
                            8 => 16,
                            9 => 256,
                            _ => 16384,
                        };
                        Some((r.slice(colors, entries * 2)?, palette_format))
                    };
                    let bytes = data
                        .get(pixels as usize..)
                        .ok_or_else(|| invalid(pixels, "particle image out of bounds"))?;
                    let texture =
                        Arc::new(decode_pixels(bytes, width, height, format, palette_data)?);
                    cache.insert(key, Arc::clone(&texture));
                    texture
                };
                textures.push(ParticleTexture {
                    group: u16::try_from(group)
                        .map_err(|_| invalid(base, "particle group overflow"))?,
                    image: u16::try_from(image)
                        .map_err(|_| invalid(base, "particle image overflow"))?,
                    palette: palette as u16,
                    indexed: palettes != 0,
                    texture,
                });
            }
        }
    }
    Ok(textures)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bank_relative_images_share_pixels_and_reject_truncation() {
        let mut bytes = vec![0u8; 80];
        for (at, value) in [
            (0, 1u32),
            (4, 8),
            (8, 2),
            (12, 0),
            (20, 8),
            (24, 8),
            (32, 48),
            (36, 48),
        ] {
            bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        bytes[48..].fill(0xf0);
        let textures = read_bank(&bytes).unwrap();
        assert_eq!(textures.len(), 2);
        assert!(Arc::ptr_eq(&textures[0].texture, &textures[1].texture));
        assert_eq!(
            &textures[0].texture.rgba[..8],
            &[255, 255, 255, 255, 0, 0, 0, 0]
        );
        assert!(read_bank(&bytes[..79]).is_err());
    }
}
