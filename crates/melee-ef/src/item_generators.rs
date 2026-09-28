//! Particle generators an item carries on its own JObj (efSync_Spawn with
//! the item's model, efLib_CreateGenerator_Attach_AddAppSRT).
use super::*;

// Item JObj identities occupy a range beyond the fighter bones.
const FIRST_ITEM_JOINT: usize = 1 << 25;

fn item_joint(item: u32) -> usize {
    FIRST_ITEM_JOINT + item as usize
}

/// An efSync_Spawn row whose generators follow the item's root JObj.
struct ItemGenerators {
    /// The particle bank (0 common, else a character's).
    bank: u8,
    kinds: &'static [u32],
    /// efLib_CreateGenerator_Attach_AddAppSRT; otherwise a bare
    /// hsd_8039EFAC on the JObj.
    add_appsrt: bool,
}
fn item_generators(id: u16) -> Result<ItemGenerators> {
    Ok(match id {
        // efsync.c:436-441: Toad's spores, the second only after the first.
        0x4D3 => ItemGenerators {
            bank: 0,
            kinds: &[0x172, 0x173],
            add_appsrt: true,
        },
        // efsync.c:104-106: the Thunder Jolt ball's trail,
        // hsd_8039EFAC(0, 7, 0x1B58, jobj).
        0x4BD => ItemGenerators {
            bank: 7,
            kinds: &[0x1B58],
            add_appsrt: false,
        },
        _ => anyhow::bail!("efSync_Spawn {id:#x} on an item JObj"),
    })
}

impl Effects {
    /// efSync_Spawn(`id`, item, jobj): each generator is
    /// efLib_CreateGenerator_Attach_AddAppSRT (eflib.c:770-789) on the item's
    /// root, with PSAPPSRT_UNK_B9/B10 cleared and B11 set.
    pub fn spawn_item_generators<T: InverseTrig>(
        &mut self,
        id: u16,
        item: u32,
        matrix: Mtx,
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
    /// root with no AppSRT (Mario's fireball trail, 1002).
    pub fn spawn_item_particle<T: InverseTrig>(
        &mut self,
        bank: u8,
        id: u32,
        item: u32,
        matrix: Mtx,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        anyhow::ensure!(
            resources::is_character_bank(bank) && id / 1000 == u32::from(bank),
            "item joint particle {bank}/{id} outside the default hsd_8039EFAC path"
        );
        let joint = item_joint(item);
        let mut spawn = SpawnRequest::new(bank, id, 0);
        spawn.joint = Some((joint, matrix));
        self.events.spawn(&spawn, false, false);
        let particle_bank = resources::character_bank(&self.character_banks, i32::from(bank))?;
        if spawn_particle::<T>(particles, particle_bank, spawn, rng, &mut self.draws)?.is_some()
            && !self.item_joints.contains(&item)
        {
            self.item_joints.push(item);
        }
        Ok(())
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
    /// generator insertion cursor at the list's tail.
    pub fn expire_item_joint(&mut self, item: u32, particles: &mut ParticleSystem) {
        if !self.item_joints.contains(&item) {
            particles.walk_unowned_joint();
            return;
        }
        let joint = item_joint(item);
        self.events.expire_joint(joint);
        particles.expire_joint(joint);
        self.forget_item(item);
    }

    fn forget_item(&mut self, item: u32) {
        let index = self.item_joints.iter().position(|&i| i == item);
        if let Some(index) = index {
            self.item_joints.remove(index);
        }
    }
}
