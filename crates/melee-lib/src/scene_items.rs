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
        YoshiEggThrow: it_yoshieggthrow::YoshiEggThrow,
        YoshiStar: it_yoshistar::YoshiStar,
        PeachExplode: it_peach::PeachExplode,
        PeachTurnip: it_peach::PeachTurnip,
        PeachParasol: it_peach::PeachParasol,
        PeachToad: it_peach::PeachToad,
        PeachToadSpore: it_peach::PeachToadSpore,
        Heiho: it_heiho::Heiho,
        PikachuTJoltGround: it_pikachu::ThunderJoltBall,
        PikachuTJoltAir: it_pikachu::ThunderJoltCrawler,
        PikachuThunder: it_pikachu::ThunderBolt,
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
                kind: item.kind,
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
                damage_multiplier: assets.collision_damage_multiplier,
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
        stage: &Archive,
        launch: melee_it::hurt::ItemLaunch,
    ) -> Result<Self> {
        let archive = |file| -> Result<Archive> { Ok(Archive::parse(&read(file)?)?) };
        let common_archive = archive("ItCo.dat")?;
        let public = common_archive
            .public("itPublicData")
            .context("itPublicData")?;
        let mut common = ItemCommonData::read(&common_archive, public)?;
        common.launch = launch;
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
        // ftYs_Init_OnLoad registers the Egg Throw egg as ftData.x48_items[0].
        let yoshi = match characters.iter().find(|c| c.descriptor.data_file == "PlYs.dat") {
            Some(character) => std::sync::Arc::clone(&character.data),
            None => std::sync::Arc::new(archive("PlYs.dat")?),
        };
        let root = yoshi.public("ftDataYoshi").context("Yoshi fighter data")?;
        let mut egg = ItemAssets::from_fighter_states(
            &yoshi,
            root,
            it_yoshieggthrow::ARTICLE_INDEX,
            &it_yoshieggthrow::ARTICLE_STATES,
            it_yoshieggthrow::SPECIAL_ATTRIBUTES,
        )?;
        egg.read_common_release(&common_archive, public)?;
        kinds.push((ItemKind::YoshiEggThrow, egg));
        // ftData.x48_items[1]: the Yoshi Bomb's star, one state.
        kinds.push((ItemKind::YoshiStar, ItemAssets::from_fighter(&yoshi, root, 1, 1)?));
        visual_archives.push((ItemKind::YoshiStar, std::sync::Arc::clone(&yoshi)));
        visual_archives.push((ItemKind::YoshiEggThrow, yoshi));
        // ftPe_Init_OnLoad: ftData.x48_items[0] is Peach Bomber's blast,
        // [1] the turnip, [2..=4] the parasol, Toad and Toad's spores.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlPe.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataPeach").context("Peach fighter data")?;
            for (kind, index, states, attributes) in [
                (
                    ItemKind::PeachExplode,
                    0,
                    &it_peach::explode::ARTICLE_STATES[..],
                    0,
                ),
                (
                    ItemKind::PeachParasol,
                    2,
                    &it_peach::parasol::ARTICLE_STATES[..],
                    0,
                ),
                (
                    ItemKind::PeachToad,
                    3,
                    &it_peach::toad::ARTICLE_STATES[..],
                    0,
                ),
                (
                    ItemKind::PeachToadSpore,
                    4,
                    &it_peach::spore::ARTICLE_STATES[..],
                    it_peach::spore::SPECIAL_ATTRIBUTES,
                ),
            ] {
                kinds.push((
                    kind,
                    ItemAssets::from_fighter_states(&a, root, index, states, attributes)?,
                ));
                visual_archives.push((kind, std::sync::Arc::clone(&a)));
            }
            // The turnip leaves the hand like a common item (it_80275BC8,
            // Item_ApplyFallingPhysics).
            let mut turnip = ItemAssets::from_fighter_states(
                &a,
                root,
                1,
                &it_peach::turnip::ARTICLE_STATES,
                it_peach::turnip::SPECIAL_ATTRIBUTES,
            )?;
            turnip.read_common_release(&common_archive, public)?;
            kinds.push((ItemKind::PeachTurnip, turnip));
            visual_archives.push((ItemKind::PeachTurnip, std::sync::Arc::clone(&a)));
        }
        // ftPk_Init_OnLoad: ftData.x48_items[0] is Thunder's bolt, [1] the
        // Thunder Jolt ball, [2] the crawler it rides (whose joint 6 the
        // ball reads).
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlPk.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataPikachu").context("Pikachu fighter data")?;
            let ball = ItemAssets::from_fighter_states(
                &a,
                root,
                it_pikachu::BALL_ARTICLE_INDEX,
                &it_pikachu::jolt::BALL_ARTICLE_STATES,
                4,
            )?;
            kinds.push((ItemKind::PikachuTJoltGround, ball));
            visual_archives.push((ItemKind::PikachuTJoltGround, std::sync::Arc::clone(&a)));
            let mut crawler = ItemAssets::from_fighter_states(
                &a,
                root,
                it_pikachu::CRAWLER_ARTICLE_INDEX,
                &it_pikachu::jolt::CRAWLER_ARTICLE_STATES,
                0,
            )?;
            crawler
                .read_pose(&a)
                .map_err(|e| anyhow::anyhow!("Thunder Jolt crawler pose: {e}"))?;
            kinds.push((ItemKind::PikachuTJoltAir, crawler));
            visual_archives.push((ItemKind::PikachuTJoltAir, std::sync::Arc::clone(&a)));
            let bolt = ItemAssets::from_fighter_states(
                &a,
                root,
                it_pikachu::THUNDER_ARTICLE_INDEX,
                &it_pikachu::thunder::ARTICLE_STATES,
                3,
            )?;
            kinds.push((ItemKind::PikachuThunder, bolt));
            visual_archives.push((ItemKind::PikachuThunder, std::sync::Arc::clone(&a)));
        }
        // Ground_801C0800 -> it_8026B40C: Yoshi's Story's Shy Guy Article.
        if let Some(mut heiho) = ItemAssets::from_stage_item(
            stage,
            ItemKind::Heiho,
            &it_heiho::ARTICLE_STATES,
            it_heiho::SPECIAL_ATTRIBUTES,
            &it_heiho::POINTER_ATTRIBUTES,
        )? {
            heiho
                .read_bone_motion(stage, it_heiho::GAIT_BONE)
                .map_err(|e| anyhow::anyhow!("Shy Guy gait: {e}"))?;
            kinds.push((ItemKind::Heiho, heiho));
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

/// The article `owner` tracks, if any (its kind's owner_report).
pub fn owner_report(
    pool: &ItemPool,
    resources: &Resources,
    owner: u8,
) -> Option<melee_it::ArticleReport> {
    pool.iter()
        .filter(|item| item.owner == Some(owner) && !item.destroyed)
        .find_map(|item| (SceneItems::logic(item.kind).owner_report)(item, resources.get(item.kind)))
}

/// What `id`'s animation callback sees of its partner: the partner's own
/// link and, for a kind that reads one, the partner's joint in world space
/// (lb_8000B1CC on xBBC_dynamicBoneTable->bones[i], under the root the
/// partner's collision proc last placed).
pub fn partner_view(
    pool: &ItemPool,
    resources: &Resources,
    id: u32,
) -> Option<melee_it::PartnerView> {
    let item = pool.iter().find(|item| item.id == id)?;
    let partner = pool.iter().find(|p| Some(p.id) == item.partner)?;
    let bone_position = SceneItems::logic(item.kind).partner_bone.map(|bone| {
        let pose = resources
            .get(partner.kind)
            .pose
            .as_ref()
            .expect("partner joint without a sampled pose");
        pose.bone_position(
            partner.article_state,
            partner.pose_steps,
            bone,
            melee_it::pose::RootSrt {
                translate: partner.root_translation,
                rotate: partner.rotation,
                scale: partner.model_scale,
            },
        )
    });
    Some(melee_it::PartnerView {
        partner: partner.partner,
        bone_position,
    })
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
    /// The requesting proc runs past item link 11 (HSD_GObj_804D7838's
    /// s_link > 11): hitboxes a new item's script creates are placed at once
    /// (it_802790C0).
    pub after_hitbox_refresh: bool,
    pub stale_multiplier: f32,
}

#[allow(clippy::too_many_arguments)] // Item pool, scene objects and the shared RNG stay separate.
pub fn request(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    request: ItemRequest,
    owner: RequestOwner<'_>,
    rng: &mut gekko_math::HsdRng,
) -> Option<u32> {
    let owner_context = &owner;
    if let ItemRequest::SpawnChain {
        spawn,
        count,
        delay,
        velocity,
    } = request
    {
        // it_802B1DF8: each member is spawned, linked from the one before
        // and set up before the next spawns.
        let mut first = None;
        let mut previous: Option<u32> = None;
        for index in 0..count {
            let member = request_one_of_chain(
                pool,
                resources,
                map,
                world,
                objects,
                spawn,
                &owner,
                rng,
            );
            if let (Some(previous), Some(member)) = (previous, member) {
                pool.get_mut(previous).unwrap().partner = Some(member);
            }
            if let Some(member) = member {
                let item = pool.get_mut(member).unwrap();
                let receive = SceneItems::logic(item.kind).link_received;
                let message = melee_it::LinkMessage::Chain {
                    index,
                    delay: index * delay,
                    velocity,
                };
                item.destroyed |= receive(item, message, resources.get(spawn.kind));
            }
            first = first.or(member);
            previous = member;
        }
        return first;
    }
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
            pool.control::<SceneItems>(owner, kind, control, resources.get(kind));
            return None;
        }
        ItemRequest::PickUp { item, part } => {
            // Item_8026AB54: attach, then the kind's pickup callback.
            let lifetime = pool.common().lifetime;
            let half_life_scale = pool.common().half_life_scale;
            let held = pool.get_mut(item).expect("picked-up item");
            let assets = resources.get(held.kind);
            held.attach_to_holder(
                owner.slot.expect("pickup by a fighter"),
                part,
                assets,
                lifetime,
                half_life_scale,
            );
            (SceneItems::logic(held.kind).picked_up)(
                held,
                &mut ItemAnimationContext {
                    owner: owner.held_item,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                },
            );
            return None;
        }
        ItemRequest::Throw {
            item,
            position,
            hand,
            velocity,
            speed,
            center,
            attack,
        } => {
            // Item_8026AD20: it_802731E0's sound, xC44, it_80273748, the
            // kind's thrown callback, it_802741F4 (it_80273F34), then
            // it_802754D4.
            let thrown = pool.get_mut(item).expect("thrown item");
            let assets = resources.get(thrown.kind);
            thrown.throw_speed = speed;
            let position = thrown.throw_release_point(position, &hand, assets);
            thrown.leave_hand(velocity, position, assets);
            (SceneItems::logic(thrown.kind).thrown)(
                thrown,
                &mut ItemAnimationContext {
                    owner: owner.held_item,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                },
            );
            thrown.end_hold(center, attack, map, assets);
            // xDCE b0 and b2: the thrower's hits land on it, and so do its
            // kin's.
            thrown.hurt_by_owner = true;
            return None;
        }
        ItemRequest::Destroy { item } => {
            destroy_object(world, objects, item);
            pool.destroy::<SceneItems>(item);
            return None;
        }
        ItemRequest::SpawnInHand { spawn, .. } => (spawn, None, None),
        ItemRequest::SpawnChain { .. } => unreachable!("handled above"),
        ItemRequest::Launch {
            owner: slot,
            kind,
            launch,
        } => {
            assert_eq!(
                kind,
                ItemKind::YoshiEggThrow,
                "held article launch for {kind:?}"
            );
            let half_life_scale = pool.common().half_life_scale;
            let item = pool
                .iter_mut()
                .find(|item| item.owner == Some(slot) && item.kind == kind && item.held)
                .expect("launched article in its owner's hand");
            it_yoshieggthrow::launch(item, &launch, half_life_scale, map, resources.get(kind));
            return None;
        }
        ItemRequest::DropArticle {
            owner,
            kind,
            hold,
            center,
            attack,
        } => {
            // it_80273B50, hold kind 8: the negated local translation of the
            // article's attach joint, through the holding joint's matrix.
            let assets = resources.get(kind);
            let t = assets.attach_translation();
            let offset = hsd_types::Vec3::new(-t.x, -t.y, -t.z);
            let mut position = hsd_types::Vec3::ZERO;
            hsd_anim::mtx::mtx_mult_vec(&hold, &offset, &mut position);
            let dropped = pool
                .iter_mut()
                .find(|i| i.owner == Some(owner) && i.kind == kind)?;
            dropped.throw_speed = 1.0;
            dropped.leave_hand(hsd_types::Vec3::ZERO, position, assets);
            (SceneItems::logic(kind).dropped)(
                dropped,
                &mut ItemAnimationContext {
                    owner: owner_context.held_item,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                },
            );
            dropped.end_hold(center, attack, map, assets);
            dropped.hurt_by_owner = true;
            return None;
        }
        ItemRequest::Drop {
            item,
            position,
            hand,
            speed,
            center,
            attack,
        } => {
            // Item_8026ABD8: it_802731A4's sound, xC44, it_80273B50 at the
            // hand with no push, the kind's dropped callback, it_80274198
            // (it_80273F34), then it_802754D4; it_8026B6C8's stage enemies
            // are not in scope.
            let dropped = pool.get_mut(item).expect("dropped item");
            let assets = resources.get(dropped.kind);
            dropped.throw_speed = speed;
            let position = dropped.drop_release_point(position, &hand, assets);
            dropped.leave_hand(hsd_types::Vec3::ZERO, position, assets);
            (SceneItems::logic(dropped.kind).dropped)(
                dropped,
                &mut ItemAnimationContext {
                    owner: owner.held_item,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                },
            );
            dropped.end_hold(center, attack, map, assets);
            dropped.hurt_by_owner = true;
            return None;
        }
    };
    let assets = resources.get(spawn.kind);
    if let Some(id) = pool.spawn_with_stale::<SceneItems>(spawn, assets, owner.stale_multiplier) {
        let item = pool.get_mut(id).unwrap();
        item.initialize_collision(spawn, assets, map);
        item.past_hitbox_refresh = owner.after_hitbox_refresh;
        if let Some((angle, speed, motion)) = ray {
            it_foxlaser::initialize_laser(pool.get_mut(id).unwrap(), assets, angle, speed, motion);
        }
        // The spawner's own set-up after Item_80268B18 (it_802BE2E8,
        // it_802BD4AC's turnip fields before its Item_8026AB54).
        let common = pool.common().clone();
        (SceneItems::logic(spawn.kind).launched)(
            pool.get_mut(id).unwrap(),
            assets,
            &common,
            &spawn,
            rng,
        );
        (SceneItems::logic(spawn.kind).spawned_with_map)(
            pool.get_mut(id).unwrap(),
            assets,
            &common,
            &spawn,
            map,
        );
        if let ItemRequest::SpawnInHand { part, .. } = request {
            // Item_8026AB54: it_802742F4's attachment, then the kind's
            // pickup callback.
            let lifetime = pool.common().lifetime;
            let half_life_scale = pool.common().half_life_scale;
            let item = pool.get_mut(id).unwrap();
            item.attach_to_holder(
                owner.slot.expect("held by a fighter"),
                part,
                assets,
                lifetime,
                half_life_scale,
            );
            (SceneItems::logic(spawn.kind).picked_up)(
                item,
                &mut ItemAnimationContext {
                    owner: None,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                },
            );
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
                    partner: None,
                },
            );
        }
        pool.get_mut(id).unwrap().past_hitbox_refresh = false;
        let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
        for phase in melee_it::ITEM_PROCESS_LINKS {
            world.add_tagged_proc(object, phase, tag(id, phase));
        }
        objects.push((id, object));
        return Some(id);
    }
    None
}

