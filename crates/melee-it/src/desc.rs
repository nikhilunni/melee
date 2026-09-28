//! Item archive boundary: `it/types.h` Article and ItemCommonData.
use hsd_archive::{Archive, Result};
use melee_cmd::Command;
use melee_types::combat::HitboxDescriptor;

#[derive(Clone, Debug)]
pub struct ItemCommonData {
    pub hold_limits: [Option<usize>; 13],
    pub lifetime: f32,
    /// +2C: frames a new hold-kind 0 or 6 item takes no hits (item.c
    /// foobar's xD40), converted from its integer.
    pub spawn_intangible_frames: f32,
    /// +4C: it_80275158's half-life (xD48) as a fraction of the lifetime.
    pub half_life_scale: f32,
    /// +94 / +98 (x80[5], x80[6]): it_8026B1D4's contact damage per unit of
    /// a thrown item's speed, and its constant term.
    pub speed_damage_scale: f32,
    pub speed_damage_base: f32,
    pub shield_bounce_degrees: f32,
    /// +B4: an item hitbox stops against another item's when its damage,
    /// less this, is below the other's (it_8026FE68).
    pub clank_priority_gap: i32,
    pub maximum_reflected_damage: u32,
    /// +B8/+BC: item hitlag frames from contact damage (it_8026B424).
    pub hitlag_scale: f32,
    pub hitlag_base: f32,
    /// +F8: it_8027518C's lifetime once an item explodes.
    pub explosion_lifetime: f32,
    /// +6C / +68: it_80274658's spin degrees at creation (Item_80267130) and
    /// while falling (Item_ApplyFallingPhysics).
    pub spawn_spin_degrees: f32,
    pub fall_spin_degrees: f32,
    /// x9C..xB0 (it_80270E30).
    pub knockback: crate::hurt::ItemKnockback,
    /// Fighter common data it_8027B798 reads; the scene fills it in from
    /// PlCo after loading ItCo.
    pub launch: crate::hurt::ItemLaunch,
    /// +58 / +5C / +60: itColl_BounceOffVictim's rebound.
    pub victim_bounce: VictimBounce,
}

