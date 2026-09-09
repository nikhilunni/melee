//! Owned resources needed by a Wait fighter; Melee archive layout stays here.
use super::commands::Command;
use crate::{
    anim::{Motion, WaitEntry},
    desc::{
        common::{read_common_data, CommonFighterData},
        playback::{read_playback_motion, read_wait_table},
        read_fighter_animations, read_fighter_attributes, read_fighter_bones, read_part_table,
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
/// Character metadata from ftdata.c, ftparts.c's PlCo tables and costume strings.
/// Character crates own these descriptors; shared loaders never select by kind.
#[derive(Clone, Copy, Debug)]
pub struct CharacterDescriptor {
    pub kind: melee_types::FighterKind,
    pub data_file: &'static str,
    pub data_symbol: &'static str,
    pub animation_file: &'static str,
    pub animation_count: u32,
    pub part_count: u32,
    pub part_animation_count: usize,
    pub costumes: &'static [CostumeDescriptor],
}
#[derive(Clone, Copy, Debug)]
pub struct CostumeDescriptor {
    pub file: &'static str,
    pub joint_symbol: &'static str,
}

pub struct FighterAssets {
    pub kind: melee_types::FighterKind,
    pub attributes: FighterAttributes,
    pub bones: FighterBones,
    pub parts: PartTable,
    pub common: CommonFighterData,
    pub input: InputCommonData,
    pub shield_health: f32,
    pub shield: super::shield::ShieldParameters,
    pub guard_pose: Vec<hsd_anim::jobj::JObj>,
    pub entry: super::entry::EntryParameters,
    pub soft_landing_speed: f32,
    pub name_tag_duration: u16,
    pub thrown_hitbox: super::caches::ThrownHitbox,
    pub damage: super::damage::DamageParameters,
    pub overlap: super::overlap::OverlapParameters,
    pub hurtboxes: Vec<super::caches::Hurtbox>,
    pub first_stale_penalty: f32,
    pub grab_friction_multiplier: f32,
    pub dynamics: Vec<crate::dynamics::DynamicSetDescriptor>,
    pub dynamics_motion_starts: BTreeMap<i32, Vec<u32>>,
    pub dynamic_colliders: Vec<super::caches::DynamicCollider>,
    pub motions: BTreeMap<i32, Motion>,
    pub rotating_effect_bones: [usize; 5],
    pub jumping: super::jump::JumpParameters,
    pub falling: super::fall::FallParameters,
    pub air_dodge: super::air_dodge::AirDodgeParameters,
    pub ledge: super::ledge::LedgeParameters,
    pub running: super::dash::RunningParameters,
    pub movement: crate::desc::common::MovementParameters,
    pub squat_choices: Option<Vec<WaitEntry>>,
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
    /// Shared Wait, Fall, Landing and EntryStart resources, caller-owned archives and AJ bytes.
    pub fn load(
        descriptor: &CharacterDescriptor,
        data: &Archive,
        common: &Archive,
        aj: &[u8],
    ) -> Result<Self> {
        let root = data
            .public(descriptor.data_symbol)
            .ok_or("missing fighter data symbol")?;
        let table = read_fighter_animations(data, root, descriptor.animation_count)?;
        let common_root = common
            .public("ftLoadCommonData")
            .ok_or("missing PlCo root")?;
        let common_data = common.link(common_root)?.ok_or("missing common data")?;
        let motion_table = table.table_offset.ok_or("missing motion table")?;
        // ftwaitanim.c chooses from each character's sentinel-terminated
        // tables. Load every referenced idle/squat motion and its script.
        let wait_choices = read_wait_table(data, root)?;
        let squat_choices = crate::desc::playback::read_squat_table(data, root)?;
        let idle_motions: BTreeSet<_> = wait_choices
            .iter()
            .chain(squat_choices.iter().flatten())
            .filter(|entry| entry.motion >= 0)
            .map(|entry| entry.motion as u32)
            .collect();
        let mut entries = BTreeMap::new();
        let mut words = BTreeMap::new();
        for id in [
            2, 3, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 20, 23, 26, 30, 31, 34, 35, 37, 38, 39, 40,
            41, 42, 43, 17, 19, 36, 44, 11, 216, 217, 220, 224, 225, 226, 227, 228, 238, 46, 58,
            167, 168, 169, 209, 242,
        ]
        .into_iter()
        .chain(idle_motions.iter().copied())
        .collect::<BTreeSet<_>>()
        {
            let entry = data
                .link(motion_table + id * 0x18 + 0xC)?
                .ok_or("missing Wait script")?;
            entries.insert(id as i32, entry);
            read_script(data, entry, &mut words)?;
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
                Command::Goto(target) => Command::Goto(indices[&(*target as u32)]),
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
        let part_table = data.link(root + 0x1C)?.ok_or("missing part animations")?;
        let mut part_animations = BTreeMap::new();
        for (group, variant) in groups {
            let set = data
                .link(part_table + group as u32 * 4)?
                .ok_or("missing part set")?;
            let animations = data.link(set + 8)?.ok_or("missing part variants")?;
            let offset = data
                .link(animations + variant as u32 * 4)?
                .ok_or("missing part variant")?;
            let source = desc::AnimJoint::read(data, offset)?;
            let mut nodes = Vec::new();
            flatten_part(&source, &mut nodes)?;
            let duration = nodes
                .iter()
                .filter_map(|n| n.aobjdesc.as_ref().map(|a| a.end_frame))
                .fold(0.0, f32::max);
            part_animations.insert(
                (group, variant),
                PartResource {
                    root: usize::from(data.reader().u16(set)?),
                    nodes,
                    duration,
                },
            );
        }
        Ok(Self {
            kind: descriptor.kind,
            attributes: read_fighter_attributes(data, root)?,
            bones: read_fighter_bones(data, root, descriptor.part_animation_count)?,
            parts: read_part_table(common, descriptor.kind, descriptor.part_count)?,
            common: read_common_data(common)?,
            input: InputCommonData::read(common)?,
            entry: super::entry::EntryParameters {
                grow_frames: common.reader().s32(common_data + 0x6BC)?,
                shrink_frames: common.reader().s32(common_data + 0x6C0)?,
                initial_scale_y: common.reader().f32(common_data + 0x6C4)?,
            },
            soft_landing_speed: -common.reader().f32(common_data + 0x310)?,
            shield: super::shield::ShieldParameters::read(common, common_data)?,
            guard_pose: read_guard_pose(data, root)?,
            shield_health: common.reader().f32(common_data + 0x260)?,
            name_tag_duration: common.reader().u32(common_data + 0x5F0)? as u16,
            thrown_hitbox: {
                let p = data.link(root + 0x34)?.ok_or("missing thrown hitbox")?;
                super::caches::ThrownHitbox {
                    bone: data.reader().u32(p)? as usize,
                    radius: data.reader().f32(p + 4)?,
                    ..super::caches::ThrownHitbox::default()
                }
            },
            damage: super::damage::DamageParameters::read(common, common_data)?,
            grab_friction_multiplier: common.reader().f32(common_data + 0x64)?,
            // Fighter_LoadCommonData: pData[3] -> Fighter_804D6548 stale weights.
            first_stale_penalty: common.reader().f32(
                common
                    .link(common_root + 12)?
                    .ok_or("missing stale weights")?,
            )?,
            overlap: super::overlap::OverlapParameters {
                center: data
                    .reader()
                    .f32(data.link(root + 0x50)?.ok_or("missing push shape")?)?,
                half_width: data.reader().f32(data.link(root + 0x50)?.unwrap() + 4)?,
                horizontal_step: common.reader().f32(common_data + 0x450)?,
                depth_step: common.reader().f32(common_data + 0x454)?,
                depth_limit: common.reader().f32(common_data + 0x458)?,
            },
            hurtboxes: read_hurtboxes(data, root)?,
            dynamics: crate::dynamics::read_sets(data, root)?,
            dynamics_motion_starts: crate::dynamics::read_motion_starts(
                data,
                root,
                descriptor.animation_count,
            )?,
            dynamic_colliders: read_dynamic_colliders(data, root)?,
            motions: [
                2, 3, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30,
                31, 34, 35, 37, 38, 39, 40, 41, 42, 43, 17, 19, 36, 44, 11, 216, 217, 220, 224,
                225, 226, 227, 228, 238, 46, 58, 167, 168, 169, 209, 242,
            ]
            .into_iter()
            .chain(idle_motions.into_iter().map(|id| id as usize))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|id| Ok((id as i32, read_playback_motion(data, root, &table, aj, id)?)))
            .collect::<Result<_>>()?,
            rotating_effect_bones: {
                let table = data
                    .link(root + 0x54)?
                    .ok_or("missing rotating effect bones")?;
                let mut bones = [0; 5];
                for (i, bone) in bones.iter_mut().enumerate() {
                    *bone = data.reader().u32(table + i as u32 * 4)? as usize;
                }
                bones
            },
            falling: super::fall::FallParameters {
                deadzone: common.reader().f32(common_data + 0x444)?,
                smoothing: common.reader().f32(common_data + 0x448)?,
            },
            air_dodge: super::air_dodge::AirDodgeParameters::read(common, common_data)?,
            ledge: super::ledge::LedgeParameters::read(common, common_data)?,
            jumping: super::jump::JumpParameters {
                backward_threshold: common.reader().f32(common_data + 0x78)?,
                release_threshold: common.reader().f32(common_data + 0x7C)?,
                fast_fall_threshold: common.reader().f32(common_data + 0x88)?,
                fast_fall_window: common.reader().s32(common_data + 0x8C)?,
            },
            running: super::dash::RunningParameters {
                turn_threshold: common.reader().f32(common_data + 0x38)?,
                early_interrupt_frames: common.reader().f32(common_data + 0x44)?,
                early_escape_frames: common.reader().f32(common_data + 0x48)?,
                redash_frames: common.reader().f32(common_data + 0x4c)?,
                interrupt_friction: common.reader().f32(common_data + 0x54)?,
                run_threshold: common.reader().f32(common_data + 0x58)?,
                acceleration_taper: common.reader().f32(common_data + 0x5c)?,
                friction_multiplier: common.reader().f32(common_data + 0x60)?,
                relaxed_jump_threshold: common.reader().f32(common_data + 0x80)?,
                brake_pause_speed: common.reader().f32(common_data + 0x42c)?,
                turn_exit_interrupt_delay: common.reader().f32(common_data + 0x430)?,
            },
            movement: crate::desc::common::MovementParameters {
                middle_threshold: common.reader().f32(common_data + 0x28)?,
                fast_threshold: common.reader().f32(common_data + 0x2C)?,
                acceleration_taper: common.reader().f32(common_data + 0x30)?,
                slippery_animation_multiplier: common.reader().f32(common_data + 0x440)?,
                squat_release_threshold: common.reader().f32(common_data + 0x94)?,
                platform_drop_threshold: common.reader().f32(common_data + 0x464)?,
                platform_drop_window: common.reader().s32(common_data + 0x468)?,
                platform_drop_delay: common.reader().f32(common_data + 0x470)?,
                platform_drop_velocity: common.reader().f32(common_data + 0x46C)?,
            },
            squat_choices,
            wait_choices,
            commands,
            instruction_offsets: words.keys().copied().collect(),
            motion_table_offset: motion_table,
            camera_extents: {
                let p = data.link(root + 0x3C)?.ok_or("missing camera extents")?;
                [read_vec(data, p)?, read_vec(data, p + 12)?]
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
            7 => Command::Goto(archive.link(offset + 4)?.ok_or("null command goto")? as usize),
            8 => Command::WaitAnimationLoop,
            // ftAction_80071D40: signed 7-bit model index, signed 19-bit selection.
            31 => Command::ModelSelection {
                group: ((word << 6) as i32) >> 25,
                variant: ((word << 13) as i32) >> 13,
            },
            // ftAction_80072A5C (80072A80/84): eight-bit ID, low 18-bit duration.
            46 => Command::ColorAnimation(super::commands::ColorAnimationRequest {
                id: ((word >> 18) & 255) as u8,
                duration: word & 0x3FFFF,
            }),
            19 => Command::SetVariable {
                index: ((word >> 24) & 3) as usize,
                value: word & 0xFFFFFF,
            },
            10 => {
                // ftAction_80071028 (0x80071028): signed offsets, unsigned ranges.
                // Retail uses the literal 0.003906f, not exact 1/256; no fusion.
                const SCALE: f32 = 0.003906;
                let r = archive.reader();
                Command::Graphics(super::effects::GraphicsCommand {
                    bone: ((word >> 18) & 255) as usize,
                    common_bone: word & (1 << 17) != 0,
                    destroy_on_state_change: word & (1 << 16) != 0,
                    item_bone: word & (1 << 15) != 0,
                    id: r.u16(offset + 4)?,
                    parameter: f32::from(r.u16(offset + 6)?),
                    offset: hsd_types::Vec3::new(
                        SCALE * f32::from(r.u16(offset + 8)? as i16),
                        SCALE * f32::from(r.u16(offset + 10)? as i16),
                        SCALE * f32::from(r.u16(offset + 12)? as i16),
                    ),
                    range: hsd_types::Vec3::new(
                        SCALE * f32::from(r.u16(offset + 14)?),
                        SCALE * f32::from(r.u16(offset + 16)?),
                        SCALE * f32::from(r.u16(offset + 18)?),
                    ),
                })
            }
            // ftAction_8007121C: five command words per attack capsule.
            11 => Command::SpawnHitbox {
                id: ((word >> 23) & 7) as usize,
                descriptor: super::hitbox::HitboxDescriptor::read(archive, offset)?,
            },
            34 => {
                let id = ((word >> 23) & 7) as usize;
                assert!(id < 2, "ftAction_80071E04: throw hitbox index");
                Command::SetThrowHitbox {
                    id,
                    descriptor: super::hitbox::ThrowHitbox::read(archive, offset)?,
                }
            }
            15 => Command::ClearHitbox(((word >> 23) & 7) as usize),
            16 => Command::ClearHitboxes,
            // ftAction_80071AE8 (80071AE8): x2218_b1 unless disabled (or holding an item).
            28 => Command::JabCombo {
                disabled: word & 0x03ff_ffff != 0,
            },
            29 => Command::JabFollowup(word & 0x03ff_ffff != 0),
            50 => Command::ToggleDynamics(((word << 6) as i32) >> 6),
            49 => Command::SwordTrail {
                duration: ((word << 7) as i32) >> 7,
                reverse: word & (1 << 25) != 0,
            },
            23 => Command::AllowInterrupt,
            // ftAction_80071A14 (80071A30 clrlwi): low 26-bit vulnerability enum.
            26 => Command::HurtStatus(match word & 0x03ff_ffff {
                0 => super::escape::HurtStatus::Normal,
                1 => super::escape::HurtStatus::Invincible,
                2 => super::escape::HurtStatus::Intangible,
                value => return Err(format!("unknown hurt status {value}").into()),
            }),
            20 if word & 0x03ff_ffff == 0 => Command::ReverseFacing,
            41 => Command::Part {
                group: ((word >> 19) & 127) as usize,
                variant: ((word >> 12) & 127) as usize,
                blend: (word & 4095) as f32,
            },
            43 => Command::Rumble {
                all_players: word & (1 << 25) != 0,
                id: ((word >> 13) & 4095) as u16,
                duration: (word & 8191) as u16,
            },
            17 | 54 => Command::FootstepSound {
                behavior: ((word >> 18) & 255) as u8,
                id: archive.reader().u32(offset + 4)?,
                volume: (archive.reader().u32(offset + 8)? >> 8) as u8,
                pan: archive.reader().u32(offset + 8)? as u8,
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
            Command::Goto(target) => {
                read_script(archive, target as u32, commands)?;
                break;
            }
            Command::Call {
                target,
                continuation,
            } => {
                read_script(archive, target as u32, commands)?;
                offset = continuation as u32;
            }
            Command::Graphics(_) | Command::SpawnHitbox { .. } => offset += 20,
            Command::LandingEffect(_)
            | Command::FootstepSound { .. }
            | Command::SetThrowHitbox { .. } => offset += 12,
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
                height: super::caches::HurtHeight::from_retail(a.reader().u32(p + 4)?),
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
    if count == 0 {
        return Ok(Vec::new());
    }
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

/// ftData.x20->x0[2]: neutral shield descriptor used by ftCo_80091E78.
fn read_guard_pose(a: &Archive, root: u32) -> Result<Vec<hsd_anim::jobj::JObj>> {
    let table = a.link(root + 0x20)?.ok_or("missing pose table")?;
    let table = a.link(table)?.ok_or("missing pose list")?;
    let joint = a.link(table + 8)?.ok_or("missing shield pose")?;
    let desc = desc::JObjDesc::read(a, joint)?;
    let (tree, root) = hsd_anim::load::load_joint_tree(a, &desc)?;
    Ok(tree
        .depth_first(root)
        .map(|id| tree.get(id).clone())
        .collect())
}
