//! HSD_TObjTevDesc (tobj.h), 32 bytes. These expressions transform a sampled
//! texture before the common color/alpha mapping stages.
use super::{unsupported, Result};
use crate::{Archive, Reader};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TevOperation {
    pub function: u8,
    pub bias: u8,
    pub scale: u8,
    pub clamp: bool,
    pub inputs: [u8; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureCombiner {
    pub color: TevOperation,
    pub alpha: TevOperation,
    /// Konst, register 0, register 1; these are authored constants, not previous
    /// stages. See MakeColorGenTExp in tobj.c.
    pub constants: [[u8; 4]; 3],
    pub active: u32,
}
impl TextureCombiner {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        Self::decode(&Reader::new(archive.data()), offset)
    }
    fn decode(r: &Reader<'_>, offset: u32) -> Result<Self> {
        let bytes = r.slice(offset, 32)?;
        let active = r.u32(offset + 28)?;
        let operation = |channel: usize| -> Result<TevOperation> {
            let op = TevOperation {
                function: bytes[channel],
                bias: bytes[2 + channel],
                scale: bytes[4 + channel],
                clamp: bytes[6 + channel] != 0,
                inputs: bytes[8 + channel * 4..12 + channel * 4].try_into().unwrap(),
            };
            if active & (1 << (30 + channel)) != 0 {
                let supported_function = matches!(op.function, 0 | 1 | 14 | 15)
                    || (channel == 0 && matches!(op.function, 8..=13));
                if !supported_function || op.bias > 3 || op.scale > 3 {
                    return Err(unsupported(
                        offset,
                        "texture combiner operation",
                        u32::from(op.function),
                    ));
                }
                for input in op.inputs {
                    let supported = if channel == 0 {
                        matches!(input, 8 | 9 | 12 | 13 | 15 | 128..=136)
                    } else {
                        matches!(input, 4 | 7 | 64..=69)
                    };
                    if !supported {
                        return Err(unsupported(
                            offset,
                            "texture combiner input",
                            u32::from(input),
                        ));
                    }
                }
            }
            Ok(op)
        };
        Ok(Self {
            color: operation(0)?,
            alpha: operation(1)?,
            constants: std::array::from_fn(|i| bytes[16 + i * 4..20 + i * 4].try_into().unwrap()),
            active,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_keeps_channels_constants_and_big_endian_activation() {
        let mut bytes = [0u8; 32];
        bytes[1] = 1;
        bytes[2] = 2;
        bytes[5] = 3;
        bytes[6] = 1;
        bytes[8..16].copy_from_slice(&[15, 8, 132, 128, 7, 4, 67, 68]);
        bytes[16..28].copy_from_slice(&[255, 128, 64, 32, 16, 8, 4, 2, 1, 3, 5, 7]);
        bytes[28..].copy_from_slice(&0xc000000fu32.to_be_bytes());
        let desc = TextureCombiner::decode(&Reader::new(&bytes), 0).unwrap();
        assert_eq!(desc.constants[1], [16, 8, 4, 2]);
        assert_eq!(desc.color.inputs, [15, 8, 132, 128]);
        assert_eq!(desc.alpha.inputs, [7, 4, 67, 68]);
        assert_eq!(desc.alpha.function, 1);
        assert_eq!(desc.active, 0xc000000f);
        assert!(TextureCombiner::decode(&Reader::new(&bytes[..31]), 0).is_err());
        bytes[8] = 0; // unsupported previous-stage selector
        assert!(TextureCombiner::decode(&Reader::new(&bytes), 0).is_err());
    }
}