/// itColl_BounceOffVictim (80272DB0): an item rebounding off what it hit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VictimBounce {
    /// +58: the horizontal speed kept.
    pub horizontal_scale: f32,
    /// +5C / +60: the vertical speed kept, and the pop added to it.
    pub vertical_scale: f32,
    pub vertical_pop: f32,
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
            spawn_intangible_frames: r.u32(base + 0x2C)? as i32 as f32,
            half_life_scale: r.f32(base + 0x4C)?,
            speed_damage_scale: r.f32(base + 0x94)?,
            speed_damage_base: r.f32(base + 0x98)?,
            shield_bounce_degrees: r.f32(base + 0xE0)?,
            maximum_reflected_damage: r.u32(base + 0xD8)?,
            clank_priority_gap: r.u32(base + 0xB4)? as i32,
            hitlag_scale: r.f32(base + 0xB8)?,
            hitlag_base: r.f32(base + 0xBC)?,
            explosion_lifetime: r.f32(base + 0xF8)?,
            spawn_spin_degrees: r.f32(base + 0x6C)?,
            fall_spin_degrees: r.f32(base + 0x68)?,
            knockback: crate::hurt::ItemKnockback {
                cap: r.f32(base + 0x9C)?,
                percent: r.f32(base + 0xA0)?,
                damage_percent: r.f32(base + 0xA4)?,
                weight_set_percent: r.f32(base + 0xA8)?,
                scale: r.f32(base + 0xAC)?,
                base: r.f32(base + 0xB0)?,
                still_speed: r.f32(base + 0x78)?,
            },
            launch: Default::default(),
            victim_bounce: VictimBounce {
                horizontal_scale: r.f32(base + 0x58)?,
                vertical_scale: r.f32(base + 0x5C)?,
                vertical_pop: r.f32(base + 0x60)?,
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct ItemAssets {
    pub visual: hsd_archive::desc::item_visual::ItemVisual,
    pub scripts: Vec<Vec<Command>>,
    pub hit_flags: Vec<Vec<Option<ItemHitFlags>>>,
    pub special_attributes: Vec<f32>,
    pub scale: f32,
    pub model: u32,
    pub rotate_to_facing: bool,
    pub collision_box: melee_types::mp::ItEcb,
    pub collision_damage_multiplier: f32,
    /// ItemAttr x1_5 (Item.xDC8 xC): contacts put this kind into hitlag.
    pub hitlag: bool,
    /// ItemAttr x1_67_cam_kind (Item.xDCD): 0 none, 1 an Active camera
    /// subject, 2 an Auto one (item.c foobar3).
    pub camera_kind: u8,
    /// ItemAttr x10 / x14: Item_ApplyFallingPhysics' gravity and the speed
    /// it stops accelerating at (it_80272860).
    pub fall_acceleration: f32,
    pub fall_speed_limit: f32,
    /// ItemAttr x18: a spawned article's vertical launch speed (it_802B322C).
    pub launch_vertical_velocity: f32,
    /// ItemAttr x1_1 (Item.xDC8 x17 at creation): the ECB rotation axis.
    pub rotation_axis: u8,
    /// ItCo common data +F8 (it_8027518C): an explosion's lifetime. Zero
    /// for character articles, which do not explode.
    pub explosion_lifetime: f32,
    /// ItemAttr xC (spin_spd).
    pub spin_rate: f32,
    /// ItCo common data +68: Item_ApplyFallingPhysics' spin degrees.
    pub fall_spin_degrees: f32,
    /// Per article state: the frame its joint animation stops, if it does.
    pub animation_ends: Vec<Option<f32>>,
    /// Article x8 (ItHurtBoneList, it_8027163C): at most two hurt capsules.
    pub hurtboxes: Vec<ItemHurtbox>,
    /// it_804D6D28->xE8 (it_80275BC8): the ECB scale for a released item's
    /// first sweep. Zero for fighter articles, which are never released.
    pub release_box_scale: f32,
    /// ItemAttr x0 bit 0x80 (itIsHeavy): picked up with HeavyGet.
    pub heavy: bool,
    /// ItemAttr x0 bits 0x78 (it_8026B30C): how a holder uses the item;
    /// 5 is consumed on pickup (ftpickupitem_8009447C).
    pub use_kind: u8,
    /// ItemAttr x0 bits 0x07 (itGetHoldKind): the holder's hand pose.
    pub hand_hold_kind: u8,
    /// ItemAttr x4 (it_80273B50): the release velocity multiplier.
    pub throw_speed_multiplier: f32,
    /// ItemAttr x58 (it_80276FC4): the speed and hitbox damage kept by a
    /// wall or ceiling bounce.
    pub bounce_scale: f32,
    /// ItemAttr x80 (Item.xD84, it_8027321C): the bounce sound.
    pub bounce_sound: u32,
    /// ItemAttr x64 (destroy_gfx): the effect when an animation callback
    /// ends the item (ItemSwitch, destroy_type 0).
    pub destroy_effect: Option<u16>,
    /// ItemAttr x68: the effect when an event callback ends it (types 1/2).
    pub event_destroy_effect: Option<u16>,
    /// ItemAttr x30 / x38: the pickup box offset and half extents.
    pub grab_offset: hsd_types::Vec2,
    pub grab_range: hsd_types::Vec2,
    /// it_80272C90: the authored translation of the model's attachment
    /// bone (ItemModelDesc x8), which it_80273B50 undoes when a character
    /// article (hold kind 8) leaves the hand.
    pub attachment_translation: hsd_types::Vec3,
    /// The joint whose animation moves the kind (itUpdateVelocityFromBone),
    /// sampled by [`Self::read_bone_motion`].
    pub bone_motion: Option<crate::bone_motion::BoneMotion>,
    /// Every joint's locals per animation step, sampled by
    /// [`Self::read_pose`] for kinds that read a joint below the root.
    pub pose: Option<crate::pose::ItemPose>,
    /// The DPtcl keys of each article state's joint animation, sampled by
    /// [`Self::read_particle_tracks`] for kinds whose model carries them.
    pub particle_tracks: Option<crate::particle_track::ParticleTracks>,
    /// Special attribute words that point at an integer (itHeiho x0), read
    /// through by [`Self::from_stage_item`]: the integer at each.
    pub special_pointees: Vec<i32>,
}
impl ItemAssets {
    /// it_80272C90 / it_2725_JObjGetTranslation: the local translation of
    /// the model's attach joint (ItemModelDesc x8), in depth-first order.
    pub fn attach_translation(&self) -> hsd_types::Vec3 {
        let mut stack = vec![&self.visual.model];
        let mut index = 0;
        while let Some(joint) = stack.pop() {
            if index == self.visual.attachment_bone {
                let p = joint.position;
                return hsd_types::Vec3::new(p.x, p.y, p.z);
            }
            index += 1;
            if let Some(next) = joint.next.as_deref() {
                stack.push(next);
            }
            if let Some(child) = joint.child.as_deref() {
                stack.push(child);
            }
        }
        panic!("item attach joint {} missing", self.visual.attachment_bone)
    }
    /// ftData.x48_items -> Article, loaded once before any item exists. A
    /// character article's state rows are its motion states in order.
    pub fn from_fighter(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        states: usize,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        let rows: Vec<i32> = (0..states as i32).collect();
        Self::from_article(archive, article, &rows, 10)
    }

    /// ftData.x48_items[item_index] for an article whose motion states do
    /// not map one to one onto article states: `article_states` is the
    /// kind's ItemStateTable anim_id column, as for [`Self::from_common`].
    pub fn from_fighter_states(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        Self::from_article(archive, article, article_states, special_attributes)
    }

    /// The ItCo common data (it_804D6D28) an article reads once it leaves
    /// the hand or explodes: +F8 (it_8027518C), +68 and +E8 (it_80275BC8).
    pub fn read_common_release(
        &mut self,
        archive: &Archive,
        public: u32,
    ) -> hsd_archive::desc::Result<()> {
        let r = archive.reader();
        let common = r.u32(public)?;
        self.explosion_lifetime = r.f32(common + 0xF8)?;
        self.fall_spin_degrees = r.f32(common + 0x68)?;
        self.release_box_scale = r.f32(common + 0xE8)?;
        Ok(())
    }

    /// it_804D6D24 (itPublicData +4)[kind]: a common item's Article in ItCo.
    /// `article_states` is the kind's ItemStateTable anim_id column: the
    /// article state each motion state plays, or -1 for none.
    pub fn from_common(
        archive: &Archive,
        public: u32,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let articles = r.u32(public + 4)?;
        let article = r.u32(articles + kind as u32 * 4)?;
        let mut assets = Self::from_article(archive, article, article_states, special_attributes)?;
        assets.read_common_release(archive, public)?;
        Ok(assets)
    }

    fn from_article(
        archive: &Archive,
        article: u32,
        article_states: &[i32],
        special_count: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let common = r.u32(article)?;
        let special = r.u32(article + 4)?;
        let state_array = r.u32(article + 12)?;
        let model_desc = r.u32(article + 16)?;
        let mut scripts = Vec::with_capacity(article_states.len());
        let mut hit_flags = Vec::with_capacity(article_states.len());
        for &row in article_states {
            let (commands, flags) = if row < 0 {
                (Vec::new(), Vec::new())
            } else {
                read_script(archive, r.u32(state_array + row as u32 * 16 + 12)?)?
            };
            scripts.push(commands);
            hit_flags.push(flags);
        }
        let article_state_count = article_states
            .iter()
            .map(|&s| s + 1)
            .max()
            .unwrap_or(0)
            .max(0);
        let special_attributes = (0..special_count)
            .map(|i| r.f32(special + i * 4))
            .collect::<Result<_>>()?;
        let visual = hsd_archive::desc::item_visual::ItemVisual::read(
            archive,
            model_desc,
            state_array,
            article_state_count as usize,
        )?;
        let attachment_translation =
            attachment_translation(&visual.model, r.u32(model_desc + 4)?, visual.attachment_bone);
        let hurtbones = r.u32(article + 8)?;
        let hurtboxes = if hurtbones == 0 {
            Vec::new()
        } else {
            let descs = r.u32(hurtbones + 4)?;
            let vec = |at: u32| -> hsd_archive::desc::Result<hsd_types::Vec3> {
                Ok(hsd_types::Vec3::new(
                    r.f32(at)?,
                    r.f32(at + 4)?,
                    r.f32(at + 8)?,
                ))
            };
            (0..r.u32(hurtbones)?)
                .map(|i| {
                    let at = descs + i * 0x20;
                    Ok(ItemHurtbox {
                        bone: r.u32(at)?,
                        offsets: [vec(at + 4)?, vec(at + 0x10)?],
                        radius: r.f32(at + 0x1C)?,
                    })
                })
                .collect::<hsd_archive::desc::Result<_>>()?
        };
        assert!(hurtboxes.len() <= 2, "it_8027163C: item hit num over!");
        Ok(Self {
            hurtboxes,
            animation_ends: visual.states.iter().map(animation_end).collect(),
            visual,
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
            hitlag: r.u8(common + 1)? & 0x08 != 0,
            camera_kind: (r.u8(common + 1)? >> 1) & 3,
            fall_acceleration: r.f32(common + 0x10)?,
            fall_speed_limit: r.f32(common + 0x14)?,
            launch_vertical_velocity: r.f32(common + 0x18)?,
            rotation_axis: r.u8(common + 1)? >> 6,
            explosion_lifetime: 0.0,
            spin_rate: r.f32(common + 0xC)?,
            fall_spin_degrees: 0.0,
            release_box_scale: 0.0,
            heavy: r.u8(common)? & 0x80 != 0,
            use_kind: (r.u8(common)? >> 3) & 0xF,
            hand_hold_kind: r.u8(common)? & 7,
            throw_speed_multiplier: r.f32(common + 4)?,
            bounce_scale: r.f32(common + 0x58)?,
            bounce_sound: r.u32(common + 0x80)?,
            destroy_effect: u16::try_from(r.s32(common + 0x64)?).ok(),
            event_destroy_effect: u16::try_from(r.s32(common + 0x68)?).ok(),
            grab_offset: hsd_types::Vec2::new(r.f32(common + 0x30)?, r.f32(common + 0x34)?),
            grab_range: hsd_types::Vec2::new(r.f32(common + 0x38)?, r.f32(common + 0x3C)?),
            attachment_translation,
            bone_motion: None,
            pose: None,
            particle_tracks: None,
            special_pointees: Vec::new(),
        })
    }

    /// Ground_801C0800 -> it_8026B40C: a stage's `itemdata` table pairs an
    /// item kind with its Article (it_804A0F60). `pointer_attributes` lists
    /// the special attribute words that hold a pointer to an integer; their
    /// integers become [`Self::special_pointees`] in that order. `None`
    /// when the stage carries no Article for `kind`.
    pub fn from_stage_item(
        archive: &Archive,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
        pointer_attributes: &[u32],
    ) -> hsd_archive::desc::Result<Option<Self>> {
        let Some(table) = archive.public("itemdata") else {
            return Ok(None);
        };
        let r = archive.reader();
        for index in 0.. {
            let entry = r.u32(table + index * 4)?;
            if entry == 0 {
                return Ok(None);
            }
            if r.u32(entry)? != kind as u32 {
                continue;
            }
            let article = r.u32(entry + 4)?;
            let mut assets =
                Self::from_article(archive, article, article_states, special_attributes)?;
            let special = r.u32(article + 4)?;
            for &word in pointer_attributes {
                let pointee = r.u32(special + word * 4)?;
                assets.special_pointees.push(r.u32(pointee)? as i32);
            }
            return Ok(Some(assets));
        }
        unreachable!()
    }

    /// Samples every article state's joint animation for its DPtcl keys.
    pub fn read_particle_tracks(
        &mut self,
        archive: &Archive,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        self.particle_tracks = Some(crate::particle_track::ParticleTracks::read(
            archive,
            &self.visual,
        )?);
        Ok(())
    }

    /// Samples dynamic bone `bone`'s animation in every article state for
    /// itUpdateVelocityFromBone.
    /// Sample [`Self::pose`] from the article's model and joint animations.
    pub fn read_pose(
        &mut self,
        archive: &Archive,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        self.pose = Some(crate::pose::ItemPose::read(archive, &self.visual)?);
        Ok(())
    }
    pub fn read_bone_motion(
        &mut self,
        archive: &Archive,
        bone: usize,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        self.bone_motion = Some(crate::bone_motion::BoneMotion::read(
            archive,
            &self.visual,
            bone,
        )?);
        Ok(())
    }
}

/// it_80272CC0 (80272CC0) on a freshly loaded model: with a bone table
/// (ItemModelDesc x4 != 0) the index counts joints depth first, as the
/// table was filled; without one it walks first children from the root.
fn attachment_translation(
    model: &hsd_archive::desc::JObjDesc,
    bone_count: u32,
    index: usize,
) -> hsd_types::Vec3 {
    fn depth_first<'a>(
        joint: &'a hsd_archive::desc::JObjDesc,
        out: &mut Vec<&'a hsd_archive::desc::JObjDesc>,
    ) {
        out.push(joint);
        if let Some(child) = &joint.child {
            depth_first(child, out);
        }
        if let Some(next) = &joint.next {
            depth_first(next, out);
        }
    }
    let joint = if bone_count != 0 {
        let mut order = Vec::new();
        depth_first(model, &mut order);
        order[index]
    } else {
        let mut joint = model;
        for _ in 0..index {
            joint = joint.child.as_deref().expect("it_80272CC0: attachment child");
        }
        joint
    };
    hsd_types::Vec3::new(joint.position.x, joint.position.y, joint.position.z)
}

