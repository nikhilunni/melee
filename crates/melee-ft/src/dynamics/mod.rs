//! Fighter dynamics: ftCo_8009CF84 (0x8009CF84), ftCo_8009CB40
//! (0x8009CB40), ftCo_8009DD94 (0x8009DD94), ftdynamics.c.
use crate::anim::attach::AnimationPart;
use hsd_anim::{jobj::JObjTree, quat::Quaternion};
use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_lb::dynamics::{DynamicBoneSet, SpringParameters};

pub struct DynamicSetDescriptor {
    pub root: usize,
    pub multipliers: Vec3,
    pub springs: Vec<SpringParameters>,
}

/// ftData +2C: count/pointer followed by collider count/pointer. The spring
/// array has stride 0x3C; the runtime linked nodes have a different layout.
pub fn read_sets(
    archive: &Archive,
    ft_data: u32,
) -> Result<Vec<DynamicSetDescriptor>, Box<dyn std::error::Error>> {
    let Some(dynamics) = archive.link(ft_data + 0x2C)? else {
        return Ok(Vec::new());
    };
    let r = archive.reader();
    let count = r.u32(dynamics)?;
    if count >= 10 {
        return Err("fighter dynamics count exceeds capacity".into());
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    let table = archive
        .link(dynamics + 4)?
        .ok_or("missing dynamics set array")?;
    let vec = |p| -> Result<Vec3, hsd_archive::Error> {
        Ok(Vec3::new(r.f32(p)?, r.f32(p + 4)?, r.f32(p + 8)?))
    };
    let mut sets = Vec::new();
    for i in 0..count {
        let set = table + i * 0x18;
        let size = r.u32(set + 8)?;
        if size == 0 || size > 140 {
            return Err("invalid dynamic chain length".into());
        }
        let data = archive.link(set + 4)?.ok_or("missing spring parameters")?;
        let mut springs = Vec::new();
        for j in 0..size {
            let p = data + j * 0x3C;
            springs.push(SpringParameters {
                stiffness: r.f32(p)?,
                convergence: r.f32(p + 4)?,
                natural_rotation: Quaternion::new(
                    r.f32(p + 8)?,
                    r.f32(p + 12)?,
                    r.f32(p + 16)?,
                    r.f32(p + 20)?,
                ),
                max_deviation: r.f32(p + 0x18)?,
                rotation_max: vec(p + 0x1C)?,
                rotation_min: vec(p + 0x28)?,
                damping: r.f32(p + 0x34)?,
                max_step: r.f32(p + 0x38)?,
            });
        }
        sets.push(DynamicSetDescriptor {
            root: r.u32(set)? as usize,
            multipliers: vec(set + 12)?,
            springs,
        });
    }
    Ok(sets)
}

/// ftCo_8009CB40: reset position/velocity when animation relinquishes a
/// joint, restore rest translation/scale, and change the part animation lock.
pub fn select(
    set: &mut DynamicBoneSet,
    tree: &mut JObjTree,
    parts: &mut [AnimationPart],
    enabled: bool,
    first: usize,
) {
    for (index, bone) in set.bones.iter_mut().enumerate() {
        let locked = if index < first { !enabled } else { enabled };
        let part = parts.iter_mut().find(|p| p.joint == bone.joint).unwrap();
        if locked && part.flags.0 & crate::anim::attach::PartFlags::LOCKED == 0 {
            let matrix = tree.get(bone.joint).mtx;
            bone.position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
            bone.angular_velocity = 0.0;
            tree.set_translate(bone.joint, &bone.rest_translate);
            tree.set_scale(bone.joint, &bone.rest_scale);
        }
        if locked {
            part.flags.0 |= crate::anim::attach::PartFlags::LOCKED;
            tree.clear_flags(bone.joint, 0x20000);
        } else {
            part.flags.0 &= !crate::anim::attach::PartFlags::LOCKED;
        }
    }
}

/// How a motion hands its dynamic bones to the solver: the animation flag
/// bits `x594_b3`/`x594_b4` and, for the table form, its row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotionDynamics {
    /// Per set, the first solver-driven bone; 0x100 disables solving.
    pub starts: Vec<u32>,
    /// `x594_b4` with a non-null row in the ftData +2C table (x10).
    pub table_row: bool,
}

/// ftCo_8009E7B4 (8009E7B4), ftdynamics.c:639-697. The apparent
/// FigaTree pointers in x10 are actually integer chain-start indices.
/// Motion blend metadata byte 1 selects the row; 0x100 disables solving.
pub fn read_motion_starts(
    archive: &Archive,
    root: u32,
    motion_count: u32,
) -> Result<std::collections::BTreeMap<i32, MotionDynamics>, Box<dyn std::error::Error>> {
    let mut result = std::collections::BTreeMap::new();
    let Some(dynamics) = archive.link(root + 0x2C)? else {
        return Ok(result);
    };
    let count = archive.reader().u32(dynamics)?;
    if count == 0 {
        return Ok(result);
    }
    let table = archive.link(dynamics + 0x10)?;
    let motions = archive.link(root + 0xC)?.ok_or("missing motion table")?;
    let blends = archive.link(root + 0x10)?.ok_or("missing blend metadata")?;
    for motion in 0..motion_count {
        let flags = archive.reader().u32(motions + motion * 0x18 + 0x10)?;
        let mut table_row = false;
        let starts = if flags & 0x0800_0000 != 0 {
            let slot = u32::from(archive.reader().u8(blends + motion * 2 + 1)?);
            let row = table
                .map(|table| archive.link(table + slot * 4))
                .transpose()?
                .flatten();
            if let Some(row) = row {
                table_row = true;
                (0..count)
                    .map(|i| archive.reader().u32(row + i * 4))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                vec![0x100; count as usize]
            }
        } else {
            vec![if flags & 0x1000_0000 != 0 { 0x100 } else { 0 }; count as usize]
        };
        result.insert(motion as i32, MotionDynamics { starts, table_row });
    }
    Ok(result)
}