/// One member of an [`ItemRequest::SpawnChain`], spawned as an ordinary
/// request for the same owner.
#[allow(clippy::too_many_arguments)] // Item pool, scene objects and the shared RNG stay separate.
fn request_one_of_chain(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    spawn: melee_it::SpawnItem,
    owner: &RequestOwner<'_>,
    rng: &mut gekko_math::HsdRng,
) -> Option<u32> {
    request(
        pool,
        resources,
        map,
        world,
        objects,
        ItemRequest::Spawn(spawn),
        RequestOwner {
            slot: owner.slot,
            held_item: owner.held_item,
            after_hitbox_refresh: owner.after_hitbox_refresh,
            stale_multiplier: owner.stale_multiplier,
        },
        rng,
    )
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
#[allow(clippy::too_many_arguments)] // Item pool, scene objects and the shared RNG stay separate.
pub fn spawn_rain_bomb(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    position: hsd_types::Vec3,
    facing: f32,
    rng: &mut gekko_math::HsdRng,
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
            after_hitbox_refresh: false,
            stale_multiplier: 1.0,
        },
        rng,
    );
    if pool.len() > before {
        let lifetime = pool.common().lifetime;
        let item = pool.iter_mut().last().expect("spawned Bob-omb");
        it_bombhei::light(item, resources.get(ItemKind::BombHei), lifetime);
    }
}