/// ItHurtBoneDesc (it/types.h:177): a capsule on bone `bone` (0 is the
/// model root) between two offsets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemHurtbox {
    pub bone: u32,
    pub offsets: [hsd_types::Vec3; 2],
    pub radius: f32,
}

/// The frame at which an article state's joint animation stops (lb_8000B09C
/// finds no running AObj), or None when it has none or loops.
fn animation_end(state: &hsd_archive::desc::item_visual::ItemVisualState) -> Option<f32> {
    let mut end: Option<f32> = None;
    let mut stack: Vec<&hsd_archive::desc::AnimJoint> = state.joint.iter().collect();
    while let Some(joint) = stack.pop() {
        if let Some(aobj) = &joint.aobjdesc {
            if aobj.flags & hsd_anim::aobj::AOBJ_LOOP != 0 {
                return None;
            }
            end = Some(end.map_or(aobj.end_frame, |e| e.max(aobj.end_frame)));
        }
        stack.extend(joint.child.as_deref());
        stack.extend(joint.next.as_deref());
    }
    end
}

/// itanimlist.c uses the shared ten control commands and its own payload table.
type DecodedScript = (Vec<Command>, Vec<Option<ItemHitFlags>>);
/// Decode a state's script and every subroutine it calls or jumps to into
/// one list; Call and Goto targets and continuations are indices into it
/// (Command_05/06/07). A target inside an already decoded block resolves to
/// that command.
fn read_script(archive: &Archive, offset: u32) -> Result<DecodedScript> {
    if offset == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut script = (Vec::new(), Vec::new());
    // Each decoded command's archive offset, in list order.
    let mut offsets: Vec<u32> = Vec::new();
    let mut jumps: Vec<(usize, u32)> = Vec::new();
    let mut pending = vec![offset];
    while let Some(block) = pending.pop() {
        if offsets.contains(&block) {
            continue;
        }
        read_block(archive, block, &mut script, &mut offsets, &mut jumps)?;
        pending.extend(jumps.iter().map(|&(_, target)| target));
    }
    for (index, target) in jumps {
        let start = offsets.iter().position(|&o| o == target).unwrap();
        match &mut script.0[index] {
            Command::Call { target, .. } | Command::Goto(target) => *target = start,
            _ => unreachable!("jump fix-up on a non-jump command"),
        }
    }
    Ok(script)
}

