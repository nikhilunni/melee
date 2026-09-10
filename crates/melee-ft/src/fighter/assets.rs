//! Owned resources needed by a Wait fighter; Melee archive layout stays here.
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
use melee_cmd::Command;
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct PartResource {
    pub root: usize,
    pub nodes: Vec<AnimJoint>,
    pub prepared: Vec<Option<hsd_anim::aobj::AObj>>,
    pub duration: f32,
}
/// Character metadata from ftdata.c, ftparts.c's PlCo tables and costume strings.
/// Character crates own these descriptors; shared loaders never select by kind.
#[derive(Clone, Copy, Debug)]
pub struct CharacterDescriptor {
    pub kind: melee_types::FighterKind,
    pub common_behavior: CommonBehavior,
    pub data_file: &'static str,
    pub data_symbol: &'static str,
    pub animation_file: &'static str,
    pub animation_count: u32,
    pub part_count: u32,
    pub part_animation_count: usize,
    pub costumes: &'static [CostumeDescriptor],
    /// Character table animations using ported shared callbacks.
    pub additional_motions: &'static [u32],
}
/// Retail common-state capability metadata. The descriptor supplies defaults
/// even for kinds whose character-owned callbacks have not yet been ported.
/// A ported character overrides the corresponding CharacterCallbacks hook.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommonBehavior {
    /// ftCo_AttackS4.c:145-166, decideFighter (8008C348).
    pub forward_smash_entry: bool,
    /// ftCo_Throw.c:145-157,346-353, per-character capture/laser callbacks.
    pub throw_callback: bool,
    /// ftCo_Attack1.c:89-110, decideAttack11 / getMotionFlags.
    pub jab_entry: bool,
    /// ftCo_AirCatch.c:54-79, ftCo_800C3B10 (800C3B10).
    pub air_dodge_tether: bool,
    /// ftCo_Landing.c:51-83, ftCo_Landing_Enter (800D5AEC).
    pub landing_reset: bool,
    /// ftCo_Escape.c:83-85, rolling-only morph-ball setup.
    pub morph_ball_roll: bool,
}
impl CommonBehavior {
    /// Descriptor-layer defaults transcribed from retail's kind selections.
    /// This is initialization data; common state execution reads named fields.
    pub const fn for_kind(kind: melee_types::FighterKind) -> Self {
        use melee_types::FighterKind;
        Self {
            forward_smash_entry: matches!(
                kind,
                FighterKind::Ness
                    | FighterKind::Peach
                    | FighterKind::GameWatch
                    | FighterKind::Pikachu
                    | FighterKind::Pichu
            ),
            throw_callback: matches!(
                kind,
                FighterKind::Fox | FighterKind::Samus | FighterKind::Kirby | FighterKind::Yoshi
            ),
            jab_entry: matches!(
                kind,
                FighterKind::GameWatch | FighterKind::Pikachu | FighterKind::Pichu
            ),
            air_dodge_tether: matches!(
                kind,
                FighterKind::Link | FighterKind::CLink | FighterKind::Samus
            ),
            landing_reset: matches!(
                kind,
                FighterKind::Mario
                    | FighterKind::DrMario
                    | FighterKind::Peach
                    | FighterKind::Emblem
                    | FighterKind::GameWatch
                    | FighterKind::Popo
                    | FighterKind::Nana
                    | FighterKind::Kirby
                    | FighterKind::Mewtwo
            ),
            morph_ball_roll: matches!(kind, FighterKind::Samus),
        }
    }
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
    pub attacks: super::attack::AttackParameters,
    pub combo: super::attack::combo::ComboParameters,
    pub overlap: melee_coll::overlap::OverlapParameters,
    pub hurtboxes: Vec<melee_coll::hurtbox::HurtCapsule>,
    pub first_stale_penalty: f32,
    pub stale_weights: [f32; 9],
    pub grab_friction_multiplier: f32,
    pub throw_weight_scale: f32,
    pub smash_sounds: Vec<u32>,
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
    pub wait_choices: Option<Vec<WaitEntry>>,
    pub commands: Vec<Command>,
    /// Archive-relative source locations for savestate import/diagnostics only.
    pub instruction_offsets: Vec<u32>,
    pub motion_table_offset: u32,
    pub life: super::life::LifeParameters,
    pub teeter: super::teeter::TeeterParameters,
    pub revival_platform: super::life::RevivalPlatform,
    pub charge_overlays: BTreeMap<u8, Vec<super::smash::OverlayCommand>>,
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
            .flatten()
            .chain(squat_choices.iter().flatten())
            .filter(|entry| entry.motion >= 0)
            .map(|entry| entry.motion as u32)
            .collect();
        let mut entries = BTreeMap::new();
        let mut words = BTreeMap::new();
        let script_ids = motion_indices(
            &[
                2, 3, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 20, 23, 26, 30, 31, 34, 35, 37, 38, 39,
                40, 41, 42, 43, 17, 19, 36, 44, 11, 216, 217, 220, 224, 225, 226, 227, 228, 238,
                46, 58, 167, 168, 169, 209, 242,
            ],
            &idle_motions,
            descriptor.additional_motions,
        );
        for id in script_ids {
            if table.entries[id as usize].aj_size == 0 {
                continue;
            }
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
        let color_table = common.link(common_root + 6 * 4)?.ok_or("color table")?;
        let mut charge_overlays = BTreeMap::new();
        for command in &commands {
            if let Command::SmashCharge(charge) = command {
                let entry = common
                    .link(color_table + u32::from(charge.color_animation) * 8)?
                    .ok_or("charge color script")?;
                charge_overlays.insert(
                    charge.color_animation,
                    super::smash::read_overlay(common, entry)?,
                );
            }
        }
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
                    prepared: nodes
                        .iter()
                        .map(|node| node.aobjdesc.as_ref().map(hsd_anim::aobj::AObj::load_desc))
                        .collect(),
                    nodes,
                    duration,
                },
            );
        }
        Ok(Self {
            smash_sounds: {
                let sound_table = data.link(root + 0x4C)?.ok_or("missing fighter SFX")?;
                if let Some(list) = data.link(sound_table)? {
                    let count = data.reader().u32(list)?;
                    let ids = data.link(list + 4)?.ok_or("missing smash sound list")?;
                    (0..count)
                        .map(|i| data.reader().u32(ids + i * 4))
                        .collect::<std::result::Result<Vec<_>, _>>()?
                } else {
                    Vec::new()
                }
            },
            throw_weight_scale: common.reader().f32(common_data + 0x37C)?,
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
            attacks: super::attack::AttackParameters::read(common, common_data)?,
            combo: super::attack::combo::ComboParameters::read(common, common_data)?,
            grab_friction_multiplier: common.reader().f32(common_data + 0x64)?,
            // Fighter_LoadCommonData: pData[3] -> Fighter_804D6548 stale weights.
            first_stale_penalty: common.reader().f32(
                common
                    .link(common_root + 12)?
                    .ok_or("missing stale weights")?,
            )?,
            stale_weights: {
                let table = common.link(common_root + 12)?.ok_or("stale weights")?;
                let mut weights = [0.0; 9];
                for (i, weight) in weights.iter_mut().enumerate() {
                    *weight = common.reader().f32(table + i as u32 * 4)?;
                }
                weights
            },
            overlap: melee_coll::overlap::OverlapParameters {
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
            motions: motion_indices(
                &[
                    2, 3, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 20, 21, 22, 23, 24, 25, 26, 27, 28,
                    30, 31, 34, 35, 37, 38, 39, 40, 41, 42, 43, 17, 19, 36, 44, 11, 216, 217, 220,
                    224, 225, 226, 227, 228, 238, 46, 58, 167, 168, 169, 209, 242,
                ],
                &idle_motions,
                descriptor.additional_motions,
            )
            .into_iter()
            .filter(|&id| table.entries[id as usize].aj_size != 0)
            .map(|id| {
                Ok((
                    id as i32,
                    read_playback_motion(data, root, &table, aj, id as usize)?,
                ))
            })
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
                multi_jump_drift_threshold: common.reader().f32(common_data + 0x258)?,
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
            revival_platform: {
                let table = common
                    .link(common_root + 8 * 4)?
                    .ok_or("missing revival platform resources")?;
                let descriptor = hsd_archive::desc::JObjDesc::read(
                    common,
                    common
                        .link(table)?
                        .ok_or("missing revival platform model")?,
                )?;
                let (mut tree, root) = hsd_anim::load::load_joint_tree(common, &descriptor)?;
                let animation = hsd_archive::desc::AnimJoint::read(
                    common,
                    common.link(table + 4)?.ok_or("missing revival animation")?,
                )?;
                hsd_anim::load::attach_anim_joint(&mut tree, root, &animation, common)?;
                tree.req_anim_all(root, 0.0);
                super::life::RevivalPlatform { tree, root }
            },
            life: super::life::LifeParameters {
                death_delay: common.reader().s32(common_data + 0x500)?,
                revival_duration: common.reader().s32(common_data + 0x5D0)?,
                platform_duration: common.reader().s32(common_data + 0x5D4)?,
                invincibility_duration: common.reader().s32(common_data + 0x5D8)?,
                death_effect_scale: common.reader().f32(common_data + 0x4F4)?,
            },
            teeter: super::teeter::TeeterParameters {
                walk_threshold: common.reader().f32(common_data + 0x474)?,
                edge_distance: common.reader().f32(common_data + 0x478)?,
                edge_margin: common.reader().f32(common_data + 0x47C)?,
                sound: {
                    let sound_table = data.link(root + 0x4C)?.ok_or("missing fighter SFX")?;
                    data.reader().u32(sound_table + 0x18)?
                },
            },
            charge_overlays,
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
        let mut words = [0; 7];
        let count = melee_cmd::decode::word_count(opcode);
        for (index, word) in words[..count].iter_mut().enumerate() {
            *word = archive.reader().u32(offset + index as u32 * 4)?;
        }
        let target = if matches!(opcode, 5 | 7) {
            archive.link(offset + 4)?.map(|x| x as usize)
        } else {
            None
        };
        let command = melee_cmd::decode::decode(&words[..count], target, (offset + 8) as usize)
            .map_err(|error| format!("{error} at {offset:#x}, opcode {opcode}"))?;
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
            Command::RandomSound(_) => offset += 28,
            Command::WindEffect(_) => offset += 16,
            Command::SmashCharge(_) => offset += 8,
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

fn read_hurtboxes(a: &Archive, root: u32) -> Result<Vec<melee_coll::hurtbox::HurtCapsule>> {
    let table = a.link(root + 0x30)?.ok_or("missing hurtboxes")?;
    let count = a.reader().u32(table)?;
    let base = a.link(table + 4)?.ok_or("missing hurtbox records")?;
    assert!(count <= 15);
    (0..count)
        .map(|i| {
            let p = base + i * 0x28;
            Ok(melee_coll::hurtbox::HurtCapsule {
                grabbable: a.reader().u32(p + 8)? != 0,
                bone: a.reader().u32(p)? as usize,
                height: melee_coll::hurtbox::HurtHeight::from_retail(a.reader().u32(p + 4)?),
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
    let Some(table) = a.link(table)? else {
        // Animated character shields do not supply the common blended pose.
        return Ok(Vec::new());
    };
    let joint = a.link(table + 8)?.ok_or("missing shield pose")?;
    let desc = desc::JObjDesc::read(a, joint)?;
    let (tree, root) = hsd_anim::load::load_joint_tree(a, &desc)?;
    Ok(tree
        .depth_first(root)
        .map(|id| tree.get(id).clone())
        .collect())
}

/// Prepare each archive animation once, in retail index order. Keeping the
/// common, idle and character lists separate avoids deeply nested iterator
/// instantiations when another common motion family adds its resources.
fn motion_indices(base: &[u32], idle: &BTreeSet<u32>, additional: &[u32]) -> BTreeSet<u32> {
    let mut indices = idle.clone();
    for list in [
        base,
        additional,
        super::down::MOTIONS,
        super::teeter::MOTIONS,
    ] {
        indices.extend(list.iter().copied());
    }
    // S2: ftData_MotionStateList[65..74] aerials and directional landing lag (motions 68..78).
    indices.extend(68..78);
    indices
}
