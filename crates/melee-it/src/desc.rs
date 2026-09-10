//! Item archive boundary: `it/types.h` Article and ItemCommonData.
use hsd_archive::{Archive, Result};
use melee_cmd::Command;
use melee_types::combat::HitboxDescriptor;

#[derive(Clone, Debug)]
pub struct ItemCommonData {
    pub hold_limits: [Option<usize>; 13],
    pub lifetime: f32,
}
impl ItemCommonData {
    /// Item_80266FCC: maps the common data fields into hold-kind counters.
    /// Categories 4, 6 and 8 have no admission limit in Item_8026784C.
    pub fn read(archive: &Archive, public: u32) -> Result<Self> {
        let r = archive.reader();
        let base = r.u32(public)?;
        let mut hold_limits = [None; 13];
        for (kind, offset) in [
            (0, 0),
            (1, 4),
            (2, 8),
            (9, 12),
            (10, 16),
            (7, 24),
            (5, 28),
            (12, 32),
            (11, 36),
            (3, 40),
        ] {
            hold_limits[kind] = Some(r.u32(base + offset)? as usize);
        }
        Ok(Self {
            hold_limits,
            lifetime: r.u32(base + 0x30)? as f32,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ItemAssets {
    pub scripts: Vec<Vec<Command>>,
    pub hit_flags: Vec<Vec<Option<ItemHitFlags>>>,
    pub special_attributes: Vec<f32>,
    pub scale: f32,
    pub model: u32,
    pub rotate_to_facing: bool,
    pub collision_box: melee_types::mp::ItEcb,
    pub collision_damage_multiplier: f32,
}
impl ItemAssets {
    /// ftData.x48_items -> Article, loaded once before any item exists.
    pub fn from_fighter(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        states: usize,
    ) -> Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        let common = r.u32(article)?;
        let special = r.u32(article + 4)?;
        let state_array = r.u32(article + 12)?;
        let model_desc = r.u32(article + 16)?;
        let mut scripts = Vec::with_capacity(states);
        let mut hit_flags = Vec::with_capacity(states);
        for state in 0..states {
            let script = r.u32(state_array + state as u32 * 16 + 12)?;
            let (commands, flags) = read_script(archive, script)?;
            scripts.push(commands);
            hit_flags.push(flags);
        }
        let special_attributes = (0..10)
            .map(|i| r.f32(special + i * 4))
            .collect::<Result<_>>()?;
        Ok(Self {
            scripts,
            hit_flags,
            special_attributes,
            scale: r.f32(common + 0x60)?,
            model: r.u32(model_desc)?,
            rotate_to_facing: r.u8(common + 1)? & 0x20 != 0,
            collision_box: melee_types::mp::ItEcb {
                top: r.f32(common + 0x40)?,
                bottom: r.f32(common + 0x44)?,
                right: r.f32(common + 0x48)?,
                left: r.f32(common + 0x4C)?,
            },
            collision_damage_multiplier: r.f32(common + 0x1C)?,
        })
    }
}

/// itanimlist.c uses the shared ten control commands and its own payload table.
type DecodedScript = (Vec<Command>, Vec<Option<ItemHitFlags>>);
fn read_script(archive: &Archive, mut offset: u32) -> Result<DecodedScript> {
    if offset == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let r = archive.reader();
    let mut commands = Vec::new();
    let mut flags = Vec::new();
    loop {
        let word = r.u32(offset)?;
        let op = word >> 26;
        let mut hit_flags = None;
        let command = match op {
            0..=4 | 6 | 8 => {
                melee_cmd::decode::decode(&[word], None, 0).expect("shared item command")
            }
            11 => {
                let w = [
                    word,
                    r.u32(offset + 4)?,
                    r.u32(offset + 8)?,
                    r.u32(offset + 12)?,
                    r.u32(offset + 16)?,
                    r.u32(offset + 20)?,
                ];
                hit_flags = Some(ItemHitFlags::decode(w[4], w[5]));
                offset += 20;
                Command::SpawnHitbox {
                    id: ((word >> 23) & 7) as usize,
                    descriptor: decode_hitbox(w),
                }
            }
            12 => Command::SetHitboxDamage {
                id: ((word >> 23) & 7) as usize,
                damage: (word & 8191) as f32,
            },
            14 => Command::ClearHitbox((word & 0x03ff_ffff) as usize),
            15 => Command::ClearHitboxes,
            17..=19 => Command::SetVariable {
                index: (op - 17) as usize,
                value: word & 0x03ff_ffff,
            },
            _ => unimplemented!("itanimlist.c item script opcode {op}"),
        };
        commands.push(command);
        flags.push(hit_flags);
        offset += 4;
        if op == 0 {
            return Ok((commands, flags));
        }
    }
}

/// it_802790C0: audited --fused has no fused instructions. Item hitbox
/// opcode 11 is SIX words; its bone/damage/flags differ from fighter opcode 11.
fn decode_hitbox(w: [u32; 6]) -> HitboxDescriptor {
    const SCALE: f32 = 0.003906;
    HitboxDescriptor {
        group: ((w[0] >> 20) & 7) as u8,
        bone: ((w[0] >> 13) & 127) as usize,
        common_bone: false,
        requires_throw_owner: false,
        damage: (w[0] & 8191) as f32,
        shield_damage: (w[4] >> 9) as u8 as i8,
        sound_severity: ((w[4] >> 6) & 7) as u8,
        radius: SCALE * (w[1] >> 16) as f32,
        offset: [
            SCALE * (w[1] as i16) as f32,
            SCALE * ((w[2] >> 16) as i16) as f32,
            SCALE * (w[2] as i16) as f32,
        ]
        .into(),
        angle: (w[3] >> 23) as u16,
        growth: ((w[3] >> 14) & 511) as u16,
        weight_knockback: ((w[3] >> 5) & 511) as u16,
        base_knockback: (w[4] >> 23) as u16,
        element: melee_types::HitElement::try_from(((w[4] >> 18) & 31) as i32)
            .expect("item element"),
        hit_ground: true,
        hit_air: true,
        ignore_scale: false,
        clank: w[4] & (1 << 17) != 0,
        rebound: false,
    }
}

/// Sixth item hitbox command word, it_802790C0: masks become named meanings
/// where ftcoll.c identifies them; remaining unknown flags stay explicit.
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemHitFlags {
    pub rehit_rate: u8,
    pub reflectable: bool,
    pub absorbable: bool,
    pub shieldable: bool,
    pub hits_hurtboxes: bool,
    pub grabbable_hurtboxes_only: bool,
    pub sound_kind: u8,
    pub auxiliary: u16,
}
impl ItemHitFlags {
    fn decode(last: u32, extra: u32) -> Self {
        Self {
            rehit_rate: (extra >> 24) as u8,
            reflectable: extra & (1 << 20) != 0,
            absorbable: extra & (1 << 17) != 0,
            shieldable: extra & (1 << 18) != 0,
            hits_hurtboxes: extra & (1 << 14) != 0,
            grabbable_hurtboxes_only: extra & (1 << 13) != 0,
            sound_kind: ((last >> 2) & 15) as u8,
            auxiliary: (extra >> 8) as u16,
        }
    }
}
