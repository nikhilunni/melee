//! Owned particle programs and texture metadata. Bank-internal offsets are
//! relative to the public bank, independently of the enclosing DAT relocations.
use hsd_archive::{Archive, Reader};
use std::{
    fmt,
    sync::{Arc, LazyLock},
};

// Empty texture groups still share one owner; constructing an empty Arc for
// every particle allocates a reference-count header even with no image bytes.
static EMPTY_IMAGES: LazyLock<Arc<[bool]>> = LazyLock::new(|| Arc::from([]));
pub(crate) fn empty_images() -> Arc<[bool]> {
    Arc::clone(&EMPTY_IMAGES)
}

/// `HSD_PSCmdList` (psstructs.h), loaded by `psInitDataBankLocate` (0x80398614).
#[derive(Clone, Debug, PartialEq)]
pub struct Descriptor {
    pub generator_type: u16,
    pub texture_group: u16,
    pub generator_life: u16,
    pub particle_life: u16,
    pub kind: u32,
    pub gravity: f32,
    pub friction: f32,
    pub velocity: [f32; 3],
    pub radius: f32,
    pub angle: f32,
    pub emission_rate: f32,
    pub size: f32,
    pub parameters: [f32; 3],
    /// Bytecode plus any alignment padding before the next descriptor.
    pub program: Arc<[u8]>,
}

/// `HSD_PSTexGroup` (psstructs.h); texture pixels stay outside simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureGroup {
    pub format: u32,
    pub palette_format: u32,
    pub width: u32,
    pub height: u32,
    pub palette_flags: u16,
    pub images: Arc<[bool]>,
    pub palettes: Vec<bool>,
}

/// `psInitDataBankLoad` (particle.c, 0x803984F4), without global bank slots.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleBank {
    pub version: u16,
    pub first_descriptor_id: u32,
    pub descriptors: Arc<[Option<Descriptor>]>,
    pub textures: Arc<[Option<TextureGroup>]>,
}

#[derive(Debug)]
pub enum BankError {
    Archive(hsd_archive::Error),
    MissingPublic(String),
    UnsupportedVersion(u16),
    InvalidLayout(&'static str),
}
impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(e) => e.fmt(f),
            Self::MissingPublic(name) => write!(f, "missing particle bank public {name}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported particle bank version {v:#x}"),
            Self::InvalidLayout(reason) => write!(f, "invalid particle bank: {reason}"),
        }
    }
}
impl std::error::Error for BankError {}
impl From<hsd_archive::Error> for BankError {
    fn from(value: hsd_archive::Error) -> Self {
        Self::Archive(value)
    }
}
type Result<T> = std::result::Result<T, BankError>;

impl ParticleBank {
    /// Read two public banks as `psInitDataBankLocate` (0x80398614) would.
    /// The next public bounds each bank; image data itself is not copied.
    pub fn from_archive(archive: &Archive, commands: &str, textures: &str) -> Result<Self> {
        Self::from_bytes(
            public_bytes(archive, commands)?,
            public_bytes(archive, textures)?,
        )
    }

    /// Read standalone bank blobs; useful for separately loaded effect banks.
    pub fn from_bytes(commands: &[u8], textures: &[u8]) -> Result<Self> {
        let reader = Reader::new(commands);
        let version = reader.u16(0)?;
        let (first_descriptor_id, count, table) = match version {
            0 => (0, reader.u32(4)?, 8),
            0x40..=0x43 => (reader.u32(4)?, reader.u32(8)?, 12),
            _ => return Err(BankError::UnsupportedVersion(version)),
        };
        first_descriptor_id
            .checked_add(count)
            .ok_or(BankError::InvalidLayout("descriptor ID overflow"))?;
        let offsets = read_table(reader, table, count)?;
        let mut descriptors = Vec::with_capacity(offsets.len());
        for &offset in &offsets {
            if offset == 0 {
                descriptors.push(None);
                continue;
            }
            let end = offsets
                .iter()
                .copied()
                .filter(|&next| next > offset)
                .min()
                .unwrap_or(commands.len() as u32);
            if end < offset || end - offset < 60 {
                return Err(BankError::InvalidLayout("overlapping descriptor header"));
            }
            descriptors.push(Some(read_descriptor(Reader::new(
                reader.slice(offset, end - offset)?,
            ))?));
        }
        let texture_reader = Reader::new(textures);
        let texture_offsets = read_table(texture_reader, 4, texture_reader.u32(0)?)?;
        let textures = texture_offsets
            .into_iter()
            .map(|offset| {
                if offset == 0 {
                    Ok(None)
                } else {
                    read_texture(texture_reader, offset).map(Some)
                }
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            version,
            first_descriptor_id,
            descriptors: descriptors.into(),
            textures,
        })
    }

    pub fn descriptor(&self, id: u32) -> Option<&Descriptor> {
        let index = id.checked_sub(self.first_descriptor_id)? as usize;
        self.descriptors.get(index)?.as_ref()
    }
}

fn public_bytes<'a>(archive: &'a Archive, name: &str) -> Result<&'a [u8]> {
    let start = archive
        .public(name)
        .ok_or_else(|| BankError::MissingPublic(name.into()))?;
    let end = archive
        .publics()
        .iter()
        .map(|p| p.offset)
        .filter(|&p| p > start)
        .min()
        .unwrap_or(archive.data().len() as u32);
    Ok(archive.reader().slice(start, end - start)?)
}

fn read_table(reader: Reader<'_>, start: u32, count: u32) -> Result<Vec<u32>> {
    let size = count
        .checked_mul(4)
        .ok_or(BankError::InvalidLayout("pointer table size overflow"))?;
    let table = Reader::new(reader.slice(start, size)?);
    (0..count).map(|i| Ok(table.u32(i * 4)?)).collect()
}

