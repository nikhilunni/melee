//! Bone references in `ftData` and the separate PlCo part mapping.
//! Indices in descriptors are skeleton indices even where the C type says
//! `Fighter_Part`: ft_081B.c:53-57 and ftcoll.c:3230-3238 index `fp->parts` directly.

use hsd_archive::reader::add_offset;
use hsd_archive::Archive;
use melee_types::{FighterKind, FtPart};

use super::read::{block, invalid, pointer, public, required, Result};

/// `FTPART_INVALID`, ft/types.h:43.
const INVALID_PART: u8 = 0xFF;
/// Runtime allocation bound, ft/ftparts.h:50-51.
pub const MAX_JOINTS: u32 = 140;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartTable {
    /// One entry per skeleton joint; unnamed semantic IDs remain numeric.
    pub joint_to_part: Vec<Option<u8>>,
    /// Missing entries remain None. This table need not be bijective.
    pub part_to_joint: Vec<Option<u8>>,
}

impl PartTable {
    pub fn joint_count(&self) -> usize {
        self.joint_to_part.len()
    }

    /// A named part may be absent from this character's mapping altogether.
    pub fn joint(&self, part: FtPart) -> Option<u8> {
        self.part_to_joint
            .get(i32::from(part) as usize)
            .copied()
            .flatten()
    }

    /// `FighterPartsTable`, ft/types.h:46-50: pointers +0/+4, u32 joint
    /// count +8; sizeof 12 on Gekko. `part_count` is external metadata, not
    /// the joint count. Never infer it from adjacent bytes or the largest enum.
    pub fn read(archive: &Archive, offset: u32, part_count: u32) -> Result<Self> {
        let r = block(archive, offset, 12)?;
        let joint_count = r.u32(8)?;
        if joint_count > MAX_JOINTS || part_count > u32::from(INVALID_PART) {
            return Err(invalid(
                "part table",
                "count exceeds the runtime index domain",
            ));
        }
        let reverse = byte_array(archive, offset, 0, joint_count, "joint_to_part")?;
        let forward = byte_array(archive, offset, 4, part_count, "part_to_joint")?;
        if reverse
            .iter()
            .any(|&v| v != INVALID_PART && u32::from(v) >= part_count)
            || forward
                .iter()
                .any(|&v| v != INVALID_PART && u32::from(v) >= joint_count)
        {
            return Err(invalid(
                "part table",
                "mapping index exceeds the corresponding table",
            ));
        }
        let decode = |v| if v == INVALID_PART { None } else { Some(v) };
        Ok(Self {
            joint_to_part: reverse.into_iter().map(decode).collect(),
            part_to_joint: forward.into_iter().map(decode).collect(),
        })
    }
}

/// `Fighter_LoadCommonData`, fighter.c:182-192: public root slot 4 is
/// `FighterPartsTable**`, indexed by FighterKind (ft/forward.h).
pub fn read_part_table(archive: &Archive, kind: FighterKind, part_count: u32) -> Result<PartTable> {
    if i32::from(kind) >= FighterKind::MAX {
        return Err(invalid("fighter kind", "not a playable data-table index"));
    }
    let root = public(archive, "ftLoadCommonData")?;
    let table = required(archive, root, 4 * 4, "ftPartsTable")?;
    let entry = required(
        archive,
        table,
        i32::from(kind) as u32 * 4,
        "ftPartsTable[kind]",
    )?;
    PartTable::read(archive, entry, part_count)
}

/// `ftAnim_8006FCE4`: an animation names its skeleton in x597_bits. The
/// final table is the shared animation skeleton (FTKIND_NONE), not a fighter.
pub struct AnimationSource {
    pub parts: PartTable,
    pub masks: Vec<u32>,
}

impl AnimationSource {
    pub fn read(archive: &Archive, source: u8, part_count: u32) -> Result<Self> {
        if i32::from(source) > FighterKind::MAX {
            return Err(invalid("animation source", "outside part tables"));
        }
        let root = public(archive, "ftLoadCommonData")?;
        let tables = required(archive, root, 4 * 4, "ftPartsTable")?;
        let entry = required(archive, tables, u32::from(source) * 4, "animation parts")?;
        let parts = PartTable::read(archive, entry, part_count)?;
        let mut masks = vec![0; parts.joint_count()];
        // ftParts_8007506C: each four-byte entry names one conditional joint.
        let tables = required(archive, root, 5 * 4, "Fighter_804D6540")?;
        if let Some(table) = archive.link(tables + u32::from(source) * 4)? {
            let count = archive.reader().u32(table + 4)?;
            if count > 32 {
                return Err(invalid("animation masks", "more than 32 mask bits"));
            }
            if count != 0 {
                let entries = required(archive, table, 0, "conditional joints")?;
                for i in 0..count {
                    let joint = usize::from(archive.reader().u8(entries + i * 4)?);
                    let mask = masks.get_mut(joint).ok_or_else(|| {
                        invalid("animation masks", "joint outside source skeleton")
                    })?;
                    if *mask == 0 {
                        *mask = 1 << i;
                    }
                }
            }
        }
        Ok(Self { parts, masks })
    }
}

