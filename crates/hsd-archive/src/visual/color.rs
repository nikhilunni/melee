//! Shared GX packed color decoding for vertices and textures.
fn expand5(v: u16) -> u8 {
    ((v << 3) | (v >> 2)) as u8
}
pub(super) fn rgb565(v: u16) -> [u8; 4] {
    let green = (v >> 5) & 63;
    [
        expand5(v >> 11),
        ((green << 2) | (green >> 4)) as u8,
        expand5(v & 31),
        255,
    ]
}
pub(super) fn rgb5a3(v: u16) -> [u8; 4] {
    if v & 0x8000 != 0 {
        [
            expand5((v >> 10) & 31),
            expand5((v >> 5) & 31),
            expand5(v & 31),
            255,
        ]
    } else {
        let a = (v >> 12) & 7;
        [
            (((v >> 8) & 15) * 17) as u8,
            (((v >> 4) & 15) * 17) as u8,
            ((v & 15) * 17) as u8,
            ((a << 5) | (a << 2) | (a >> 1)) as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packed_colors_expand_bits_and_distinguish_alpha_mode() {
        assert_eq!(rgb565(16 << 5), [0, 65, 0, 255]);
        assert_eq!(rgb565(0xf800), [255, 0, 0, 255]);
        assert_eq!(rgb5a3(0xffff), [255, 255, 255, 255]);
        assert_eq!(rgb5a3(0x4f08), [255, 0, 136, 146]);
    }
}
