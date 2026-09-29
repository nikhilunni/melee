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
        PichuTJoltGround: it_pikachu::ThunderJoltBall<it_pikachu::Pichu>,
        PichuTJoltAir: it_pikachu::ThunderJoltCrawler<it_pikachu::Pichu>,
        PichuThunder: it_pikachu::ThunderBolt<it_pikachu::Pichu>,
        MarioFire: it_mariofire::MarioFire,
        IceClimberIce: it_climbersice::ClimbersIce,
        IceClimberGumStrings: it_climbersice::ClimbersString,
        MarioCape: it_mariocape::MarioCape,
        DrMarioVitamin: it_drmariopill::DrMarioPill,
        DrMarioSheet: it_mariocape::DrMarioSheet,
        LuigiFire: it_luigifire::LuigiFire,
        SamusMissile: it_samus::SamusMissile,
        SamusCharge: it_samus::SamusCharge,
        SamusBomb: it_samus::SamusBomb,
        SamusGBeam: it_samus::SamusGrapple,
        SeakVanish: it_seak::SeakVanish,
        SeakNeedleThrow: it_seak::SeakNeedleThrow,
        SeakNeedleHeld: it_seak::SeakNeedleHeld,
        SeakChain: it_seak::SeakChain,
        ZeldaDinFire: it_zelda::DinFire,
        ZeldaDinFireExplode: it_zelda::DinFireExplode,
        LinkBoomerang: it_link::LinkBoomerang,
        CLinkBoomerang: it_link::YoungLinkBoomerang,
        LinkHShot: it_link::LinkHookshot,
        CLinkHShot: it_link::YoungLinkHookshot,
        LinkBow: it_link::LinkBow,
        CLinkBow: it_link::YoungLinkBow,
        LinkArrow: it_link::LinkArrow,
        CLinkArrow: it_link::YoungLinkArrow,
        LinkBomb: it_link::LinkBomb,
        CLinkBomb: it_link::YoungLinkBomb,
        CLinkMilk: it_link::milk::Milk,
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
    /// Articles whose bones effects follow (crate::article_pose).
    pub(crate) article_skeletons: Vec<crate::article_pose::ArticleSkeleton>,
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
        let mut article_skeletons = Vec::new();
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
        let yoshi = match characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlYs.dat")
        {
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
        kinds.push((
            ItemKind::YoshiStar,
            ItemAssets::from_fighter(&yoshi, root, 1, 1)?,
        ));
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
        // ftPk_Init_OnLoad and ftPc_Init_OnLoad: ftData.x48_items[0] is
        // Thunder's bolt, [1] the Thunder Jolt ball, [2] the crawler it
        // rides (whose joint 6 the ball reads); each file its own kinds.
        for (file, symbol, [ball_kind, crawler_kind, bolt_kind]) in [
            (
                "PlPk.dat",
                "ftDataPikachu",
                [
                    ItemKind::PikachuTJoltGround,
                    ItemKind::PikachuTJoltAir,
                    ItemKind::PikachuThunder,
                ],
            ),
            (
                "PlPc.dat",
                "ftDataPichu",
                [
                    ItemKind::PichuTJoltGround,
                    ItemKind::PichuTJoltAir,
                    ItemKind::PichuThunder,
                ],
            ),
        ] {
            let Some(character) = characters
                .iter()
                .find(|c| c.descriptor.data_file == file)
            else {
                continue;
            };
            let a = std::sync::Arc::clone(&character.data);
            let root = a
                .public(symbol)
                .with_context(|| format!("{symbol} fighter data"))?;
            let ball = ItemAssets::from_fighter_states(
                &a,
                root,
                it_pikachu::BALL_ARTICLE_INDEX,
                &it_pikachu::jolt::BALL_ARTICLE_STATES,
                4,
            )?;
            kinds.push((ball_kind, ball));
            visual_archives.push((ball_kind, std::sync::Arc::clone(&a)));
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
            kinds.push((crawler_kind, crawler));
            visual_archives.push((crawler_kind, std::sync::Arc::clone(&a)));
            let bolt = ItemAssets::from_fighter_states(
                &a,
                root,
                it_pikachu::THUNDER_ARTICLE_INDEX,
                &it_pikachu::thunder::ARTICLE_STATES,
                3,
            )?;
            kinds.push((bolt_kind, bolt));
            visual_archives.push((bolt_kind, std::sync::Arc::clone(&a)));
        }
        // ftMr_Init_OnLoad: ftData.x48_items[0] is the fireball.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlMr.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataMario").context("Mario fighter data")?;
            let mut fire = ItemAssets::from_fighter_states(
                &a,
                root,
                it_mariofire::ARTICLE_INDEX,
                &it_mariofire::ARTICLE_STATES,
                it_mariofire::SPECIAL_ATTRIBUTES,
            )?;
            // Item_ApplyFallingPhysics reads the common falling spin.
            fire.fall_spin_degrees = common.fall_spin_degrees;
            // The fireball's joint animation carries its trail (DPtcl 1/1002).
            fire.read_particle_tracks(&a)
                .map_err(|e| anyhow::anyhow!("fireball particle track: {e}"))?;
            kinds.push((ItemKind::MarioFire, fire));
            visual_archives.push((ItemKind::MarioFire, std::sync::Arc::clone(&a)));
            // ftData.x48_items[2]: the cape, whose sparkles follow its bones.
            let cape = ItemAssets::from_fighter_states(
                &a,
                root,
                it_mariocape::ARTICLE_INDEX,
                &it_mariocape::ARTICLE_STATES,
                0,
            )?;
            article_skeletons.push(crate::article_pose::ArticleSkeleton::load(
                ItemKind::MarioCape,
                &a,
                &cape.visual,
            )?);
            kinds.push((ItemKind::MarioCape, cape));
            visual_archives.push((ItemKind::MarioCape, a));
        }
        // ftDr_Init_OnLoad: ftData.x48_items[1] is the Megavitamin, [3] the
        // Super Sheet (the cape's logic row).
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlDr.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a
                .public("ftDataDrmario")
                .context("Dr. Mario fighter data")?;
            let mut pill = ItemAssets::from_fighter_states(
                &a,
                root,
                it_drmariopill::ARTICLE_INDEX,
                &it_drmariopill::ARTICLE_STATES,
                it_drmariopill::SPECIAL_ATTRIBUTES,
            )?;
            // it_80274658 reads the common falling spin.
            pill.fall_spin_degrees = common.fall_spin_degrees;
            kinds.push((ItemKind::DrMarioVitamin, pill));
            visual_archives.push((ItemKind::DrMarioVitamin, std::sync::Arc::clone(&a)));
            let sheet = ItemAssets::from_fighter_states(
                &a,
                root,
                it_mariocape::SHEET_ARTICLE_INDEX,
                &it_mariocape::ARTICLE_STATES,
                0,
            )?;
            article_skeletons.push(crate::article_pose::ArticleSkeleton::load(
                ItemKind::DrMarioSheet,
                &a,
                &sheet.visual,
            )?);
            kinds.push((ItemKind::DrMarioSheet, sheet));
            visual_archives.push((ItemKind::DrMarioSheet, a));
        }
        // ftLg_Init_OnLoad: ftData.x48_items[0] is the fireball.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlLg.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataLuigi").context("Luigi fighter data")?;
            let mut fire = ItemAssets::from_fighter_states(
                &a,
                root,
                it_luigifire::ARTICLE_INDEX,
                &it_luigifire::ARTICLE_STATES,
                it_luigifire::SPECIAL_ATTRIBUTES,
            )?;
            // Item_ApplyFallingPhysics reads the common falling spin.
            fire.fall_spin_degrees = common.fall_spin_degrees;
            // The fireball's joint animation may carry DPtcl keys.
            fire.read_particle_tracks(&a)
                .map_err(|e| anyhow::anyhow!("Luigi fireball particle track: {e}"))?;
            kinds.push((ItemKind::LuigiFire, fire));
            visual_archives.push((ItemKind::LuigiFire, a));
        }
        // ftPp_Init_OnLoad's it_8026B3F8: Popo's ice block, which Nana's
        // Ice Shot makes too.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlPp.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataPopo").context("Popo fighter data")?;
            let mut ice = ItemAssets::from_fighter_states(
                &a,
                root,
                it_climbersice::ARTICLE_INDEX,
                &it_climbersice::ARTICLE_STATES,
                it_climbersice::SPECIAL_ATTRIBUTES,
            )?;
            // Item_ApplyFallingPhysics reads the common falling spin.
            ice.fall_spin_degrees = common.fall_spin_degrees;
            // The generators follow the model root's child.
            ice.read_pose(&a)
                .map_err(|e| anyhow::anyhow!("ice block pose: {e}"))?;
            kinds.push((ItemKind::IceClimberIce, ice));
            visual_archives.push((ItemKind::IceClimberIce, std::sync::Arc::clone(&a)));
            // [2]: the Belay's rope handle (its links are Popo's).
            let rope = ItemAssets::from_fighter_states(
                &a,
                root,
                it_climbersice::string::ARTICLE_INDEX,
                &it_climbersice::string::ARTICLE_STATES,
                it_climbersice::string::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::IceClimberGumStrings, rope));
            visual_archives.push((ItemKind::IceClimberGumStrings, a));
        }
        // ftSs_Init_OnLoad: ftData.x48_items[2] is the missile.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlSs.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataSamus").context("Samus fighter data")?;
            let mut missile = ItemAssets::from_fighter_states(
                &a,
                root,
                it_samus::missile::ARTICLE_INDEX,
                &it_samus::missile::ARTICLE_STATES,
                it_samus::missile::SPECIAL_ATTRIBUTES,
            )?;
            // it_8027518C: the explosion's common lifetime.
            missile.read_common_release(&common_archive, public)?;
            // The trail follows the model's grandchild.
            missile
                .read_pose(&a)
                .map_err(|e| anyhow::anyhow!("missile pose: {e}"))?;
            kinds.push((ItemKind::SamusMissile, missile));
            visual_archives.push((ItemKind::SamusMissile, std::sync::Arc::clone(&a)));
            // ftData.x48_items[1]: the charge shot.
            let charge = ItemAssets::from_fighter_states(
                &a,
                root,
                it_samus::charge::ARTICLE_INDEX,
                &it_samus::charge::ARTICLE_STATES,
                it_samus::charge::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::SamusCharge, charge));
            visual_archives.push((ItemKind::SamusCharge, std::sync::Arc::clone(&a)));
            // ftData.x48_items[0]: the bomb, whose blast takes the common
            // explosion lifetime (it_8027518C) and whose landing reads
            // ItCo +74 (it_8026DC24).
            let mut bomb = ItemAssets::from_fighter_states(
                &a,
                root,
                it_samus::bomb::ARTICLE_INDEX,
                &it_samus::bomb::ARTICLE_STATES,
                it_samus::bomb::SPECIAL_ATTRIBUTES,
            )?;
            bomb.read_common_release(&common_archive, public)?;
            kinds.push((ItemKind::SamusBomb, bomb));
            visual_archives.push((ItemKind::SamusBomb, std::sync::Arc::clone(&a)));
            // ftData.x48_items[3]: the grapple beam, whose rope its owner
            // simulates.
            let grapple = ItemAssets::from_fighter_states(
                &a,
                root,
                it_samus::grapple::ARTICLE_INDEX,
                &it_samus::grapple::ARTICLE_STATES,
                it_samus::grapple::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::SamusGBeam, grapple));
            visual_archives.push((ItemKind::SamusGBeam, a));
        }
        // ftSk_Init_OnLoad: ftData.x48_items[2] is Vanish's smoke. Sheik's
        // articles load with Zelda too: both forms come to every match.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlSk.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataSeak").context("Sheik fighter data")?;
            let smoke = ItemAssets::from_fighter_states(
                &a,
                root,
                it_seak::vanish::ARTICLE_INDEX,
                &it_seak::vanish::ARTICLE_STATES,
                0,
            )?;
            kinds.push((ItemKind::SeakVanish, smoke));
            visual_archives.push((ItemKind::SeakVanish, std::sync::Arc::clone(&a)));
            // [0]: a thrown needle; [1]: the bundle in her hand.
            let mut thrown = ItemAssets::from_fighter_states(
                &a,
                root,
                it_seak::needle::THROWN_ARTICLE_INDEX,
                &it_seak::needle::THROWN_ARTICLE_STATES,
                it_seak::needle::THROWN_SPECIAL_ATTRIBUTES,
            )?;
            // The needle's hitbox rides its model's child joint.
            thrown
                .read_pose(&a)
                .map_err(|e| anyhow::anyhow!("needle pose: {e}"))?;
            kinds.push((ItemKind::SeakNeedleThrow, thrown));
            visual_archives.push((ItemKind::SeakNeedleThrow, std::sync::Arc::clone(&a)));
            let held = ItemAssets::from_fighter_states(
                &a,
                root,
                it_seak::needle::HELD_ARTICLE_INDEX,
                &it_seak::needle::HELD_ARTICLE_STATES,
                0,
            )?;
            kinds.push((ItemKind::SeakNeedleHeld, held));
            visual_archives.push((ItemKind::SeakNeedleHeld, std::sync::Arc::clone(&a)));
            // [3]: the chain's handle (its links are Sheik's).
            let chain = ItemAssets::from_fighter_states(
                &a,
                root,
                it_seak::chain::ARTICLE_INDEX,
                &it_seak::chain::ARTICLE_STATES,
                0,
            )?;
            kinds.push((ItemKind::SeakChain, chain));
            visual_archives.push((ItemKind::SeakChain, std::sync::Arc::clone(&a)));
        }
        // ftZd_Init_OnLoad: ftData.x48_items[0] is Din's Fire, [1] its
        // explosion; they load with Sheik too.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlZd.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataZelda").context("Zelda fighter data")?;
            for (kind, index, states, attributes) in [
                (
                    ItemKind::ZeldaDinFire,
                    it_zelda::din_fire::ARTICLE_INDEX,
                    &it_zelda::din_fire::ARTICLE_STATES[..],
                    it_zelda::din_fire::SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::ZeldaDinFireExplode,
                    it_zelda::explode::ARTICLE_INDEX,
                    &it_zelda::explode::ARTICLE_STATES[..],
                    it_zelda::explode::SPECIAL_ATTRIBUTES,
                ),
            ] {
                kinds.push((
                    kind,
                    ItemAssets::from_fighter_states(&a, root, index, states, attributes)?,
                ));
                visual_archives.push((kind, std::sync::Arc::clone(&a)));
            }
        }
        // ftLk_Init_OnLoad / ftCl_Init_OnLoad: ftData.x48_items[1] is the
        // boomerang, [2] the hookshot.
        for (file, symbol, kind) in [
            ("PlLk.dat", "ftDataLink", ItemKind::LinkBoomerang),
            ("PlCl.dat", "ftDataClink", ItemKind::CLinkBoomerang),
        ] {
            let Some(character) = characters.iter().find(|c| c.descriptor.data_file == file) else {
                continue;
            };
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public(symbol).context("Link fighter data")?;
            let mut boomerang = ItemAssets::from_fighter_states(
                &a,
                root,
                it_link::boomerang::ARTICLE_INDEX,
                &it_link::boomerang::ARTICLE_STATES,
                it_link::boomerang::SPECIAL_ATTRIBUTES,
            )?;
            // The throw's release sweep grows its box (it_80275D5C).
            boomerang.read_common_release(&common_archive, public)?;
            kinds.push((kind, boomerang));
            visual_archives.push((kind, std::sync::Arc::clone(&a)));
            // ftData.x48_items[2]: the hookshot.
            let hookshot_kind = if kind == ItemKind::LinkBoomerang {
                ItemKind::LinkHShot
            } else {
                ItemKind::CLinkHShot
            };
            let hookshot = ItemAssets::from_fighter_states(
                &a,
                root,
                it_link::hookshot::ARTICLE_INDEX,
                &it_link::hookshot::ARTICLE_STATES,
                it_link::hookshot::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((hookshot_kind, hookshot));
            visual_archives.push((hookshot_kind, std::sync::Arc::clone(&a)));
            // [3] the arrow, [4] the bow.
            let young = kind == ItemKind::CLinkBoomerang;
            let (arrow_kind, bow_kind) = if young {
                (ItemKind::CLinkArrow, ItemKind::CLinkBow)
            } else {
                (ItemKind::LinkArrow, ItemKind::LinkBow)
            };
            let mut arrow = ItemAssets::from_fighter_states(
                &a,
                root,
                it_link::arrow::ARTICLE_INDEX,
                &it_link::arrow::ARTICLE_STATES,
                it_link::arrow::SPECIAL_ATTRIBUTES,
            )?;
            // The shot's release sweep (it_80275D5C).
            arrow.read_common_release(&common_archive, public)?;
            kinds.push((arrow_kind, arrow));
            visual_archives.push((arrow_kind, std::sync::Arc::clone(&a)));
            let bow = ItemAssets::from_fighter_states(
                &a,
                root,
                it_link::bow::ARTICLE_INDEX,
                &it_link::bow::ARTICLE_STATES,
                it_link::bow::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((bow_kind, bow));
            visual_archives.push((bow_kind, std::sync::Arc::clone(&a)));
            // [0] the bomb, which falls, spins, explodes and lands.
            let bomb_kind = if young {
                ItemKind::CLinkBomb
            } else {
                ItemKind::LinkBomb
            };
            let mut bomb = ItemAssets::from_fighter_states_with_count(
                &a,
                root,
                it_link::bomb::ARTICLE_INDEX,
                &it_link::bomb::ARTICLE_STATES,
                it_link::bomb::SPECIAL_ATTRIBUTES,
                it_link::bomb::ARTICLE_STATE_COUNT,
            )?;
            bomb.read_common_release(&common_archive, public)?;
            // The lit fuse's joint animation may carry DPtcl keys.
            bomb.read_particle_tracks(&a)
                .map_err(|e| anyhow::anyhow!("Link bomb particle track: {e}"))?;
            kinds.push((bomb_kind, bomb));
            visual_archives.push((bomb_kind, std::sync::Arc::clone(&a)));
            // Young Link's [5]: the taunt milk.
            if young {
                let milk = ItemAssets::from_fighter_states(
                    &a,
                    root,
                    it_link::milk::ARTICLE_INDEX,
                    &it_link::milk::ARTICLE_STATES,
                    it_link::milk::SPECIAL_ATTRIBUTES,
                )?;
                kinds.push((ItemKind::CLinkMilk, milk));
                visual_archives.push((ItemKind::CLinkMilk, a));
            }
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
            article_skeletons,
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

/// The article `owner` (its `secondary` fighter for Nana) tracks, if any
/// (its kind's owner_report).
pub fn owner_report(
    pool: &ItemPool,
    resources: &Resources,
    owner: u8,
    secondary: bool,
) -> Option<melee_it::ArticleReport> {
    pool.iter()
        .filter(|item| {
            item.owner == Some(owner) && item.owner_secondary == secondary && !item.destroyed
        })
        .find_map(|item| {
            (SceneItems::logic(item.kind).owner_report)(item, resources.get(item.kind))
        })
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
    /// The requesting fighter is its player's second (Nana), whose
    /// articles are its own.
    pub secondary: bool,
    pub held_item: Option<&'a melee_it::ItemOwner>,
    /// The requesting proc runs past item link 11 (HSD_GObj_804D7838's
    /// s_link > 11): hitboxes a new item's script creates are placed at once
    /// (it_802790C0).
    pub after_hitbox_refresh: bool,
    /// The requesting proc runs at s_link 9 or later: efAsync_Spawn
    /// (efasync.c:1458) dispatches a request at once rather than queue it.
    pub efasync_immediate: bool,
    pub stale_multiplier: f32,
    /// The stage limits item procs read (it_802750F8's immediate procs);
    /// the stage's own spawners never run them.
    pub bounds: Option<&'a melee_it::ItemBounds>,
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
        // it_802B1DF8: one Item_8026AE60 id makes the chain one hit group.
        let hit_group = pool.allocate_hit_group();
        for index in 0..count {
            let member =
                request_one_of_chain(pool, resources, map, world, objects, spawn, &owner, rng);
            if let (Some(previous), Some(member)) = (previous, member) {
                pool.get_mut(previous).unwrap().partner = Some(member);
            }
            if let Some(member) = member {
                pool.join_hit_group(member, hit_group);
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
            let secondary = owner_context.secondary && owner_context.slot == Some(owner);
            pool.control_owned::<SceneItems>(
                owner,
                secondary,
                kind,
                control,
                resources.get(kind),
                owner_context.efasync_immediate,
            );
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
            held.owner_secondary = owner.secondary;
            (SceneItems::logic(held.kind).picked_up)(
                held,
                &mut ItemAnimationContext {
                    owner: owner.held_item,
                    holder: None,
                    map,
                    assets,
                    partner: None,
                    rng: None,
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
            // it_802790C0 reads the running proc's s_link: a hitbox the
            // thrown callback makes is placed at once past link 11.
            thrown.past_hitbox_refresh = owner_context.after_hitbox_refresh;
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
                    rng: None,
                },
            );
            thrown.end_hold(center, attack, map, assets);
            // xDCE b0 and b2: the thrower's hits land on it, and so do its
            // kin's.
            thrown.hurt_by_owner = true;
            return None;
        }
        ItemRequest::Stow { item, stowed } => {
            let held = pool.get_mut(item).expect("stowed item");
            // it_8026B73C also sets x5 when x7 is set; no stowable kind sets x7.
            held.hidden = stowed;
            held.frozen = stowed;
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
            let (common, mut items) = pool.common_and_items_mut();
            let item = items
                .find(|item| {
                    item.owner == Some(slot)
                        && item.owner_secondary == owner_context.secondary
                        && item.kind == kind
                        && item.held
                })
                .expect("launched article in its owner's hand");
            (SceneItems::logic(kind).launch)(item, &launch, common, map, resources.get(kind));
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
            let dropped = pool.iter_mut().find(|i| {
                i.owner == Some(owner)
                    && i.owner_secondary == owner_context.secondary
                    && i.kind == kind
            })?;
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
                    rng: None,
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
            // it_802790C0: as for a throw.
            dropped.past_hitbox_refresh = owner_context.after_hitbox_refresh;
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
                    rng: None,
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
        item.owner_secondary = owner.secondary && spawn.owner == owner.slot;
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
        if SceneItems::logic(spawn.kind).procs_at_spawn {
            let bounds = owner.bounds.expect("it_802750F8 from a fighter's spawn");
            // it_802750F8: physics and collision now, without Item_802696CC's
            // blast-zone test (xDCC b3 cleared, then set again).
            pool.get_mut(id).unwrap().blast_zone_checked = false;
            let cell = std::cell::Cell::new(*rng);
            pool.physics::<SceneItems>(id, None, &Default::default(), bounds, assets, &cell);
            *rng = cell.get();
            let contact = pool.stage_contact(id, map);
            pool.collide::<SceneItems>(id, None, contact, map, bounds, assets, Some(&cell));
            *rng = cell.get();
            if let Some(item) = pool.get_mut(id) {
                item.blast_zone_checked = true;
            }
        }
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
                    rng: None,
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
                    rng: None,
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
            secondary: owner.secondary,
            held_item: owner.held_item,
            after_hitbox_refresh: owner.after_hitbox_refresh,
            efasync_immediate: owner.efasync_immediate,
            stale_multiplier: owner.stale_multiplier,
            bounds: owner.bounds,
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
            secondary: false,
            held_item: None,
            after_hitbox_refresh: false,
            efasync_immediate: false,
            stale_multiplier: 1.0,
            bounds: None,
        },
        rng,
    );
    if pool.len() > before {
        let lifetime = pool.common().lifetime;
        let item = pool.iter_mut().last().expect("spawned Bob-omb");
        it_bombhei::light(item, resources.get(ItemKind::BombHei), lifetime);
    }
}

/// The world matrix of a free item's model bone: its sampled pose under the
/// item's own root, in the rest pose while its motion plays no article
/// state; a melting ice block's child takes its scale (it_80272F7C).
pub fn free_item_bone_matrix(
    resources: &Resources,
    item: &melee_it::ItemCore,
    bone: usize,
) -> hsd_types::Mtx {
    let pose = resources
        .get(item.kind)
        .pose
        .as_ref()
        .expect("an item bone without a sampled pose");
    let root = item.root_srt();
    let animated = SceneItems::logic(item.kind).states[usize::from(item.motion)].animation_id >= 0;
    if animated {
        return pose.bone_matrix(item.article_state, item.pose_steps, bone, root);
    }
    let scale = match &item.scratch {
        melee_it::ItemScratch::ClimbersIce(ice) => Some((1, ice.scale)),
        _ => None,
    };
    pose.rest_bone_matrix(bone, root, scale)
}

/// The fighter-list index of the fighter owning an item: its player
/// `owner`, and its second fighter when `secondary` (retail keeps the
/// owner's GObj).
pub fn owner_index(
    fighters: &[crate::scene_fighter::SceneFighter],
    owner: Option<u8>,
    secondary: bool,
) -> Option<usize> {
    let owner = owner?;
    fighters
        .iter()
        .position(|f| f.player.id == owner && f.player.secondary == secondary)
}

/// An item accessory's blast reaching its owner at once (it_802B5478 ->
/// ftSs_Init_80128A1C), before the next item's procs.
pub fn owner_blast(state: &mut crate::initial_state::InitialState, blast: &melee_it::OwnerBlast) {
    for (index, fighter) in state.fighters.iter_mut().enumerate() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            if f.player.id == blast.owner && !f.player.secondary {
                f.owner_blast(blast, &state.assets.fighters[index]);
            }
        });
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
            if f.player.id == owner && f.player.secondary == item.owner_secondary {
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
            secondary: false,
            held_item: None,
            after_hitbox_refresh: false,
            efasync_immediate: false,
            stale_multiplier: 1.0,
            bounds: None,
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
