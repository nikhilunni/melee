//! Pixel-engine state from HSD_PEDesc, with HSD_SetupPEMode defaults.
use super::Result;
use crate::{Archive, Reader};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PixelState {
    pub color_write: bool,
    pub alpha_write: bool,
    pub depth_write: bool,
    /// GX comparison codes: never, less, equal, less/equal, greater, not/equal,
    /// greater/equal, always.
    pub depth_compare: u8,
    /// GX blend type and source/destination factor codes.
    pub blend: [u8; 3],
    pub alpha_compare: [u8; 2],
    pub alpha_reference: [u8; 2],
    pub alpha_operation: u8,
}
impl PixelState {
    pub fn from_render_mode(mode: u32) -> Self {
        let translucent = mode & (1 << 30) != 0;
        let depth_write = mode & (1 << 29) == 0;
        Self {
            color_write: true,
            alpha_write: false,
            depth_write,
            depth_compare: if mode & (1 << 27) != 0 { 7 } else { 3 },
            blend: [u8::from(translucent), 4, 5],
            alpha_compare: [if translucent && depth_write { 4 } else { 7 }; 2],
            alpha_reference: [0; 2],
            alpha_operation: 0,
        }
    }
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.data());
        let bytes = r.slice(offset, 12)?;
        let flags = bytes[0];
        Ok(Self {
            color_write: flags & 1 != 0,
            alpha_write: flags & 2 != 0,
            depth_write: flags & 0x20 != 0,
            depth_compare: if flags & 0x10 != 0 { bytes[8] } else { 7 },
            blend: [bytes[4], bytes[5], bytes[6]],
            alpha_compare: [bytes[9], bytes[11]],
            alpha_reference: [bytes[1], bytes[2]],
            alpha_operation: bytes[10],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translucent_depth_writers_reject_zero_alpha() {
        let opaque = PixelState::from_render_mode(0);
        assert_eq!(opaque.blend[0], 0);
        assert!(opaque.depth_write);
        assert_eq!(opaque.alpha_compare, [7; 2]);
        let cutout = PixelState::from_render_mode(1 << 30);
        assert_eq!(cutout.alpha_compare, [4; 2]);
        let overlay = PixelState::from_render_mode((1 << 30) | (1 << 29));
        assert!(!overlay.depth_write);
        assert_eq!(overlay.alpha_compare, [7; 2]);
    }
}
