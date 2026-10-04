//! Particle generators an item carries on its own JObj (efSync_Spawn with
//! the item's model, efLib_CreateGenerator_Attach_AddAppSRT).
use super::*;

// Item JObj identities occupy a range beyond the fighter bones.
const FIRST_ITEM_JOINT: usize = 1 << 25;

fn item_joint(item: u32) -> usize {
    FIRST_ITEM_JOINT + item as usize
}

// An item's model bones occupy their own range, 32 per item.
const FIRST_ITEM_BONE_JOINT: usize = 1 << 26;
/// efLib_SpawnParticleEffect's switch (eflib.c:864-980): the common-bank
/// generator ids that do not take the default hsd_8039EFAC(0, ...) path.
const COMMON_SPECIAL_PARTICLES: [u32; 26] = [
    0x2D, 0x2E, 0x31, 0x127, 0x2, 0x6, 0xA3, 0xA7, 0xAA, 0xF2, 0x12A, 0x132, 0x133, 0x16D, 0x16E,
    0x16F, 0x170, 0xD4, 0x243, 0xE3, 0xFC, 0xFF, 0xF7, 0x4A38, 0x4A39, 0x4A3A,
];
const ITEM_BONE_STRIDE: usize = 32;

fn item_bone_joint(item: u32, bone: usize) -> usize {
    assert!(bone < ITEM_BONE_STRIDE);
    FIRST_ITEM_BONE_JOINT + item as usize * ITEM_BONE_STRIDE + bone
}

/// efAlt/efSync rows that attach one generator to a joint (hsd_8039EFAC(0,
/// bank, generator, jobj)), and whether the row is
/// efLib_CreateGenerator_AppSRT_SetFacingDir (an AppSRT turned to the
/// facing passed with the request).
fn bone_generator(id: u16) -> Result<(u8, u32, bool)> {
    Ok(match id {
        // efalt.c:68-73: the sparkles along Mario's cape.
        0x47D => (1, 0x3F0, false),
        0x47E => (1, 0x3F1, false),
        // efsync.c:414-422: the Ice Climbers' ice block, made (0x4E9),
        // launched (0x4EA) and sliding (0x4EB).
        0x4E9 => (14, 0x36B0, false),
        0x4EA => (14, 0x36B1, false),
        0x4EB => (14, 0x36B6, true),
        _ => anyhow::bail!("efAsync kind 0 {id:#x} on an item bone"),
    })
}

/// An efSync_Spawn row whose generators follow the item's root JObj.
struct ItemGenerators {
    /// The particle bank (0 common, else a character's).
    bank: u8,
    kinds: &'static [u32],
    /// efLib_CreateGenerator_Attach_AddAppSRT; otherwise a bare
    /// hsd_8039EFAC on the JObj.
    add_appsrt: bool,
    /// efLib_CreateGenerator_Attach_Scale: the AppSRT also takes the item
    /// JObj's Y scale, uniformly.
    scaled: bool,
}
fn item_generators(id: u16) -> Result<ItemGenerators> {
    Ok(match id {
        // efsync.c:436-441: Toad's spores, the second only after the first.
        0x4D3 => ItemGenerators {
            bank: 0,
            kinds: &[0x172, 0x173],
            add_appsrt: true,
            scaled: false,
        },
        // efsync.c:104-106: the Thunder Jolt ball's trail,
        // hsd_8039EFAC(0, 7, 0x1B58, jobj).
        0x4BD => ItemGenerators {
            bank: 7,
            kinds: &[0x1B58],
            add_appsrt: false,
            scaled: false,
        },
        // efsync.c:485-493: efLib_CreateGenerator_Attach_Scale(0x6E / 0x1C8
        // / 0x166): Din's Fire flying, bursting, and its explosion.
        0x4F8..=0x4FA => ItemGenerators {
            bank: 0,
            kinds: match id {
                0x4F8 => &[0x6E],
                0x4F9 => &[0x1C8],
                _ => &[0x166],
            },
            add_appsrt: true,
            scaled: true,
        },
        // efsync.c:357-368: efLib_CreateGenerator_Attach_Scale(0x2EE5 ..
        // 0x2EE8): the four flames of Bowser's Fire Breath.
        0x4DB..=0x4DE => ItemGenerators {
            bank: 12,
            kinds: match id {
                0x4DB => &[0x2EE5],
                0x4DC => &[0x2EE6],
                0x4DD => &[0x2EE7],
                _ => &[0x2EE8],
            },
            add_appsrt: true,
            scaled: true,
        },
        // efalt.c:92-97: Samus's missile trails, hsd_8039EFAC(0, 2, 0x7DB /
        // 0x7DE, jobj) on the missile model's grandchild.
        // efalt.c:77-85: the charge shot's glow in hand (0x7D4, on the
        // model's grandchild) and a full shot's two sparkles (0x7D2, 0x7D3).
        0x47F => ItemGenerators {
            bank: 2,
            kinds: &[0x7D4],
            add_appsrt: false,
            scaled: false,
        },
        0x480 => ItemGenerators {
            bank: 2,
            kinds: &[0x7D2],
            add_appsrt: false,
            scaled: false,
        },
        0x481 => ItemGenerators {
            bank: 2,
            kinds: &[0x7D3],
            add_appsrt: false,
            scaled: false,
        },
        0x484 => ItemGenerators {
            bank: 2,
            kinds: &[0x7DB],
            add_appsrt: false,
            scaled: false,
        },
        0x485 => ItemGenerators {
            bank: 2,
            kinds: &[0x7DE],
            add_appsrt: false,
            scaled: false,
        },
        _ => anyhow::bail!("efSync_Spawn {id:#x} on an item JObj"),
    })
}