/// `ftData_x44_t`, ft/types.h:584-595, sizeof 0x1C. Six signed bone
/// indices at +0,+2,+4,+6,+8,+A; center and ledge dimensions at +C..+18.
/// These are unscaled data. ft_081B.c:53-61 applies *player* scale later.
#[derive(Debug, Clone, PartialEq)]
pub struct EcbBones {
    pub joints: [i16; 6],
    pub center_y: f32,
    pub ledge_snap_x: f32,
    pub ledge_snap_y: f32,
    pub ledge_snap_height: f32,
}

impl EcbBones {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = block(archive, offset, 0x1C)?;
        Ok(Self {
            joints: [
                r.s16(0)?,
                r.s16(2)?,
                r.s16(4)?,
                r.s16(6)?,
                r.s16(8)?,
                r.s16(10)?,
            ],
            center_y: r.f32(0xC)?,
            ledge_snap_x: r.f32(0x10)?,
            ledge_snap_y: r.f32(0x14)?,
            ledge_snap_height: r.f32(0x18)?,
        })
    }
}

/// Five bytes at ftData.x8+0x10..0x14 (ft/types.h:618-622).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBones {
    /// x10; ftanim.c:138 uses this joint for animation translation.
    pub animation_translation: u8,
    /// x11; ftCo_Guard.c:197 and ftCo_CatchWait.c:25.
    pub shield: u8,
    /// x12; ftcommon.c:1541 attaches a held item here.
    pub held_item: u8,
    /// x13/x14; ftaction.c:1172-1174 selects these foot-effect joints.
    pub left_foot: u8,
    pub right_foot: u8,
}

/// ftData.x1C entries, ft/types.h:628-633. Five slots are fixed by the
/// runtime `Fighter.x8B0[5]` at types.h:1301-1308. The archive only stores
/// the character's used prefix (ftAnim_800707B0 accesses active slots only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationBoneSet {
    pub root_joint: u16,
    pub joints: Vec<u8>,
}

/// Grounded leg IK, ftData.x58 (ft/types.h:659,1910-1923), used by
/// ft_0899.c:95-105,136-140. Leg entries are upper/lower/foot joint indices.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundPoseBones {
    /// +0/+8/+10, ft/types.h:1911,1915,1919.
    pub right_leg: [u8; 3],
    /// +1/+9/+11, ft/types.h:1912,1916,1920.
    pub left_leg: [u8; 3],
    /// +4, ft/types.h:1914.
    pub upper_length: f32,
    /// +C, ft/types.h:1918.
    pub lower_length: f32,
    /// +18, ft/types.h:1922; added to lower_length by ft_0899.c:96-97.
    pub foot_extension: f32,
}

/// Bone lists needed to attach fighter systems. Animation resources,
/// dynamics parameters and hurtbox shapes remain the responsibility of
/// their respective readers; their bone references are retained here.
#[derive(Debug, Clone, PartialEq)]
pub struct FighterBones {
    pub model: ModelBones,
    pub ecb: EcbBones,
    pub animation_sets: [Option<AnimationBoneSet>; 5],
    pub dynamics_roots: Vec<i32>,
    pub dynamics_collision: Vec<i32>,
    pub hurtboxes: Vec<i32>,
    pub scaled_joint: Option<i32>,
    pub attachment_joint: Option<i32>,
    pub ground_pose: Option<GroundPoseBones>,
}

/// ftData pointer slots: x8 +8, x1C +1C, dynamics +2C, hurtboxes +30,
/// scaled joint +34, attachment +38, ECB +44 (`ft/types.h:615-653`).
pub fn read_fighter_bones(
    archive: &Archive,
    ft_data: u32,
    part_animation_count: usize,
) -> Result<FighterBones> {
    let model_offset = required(archive, ft_data, 8, "ftData.x8")?;
    let r = block(archive, model_offset, 0x15)?;
    let model = ModelBones {
        animation_translation: r.u8(0x10)?,
        shield: r.u8(0x11)?,
        held_item: r.u8(0x12)?,
        left_foot: r.u8(0x13)?,
        right_foot: r.u8(0x14)?,
    };
    let ecb = EcbBones::read(archive, required(archive, ft_data, 0x44, "ftData.x44")?)?;
    let animation_sets = read_animation_sets(archive, ft_data, part_animation_count)?;
    let (dynamics_roots, dynamics_collision) = read_dynamics_bones(archive, ft_data)?;
    let hurtboxes = if let Some(offset) = pointer(archive, ft_data, 0x30, "ftData.x30")? {
        // ftHurtboxInit: ft/kinds/ftCommon/types.h:23-30; bone +0,
        // two 32-bit flags/enums, two Vec3s, float scale => stride 0x28.
        read_bone_array(archive, offset, 0, 0x28, 15, "hurtboxes")?
    } else {
        Vec::new()
    };
    Ok(FighterBones {
        model,
        ecb,
        animation_sets,
        dynamics_roots,
        dynamics_collision,
        hurtboxes,
        ground_pose: read_ground_pose(archive, ft_data)?,
        scaled_joint: optional_bone(archive, ft_data, 0x34)?,
        attachment_joint: optional_bone(archive, ft_data, 0x38)?,
    })
}