fn read_descriptor(r: Reader<'_>) -> Result<Descriptor> {
    Ok(Descriptor {
        generator_type: r.u16(0)?,
        texture_group: r.u16(2)?,
        generator_life: r.u16(4)?,
        particle_life: r.u16(6)?,
        // Locate clears bits 25..27 and selects blend mode 4 (bit 27).
        kind: (r.u32(8)? & 0xf1ff_ffff) | 0x0800_0000,
        gravity: r.f32(12)?,
        friction: r.f32(16)?,
        velocity: [r.f32(20)?, r.f32(24)?, r.f32(28)?],
        radius: r.f32(32)?,
        angle: r.f32(36)?,
        emission_rate: r.f32(40)?,
        size: r.f32(44)?,
        parameters: [r.f32(48)?, r.f32(52)?, r.f32(56)?],
        program: r.slice(60, (r.len() - 60) as u32)?.into(),
    })
}

fn read_texture(reader: Reader<'_>, offset: u32) -> Result<TextureGroup> {
    let remaining = (reader.len() as u32)
        .checked_sub(offset)
        .ok_or(BankError::InvalidLayout("texture offset out of range"))?;
    let r = Reader::new(reader.slice(offset, remaining)?);
    let count = r.u32(0)?;
    let format = r.u32(4)?;
    let palette_flags = r.u16(22)?;
    let palette_count = if matches!(format, 8..=10) {
        if palette_flags & 1 != 0 {
            1
        } else if r.u16(20)? != 0 {
            u32::from(r.u16(20)?)
        } else {
            count
        }
    } else {
        0
    };
    let total = count
        .checked_add(palette_count)
        .ok_or(BankError::InvalidLayout("texture count overflow"))?;
    let pointers = read_table(r, 24, total)?;
    // Validate every non-null bank-relative pointer before discarding addresses.
    for &pointer in &pointers {
        if pointer != 0 {
            reader.slice(pointer, 1)?;
        }
    }
    Ok(TextureGroup {
        format,
        palette_format: r.u32(8)?,
        width: r.u32(12)?,
        height: r.u32(16)?,
        palette_flags,
        images: pointers[..count as usize].iter().map(|&p| p != 0).collect(),
        palettes: pointers[count as usize..].iter().map(|&p| p != 0).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn word(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    fn commands(version: u16, first: u32) -> Vec<u8> {
        let mut bytes = vec![0; 84];
        bytes[..2].copy_from_slice(&version.to_be_bytes());
        if version == 0 {
            word(&mut bytes, 4, 2);
            word(&mut bytes, 8, 20);
        } else {
            word(&mut bytes, 4, first);
            word(&mut bytes, 8, 2);
            word(&mut bytes, 12, 20);
        }
        bytes[20..24].copy_from_slice(&[0, 8, 0, 2]);
        word(&mut bytes, 28, 0xffff_ffff);
        word(&mut bytes, 40, (-0.0f32).to_bits());
        bytes[80] = 0xff;
        bytes
    }
    #[test]
    fn versions_preserve_ids_null_slots_and_float_bits() {
        for version in [0, 0x40, 0x41, 0x42, 0x43] {
            let first = if version == 0 { 0 } else { 30000 };
            let bank = ParticleBank::from_bytes(&commands(version, first), &[0; 4]).unwrap();
            let descriptor = bank.descriptor(first).unwrap();
            assert_eq!(descriptor.generator_type, 8);
            assert_eq!(descriptor.texture_group, 2);
            assert_eq!(descriptor.kind, 0xf9ff_ffff);
            assert_eq!(descriptor.velocity[0].to_bits(), (-0.0f32).to_bits());
            assert_eq!(descriptor.program.as_ref(), [0xff, 0, 0, 0]);
            assert!(bank.descriptor(first + 1).is_none());
            assert!(bank.descriptor(first + 2).is_none());
        }
    }
    #[test]
    fn palette_modes_keep_null_images_and_palettes() {
        for (format, flags, palettes, expected) in
            [(8, 1, 0, 1), (9, 0, 3, 3), (10, 0, 0, 2), (6, 0, 3, 0)]
        {
            let mut textures = vec![0; 80];
            word(&mut textures, 0, 2);
            word(&mut textures, 4, 12);
            word(&mut textures, 12, 2);
            word(&mut textures, 16, format);
            textures[32..34].copy_from_slice(&(palettes as u16).to_be_bytes());
            textures[34..36].copy_from_slice(&(flags as u16).to_be_bytes());
            word(&mut textures, 36, 76);
            let bank = ParticleBank::from_bytes(&commands(0, 0), &textures).unwrap();
            let texture = bank.textures[0].as_ref().unwrap();
            assert_eq!(texture.images.as_ref(), [true, false]);
            assert_eq!(texture.palettes.len(), expected);
            assert!(bank.textures[1].is_none());
        }
    }
    #[test]
    fn malformed_banks_return_errors() {
        assert!(matches!(
            ParticleBank::from_bytes(&commands(1, 0), &[0; 4]),
            Err(BankError::UnsupportedVersion(1))
        ));
        let mut bytes = commands(0x42, 0);
        word(&mut bytes, 8, u32::MAX);
        assert!(ParticleBank::from_bytes(&bytes, &[0; 4]).is_err());
        let mut bytes = commands(0, 0);
        word(&mut bytes, 8, 82);
        assert!(ParticleBank::from_bytes(&bytes, &[0; 4]).is_err());
        assert!(ParticleBank::from_bytes(&[], &[]).is_err());
    }
}
