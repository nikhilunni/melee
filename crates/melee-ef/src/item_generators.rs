//! Particle generators an item carries on its own JObj (efSync_Spawn with
//! the item's model, efLib_CreateGenerator_Attach_AddAppSRT).
use super::*;

// Item JObj identities occupy a range beyond the fighter bones.
const FIRST_ITEM_JOINT: usize = 1 << 25;

fn item_joint(item: u32) -> usize {
    FIRST_ITEM_JOINT + item as usize
}

/// efSync_Spawn rows whose generators follow the item's root JObj.
fn item_generators(id: u16) -> Result<&'static [u32]> {
    Ok(match id {
        // efsync.c:436-441: Toad's spores, the second only after the first.
        0x4D3 => &[0x172, 0x173],
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
        for &kind in item_generators(id)? {
            let mut spawn = SpawnRequest::new(0, kind, 0);
            spawn.joint = Some((joint, matrix));
            self.events.spawn(&spawn, false, false);
            let Some(generator) = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)?
            else {
                break;
            };
            self.events.flags(joint, 0x600, 0x800);
            let generator = particles.generator_mut(generator).unwrap();
            generator.flags = (generator.flags & !0x600) | 0x800;
            if !self.item_joints.contains(&item) {
                self.item_joints.push(item);
            }
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
    pub fn expire_item_joint(&mut self, item: u32, particles: &mut ParticleSystem) {
        if !self.item_joints.contains(&item) {
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
