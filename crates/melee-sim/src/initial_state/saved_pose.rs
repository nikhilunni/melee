//! Read the owned Dolphin boundary, never a later oracle row. No game data is
//! generated or checked in. Extended savestate header v1 uses LZ4 blocks.
use super::{float, vector, word};
use hsd_anim::{jobj::JObj, quat::Quaternion};
use melee_ft::fighter::{CharacterCallbacks, Fighter};
use std::{fs, path::Path};

pub struct SavedPose {
    payload: Vec<u8>,
    ram_offset: usize,
}
impl SavedPose {
    pub fn load(path: &Path, fighter: &[u8], address: u32) -> Self {
        let file = fs::read(path).unwrap();
        assert_eq!(&file[..6], b"GALE01");
        let le32 = |i| u32::from_le_bytes(file[i..i + 4].try_into().unwrap()) as usize;
        let header = 32 + le32(28);
        assert_eq!(&file[header..header + 4], &[1, 0, 1, 0], "savestate v1/LZ4");
        let size = u64::from_le_bytes(file[header + 8..header + 16].try_into().unwrap()) as usize;
        let mut cursor = header + 16 + le32(header + 4);
        let mut payload = Vec::with_capacity(size);
        while payload.len() < size {
            let length = le32(cursor);
            cursor += 4;
            decode_block(&file[cursor..cursor + length], &mut payload, size);
            cursor += length;
        }
        assert_eq!(payload.len(), size);
        assert_eq!(cursor, file.len());
        // Locate MEM1 using the ledger's saved Fighter prefix, not an emulator
        // version-dependent serialization offset. Both players are checked.
        let at = payload
            .windows(256)
            .position(|b| b == &fighter[..256])
            .expect("savestate/ledger mismatch");
        let ram_offset = at.checked_sub((address - 0x8000_0000) as usize).unwrap();
        Self {
            payload,
            ram_offset,
        }
    }
    pub(super) fn bytes(&self, address: u32, length: usize) -> &[u8] {
        assert!((0x8000_0000..0x8180_0000).contains(&address));
        let offset = self.ram_offset + (address - 0x8000_0000) as usize;
        &self.payload[offset..offset + length]
    }
    fn joint(&self, joint: &mut JObj, address: u32) {
        // HSD_JObj, jobj.h:104-125. Keep owned links/AObjs; restore saved SRT,
        // matrix validity and accumulated scale, including quaternion flags.
        let data = self.bytes(address, 0x88);
        joint.flags = word(data, 0x14);
        joint.rotate = Quaternion::new(
            float(data, 0x1C),
            float(data, 0x20),
            float(data, 0x24),
            float(data, 0x28),
        );
        joint.scale = vector(data, 0x2C);
        joint.translate = vector(data, 0x38);
        for row in 0..3 {
            for col in 0..4 {
                joint.mtx.0[row][col] = float(data, 0x44 + row * 16 + col * 4);
            }
        }
        let scale = word(data, 0x74);
        joint.scl = (scale != 0).then(|| vector(self.bytes(scale, 12), 0));
    }
    pub fn restore<C: CharacterCallbacks>(&self, fighter: &mut Fighter<C>, raw: &[u8]) {
        let gobj = word(raw, 0);
        let address = word(self.bytes(gobj, 0x30), 0x2C);
        assert_eq!(&self.bytes(address, 256)[..256], &raw[..256]);
        // These animation and command scalars must describe the same boundary.
        assert_eq!(self.bytes(address + 0x894, 0x18), &raw[0x894..0x8AC]);
        let parts = word(raw, 0x5E8);
        for (i, part) in fighter.animation.parts.iter_mut().enumerate() {
            let data = self.bytes(parts + i as u32 * 16, 16);
            part.flags.0 = data[8];
            self.joint(fighter.skeleton.get_mut(part.joint), word(data, 0));
            self.joint(
                fighter.animation.blend_tree.get_mut(part.joint),
                word(data, 4),
            );
        }
    }
}

// LZ4 block format: token nibbles encode literal and match lengths; length
// nibble 15 extends through bytes of 255. Matches can overlap their output.
// Bounds are checked and the declared savestate size caps expansion.
fn decode_block(input: &[u8], output: &mut Vec<u8>, limit: usize) {
    fn length(input: &[u8], cursor: &mut usize, nibble: u8) -> usize {
        let mut length = usize::from(nibble);
        if nibble == 15 {
            loop {
                let byte = input[*cursor];
                *cursor += 1;
                length += usize::from(byte);
                if byte != 255 {
                    break;
                }
            }
        }
        length
    }
    let mut cursor = 0;
    while cursor < input.len() {
        let token = input[cursor];
        cursor += 1;
        let literals = length(input, &mut cursor, token >> 4);
        assert!(output.len() + literals <= limit);
        output.extend_from_slice(&input[cursor..cursor + literals]);
        cursor += literals;
        if cursor == input.len() {
            break;
        }
        let offset = u16::from_le_bytes(input[cursor..cursor + 2].try_into().unwrap()) as usize;
        cursor += 2;
        assert!(offset > 0 && offset <= output.len());
        let count = 4 + length(input, &mut cursor, token & 15);
        assert!(output.len() + count <= limit);
        for _ in 0..count {
            output.push(output[output.len() - offset]);
        }
    }
}