fn read_animation_sets(
    archive: &Archive,
    ft_data: u32,
    count: usize,
) -> Result<[Option<AnimationBoneSet>; 5]> {
    let mut sets = std::array::from_fn(|_| None);
    if let Some(table) = pointer(archive, ft_data, 0x1C, "ftData.x1C")? {
        if count > sets.len() {
            return Err(invalid(
                "part animations",
                "count exceeds five runtime slots",
            ));
        }
        block(archive, table, count as u32 * 4)?;
        for (i, set) in sets.iter_mut().take(count).enumerate() {
            if let Some(offset) = pointer(archive, table, i as u32 * 4, "animation bone set")? {
                let r = block(archive, offset, 12)?;
                *set = Some(AnimationBoneSet {
                    root_joint: r.u16(0)?,
                    joints: byte_array(
                        archive,
                        offset,
                        4,
                        u32::from(r.u16(2)?),
                        "animation joints",
                    )?,
                });
            }
        }
    }
    Ok(sets)
}

fn read_dynamics_bones(archive: &Archive, ft_data: u32) -> Result<(Vec<i32>, Vec<i32>)> {
    let Some(offset) = pointer(archive, ft_data, 0x2C, "ftData.x2C")? else {
        return Ok((Vec::new(), Vec::new()));
    };
    // ftDynamics count/pointer pairs +0/+4 and +8/+C, ft/types.h:1842-1850.
    // BoneDynamicsDesc: lb/types.h:490-499 => bone +0, stride 0x18.
    // ftData_x38: ft/types.h:646-650 => bone +0, stride 0x14.
    // Runtime limits: ftdynamics.c:89 (<10), ftcoll.c:3241 (<=11).
    let roots = read_bone_array(archive, offset, 0, 0x18, 9, "dynamics roots")?;
    let collision = read_bone_array(archive, offset, 8, 0x14, 11, "dynamics collision")?;
    Ok((roots, collision))
}

fn optional_bone(archive: &Archive, root: u32, field: u32) -> Result<Option<i32>> {
    pointer(archive, root, field, "bone attachment")?
        .map(|offset| Ok(archive.reader().s32(offset)?))
        .transpose()
}

fn byte_array(
    archive: &Archive,
    base: u32,
    field: u32,
    count: u32,
    name: &'static str,
) -> Result<Vec<u8>> {
    let offset = pointer(archive, base, field, name)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let offset = offset.ok_or_else(|| invalid(name, "nonempty array has a null pointer"))?;
    Ok(archive.reader().slice(offset, count)?.to_vec())
}

fn read_bone_array(
    archive: &Archive,
    base: u32,
    pair: u32,
    stride: u32,
    max: i32,
    name: &'static str,
) -> Result<Vec<i32>> {
    let r = block(archive, add_offset(base, pair)?, 8)?;
    let count = r.s32(0)?;
    if count < 0 || count > max {
        return Err(invalid(name, "count exceeds runtime capacity"));
    }
    let target = pointer(archive, base, pair + 4, name)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let target = target.ok_or_else(|| invalid(name, "nonempty array has a null pointer"))?;
    let r = block(archive, target, count as u32 * stride)?;
    (0..count as u32).map(|i| Ok(r.s32(i * stride)?)).collect()
}

fn read_ground_pose(archive: &Archive, ft_data: u32) -> Result<Option<GroundPoseBones>> {
    let Some(offset) = pointer(archive, ft_data, 0x58, "ftData.x58")? else {
        return Ok(None);
    };
    let r = block(archive, offset, 0x1C)?;
    Ok(Some(GroundPoseBones {
        right_leg: [r.u8(0)?, r.u8(8)?, r.u8(0x10)?],
        left_leg: [r.u8(1)?, r.u8(9)?, r.u8(0x11)?],
        upper_length: r.f32(4)?,
        lower_length: r.f32(0xC)?,
        foot_extension: r.f32(0x18)?,
    }))
}
