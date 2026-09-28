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
    /// Part sources installed by character callbacks rather than subaction commands.
    pub additional_part_animations: &'static [(usize, usize)],
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
    /// ftCo_ShieldBreakFly.c:30: Jigglypuff can KO above the top boundary.
    pub shield_break_top_exit: bool,
    /// ftCo_8008A7A8 (ftwaitanim.c:67): Fox and Mewtwo keep choosing idle
    /// animations while holding an item; everyone else replays the current one.
    pub idle_variants_while_holding: bool,
    /// ftCo_8009E614 / ftCo_8009E7B4 (ftdynamics.c:555-583, 617-625): Marth
    /// and Roy hand every dynamic bone to the solver while stage wind blows.
    pub stage_wind_dynamics: bool,
    /// ftData_OnItemPickupExt / OnItemDropExt: the x8B0 hand-pose slots the
    /// kind's Fighter_OnItemPickup call names. None: not ported.
    pub item_hand: Option<ItemHandSlots>,
}

/// Fighter_OnItemPickup(gobj, flag, pose, shown) (ft/inlines.h:143): the
/// held item's hold kind selects slot `pose`'s hand animation (x8B0.x10);
/// a pickup then applies slot `shown`'s selection (ftAnim_80070C48).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemHandSlots {
    pub pose: usize,
    pub shown: usize,
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
            shield_break_top_exit: matches!(kind, FighterKind::Purin),
            idle_variants_while_holding: matches!(kind, FighterKind::Fox | FighterKind::Mewtwo),
            stage_wind_dynamics: matches!(kind, FighterKind::Mars | FighterKind::Emblem),
            item_hand: match kind {
                FighterKind::Mars | FighterKind::Emblem => {
                    Some(ItemHandSlots { pose: 0, shown: 1 })
                }
                FighterKind::Pikachu
                | FighterKind::Pichu
                | FighterKind::Samus
                | FighterKind::Boy
                | FighterKind::Girl => Some(ItemHandSlots { pose: 0, shown: 0 }),
                FighterKind::Mario
                | FighterKind::Fox
                | FighterKind::Captain
                | FighterKind::Donkey
                | FighterKind::Koopa
                | FighterKind::Link
                | FighterKind::Seak
                | FighterKind::Ness
                | FighterKind::Peach
                | FighterKind::Popo
                | FighterKind::Yoshi
                | FighterKind::Luigi
                | FighterKind::Zelda
                | FighterKind::CLink
                | FighterKind::DrMario
                | FighterKind::Falco
                | FighterKind::GameWatch
                | FighterKind::Ganon
                | FighterKind::GKoops => Some(ItemHandSlots { pose: 1, shown: 1 }),
                // Kirby, Jigglypuff, Nana, Mewtwo and the bosses: own callbacks.
                _ => None,
            },
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
    pub clank: super::clank::Parameters,
    pub attacks: super::attack::AttackParameters,
    pub combo: super::attack::combo::ComboParameters,
    pub overlap: melee_coll::overlap::OverlapParameters,
    pub hurtboxes: Vec<melee_coll::hurtbox::HurtCapsule>,
    pub first_stale_penalty: f32,
    pub stale_weights: [f32; 9],
    pub grab_friction_multiplier: f32,
    pub throw_weight_scale: f32,
    pub grab_escape: super::grab_escape::Parameters,
    pub smash_sounds: Vec<u32>,
    /// Authored AJ availability for high, high-mid, low-mid and low forward smash.
    pub forward_smash_variants: [bool; 4],
    /// ftData_80085FD4(..., AppealSL)->x8 (AJ size).
    pub left_taunt_available: bool,
    /// ft_data->x4C_sfx->x1C / x20: the medium and heavy damage voice tables.
    pub medium_voices: Vec<u32>,
    pub heavy_voices: Vec<u32>,
    pub dynamics: Vec<crate::dynamics::DynamicSetDescriptor>,
    pub dynamics_motion_starts: BTreeMap<i32, crate::dynamics::MotionDynamics>,
    pub dynamic_colliders: Vec<super::caches::DynamicCollider>,
    pub motions: BTreeMap<i32, Motion>,
    pub rotating_effect_bones: [usize; 5],
    pub wall_jump: super::wall_jump::Parameters,
    pub wall_jump_sound: u32,
    pub jumping: super::jump::JumpParameters,
    pub falling: super::fall::FallParameters,
    pub parasol: super::parasol::ParasolParameters,
    pub air_dodge: super::air_dodge::AirDodgeParameters,
    pub ledge: super::ledge::LedgeParameters,
    pub running: super::dash::RunningParameters,
    pub movement: crate::desc::common::MovementParameters,
    pub squat_choices: Option<Vec<WaitEntry>>,
    pub wait_choices: Option<Vec<WaitEntry>>,
    pub commands: std::sync::Arc<[Command]>,
    /// Archive-relative source locations for savestate import/diagnostics only.
    pub instruction_offsets: Vec<u32>,
    pub motion_table_offset: u32,
    pub life: super::life::LifeParameters,
    pub teeter: super::teeter::TeeterParameters,
    pub revival_platform: super::life::RevivalPlatform,
    /// PlCo's color-animation table (Fighter_804D653C): each id's priority,
    /// slot and decoded program.
    pub color_overlays: super::color_overlay::ColorOverlayTable,
    pub camera_extents: [hsd_types::Vec3; 2],
    /// ftData x40 (itPickup): the item pickup boxes.
    pub pickup: super::item_pickup::PickupBoxes,
    /// Fighter_804D6550: the item throw table.
    pub item_throws: [super::item_throw::ItemThrowRow; super::item_throw::ITEM_THROW_ROWS],
    /// PlCo +400: the animation rate of a smash item throw (LightThrowF4 on).
    pub smash_throw_rate: f32,
    /// PlCo +1B8..1C0: the tumble bounce off a wall.
    pub fly_reflect: super::fly_reflect::BounceParameters,
    /// PlCo +3FC: an air throw within this many frames of the stick's move
    /// is a smash throw (ftCo_80095328).
    pub air_smash_throw_window: i32,
    /// PlCo +404/+408/+40C: a dash throw's friction multiplier, the frames
    /// it applies scaled, and that scale (ftCo_LightThrowDash_Phys).
    pub dash_throw_friction: [f32; 3],
    /// CommonBehavior's item hand slots and held-item idle choice.
    pub item_hand: Option<ItemHandSlots>,
    pub idle_variants_while_holding: bool,
    pub magnifier: super::offscreen::MagnifierDamage,
    pub command_entries: BTreeMap<i32, usize>,
    /// Rows this fighter authors no animation for (ftData_80085CD8 leaves
    /// x590 NULL) that are still entered: thrown-victim rows (ftCo_800DD4B0)
    /// and Yoshi's egg shield stun (ftYs_Shield_8012C600). Their flags and
    /// blend byte still apply.
    pub unanimated: BTreeMap<i32, (crate::anim::MotionFlags, f32)>,
    pub part_animations: BTreeMap<(usize, usize), PartResource>,
}
/// ftCo_SM_GuardDamage: the shield-stun animation of ftCo_MS_GuardSetOff.
const GUARD_DAMAGE_ANIMATION: i32 = 40;
/// Another fighter's animations a fighter plays where its own table
/// authors none: ftData_80085FD4 hands Nana Popo's row (gFtDataList[POPO]
/// ->xC[msid]) whenever her own row's animation is absent.
pub struct AnimationFallback<'a> {
    pub descriptor: &'a CharacterDescriptor,
    pub data: &'a Archive,
    pub aj: &'a [u8],
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
        Self::load_with_fallback(descriptor, data, common, aj, None)
    }

    /// [`Self::load`] with the animations of `fallback` standing in for rows
    /// this fighter's table leaves without animation. The row's own flags,
    /// blend frames and script still apply (Fighter_ChangeMotionState reads
    /// fp->x24; only ftData_80085CD8's figatree comes from the fallback).
    pub fn load_with_fallback(
        descriptor: &CharacterDescriptor,
        data: &Archive,
        common: &Archive,
        aj: &[u8],
        fallback: Option<AnimationFallback<'_>>,
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
        let wait_choices = read_wait_table(data, root)?;
        let squat_choices = crate::desc::playback::read_squat_table(data, root)?;
        let mut entries = BTreeMap::new();
        let mut words = BTreeMap::new();
        // Fighter_ChangeMotionState loads any motion row on demand (ftData_80085CD8), so every
        // row with authored animation data carries its script and animation.
        let own_ids = authored_motions(&table);
        let fallback = fallback
            .map(|fallback| -> Result<_> {
                let root = fallback
                    .data
                    .public(fallback.descriptor.data_symbol)
                    .ok_or("missing fallback fighter data symbol")?;
                let table = read_fighter_animations(
                    fallback.data,
                    root,
                    fallback.descriptor.animation_count,
                )?;
                Ok((fallback, root, table))
            })
            .transpose()?;
        let borrowed_ids: Vec<u32> = fallback.as_ref().map_or(Vec::new(), |(_, _, table)| {
            authored_motions(table)
                .into_iter()
                .filter(|id| !own_ids.contains(id) && (*id as usize) < table.entries.len())
                .collect()
        });
        let mut script_ids = own_ids.clone();
        script_ids.extend(borrowed_ids.iter().copied());
        script_ids.sort_unstable();
        // ftData_80085CD8 with x590 NULL: a thrower's unanimated thrown-victim
        // row, or an unanimated shield stun (Yoshi's egg), still supplies
        // its flags and script.
        let mut unanimated = BTreeMap::new();
        let entered_unanimated = super::grab_throw::THROWS
            .iter()
            .map(|throw| throw.victim_motion)
            .chain([GUARD_DAMAGE_ANIMATION]);
        for motion in entered_unanimated {
            let id = motion as u32;
            if !script_ids.contains(&id) {
                unanimated.insert(
                    motion,
                    crate::desc::playback::read_motion_header(data, root, &table, id as usize)?,
                );
            }
        }
        let unanimated_ids = unanimated.keys().map(|&id| id as u32);
        for id in script_ids.iter().copied().chain(unanimated_ids) {
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
        let color_overlays = super::color_overlay::ColorOverlayTable::read(common, color_table)?;
        let groups = commands
            .iter()
            .filter_map(|c| {
                if let Command::Part { group, variant, .. } = c {
                    Some((*group, *variant))
                } else {
                    None
                }
            })
            .chain(descriptor.additional_part_animations.iter().copied())
            .collect::<BTreeSet<_>>();
        // Fighter_OnItemPickup selects hand poses 0..=3 by the item's hold
        // kind; ftData_x1C has no variant count, so absent ones are skipped.
        let item_hand_poses = descriptor
            .common_behavior
            .item_hand
            .into_iter()
            .flat_map(|hand| [hand.pose, hand.shown])
            .flat_map(|group| (0..4).map(move |variant| (group, variant)))
            .filter(|entry| !groups.contains(entry))
            .collect::<BTreeSet<_>>();
        let part_table = data.link(root + 0x1C)?.ok_or("missing part animations")?;
        let mut part_animations = BTreeMap::new();
        for (group, variant) in groups.into_iter().chain(item_hand_poses.iter().copied()) {
            let optional = item_hand_poses.contains(&(group, variant));
            let Some(set) = data.link(part_table + group as u32 * 4)? else {
                if optional {
                    continue;
                }
                return Err("missing part set".into());
            };
            let animations = data.link(set + 8)?.ok_or("missing part variants")?;
            let Some(offset) = data.link(animations + variant as u32 * 4)? else {
                if optional {
                    continue;
                }
                return Err("missing part variant".into());
            };
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
            wall_jump_sound: {
                let sound_table = data.link(root + 0x4C)?.ok_or("missing fighter SFX")?;
                data.reader().u32(sound_table + 0x24)?
            },
            wall_jump: super::wall_jump::Parameters::read(common, common_data)?,
            medium_voices: read_sfx_array(data, root, 0x1C)?,
            heavy_voices: read_sfx_array(data, root, 0x20)?,
            throw_weight_scale: common.reader().f32(common_data + 0x37C)?,
            grab_escape: super::grab_escape::Parameters::read(common, common_data)?,
            magnifier: super::offscreen::MagnifierDamage::read(common, common_data)?,
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
            clank: super::clank::Parameters::read(common, common_data)?,
            attacks: super::attack::AttackParameters::read(common, common_data)?,
            // ftCo_AttackS4 doEnter probes submotion indices, not action IDs.
            forward_smash_variants: [60, 61, 63, 64].map(|id| table.entries[id].aj_size != 0),
            left_taunt_available: table.entries[240].aj_size != 0,
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
                partner_depth_step: common.reader().f32(common_data + 0x45C)?,
                partner_depth_limit: common.reader().f32(common_data + 0x460)?,
            },
            hurtboxes: read_hurtboxes(data, root)?,
            dynamics: crate::dynamics::read_sets(data, root)?,
            dynamics_motion_starts: crate::dynamics::read_motion_starts(
                data,
                root,
                descriptor.animation_count,
            )?,
            dynamic_colliders: read_dynamic_colliders(data, root)?,
            motions: {
                let mut motions: BTreeMap<i32, Motion> = own_ids
                    .iter()
                    .copied()
                    .map(|id| {
                        Ok((
                            id as i32,
                            read_playback_motion(data, root, &table, aj, id as usize)?,
                        ))
                    })
                    .collect::<Result<_>>()?;
                if let Some((fallback, fallback_root, fallback_table)) = &fallback {
                    for &id in &borrowed_ids {
                        let mut motion = read_playback_motion(
                            fallback.data,
                            *fallback_root,
                            fallback_table,
                            fallback.aj,
                            id as usize,
                        )?;
                        let (flags, blend_frames) = crate::desc::playback::read_motion_header(
                            data,
                            root,
                            &table,
                            id as usize,
                        )?;
                        motion.flags = flags;
                        motion.blend_frames = blend_frames;
                        motions.insert(id as i32, motion);
                    }
                }
                // Borrowed throw motions own their prepared maps through the existing
                // MotionRemap storage, keeping resource destruction in the same owners.
                // Yoshi's egg (ftCo_SM_YoshiEgg) is borrowed the same way.
                let borrowed = super::grab_throw::THROWS
                    .iter()
                    .map(|throw| throw.victim_motion)
                    .chain([
                        super::capture_yoshi::EGG_MOTION,
                        super::capture_captain::VICTIM_MOTION,
                    ]);
                for borrowed in borrowed {
                    if let Some(motion) = motions.get_mut(&borrowed) {
                        let source = crate::desc::bones::AnimationSource::read(
                            common,
                            motion.flags.source_skeleton(),
                            descriptor.part_count,
                        )?;
                        motion.remap = Some(crate::anim::attach::MotionRemap {
                            source: source.parts,
                            destination: read_part_table(
                                common,
                                descriptor.kind,
                                descriptor.part_count,
                            )?,
                            source_masks: source.masks,
                        });
                    }
                }
                motions
            },
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
            parasol: super::parasol::ParasolParameters::read(common, common_data)?,
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
                shield_grab_delay: common.reader().f32(common_data + 0x68)?,
                shield_item_throw_frames: common.reader().s32(common_data + 0x410)?,
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
                platform_drop_window: common.reader().f32(common_data + 0x468)?,
                platform_drop_delay: common.reader().f32(common_data + 0x470)?,
                platform_drop_velocity: common.reader().f32(common_data + 0x46C)?,
            },
            squat_choices,
            wait_choices,
            commands: commands.into(),
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
                top_knockback_threshold: common.reader().f32(common_data + 0x4F0)?,
                star: super::life::StarKoParameters {
                    hold: common.reader().s32(common_data + 0x504)?,
                    flight: common.reader().s32(common_data + 0x508)?,
                    vanish_delay: common.reader().s32(common_data + 0x50C)?,
                    depth: common.reader().f32(common_data + 0x510)?,
                    height_ratio: common.reader().f32(common_data + 0x514)?,
                },
                screen_ko: {
                    let r = common.reader();
                    let at = |o: u32| common_data + o;
                    let vec3 = |o: u32| -> Result<hsd_types::Vec3> {
                        Ok(hsd_types::Vec3::new(
                            r.f32(at(o))?,
                            r.f32(at(o + 4))?,
                            r.f32(at(o + 8))?,
                        ))
                    };
                    super::life::ScreenKoParameters {
                        threshold: r.s32(at(0x520))?,
                        hold: r.s32(at(0x524))?,
                        approach_frames: r.s32(at(0x528))?,
                        impact_hold: r.s32(at(0x52C))?,
                        fall_frames: r.s32(at(0x530))?,
                        vanish_delay: r.s32(at(0x534))?,
                        start: vec3(0x538)?,
                        end: vec3(0x544)?,
                        fall_speed_y: r.f32(at(0x550))?,
                        gravity: r.f32(at(0x554))?,
                        terminal_velocity: r.f32(at(0x558))?,
                        fall_speed_z: r.f32(at(0x55C))?,
                    }
                },
                death_sounds: {
                    let sound_table = data.link(root + 0x4C)?.ok_or("missing fighter SFX")?;
                    super::life::DeathSounds {
                        cries: [
                            data.reader().u32(sound_table + 0x4)?,
                            data.reader().u32(sound_table + 0x8)?,
                        ],
                        star: data.reader().u32(sound_table + 0xC)?,
                    }
                },
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
            color_overlays,
            item_throws: super::item_throw::read_throw_table(common, common_root)?,
            smash_throw_rate: common.reader().f32(common_data + 0x400)?,
            fly_reflect: super::fly_reflect::BounceParameters::read(common, common_data)?,
            air_smash_throw_window: common.reader().s32(common_data + 0x3FC)?,
            dash_throw_friction: [
                common.reader().f32(common_data + 0x404)?,
                common.reader().f32(common_data + 0x408)?,
                common.reader().f32(common_data + 0x40C)?,
            ],
            item_hand: descriptor.common_behavior.item_hand,
            idle_variants_while_holding: descriptor.common_behavior.idle_variants_while_holding,
            pickup: {
                let p = data.link(root + 0x40)?.ok_or("missing item pickup boxes")?;
                super::item_pickup::PickupBoxes::read(data, p)?
            },
            camera_extents: {
                let p = data.link(root + 0x3C)?.ok_or("missing camera extents")?;
                [read_vec(data, p)?, read_vec(data, p + 12)?]
            },
            command_entries: entries
                .into_iter()
                .map(|(id, offset)| (id, indices[&offset]))
                .collect(),
            unanimated,
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
            // An unported opcode's length is unknown; the interpreter stops there.
            Command::End | Command::Return | Command::Unported(_) => break,
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

/// The motion table rows with authored AJ data (ftData_80085FD4 `x8`
/// size): the rows a motion change can attach.
fn authored_motions(table: &crate::desc::FighterAnimations) -> Vec<u32> {
    (0..table.entries.len() as u32)
        .filter(|&id| table.entries[id as usize].aj_size != 0)
        .collect()
}

/// One `FtSFXArr` (`{ int num; s32* sfx_ids; }`) hanging off the fighter's `FtSFX`
/// block (ft_data +4C) at `offset`; an absent pointer is an empty table.
fn read_sfx_array(data: &Archive, root: u32, offset: u32) -> Result<Vec<u32>> {
    let sound_table = data.link(root + 0x4C)?.ok_or("missing fighter SFX")?;
    let Some(list) = data.link(sound_table + offset)? else {
        return Ok(Vec::new());
    };
    let count = data.reader().u32(list)?;
    let ids = data.link(list + 4)?.ok_or("missing voice list")?;
    (0..count)
        .map(|i| data.reader().u32(ids + i * 4).map_err(Into::into))
        .collect()
}
