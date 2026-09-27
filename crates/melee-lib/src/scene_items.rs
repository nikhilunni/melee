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

melee_it::item_kinds! {
    pub enum SceneItems {
        FoxLaser: it_foxlaser::FoxLaser,
        FoxIllusion: it_foxillusion::FoxIllusion,
        FalcoPhantasm: it_foxillusion::FalcoPhantasm,
        FalcoLaser: it_foxlaser::FalcoLaser,
        FoxBlaster: it_foxlaser::FoxBlaster,
        FalcoBlaster: it_foxlaser::FalcoBlaster,
        BombHei: it_bombhei::BombHei,
    }
}

/// The grabbable items (Item_IsGrabbable: xDC8 x15 and a pickup callback)
/// in list order, as ftpickupitem_800942A0 walks HSD_GObj_Entities->items.
pub fn pickup_candidates<'a>(
    items: &'a ItemPool,
    resources: &'a Resources,
) -> impl Iterator<Item = melee_ft::fighter::item_pickup::PickupCandidate> + 'a {
    items
        .iter()
        .filter(|item| {
            !item.destroyed
                && item.grabbable
                && (SceneItems::logic(item.kind).pickup_possible)(item)
        })
        .map(|item| {
            let assets = resources.get(item.kind);
            melee_ft::fighter::item_pickup::PickupCandidate {
                item: item.id,
                // it_8026B344 (retail 8026B354: fmadds).
                position: hsd_types::Vec2::new(
                    gekko_math::fma::fmadds(item.facing, item.grab_offset.x, item.position.x),
                    item.position.y + item.grab_offset.y,
                ),
                range: item.grab_range,
                heavy: assets.heavy,
                // it_8026B4F0: food, hearts, tomatoes, coins, eggs, apples.
                consumable: false,
                use_kind: assets.use_kind,
                hand_hold_kind: assets.hand_hold_kind,
            }
        })
}

pub struct Resources {
    pub common: ItemCommonData,
    kinds: Vec<(ItemKind, ItemAssets)>,
    visual_archives: Vec<(ItemKind, std::sync::Arc<Archive>)>,
}
impl Resources {
    pub fn load(
        read: &impl Fn(&str) -> Result<Vec<u8>>,
        characters: &[crate::assets::CharacterArchive],
    ) -> Result<Self> {
        let archive = |file| -> Result<Archive> { Ok(Archive::parse(&read(file)?)?) };
        let common_archive = archive("ItCo.dat")?;
        let public = common_archive
            .public("itPublicData")
            .context("itPublicData")?;
        let common = ItemCommonData::read(&common_archive, public)?;
        let mut kinds = vec![(
            ItemKind::BombHei,
            ItemAssets::from_common(
                &common_archive,
                public,
                ItemKind::BombHei,
                &it_bombhei::ARTICLE_STATES,
                it_bombhei::SPECIAL_ATTRIBUTES,
            )?,
        )];
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
            // Article visuals share the fighter archive, including both afterimage copies.
            visual_archives.push((ghost, std::sync::Arc::clone(&a)));
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

/// Fighter-owned context captured when its item request is dispatched.
pub struct RequestOwner<'a> {
    /// The requesting fighter; stage requests (the Bob-omb rain) have none.
    pub slot: Option<u8>,
    pub held_item: Option<&'a melee_it::ItemOwner>,
    pub stale_multiplier: f32,
}

pub fn request(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    request: ItemRequest,
    owner: RequestOwner<'_>,
) {
    let (spawn, ray, held_owner) = match request {
        ItemRequest::Spawn(spawn) => (spawn, None, None),
        ItemRequest::SpawnHeld(spawn) => (spawn, None, owner.held_item),
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
        ItemRequest::PickUp { item, part } => {
            // Item_8026AB54: attach, then the kind's pickup callback.
            let lifetime = pool.common().lifetime;
            let held = pool.get_mut(item).expect("picked-up item");
            let assets = resources.get(held.kind);
            held.attach_to_holder(
                owner.slot.expect("pickup by a fighter"),
                part,
                assets,
                lifetime,
            );
            (SceneItems::logic(held.kind).picked_up)(
                held,
                &mut ItemAnimationContext {
                    owner: owner.held_item,
                    holder: None,
                    map,
                    assets,
                },
            );
            return;
        }
    };
    let assets = resources.get(spawn.kind);
    if let Some(id) = pool.spawn_with_stale::<SceneItems>(spawn, assets, owner.stale_multiplier) {
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
                &mut ItemAnimationContext {
                    owner: Some(owner),
                    holder: None,
                    map,
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

/// ftLib_800864A8 (800864A8) with no excluded fighter: face toward where more
/// fighters stand, by the sign of each camera bone's x relative to
/// `position`; a tie is a coin flip (HSD_Randi(2)).
pub fn facing_toward_fighters(
    position: hsd_types::Vec3,
    fighters: &mut [crate::scene_fighter::SceneFighter],
    rng: &mut gekko_math::HsdRng,
) -> f32 {
    let mut balance = 0;
    for fighter in fighters.iter_mut() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            // x221F_b3: sleeping fighters do not count.
            if !f.status.disabled {
                let x = f.camera_bone_position().x - position.x;
                balance += if x > 0.0 {
                    1
                } else if x < 0.0 {
                    -1
                } else {
                    0
                };
            }
        });
    }
    if balance == 0 {
        balance = if rng.randi(2) != 0 { 1 } else { -1 };
    }
    if balance < 0 {
        -1.0
    } else {
        1.0
    }
}

/// it_8026BE84 (BobOmbRain kind 6) -> it_8027D670: a lit Bob-omb at `position`.
pub fn spawn_rain_bomb(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    position: hsd_types::Vec3,
    facing: f32,
) {
    let before = pool.len();
    request(
        pool,
        resources,
        map,
        world,
        objects,
        ItemRequest::Spawn(it_bombhei::rain_spawn(position, facing)),
        RequestOwner {
            slot: None,
            held_item: None,
            stale_multiplier: 1.0,
        },
    );
    if pool.len() > before {
        let lifetime = pool.common().lifetime;
        let item = pool.iter_mut().last().expect("spawned Bob-omb");
        it_bombhei::light(item, resources.get(ItemKind::BombHei), lifetime);
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