/// One block up to its End (Command_00), Return (Command_06) or Goto
/// (Command_07).
fn read_block(
    archive: &Archive,
    mut offset: u32,
    (commands, flags): &mut DecodedScript,
    offsets: &mut Vec<u32>,
    jumps: &mut Vec<(usize, u32)>,
) -> Result<()> {
    let r = archive.reader();
    loop {
        let word = r.u32(offset)?;
        let op = word >> 26;
        offsets.push(offset);
        let mut hit_flags = None;
        let command = match op {
            0..=4 | 6 | 8 => {
                melee_cmd::decode::decode(&[word], None, 0).expect("shared item command")
            }
            // Command_05: the target word follows; resolved once decoded.
            5 => {
                jumps.push((commands.len(), r.u32(offset + 4)?));
                offset += 4;
                Command::Call {
                    target: usize::MAX,
                    continuation: commands.len() + 1,
                }
            }
            // Command_07: a jump to the address in the next word.
            7 => {
                jumps.push((commands.len(), r.u32(offset + 4)?));
                offset += 4;
                Command::Goto(usize::MAX)
            }
            // it_80278F2C: five words, an effect at a joint with a random spread.
            10 => {
                let w = [
                    word,
                    r.u32(offset + 4)?,
                    r.u32(offset + 8)?,
                    r.u32(offset + 12)?,
                    r.u32(offset + 16)?,
                ];
                offset += 16;
                Command::Graphics(item_graphics(w))
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
            // it_80279888 -> it_80273598: rumble on the owner's controller.
            21 => Command::Rumble {
                all_players: false,
                id: ((word >> 16) & 0x3FF) as u16,
                duration: word as u16,
            },
            // it_8027990C -> ftLib_80086DC4: rumble on every fighter's controller.
            23 => Command::Rumble {
                all_players: true,
                id: ((word >> 16) & 0x3FF) as u16,
                duration: word as u16,
            },
            _ => unimplemented!("itanimlist.c item script opcode {op}"),
        };
        commands.push(command);
        flags.push(hit_flags);
        offset += 4;
        if op == 0 || op == 6 || op == 7 {
            return Ok(());
        }
    }
}

/// it_80278F2C (80278F2C): joint (low ten bits), effect id and parameter,
/// then offset and random range in 1/256 units.
fn item_graphics(w: [u32; 5]) -> melee_types::combat::GraphicsCommand {
    const SCALE: f32 = 0.003906;
    let half = |word: u32, high: bool| -> f32 {
        SCALE
            * (if high {
                (word >> 16) as i16
            } else {
                word as i16
            }) as f32
    };
    melee_types::combat::GraphicsCommand {
        bone: (w[0] & 0x3FF) as usize,
        common_bone: false,
        item_bone: false,
        destroy_on_state_change: false,
        id: (w[1] >> 16) as u16,
        parameter: (w[1] & 0xFFFF) as f32,
        offset: hsd_types::Vec3::new(half(w[2], true), half(w[2], false), half(w[3], true)),
        range: hsd_types::Vec3::new(half(w[3], false), half(w[4], true), half(w[4], false)),
        issued_facing: None,
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
    pub defense_interaction: bool,
    pub damage_without_hitlag: bool,
    pub absorbable: bool,
    pub shieldable: bool,
    pub shield_bounce: bool,
    pub hits_hurtboxes: bool,
    pub grabbable_hurtboxes_only: bool,
    /// x42_b7: the hitbox can land on other items (it_802706D0).
    pub hits_items: bool,
    pub sound_kind: u8,
    pub auxiliary: u16,
}
impl ItemHitFlags {
    fn decode(last: u32, extra: u32) -> Self {
        Self {
            rehit_rate: (extra >> 24) as u8,
            reflectable: extra & (1 << 20) != 0,
            defense_interaction: extra & (1 << 15) != 0,
            damage_without_hitlag: extra & (1 << 22) != 0,
            absorbable: extra & (1 << 17) != 0,
            shieldable: extra & (1 << 18) != 0,
            shield_bounce: extra & (1 << 16) != 0,
            hits_hurtboxes: extra & (1 << 14) != 0,
            grabbable_hurtboxes_only: extra & (1 << 13) != 0,
            hits_items: extra & (1 << 12) != 0,
            sound_kind: ((last >> 2) & 15) as u8,
            auxiliary: (extra >> 8) as u16,
        }
    }
}
