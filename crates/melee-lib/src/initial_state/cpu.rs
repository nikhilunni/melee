//! CpuFighter (fp+1A88) from a retail Fighter dump. Fighter pointers
//! become fighter-list indices; the ring and script cursors become offsets.
use super::{float, word};
use hsd_types::{Vec2, Vec3};
use melee_ft::fighter::cpu::{
    CommandScript, CpuState, FollowRing, FollowSample, FOLLOW_ENTRIES, SCRIPT_BYTES,
};

/// CpuFighter's offset in Fighter.
const CPU: usize = 0x1A88;
/// Fighter_x1A88_xFC_t: the ring's entry size.
const SAMPLE_BYTES: u32 = 0x1C;

/// Restore `raw`'s CpuFighter; `base` is the dump's Fighter address and
/// `fighters` every dumped Fighter address in list order.
pub(super) fn restore(raw: &[u8], base: u32, fighters: &[u32]) -> CpuState {
    let at = |offset: usize| CPU + offset;
    let w = |offset: usize| word(raw, at(offset));
    let i = |offset: usize| w(offset) as i32;
    let f = |offset: usize| float(raw, at(offset));
    let v2 = |offset: usize| Vec2::new(f(offset), f(offset + 4));
    let byte = |offset: usize| raw[at(offset)];
    let bit = |offset: usize, mask: u8| byte(offset) & mask != 0;
    let fighter = |offset: usize| -> Option<usize> {
        let pointer = w(offset);
        if pointer == 0 {
            return None;
        }
        Some(
            fighters
                .iter()
                .position(|&b| b == pointer)
                .unwrap_or_else(|| panic!("CpuFighter +{offset:X}: {pointer:08X} is no fighter")),
        )
    };
    // x4C, x50: items the port cannot name at a boundary. HSD_ObjAlloc
    // leaves them uncleared for fighters that never chose one, so only a
    // pointer into MEM1 is a real item.
    let item = |offset: usize| -> Option<u32> {
        let pointer = w(offset);
        if (0x8000_0000..0x8180_0000).contains(&pointer) {
            unimplemented!("CpuFighter +{offset:X}: an item target at the boundary");
        }
        None
    };
    let attack_list = |array: usize, len: usize| {
        let mut list = melee_ft::fighter::cpu::AttackList {
            len: byte(len),
            ..Default::default()
        };
        for (k, script) in list.scripts.iter_mut().enumerate() {
            *script = i(array + 4 * k);
        }
        list
    };
    let ring_base = base + (CPU + 0xFC) as u32;
    let ring_index = |offset: usize| -> usize {
        let pointer = w(offset);
        let index = (pointer.wrapping_sub(ring_base) / SAMPLE_BYTES) as usize;
        assert!(
            index < FOLLOW_ENTRIES,
            "CpuFighter ring cursor {pointer:08X}"
        );
        index
    };
    let mut entries = [FollowSample::default(); FOLLOW_ENTRIES];
    for (n, entry) in entries.iter_mut().enumerate() {
        let o = 0xFC + n * SAMPLE_BYTES as usize;
        *entry = FollowSample {
            buttons: w(o),
            triggers: [byte(o + 4), byte(o + 5)],
            stick: [byte(o + 6) as i8, byte(o + 7) as i8],
            cstick: [byte(o + 8) as i8, byte(o + 9) as i8],
            position: Vec3::new(f(o + 0xC), f(o + 0x10), f(o + 0x14)),
            facing: f(o + 0x18),
        };
    }
    let buffer_base = base + (CPU + 0x454) as u32;
    let buffer_index = |pointer: u32| -> usize {
        let index = pointer.wrapping_sub(buffer_base) as usize;
        assert!(
            index <= SCRIPT_BYTES,
            "CpuFighter script cursor {pointer:08X}"
        );
        index
    };
    let mut buffer = [0; SCRIPT_BYTES];
    buffer.copy_from_slice(&raw[at(0x454)..at(0x454) + SCRIPT_BYTES]);
    let cursor = match w(0x450) {
        0 => None,
        pointer => Some(buffer_index(pointer)),
    };
    CpuState {
        buttons: w(0),
        stick: [byte(4) as i8, byte(5) as i8],
        cstick: [byte(6) as i8, byte(7) as i8],
        triggers: [byte(8), byte(9)],
        mode: i(0xC),
        level: i(0x10),
        x14: i(0x14),
        behavior: i(0x18),
        home_behavior: i(0x1C),
        x20: i(0x20),
        x24: i(0x24),
        x28: i(0x28),
        x2c: i(0x2C),
        target_lock_timer: i(0x30),
        attack_delay: i(0x34),
        destination_radius: f(0x38),
        x3c: f(0x3C),
        x40: f(0x40),
        target: fighter(0x44),
        locked_target: fighter(0x48),
        item_target: item(0x4C),
        x50: item(0x50).unwrap_or(0),
        destination: v2(0x54),
        x5c: f(0x5C),
        route_timer: i(0x60),
        route_destination: v2(0x64),
        x6c: v2(0x6C),
        x74: v2(0x74),
        reaction_timer: i(0x7C),
        x80: i(0x80),
        still_ticks: i(0x84),
        x88: i(0x88),
        x8c: i(0x8C),
        x90: i(0x90),
        x94: i(0x94),
        spawn_position: Vec3::new(f(0x98), f(0x9C), f(0xA0)),
        xa4: i(0xA4),
        allowed_attacks: attack_list(0xA8, 0xC8),
        excluded_attacks: attack_list(0xCC, 0xEC),
        xf8_b0: bit(0xF8, 0x80),
        xf8_b12: (byte(0xF8) >> 5) & 3,
        xf8_b34: (byte(0xF8) >> 3) & 3,
        xf8_b5: bit(0xF8, 0x04),
        xf8_b6: bit(0xF8, 0x02),
        xf8_b7: bit(0xF8, 0x01),
        target_locked: bit(0xF9, 0x80),
        xf9_b1: bit(0xF9, 0x40),
        xf9_b2: bit(0xF9, 0x20),
        xf9_b3: bit(0xF9, 0x10),
        xf9_b4: bit(0xF9, 0x08),
        xf9_b5: bit(0xF9, 0x04),
        xf9_b6: bit(0xF9, 0x02),
        xf9_b7: bit(0xF9, 0x01),
        xfa_b1: bit(0xFA, 0x40),
        xfa_b2: bit(0xFA, 0x20),
        xfa_b34: (byte(0xFA) >> 3) & 3,
        over_stage: bit(0xFA, 0x04),
        xfa_b6: bit(0xFA, 0x02),
        following: bit(0xFA, 0x01),
        xfb_b0: bit(0xFB, 0x80),
        follow: FollowRing {
            entries,
            write: ring_index(0x444),
            read: ring_index(0x448),
        },
        script: CommandScript {
            buffer,
            write: buffer_index(w(0x554)),
            cursor,
            duration: w(0x44C),
        },
        jump_height: f(0x558),
        hurtbox_extents: [f(0x55C), f(0x560), f(0x564), f(0x568)],
        x56c: f(0x56C),
        x570: f(0x570),
        half_size: [f(0x574), f(0x578)],
    }
}
