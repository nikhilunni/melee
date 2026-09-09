//! Owned resources needed by a Wait fighter; Melee archive layout stays here.
use super::commands::Command;
use crate::{
    anim::{Motion, WaitEntry},
    desc::{
        common::{read_common_data, CommonFighterData},
        playback::{read_playback_motion, read_wait_table},
        read_fighter_attributes, read_fighter_bones, read_fox_animations, read_part_table,
        FighterAttributes, FighterBones, PartTable,
    },
    input::InputCommonData,
};
use hsd_anim::{aobj::AObjDesc, fobj::FObjDesc, jobj::AnimJoint};
use hsd_archive::{desc, Archive};
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct PartResource {
    pub root: usize,
    pub nodes: Vec<AnimJoint>,
    pub duration: f32,
}
pub struct FighterAssets {
    pub attributes: FighterAttributes,
    pub bones: FighterBones,
    pub parts: PartTable,
    pub common: CommonFighterData,
    pub input: InputCommonData,
    pub shield_health: f32,
    pub entry: super::entry::EntryParameters,
    pub soft_landing_speed: f32,
    pub name_tag_duration: u16,
    pub thrown_hitbox: super::caches::ThrownHitbox,
    pub hurtboxes: Vec<super::caches::Hurtbox>,
    pub dynamic_colliders: Vec<super::caches::DynamicCollider>,
    pub motions: BTreeMap<i32, Motion>,
    pub wait_choices: Vec<WaitEntry>,
    pub commands: Vec<Command>,
    /// Archive-relative source locations for savestate import/diagnostics only.
    pub instruction_offsets: Vec<u32>,
    pub motion_table_offset: u32,
    pub camera_extents: [hsd_types::Vec3; 2],
    pub command_entries: BTreeMap<i32, usize>,
    pub part_animations: BTreeMap<(usize, usize), PartResource>,
}
impl FighterAssets {
    /// Fighter_LoadCommonData (0x80067ABC) / ftData_80085CD8:
    /// Fox's Wait, Fall, Landing and EntryStart resources, caller-owned archives and AJ bytes.
    pub fn fox(fox: &Archive, common: &Archive, aj: &[u8]) -> Result<Self> {
        let root = fox.public("ftDataFox").ok_or("missing ftDataFox")?;
        let table = read_fox_animations(fox)?;
        let common_root = common
            .public("ftLoadCommonData")
            .ok_or("missing PlCo root")?;
        let common_data = common.link(common_root)?.ok_or("missing common data")?;
        let motion_table = table.table_offset.ok_or("missing motion table")?;
        let mut entries = BTreeMap::new();
        let mut words = BTreeMap::new();
        for id in [2, 3, 20, 35, 238] {
            let entry = fox
                .link(motion_table + id * 0x18 + 0xC)?
                .ok_or("missing Wait script")?;
            entries.insert(id as i32, entry);
            read_script(fox, entry, &mut words)?;
        }
        let indices: BTreeMap<_, _> = words
            .keys()
            .copied()
            .enumerate()
            .map(|(i, offset)| (offset, i))
            .collect();
        let commands = words
            .values()
            .map(|command| match command {
                Command::Call {
                    target,
                    continuation,
                } => Command::Call {
                    target: indices[&(*target as u32)],
                    continuation: indices[&(*continuation as u32)],
                },
                other => other.clone(),
            })
            .collect::<Vec<_>>();
        let groups = commands
            .iter()
            .filter_map(|c| {
                if let Command::Part { group, variant, .. } = c {
                    Some((*group, *variant))
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        let part_table = fox.link(root + 0x1C)?.ok_or("missing part animations")?;
        let mut part_animations = BTreeMap::new();
        for (group, variant) in groups {
            let set = fox
                .link(part_table + group as u32 * 4)?
                .ok_or("missing part set")?;
            let animations = fox.link(set + 8)?.ok_or("missing part variants")?;
            let offset = fox
                .link(animations + variant as u32 * 4)?
                .ok_or("missing part variant")?;
            let source = desc::AnimJoint::read(fox, offset)?;
            let mut nodes = Vec::new();
            flatten_part(&source, &mut nodes)?;
            let duration = nodes
                .iter()
                .filter_map(|n| n.aobjdesc.as_ref().map(|a| a.end_frame))
                .fold(0.0, f32::max);
            part_animations.insert(
                (group, variant),
                PartResource {
                    root: usize::from(fox.reader().u16(set)?),
                    nodes,
                    duration,
                },
            );
        }
        Ok(Self {
            attributes: read_fighter_attributes(fox, root)?,
            bones: read_fighter_bones(fox, root)?,
            parts: read_part_table(common, melee_types::FighterKind::Fox, 54)?,
            common: read_common_data(common)?,
            input: InputCommonData::read(common)?,
            entry: super::entry::EntryParameters {
                grow_frames: common.reader().s32(common_data + 0x6BC)?,
                shrink_frames: common.reader().s32(common_data + 0x6C0)?,
                initial_scale_y: common.reader().f32(common_data + 0x6C4)?,
            },
            soft_landing_speed: -common.reader().f32(common_data + 0x310)?,
            shield_health: common.reader().f32(common_data + 0x260)?,
            name_tag_duration: common.reader().u32(common_data + 0x5F0)? as u16,
            thrown_hitbox: {
                let p = fox.link(root + 0x34)?.ok_or("missing thrown hitbox")?;
                super::caches::ThrownHitbox {
                    bone: fox.reader().u32(p)? as usize,
                    radius: fox.reader().f32(p + 4)?,
                    ..super::caches::ThrownHitbox::default()
                }
            },
            hurtboxes: read_hurtboxes(fox, root)?,
            dynamic_colliders: read_dynamic_colliders(fox, root)?,
            motions: [2, 3, 20, 35, 238]
                .into_iter()
                .map(|id| Ok((id as i32, read_playback_motion(fox, root, &table, aj, id)?)))
                .collect::<Result<_>>()?,
            wait_choices: read_wait_table(fox, root)?,
            commands,
            instruction_offsets: words.keys().copied().collect(),
            motion_table_offset: motion_table,
            camera_extents: {
                let p = fox.link(root + 0x3C)?.ok_or("missing camera extents")?;
                [read_vec(fox, p)?, read_vec(fox, p + 12)?]
            },
            command_entries: entries
                .into_iter()
                .map(|(id, offset)| (id, indices[&offset]))
                .collect(),
            part_animations,
        })
    }
}

/// lbcommand.c:13-98; ftAction_800727C8/800726F4/80072C6C.
fn read_script(
    archive: &Archive,
    mut offset: u32,
    commands: &mut BTreeMap<u32, Command>,
) -> Result<()> {
    while !commands.contains_key(&offset) {
        let word = archive.reader().u32(offset)?;
        let opcode = word >> 26;
        let command = match opcode {
            0 => Command::End,
            1 => Command::Wait((word & 0x03ff_ffff) as f32),
            2 => Command::AtFrame((word & 0x03ff_ffff) as f32),
            5 => Command::Call {
                target: archive.link(offset + 4)?.ok_or("null command call")? as usize,
                continuation: (offset + 8) as usize,
            },
            6 => Command::Return,
            41 => Command::Part {
                group: ((word >> 19) & 127) as usize,
                variant: ((word >> 12) & 127) as usize,
                blend: (word & 4095) as f32,
            },
            55 => Command::LandingEffect((word & 0xFFFF) as u16),
            52 => Command::GroundPose((word & 7) as u8),
            40 => {
                let mut indices = vec![((word >> 18) & 127) as usize];
                if word & (1 << 25) != 0 {
                    indices.push(((word >> 11) & 127) as usize);
                }
                Command::Texture {
                    indices,
                    frame: (word & 2047) as f32,
                }
            }
            _ => {
                return Err(format!(
                    "unsupported fighter opcode {opcode} at {offset:#x} (ftaction.c:1344)"
                )
                .into())
            }
        };
        commands.insert(offset, command.clone());
        match command {
            Command::End | Command::Return => break,
            Command::Call {
                target,
                continuation,
            } => {
                read_script(archive, target as u32, commands)?;
                offset = continuation as u32;
            }
            Command::LandingEffect(_) => offset += 12,
            _ => offset += 4,
        }
    }
    Ok(())
}
fn flatten_part(source: &desc::AnimJoint, nodes: &mut Vec<AnimJoint>) -> Result<()> {
    if source.robj_anim.is_some() {
        return Err("part RObj animation not supported".into());
    }
    let aobjdesc = source.aobjdesc.as_ref().map(|a| {
        assert_eq!(a.obj_id, 0, "part object reference");
        AObjDesc {
            flags: a.flags,
            end_frame: a.end_frame,
            obj_id: 0,
            fobjdesc: a
                .tracks()
                .map(|t| FObjDesc {
                    length: t.length,
                    startframe: t.startframe,
                    obj_type: t.type_,
                    frac_value: t.frac_value,
                    frac_slope: t.frac_slope,
                    ad: t.ad.clone(),
                })
                .collect(),
        }
    });
    nodes.push(AnimJoint {
        aobjdesc,
        flags: source.flags,
        children: Vec::new(),
    });
    if let Some(child) = &source.child {
        flatten_part(child, nodes)?;
    }
    if let Some(next) = &source.next {
        flatten_part(next, nodes)?;
    }
    Ok(())
}

fn read_vec(archive: &Archive, offset: u32) -> Result<hsd_types::Vec3> {
    let r = archive.reader();
    Ok(hsd_types::Vec3::new(
        r.f32(offset)?,
        r.f32(offset + 4)?,
        r.f32(offset + 8)?,
    ))
}

fn read_hurtboxes(a: &Archive, root: u32) -> Result<Vec<super::caches::Hurtbox>> {
    let table = a.link(root + 0x30)?.ok_or("missing hurtboxes")?;
    let count = a.reader().u32(table)?;
    let base = a.link(table + 4)?.ok_or("missing hurtbox records")?;
    assert!(count <= 15);
    (0..count)
        .map(|i| {
            let p = base + i * 0x28;
            Ok(super::caches::Hurtbox {
                bone: a.reader().u32(p)? as usize,
                offsets: [read_vec(a, p + 12)?, read_vec(a, p + 24)?],
                radius: a.reader().f32(p + 36)?,
                positions: [hsd_types::Vec3::ZERO; 2],
                cached: false,
            })
        })
        .collect()
}
fn read_dynamic_colliders(a: &Archive, root: u32) -> Result<Vec<super::caches::DynamicCollider>> {
    let table = a.link(root + 0x2C)?.ok_or("missing dynamics")?;
    let count = a.reader().u32(table + 8)?;
    let base = a.link(table + 12)?.ok_or("missing dynamic colliders")?;
    assert!(count <= 11);
    (0..count)
        .map(|i| {
            let p = base + i * 0x14;
            Ok(super::caches::DynamicCollider {
                bone: a.reader().u32(p)? as usize,
                offset: read_vec(a, p + 4)?,
                radius: a.reader().f32(p + 16)?,
                position: hsd_types::Vec3::ZERO,
            })
        })
        .collect()
}