/// Item_8026A8EC's kind callback reaching the owning fighter: the article
/// lets go of it (ftPe_8011D518, ftPe_SpecialN_DoDeath2).
pub fn article_destroyed(
    fighters: &mut [crate::scene_fighter::SceneFighter],
    item: &melee_it::ItemCore,
) {
    let Some(owner) = item.owner else {
        return;
    };
    if !(SceneItems::logic(item.kind).notifies_owner)(item) {
        return;
    }
    for fighter in fighters.iter_mut() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            if f.player.id == owner {
                f.article_destroyed(item.kind);
            }
        });
    }
}

/// it_802D8618 (802D8618): one Shy Guy of a grStory_801E3418 group, created
/// airborne (it_8027B5B0 -> Item_80268B18) facing `facing`, then placed in
/// its group.
#[allow(clippy::too_many_arguments)] // Item pool, scene objects and the shared RNG stay separate.
pub fn spawn_shy_guy(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    spawn: melee_gr::story::ShyGuySpawn,
    facing: f32,
    rng: &mut gekko_math::HsdRng,
) {
    let before = pool.len();
    request(
        pool,
        resources,
        map,
        world,
        objects,
        ItemRequest::Spawn(it_heiho::spawn(spawn.position, facing)),
        RequestOwner {
            slot: None,
            held_item: None,
            after_hitbox_refresh: false,
            stale_multiplier: 1.0,
        },
        rng,
    );
    if pool.len() > before {
        let item = pool.iter_mut().last().expect("spawned Shy Guy");
        it_heiho::join_group(
            item,
            spawn.group_index,
            spawn.speed_variant,
            spawn.delay,
            resources.get(ItemKind::Heiho),
        );
    }
}

pub fn cleanup(pool: &mut ItemPool, world: &mut World, objects: &mut Objects) {
    for item in pool.iter().filter(|item| item.destroyed) {
        destroy_object(world, objects, item.id);
    }
    pool.remove_destroyed::<SceneItems>();
}

/// HSD_GObjPLink_80390228 for an item: its scene object goes too.
fn destroy_object(world: &mut World, objects: &mut Objects, item: u32) {
    let index = objects
        .iter()
        .position(|(id, _)| *id == item)
        .expect("item GObj");
    world.destroy(objects.remove(index).1);
}
