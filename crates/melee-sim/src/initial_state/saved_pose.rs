//! Read the owned Dolphin boundary, never a later oracle row. No game data is
//! generated or checked in. Extended savestate header v1 uses LZ4 blocks.
use super::{float, vector, word};
use hsd_anim::{jobj::JObj, quat::Quaternion};
use std::{fs, path::Path};

pub struct SavedPose {
    payload: Vec<u8>,
    ram_offset: usize,
}
impl SavedPose {
    #[cfg(test)]
    pub(super) fn test_memory(payload: Vec<u8>) -> Self {
        Self {
            payload,
            ram_offset: 0,
        }
    }

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
    /// Dolphin PowerPCManager::DoState serializes GPR[32], PC, NPC in
    /// host byte order. Locate that record using retail's immutable SDA bases,
    /// as MEM1 is located by its Fighter prefix, instead of a version-specific
    /// offset into the emulator payload. Require one unique register record.
    pub(super) fn cpu_general_registers(&self) -> anyhow::Result<([u32; 32], u32)> {
        let mut matches = self
            .payload
            .windows(48)
            .enumerate()
            .filter_map(|(offset, bytes)| {
                // r2 = _SDA2_BASE_, r13 = _SDA_BASE_ in NTSC-U 1.02.
                if bytes[..4] != 0x804D_F9E0_u32.to_le_bytes()
                    || bytes[44..48] != 0x804D_B6A0_u32.to_le_bytes()
                {
                    return None;
                }
                let start = offset.checked_sub(8)?;
                let record = self.payload.get(start..start + 136)?;
                let read = |i| u32::from_le_bytes(record[i..i + 4].try_into().unwrap());
                let registers: [u32; 32] = std::array::from_fn(|i| read(i * 4));
                let pc = read(128);
                ((0x8000_0000..0x8180_0000).contains(&registers[1])
                    && (0x8000_0000..0x8040_0000).contains(&pc))
                .then_some((registers, pc))
            });
        let result = matches
            .next()
            .ok_or_else(|| anyhow::anyhow!("saved CPU register record absent"))?;
        anyhow::ensure!(
            matches.next().is_none(),
            "ambiguous saved CPU register record"
        );
        Ok(result)
    }
    /// Saved linkage areas, bounded and strictly ascending within MEM1.
    pub(super) fn stack_returns(&self, mut stack: u32) -> anyhow::Result<Vec<(u32, u32)>> {
        let mut result = Vec::new();
        for _ in 0..128 {
            let frame = self.bytes(stack, 8);
            result.push((stack, word(frame, 4)));
            let next = word(frame, 0);
            if !(0x8000_0000..0x8180_0000).contains(&next) {
                return Ok(result);
            }
            anyhow::ensure!(
                next > stack && next.is_multiple_of(4),
                "invalid saved stack chain"
            );
            stack = next;
        }
        anyhow::bail!("saved stack exceeds 128 frames")
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
    /// Independent part poses can outlive the motion that selected them.
    /// Restore their saved AObj/FObj streams, rather than attaching the current
    /// main motion to the part-owned blend joints (jobj.h/aobj.h/fobj.h).
    fn part_animation(&self, joint: &mut JObj, address: u32) {
        use hsd_anim::{aobj::AObj, fobj::FObj};
        let pointer = word(self.bytes(address, 0x88), 0x7C);
        if pointer == 0 {
            joint.aobj = None;
            return;
        }
        let data = self.bytes(pointer, 0x1C);
        let mut tracks = Vec::new();
        let mut track = word(data, 0x14);
        while track != 0 {
            let f = self.bytes(track, 0x34);
            let head = word(f, 8);
            let length = word(f, 12);
            let mut decoded = FObj::new(
                self.bytes(head, length as usize),
                0.0,
                f[0x13],
                f[0x14],
                f[0x15],
            );
            decoded.pos = word(f, 4)
                .checked_sub(head)
                .expect("FObj cursor before stream") as usize;
            decoded.flags = f[0x10];
            decoded.op = f[0x11];
            decoded.op_intrp = f[0x12];
            decoded.nb_pack = u16::from_be_bytes(f[0x16..0x18].try_into().unwrap());
            decoded.startframe = i16::from_be_bytes(f[0x18..0x1A].try_into().unwrap());
            decoded.fterm = u16::from_be_bytes(f[0x1A..0x1C].try_into().unwrap());
            decoded.time = float(f, 0x1C);
            decoded.p0 = float(f, 0x20);
            decoded.p1 = float(f, 0x24);
            decoded.d0 = float(f, 0x28);
            decoded.d1 = float(f, 0x2C);
            tracks.push(decoded);
            track = word(f, 0);
        }
        joint.aobj = Some(AObj {
            flags: word(data, 0),
            curr_frame: float(data, 4),
            rewind_frame: float(data, 8),
            end_frame: float(data, 12),
            framerate: float(data, 16),
            fobj: tracks,
        });
    }
    pub fn restore(&self, fighter: &mut melee_ft::fighter::FighterCore, raw: &[u8]) {
        let gobj = word(raw, 0);
        let address = word(self.bytes(gobj, 0x30), 0x2C);
        assert_eq!(&self.bytes(address, 256)[..256], &raw[..256]);
        // These animation and command scalars must describe the same boundary.
        assert_eq!(self.bytes(address + 0x894, 0x18), &raw[0x894..0x8AC]);
        fighter.dynamics_use_floor_plane = raw[0x2228] & 0x40 != 0;
        for (i, set) in fighter.dynamics.iter_mut().enumerate() {
            let desc = 0x2F4 + i * 0x18;
            let mut address = word(raw, desc);
            assert_eq!(word(raw, desc + 4) as usize, set.bones.len());
            set.multipliers = vector(raw, desc + 8);
            for bone in &mut set.bones {
                let bytes = self.bytes(address, 0x98);
                bone.rest_rotation = Quaternion::new(
                    float(bytes, 4),
                    float(bytes, 8),
                    float(bytes, 12),
                    float(bytes, 16),
                );
                bone.rest_translate = vector(bytes, 0x14);
                bone.rest_scale = vector(bytes, 0x20);
                bone.position = vector(bytes, 0x2C);
                bone.velocity_axis = vector(bytes, 0x38);
                bone.angular_velocity = float(bytes, 0x44);
                bone.length = float(bytes, 0x48);
                bone.dominant_axis = word(bytes, 0x54) as i32;
                bone.gravity = float(bytes, 0x8C);
                // Static parameters must agree with the ftData reader.
                assert_eq!(bone.parameters.stiffness.to_bits(), word(bytes, 0x4C));
                assert_eq!(bone.parameters.damping.to_bits(), word(bytes, 0x84));
                assert_eq!(bone.parameters.max_step.to_bits(), word(bytes, 0x88));
                address = word(bytes, 0x90);
            }
            assert_eq!(address, 0);
        }
        let parts = word(raw, 0x5E8);
        for (i, part) in fighter.animation.parts.iter_mut().enumerate() {
            let data = self.bytes(parts + i as u32 * 16, 16);
            part.flags.0 = data[8];
            self.joint(fighter.skeleton.get_mut(part.joint), word(data, 0));
            self.joint(
                fighter.animation.blend_tree.get_mut(part.joint),
                word(data, 4),
            );
            if part
                .flags
                .contains(melee_ft::anim::attach::PartFlags::PART_ANIMATION)
            {
                self.part_animation(fighter.skeleton.get_mut(part.joint), word(data, 0));
                self.part_animation(
                    fighter.animation.blend_tree.get_mut(part.joint),
                    word(data, 4),
                );
            }
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
