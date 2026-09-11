//! Scene-owned item inventory and retail GObj scheduling; the engine knows no kinds.
use anyhow::{Context, Result};
use ft_fox_family::FoxFamily;
use hsd_archive::Archive;
use hsd_gobj::{GObjId, TaggedWorld as World};
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    ItemAnimationContext, ItemDispatch, ItemPool, ItemRequest,
};
use melee_types::{fixed::FixedVec, ItemKind};
use std::path::Path;

melee_it::item_kinds! {
    pub enum SceneItems {
        FoxLaser: it_foxlaser::FoxLaser,
        FoxIllusion: it_foxillusion::FoxIllusion,
        FalcoPhantasm: it_foxillusion::FalcoPhantasm,
        FalcoLaser: it_foxlaser::FalcoLaser,
        FoxBlaster: it_foxlaser::FoxBlaster,
        FalcoBlaster: it_foxlaser::FalcoBlaster,
    }
}

pub struct Resources {
    pub common: ItemCommonData,
    kinds: Vec<(ItemKind, ItemAssets)>,
    visual_archives: Vec<(ItemKind, std::sync::Arc<Archive>)>,
}
impl Resources {
    pub fn load(files: &Path, characters: &[crate::assets::CharacterArchive]) -> Result<Self> {
        let archive =
            |file| -> Result<Archive> { Ok(Archive::parse(&std::fs::read(files.join(file))?)?) };
        let common = archive("ItCo.dat")?;
        let common = ItemCommonData::read(
            &common,
            common.public("itPublicData").context("itPublicData")?,
        )?;
        let mut kinds = Vec::new();
        let mut visual_archives = Vec::new();
        for (file, symbol, laser, blaster, ghost, ghost_index) in [
            (
                "PlFx.dat",
                "ftDataFox",
                ItemKind::FoxLaser,
                ItemKind::FoxBlaster,
                ItemKind::FoxIllusion,
                ft_fox::init::Fox::GHOST_ARTICLE_INDEX,
            ),
            (
                "PlFc.dat",
                "ftDataFalco",
                ItemKind::FalcoLaser,
                ItemKind::FalcoBlaster,
                ItemKind::FalcoPhantasm,
                ft_falco::init::Falco::GHOST_ARTICLE_INDEX,
            ),
        ] {
            let a = match characters.iter().find(|c| c.descriptor.data_file == file) {
                Some(character) => std::sync::Arc::clone(&character.data),
                None => std::sync::Arc::new(archive(file)?),
            };
            let root = a.public(symbol).context("family fighter data")?;
            kinds.push((laser, ItemAssets::from_fighter(&a, root, 0, 2)?));
            // Rows 9 and 10 have animation -1, so the archive contains nine animations.
            kinds.push((blaster, ItemAssets::from_fighter(&a, root, 1, 9)?));
            kinds.push((ghost, ItemAssets::from_fighter(&a, root, ghost_index, 3)?));
            // Article visuals share the fighter archive; afterimages additionally
            // need captured historical fighter poses.
            visual_archives.push((blaster, std::sync::Arc::clone(&a)));
            visual_archives.push((laser, a));
        }
        Ok(Self {
            common,
            kinds,
            visual_archives,
        })
    }
    pub fn visual_models(&self) -> impl Iterator<Item = (ItemKind, &Archive, u32)> {
        self.visual_archives
            .iter()
            .map(|(kind, archive)| (*kind, &**archive, self.get(*kind).model))
    }
    pub fn get(&self, kind: ItemKind) -> &ItemAssets {
        &self
            .kinds
            .iter()
            .find(|(k, _)| *k == kind)
            .expect("registered item assets")
            .1
    }
}

pub const TAG_BASE: usize = 1 << 16;
pub fn tag(id: u32, phase: u8) -> usize {
    TAG_BASE + id as usize * 32 + usize::from(phase)
}
pub fn decode_tag(tag: usize) -> (u32, u8) {
    (
        ((tag - TAG_BASE) / 32) as u32,
        ((tag - TAG_BASE) % 32) as u8,
    )
}
pub type Objects = FixedVec<(u32, GObjId), { melee_it::ITEM_CAPACITY }>;

/// Provision GObj/proc slabs and their LIFO free lists before allocation counting.
pub fn prepare_scheduler(world: &mut World) {
    let mut objects = Vec::with_capacity(melee_it::ITEM_CAPACITY);
    for _ in 0..melee_it::ITEM_CAPACITY {
        let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
        for phase in melee_it::ITEM_PROCESS_LINKS {
            world.add_tagged_proc(object, phase, TAG_BASE);
        }
        objects.push(object);
    }
    for object in objects {
        world.destroy(object);
    }
}

pub fn request(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    request: ItemRequest,
    owner: Option<&melee_it::ItemOwner>,
) {
    let (spawn, ray, held_owner) = match request {
        ItemRequest::Spawn(spawn) => (spawn, None, None),
        ItemRequest::SpawnHeld(spawn) => (spawn, None, owner),
        ItemRequest::SpawnLaser {
            spawn,
            angle,
            speed,
            motion,
        } => (spawn, Some((angle, speed, motion)), None),
        ItemRequest::Control {
            owner,
            kind,
            control,
        } => {
            pool.control::<SceneItems>(owner, kind, control);
            return;
        }
    };
    let assets = resources.get(spawn.kind);
    if let Some(id) = pool.spawn::<SceneItems>(spawn, assets) {
        pool.get_mut(id)
            .unwrap()
            .initialize_collision(spawn, assets, map);
        if let Some((angle, speed, motion)) = ray {
            it_foxlaser::initialize_laser(pool.get_mut(id).unwrap(), assets, angle, speed, motion);
        }
        if let Some(owner) = held_owner {
            // Item_8026AB54 invokes the kind's pickup callback after attachment.
            (SceneItems::logic(spawn.kind).picked_up)(
                pool.get_mut(id).unwrap(),
                &ItemAnimationContext {
                    owner: Some(owner),
                    assets,
                },
            );
        }
        let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
        for phase in melee_it::ITEM_PROCESS_LINKS {
            world.add_tagged_proc(object, phase, tag(id, phase));
        }
        objects.push((id, object));
    }
}

pub fn cleanup(pool: &mut ItemPool, world: &mut World, objects: &mut Objects) {
    for item in pool.iter().filter(|item| item.destroyed) {
        let index = objects
            .iter()
            .position(|(id, _)| *id == item.id)
            .expect("item GObj");
        world.destroy(objects.remove(index).1);
    }
    pool.remove_destroyed::<SceneItems>();
}