impl Effects {
    /// efSync_Spawn(`id`, item, jobj): each generator is
    /// efLib_CreateGenerator_Attach_AddAppSRT (eflib.c:770-789) on the item's
    /// root, with PSAPPSRT_UNK_B9/B10 cleared and B11 set. `scale` is the
    /// JObj's Y scale, for the Attach_Scale rows.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item_generators<T: InverseTrig>(
        &mut self,
        id: u16,
        item: u32,
        matrix: Mtx,
        scale: f32,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let joint = item_joint(item);
        let row = item_generators(id)?;
        let bank = if row.bank == 0 {
            bank
        } else {
            resources::character_bank(&self.character_banks, row.bank.into())?
        };
        for &kind in row.kinds {
            let mut spawn = SpawnRequest::new(row.bank, kind, 0);
            spawn.joint = Some((joint, matrix));
            if row.scaled {
                spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                    scale: Vec3::new(scale, scale, scale),
                    ..Default::default()
                });
            }
            self.events.spawn(&spawn, false, false);
            let Some(generator) =
                spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?
            else {
                break;
            };
            if row.add_appsrt {
                self.events.flags(joint, 0x600, 0x800);
                let generator = particles.generator_mut(generator).unwrap();
                generator.flags = (generator.flags & !0x600) | 0x800;
            }
            if !self.item_joints.contains(&item) {
                self.item_joints.push(item);
            }
        }
        Ok(())
    }

    /// efLib_Cb_DPtcl from an item's joint animation: efLib_SpawnParticleEffect
    /// (eflib.c:857-990) with a generator id outside its special cases takes
    /// hsd_8039EFAC(0, bank, id, jobj), a generator that follows the item's
    /// root with no AppSRT (Mario's fireball trail, 1002; the PK Fire
    /// pillar's flames, 291..294 of the common bank, `common`).
    #[allow(clippy::too_many_arguments)] // The item, its joint and the particle state stay separate.
    pub fn spawn_item_particle<T: InverseTrig>(
        &mut self,
        bank: u8,
        id: u32,
        item: u32,
        matrix: Mtx,
        common: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        anyhow::ensure!(
            (resources::is_character_bank(bank) && id / 1000 == u32::from(bank))
                || (bank == 0 && id < 1000 && !COMMON_SPECIAL_PARTICLES.contains(&id)),
            "item joint particle {bank}/{id} outside the default hsd_8039EFAC path"
        );
        let joint = item_joint(item);
        let mut spawn = SpawnRequest::new(bank, id, 0);
        spawn.joint = Some((joint, matrix));
        self.events.spawn(&spawn, false, false);
        let particle_bank = if bank == 0 {
            common
        } else {
            resources::character_bank(&self.character_banks, i32::from(bank))?
        };
        if spawn_particle::<T>(particles, particle_bank, spawn, rng, &mut self.draws)?.is_some()
            && !self.item_joints.contains(&item)
        {
            self.item_joints.push(item);
        }
        Ok(())
    }

    /// efAsync kind 0 on an item's model bone, dispatched at the item's
    /// queue flush: efAlt's hsd_8039EFAC(0, bank, generator, bone), a
    /// generator that follows the bone with no AppSRT.
    pub fn spawn_item_bone_generator<T: InverseTrig>(
        &mut self,
        id: u16,
        item: u32,
        bone: usize,
        matrix: Mtx,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        self.spawn_item_bone_generator_facing::<T>(id, item, bone, matrix, None, particles, rng)
    }

    /// [`Self::spawn_item_bone_generator`] with the facing an
    /// EF_SPAWN_ATTACH_PARAM request carries; an AppSRT row
    /// (efLib_CreateGenerator_AppSRT_SetFacingDir, eflib.c:822-835) turns
    /// its AppSRT a quarter turn toward it.
    #[allow(clippy::too_many_arguments)] // The item, its bone and the particle state stay separate.
    pub fn spawn_item_bone_generator_facing<T: InverseTrig>(
        &mut self,
        id: u16,
        item: u32,
        bone: usize,
        matrix: Mtx,
        facing: Option<f32>,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let (bank, generator, facing_appsrt) = bone_generator(id)?;
        let joint = item_bone_joint(item, bone);
        let mut spawn = SpawnRequest::new(bank, generator, 0);
        spawn.joint = Some((joint, matrix));
        if facing_appsrt {
            let facing = facing.expect("an AppSRT facing row without a facing");
            spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
                rotation: Vec3::new(
                    0.0,
                    if facing < 0.0 {
                        -std::f32::consts::FRAC_PI_2
                    } else {
                        std::f32::consts::FRAC_PI_2
                    },
                    0.0,
                ),
                ..Default::default()
            });
        }
        self.events.spawn(&spawn, false, false);
        let particle_bank = resources::character_bank(&self.character_banks, i32::from(bank))?;
        let spawned = spawn_particle::<T>(particles, particle_bank, spawn, rng, &mut self.draws)?;
        if let Some(generator) = spawned {
            if facing_appsrt {
                // eflib_create_generator_add_appsrt: PSAPPSRT_UNK_B9/B10
                // cleared, B11 set.
                self.events.flags(joint, 0x600, 0x800);
                let generator = particles.generator_mut(generator).unwrap();
                generator.flags = (generator.flags & !0x600) | 0x800;
            }
            if !self.item_bones.contains(&(item, bone as u8)) {
                self.item_bones.push((item, bone as u8));
            }
        }
        Ok(())
    }

    /// The item bones generators follow, as (item, bone).
    pub fn followed_item_bones(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.item_bones
            .iter()
            .map(|&(item, bone)| (item, usize::from(bone)))
    }

    /// A followed item bone's world matrix for its generators.
    pub fn update_item_bone(
        &mut self,
        item: u32,
        bone: usize,
        matrix: Mtx,
        particles: &mut ParticleSystem,
    ) {
        let joint = item_bone_joint(item, bone);
        if !particles
            .generators
            .iter()
            .any(|g| g.attachment_id == Some(joint))
        {
            self.forget_item_bone(item, bone);
            return;
        }
        self.events.update_joint(joint, matrix);
        particles.update_joint(joint, matrix);
    }

    pub fn forget_item_bone(&mut self, item: u32, bone: usize) {
        let index = self
            .item_bones
            .iter()
            .position(|&entry| entry == (item, bone as u8));
        if let Some(index) = index {
            self.item_bones.remove(index);
        }
    }

    /// Whether generators still follow `item`'s JObj.
    pub fn follows_item(&self, item: u32) -> bool {
        self.item_joints.contains(&item)
    }

    /// The item JObj's world matrix for its generators' AppSRT.
    pub fn update_item_joint(&mut self, item: u32, matrix: Mtx, particles: &mut ParticleSystem) {
        let joint = item_joint(item);
        if !particles
            .generators
            .iter()
            .any(|g| g.attachment_id == Some(joint))
        {
            self.forget_item(item);
            return;
        }
        self.events.update_joint(joint, matrix);
        particles.update_joint(joint, matrix);
    }

    /// Item_8026A8EC: the item's JObj goes, and with it the generators on it.
    /// efLib_DestroyAll (eflib.c:280-282) walks the item's JObj tree with
    /// hsd_8039D688 even when no generator follows it, which still parks the
    /// generator insertion cursor at the list's tail. A generator kept for
    /// its particles (hsd_8039D3AC) still reads the JObj, which lives until
    /// the frame's end, so the item stays followed until [`Self::forget_item`].
    pub fn expire_item_joint(&mut self, item: u32, particles: &mut ParticleSystem) {
        // The walk reaches the model's bones too. A kept generator goes on
        // reading its bone while the item lives (the Ice Climbers' ice
        // falling off an edge), so the bone stays followed until nothing
        // is attached or the item is gone.
        let mut bones = [0u8; 64];
        let mut count = 0;
        for &(i, bone) in self.item_bones.iter() {
            if i == item {
                bones[count] = bone;
                count += 1;
            }
        }
        for &bone in &bones[..count] {
            let joint = item_bone_joint(item, usize::from(bone));
            self.events.expire_joint(joint);
            particles.expire_joint(joint);
        }
        if !self.item_joints.contains(&item) {
            particles.walk_unowned_joint();
            return;
        }
        let joint = item_joint(item);
        self.events.expire_joint(joint);
        particles.expire_joint(joint);
    }

    /// The item's GObj is gone (end of frame): nothing reads its JObj.
    pub fn forget_item(&mut self, item: u32) {
        let index = self.item_joints.iter().position(|&i| i == item);
        if let Some(index) = index {
            self.item_joints.remove(index);
        }
    }
}
