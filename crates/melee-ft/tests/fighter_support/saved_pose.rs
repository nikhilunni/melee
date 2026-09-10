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
    pub fn fighter_bytes(&self, address: u32, length: usize) -> Vec<u8> {
        self.bytes(address, length).to_vec()
    }
    pub fn resume_link(&self) -> u8 {
        let current_proc = word(self.bytes(0x804D_7838, 4), 0);
        let link = word(self.bytes(0x804D_7834, 4), 0);
        if current_proc == 0 && link == 24 {
            return 0;
        }
        assert_eq!(link, 4, "idle test supports the physics resume boundary");
        assert_eq!(word(self.bytes(current_proc + 0x14, 4), 0), 0x8006_B82C);
        link as u8
    }
    fn bytes(&self, address: u32, length: usize) -> &[u8] {
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
    /// Restore exact saved AObj/FObj cursors for a mid-tick animation boundary.
    /// Requesting the motion at its scalar frame cannot recover pending stream
    /// interpolation or the independently advancing blend/part tree.
    fn joint_animation(&self, joint: &mut JObj, address: u32) {
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
    pub fn restore<C: CharacterCallbacks>(&self, fighter: &mut Fighter<C>, raw: &[u8]) {
        let gobj = word(raw, 0);
        let address = word(self.bytes(gobj, 0x30), 0x2C);
        // A savestate may interrupt a tick; the trace records its completed
        // boundary. Only object identities survive the remaining procs unchanged:
        // Wait-animation data, the secondary animation table and bone-parts
        // array. Animation scalars, positions and state counters can advance
        // before the trace is captured.
        for offset in [0x24, 0x28, 0x5E8] {
            assert_eq!(
                word(self.bytes(address + offset as u32, 4), 0),
                word(raw, offset)
            );
        }
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
            self.joint_animation(fighter.skeleton.get_mut(part.joint), word(data, 0));
            self.joint_animation(
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
