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
        Capsule: it_container::Capsule,
        Box: it_container::Box,
        Taru: it_container::Barrel,
        Kusudama: it_container::PartyBall,
        Sword: it_sword::Sword,
        Bat: it_sword::Bat,
        Harisen: it_sword::Harisen,
        StarRod: it_sword::StarRod,
        StarRodStar: it_sword::StarRodStar,
        GShell: it_shell::GreenShell,
        RShell: it_shell::RedShell,
        Freeze: it_freeze::Freezie,
        MSBomb: it_msbomb::MotionSensorBomb,
        Heart: it_heal::HeartContainer,
        Tomato: it_heal::MaximTomato,
        Foods: it_heal::Food,
        WStar: it_wstar::WarpStar,
        Klap: it_klap::Klaptrap,
        Kuriboh: it_monsters::Kuriboh,
        Leadead: it_monsters::ReDead,
        Dosei: it_dosei::Dosei,
        Unk4: it_coin::Coin,
        YoshiEggThrow: it_yoshieggthrow::YoshiEggThrow,
        YoshiStar: it_yoshistar::YoshiStar,
        PeachExplode: it_peach::PeachExplode,
        PeachTurnip: it_peach::PeachTurnip,
        PeachParasol: it_peach::PeachParasol,
        PeachToad: it_peach::PeachToad,
        PeachToadSpore: it_peach::PeachToadSpore,
        Heiho: it_heiho::Heiho,
        Tools: it_tools::Tools,
        Yaku: it_yaku::Yaku,
        PikachuTJoltGround: it_pikachu::ThunderJoltBall,
        PikachuTJoltAir: it_pikachu::ThunderJoltCrawler,
        PikachuThunder: it_pikachu::ThunderBolt,
        PichuTJoltGround: it_pikachu::ThunderJoltBall<it_pikachu::Pichu>,
        PichuTJoltAir: it_pikachu::ThunderJoltCrawler<it_pikachu::Pichu>,
        PichuThunder: it_pikachu::ThunderBolt<it_pikachu::Pichu>,
        MarioFire: it_mariofire::MarioFire,
        IceClimberIce: it_climbersice::ClimbersIce,
        IceClimberGumStrings: it_climbersice::ClimbersString,
        IceClimberBlizzard: it_climbersice::blizzard::ClimbersBlizzard,
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
        KoopaFlame: it_koopaflame::KoopaFlame,
        GameWatchGreenhouse: it_gamewatch::Greenhouse,
        GameWatchManhole: it_gamewatch::Manhole,
        GameWatchFire: it_gamewatch::Fire,
        GameWatchParachute: it_gamewatch::Parachute,
        GameWatchTurtle: it_gamewatch::Turtle,
        GameWatchBreath: it_gamewatch::Breath,
        GameWatchChef: it_gamewatch::Chef,
        GameWatchJudge: it_gamewatch::Judge,
        GameWatchPanic: it_gamewatch::Panic,
        GameWatchRescue: it_gamewatch::Rescue,
        NessBat: it_ness::NessBat,
        NessYoyo: it_ness::NessYoyo,
        NessPKFire: it_ness::NessPkFire,
        NessPKFireFlame: it_ness::NessPkFirePillar,
        NessPKFlush: it_ness::NessPkFlash,
        NessPKFlushExplode: it_ness::NessPkFlashExplode,
        NessPKThunder: it_ness::NessPkThunder,
        NessPKThunder1: it_ness::NessPkThunderTrail<0>,
        NessPKThunder2: it_ness::NessPkThunderTrail<1>,
        NessPKThunder3: it_ness::NessPkThunderTrail<2>,
        NessPKThunder4: it_ness::NessPkThunderTrail<3>,
        MewtwoDisable: it_mewtwo::MewtwoDisable,
        MewtwoShadowBall: it_mewtwo::MewtwoShadowBall,
        KirbyCBeam: it_kirby::CutterBeam,
        KirbyHammer: it_kirby::Hammer,
        Unk1: it_kirby::AbilityStar,
        KirbyFoxLaser: it_foxlaser::KirbyFoxLaser,
        KirbyFalcoLaser: it_foxlaser::KirbyFalcoLaser,
        KirbyFoxBlaster: it_foxlaser::KirbyFoxBlaster,
        KirbyFalcoBlaster: it_foxlaser::KirbyFalcoBlaster,
        KirbyGameWatchChef: it_gamewatch::KirbyChef,
        KirbyGameWatchChefPan: it_kirby::ChefPan,
        KirbyMarioFire: it_mariofire::KirbyMarioFire,
        KirbyDrMarioVitamin: it_drmariopill::KirbyDrMarioPill,
        KirbyLuigiFire: it_luigifire::KirbyLuigiFire,
        KirbySamusCharge: it_samus::KirbySamusCharge,
        KirbyMewtwoShadowBall: it_mewtwo::KirbyMewtwoShadowBall,
        KirbyPikachuTJoltGround: it_pikachu::ThunderJoltBall<it_pikachu::KirbyPikachu>,
        KirbyPikachuTJoltAir: it_pikachu::ThunderJoltCrawler<it_pikachu::KirbyPikachu>,
        KirbyPichuTJoltGround: it_pikachu::ThunderJoltBall<it_pikachu::KirbyPichu>,
        KirbyPichuTJoltAir: it_pikachu::ThunderJoltCrawler<it_pikachu::KirbyPichu>,
        KirbyPeachToad: it_peach::KirbyPeachToad,
        KirbyPeachToadSpore: it_peach::KirbyPeachToadSpore,
        KirbyNessPKFlush: it_ness::KirbyNessPkFlash,
        KirbyNessPKFlushExplode: it_ness::KirbyNessPkFlashExplode,
        KirbyLinkArrow: it_link::KirbyLinkArrow,
        KirbyLinkBow: it_link::KirbyLinkBow,
        KirbyCLinkArrow: it_link::KirbyYoungLinkArrow,
        KirbyCLinkBow: it_link::KirbyYoungLinkBow,
        KirbyKoopaFlame: it_koopaflame::KirbyKoopaFlame,
        KirbyIceClimberIce: it_climbersice::KirbyClimbersIce,
        KirbySeakNeedleThrow: it_seak::KirbySeakNeedleThrow,
        KirbySeakNeedleHeld: it_seak::KirbySeakNeedleHeld,
    }
}

/// One of Kirby's copy archives (ftKb_Init_803CA9D0[kind]) whose articles
/// the port registers: KirbyHatStruct `symbol`, and per article its
/// hat_dynamics slot (+C + 4 * slot) and motion-state count.
pub(crate) struct KirbyCopyArchive {
    pub kind: melee_types::FighterKind,
    pub file: &'static str,
    pub symbol: &'static str,
    pub articles: &'static [(ItemKind, u32, usize)],
}

/// ftKb_SpecialN_800F16D0's registrations, by copied kind. A new copied
/// special whose code spawns an article adds its kind's row here.
pub(crate) const KIRBY_COPY_ARCHIVES: [KirbyCopyArchive; 17] = [
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Fox,
        file: "PlKbCpFx.dat",
        symbol: "ftDataKirbyCopyFox",
        // The laser's two states and the blaster's nine, as Fox's own.
        articles: &[
            (ItemKind::KirbyFoxLaser, 0, 2),
            (ItemKind::KirbyFoxBlaster, 1, 9),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Falco,
        file: "PlKbCpFc.dat",
        symbol: "ftDataKirbyCopyFalco",
        // hat_dynamics[0..3] are the grafted hat's spring chains.
        articles: &[
            (ItemKind::KirbyFalcoLaser, 3, 2),
            (ItemKind::KirbyFalcoBlaster, 4, 9),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::GameWatch,
        file: "PlKbCpGw.dat",
        symbol: "ftDataKirbyCopyGamewatch",
        // hat_dynamics[0..5] are the grafted hat's; the food's two states
        // as Mr. Game & Watch's own, and the pan's one.
        articles: &[
            (ItemKind::KirbyGameWatchChef, 5, 2),
            (ItemKind::KirbyGameWatchChefPan, 6, 1),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Mario,
        file: "PlKbCpMr.dat",
        symbol: "ftDataKirbyCopyMario",
        // ft_80459B88.x0->xC: the fireball's one state, as Mario's own.
        articles: &[(ItemKind::KirbyMarioFire, 0, 1)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::DrMario,
        file: "PlKbCpDr.dat",
        symbol: "ftDataKirbyCopyDrmario",
        // hats[20]->hat_dynamics[0]: the pill's seven states, as Dr. Mario's.
        articles: &[(ItemKind::KirbyDrMarioVitamin, 0, 7)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Luigi,
        file: "PlKbCpLg.dat",
        symbol: "ftDataKirbyCopyLuigi",
        // hats[16]->hat_dynamics[0]: the fireball's one state.
        articles: &[(ItemKind::KirbyLuigiFire, 0, 1)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Samus,
        file: "PlKbCpSs.dat",
        symbol: "ftDataKirbyCopySamus",
        // The charge shot's nine states, as Samus's own.
        articles: &[(ItemKind::KirbySamusCharge, 0, 9)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Mewtwo,
        file: "PlKbCpMt.dat",
        symbol: "ftDataKirbyCopyMewtwo",
        // hat_dynamics[0..3] are the grafted hat's; the ball's eighteen
        // states, as Mewtwo's own.
        articles: &[(ItemKind::KirbyMewtwoShadowBall, 3, 18)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Pikachu,
        file: "PlKbCpPk.dat",
        symbol: "ftDataKirbyCopyPikachu",
        // The Thunder Jolt ball's two states and the crawler's one.
        articles: &[
            (ItemKind::KirbyPikachuTJoltGround, it_pikachu::KIRBY_BALL_SLOT, 2),
            (ItemKind::KirbyPikachuTJoltAir, it_pikachu::KIRBY_CRAWLER_SLOT, 1),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Pichu,
        file: "PlKbCpPc.dat",
        symbol: "ftDataKirbyCopyPichu",
        articles: &[
            (ItemKind::KirbyPichuTJoltGround, it_pikachu::KIRBY_BALL_SLOT, 2),
            (ItemKind::KirbyPichuTJoltAir, it_pikachu::KIRBY_CRAWLER_SLOT, 1),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Peach,
        file: "PlKbCpPe.dat",
        symbol: "ftDataKirbyCopyPeach",
        // Toad's two states and the spore's one, as Peach's own.
        articles: &[
            (ItemKind::KirbyPeachToad, it_peach::KIRBY_TOAD_SLOT, 2),
            (ItemKind::KirbyPeachToadSpore, it_peach::KIRBY_SPORE_SLOT, 1),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Ness,
        file: "PlKbCpNs.dat",
        symbol: "ftDataKirbyCopyNess",
        // hats[7]->hat_dynamics[0, 1]: the flash's three states and the
        // explosion's one, as Ness's own.
        articles: &[
            (ItemKind::KirbyNessPKFlush, 0, 3),
            (ItemKind::KirbyNessPKFlushExplode, 1, 1),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Link,
        file: "PlKbCpLk.dat",
        symbol: "ftDataKirbyCopyLink",
        // hats[5]->hat_dynamics[0, 1]: the arrow's five states and the
        // bow's seven, as Link's own.
        articles: &[
            (ItemKind::KirbyLinkArrow, 0, 5),
            (ItemKind::KirbyLinkBow, 1, 7),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::CLink,
        file: "PlKbCpCl.dat",
        symbol: "ftDataKirbyCopyClink",
        // hats[19]->hat_dynamics[0, 1]: the fire arrow and the bow.
        articles: &[
            (ItemKind::KirbyCLinkArrow, 0, 5),
            (ItemKind::KirbyCLinkBow, 1, 7),
        ],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Koopa,
        file: "PlKbCpKp.dat",
        symbol: "ftDataKirbyCopyKoopa",
        // hats[4]->hat_dynamics[0]: the flame's one state, as Bowser's.
        articles: &[(ItemKind::KirbyKoopaFlame, 0, 1)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Popo,
        file: "PlKbCpPp.dat",
        symbol: "ftDataKirbyCopyPopo",
        // hats[9]->hat_dynamics[0]: the ice block's four states, as Popo's.
        articles: &[(ItemKind::KirbyIceClimberIce, 0, 4)],
    },
    KirbyCopyArchive {
        kind: melee_types::FighterKind::Seak,
        file: "PlKbCpSk.dat",
        symbol: "ftDataKirbyCopySeak",
        // hats[6]->hat_dynamics[0, 1]: the thrown needle's five states and
        // the bundle's one, as Sheik's own.
        articles: &[
            (ItemKind::KirbySeakNeedleThrow, 0, 5),
            (ItemKind::KirbySeakNeedleHeld, 1, 1),
        ],
    },
];

/// A copy article whose ItemStateTable's anim_id column is not its motion
/// states in order, or whose special attributes are more than ten words:
/// that column and the word count.
fn kirby_copy_article_layout(kind: ItemKind) -> Option<(&'static [i32], u32)> {
    match kind {
        ItemKind::KirbyGameWatchChef => Some((
            &it_gamewatch::chef::ARTICLE_STATES,
            it_gamewatch::chef::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyGameWatchChefPan => Some((&it_kirby::chef_pan::ARTICLE_STATES, 10)),
        ItemKind::KirbyMarioFire => Some((
            &it_mariofire::ARTICLE_STATES,
            it_mariofire::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyDrMarioVitamin => Some((
            &it_drmariopill::ARTICLE_STATES,
            it_drmariopill::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyLuigiFire => Some((
            &it_luigifire::ARTICLE_STATES,
            it_luigifire::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbySamusCharge => Some((
            &it_samus::charge::ARTICLE_STATES,
            it_samus::charge::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyMewtwoShadowBall => Some((
            &it_mewtwo::shadow_ball::ARTICLE_STATES,
            it_mewtwo::shadow_ball::SPECIAL_ATTRIBUTES,
        )),
        // The Thunder Jolt ball's four words, the crawler's none (as
        // ftPk_Init_OnLoad registers Pikachu's own).
        ItemKind::KirbyPikachuTJoltGround | ItemKind::KirbyPichuTJoltGround => {
            Some((&it_pikachu::jolt::BALL_ARTICLE_STATES, 4))
        }
        ItemKind::KirbyPikachuTJoltAir | ItemKind::KirbyPichuTJoltAir => {
            Some((&it_pikachu::jolt::CRAWLER_ARTICLE_STATES, 0))
        }
        // Toad's none, the spore's four (as ftPe_Init_OnLoad registers
        // Peach's own).
        ItemKind::KirbyPeachToad => Some((&it_peach::toad::ARTICLE_STATES, 0)),
        ItemKind::KirbyPeachToadSpore => Some((
            &it_peach::spore::ARTICLE_STATES,
            it_peach::spore::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyNessPKFlush => Some((
            &it_ness::pk_flash::FLASH_ARTICLE_STATES,
            it_ness::pk_flash::FLASH_SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyNessPKFlushExplode => Some((
            &it_ness::pk_flash::EXPLODE_ARTICLE_STATES,
            it_ness::pk_flash::EXPLODE_SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyLinkArrow | ItemKind::KirbyCLinkArrow => Some((
            &it_link::arrow::ARTICLE_STATES,
            it_link::arrow::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyLinkBow | ItemKind::KirbyCLinkBow => Some((
            &it_link::bow::ARTICLE_STATES,
            it_link::bow::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyKoopaFlame => Some((
            &it_koopaflame::ARTICLE_STATES,
            it_koopaflame::SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbyIceClimberIce => Some((
            &it_climbersice::ARTICLE_STATES,
            it_climbersice::SPECIAL_ATTRIBUTES,
        )),
        // As ftSk_Init_OnLoad registers Sheik's own: the needle's three
        // words, the bundle's none (and no animation).
        ItemKind::KirbySeakNeedleThrow => Some((
            &it_seak::needle::THROWN_ARTICLE_STATES,
            it_seak::needle::THROWN_SPECIAL_ATTRIBUTES,
        )),
        ItemKind::KirbySeakNeedleHeld => Some((&it_seak::needle::HELD_ARTICLE_STATES, 0)),
        _ => None,
    }
}

/// What a copy article reads beyond its slot, as the original's load does:
/// the falling articles' common spin (Item_ApplyFallingPhysics,
/// it_80274658) and the fireballs' DPtcl keys.
fn kirby_copy_article_finish(
    kind: ItemKind,
    assets: &mut ItemAssets,
    archive: &Archive,
    common: &ItemCommonData,
) -> Result<()> {
    match kind {
        ItemKind::KirbyMarioFire | ItemKind::KirbyLuigiFire => {
            assets.fall_spin_degrees = common.fall_spin_degrees;
            assets
                .read_particle_tracks(archive)
                .map_err(|e| anyhow::anyhow!("Kirby's copied fireball particle track: {e}"))?;
        }
        ItemKind::KirbyDrMarioVitamin => assets.fall_spin_degrees = common.fall_spin_degrees,
        // As Popo's block: the common falling spin, and the generators
        // follow the model root's child.
        ItemKind::KirbyIceClimberIce => {
            assets.fall_spin_degrees = common.fall_spin_degrees;
            assets
                .read_pose(archive)
                .map_err(|e| anyhow::anyhow!("Kirby's copied ice block pose: {e}"))?;
        }
        // As Mewtwo's own: the hitbox rides the model's grandchild, and the
        // joint animations carry DPtcl keys.
        ItemKind::KirbyMewtwoShadowBall => {
            assets
                .read_pose(archive)
                .map_err(|e| anyhow::anyhow!("Kirby's Shadow Ball pose: {e}"))?;
            assets
                .read_particle_tracks(archive)
                .map_err(|e| anyhow::anyhow!("Kirby's Shadow Ball particle track: {e}"))?;
        }
        // As Pikachu's own: the Thunder Jolt crawler's joint 6, which the
        // ball rides and its hitbox follows.
        ItemKind::KirbyPikachuTJoltAir | ItemKind::KirbyPichuTJoltAir => {
            assets
                .read_pose(archive)
                .map_err(|e| anyhow::anyhow!("Kirby's Thunder Jolt crawler pose: {e}"))?;
        }
        // As Sheik's own: the needle's hitbox rides its model's child joint.
        ItemKind::KirbySeakNeedleThrow => assets
            .read_pose(archive)
            .map_err(|e| anyhow::anyhow!("Kirby's copied needle pose: {e}"))?,
        // The flash's and the explosion's DPtcl keys, as Ness's articles.
        ItemKind::KirbyNessPKFlush | ItemKind::KirbyNessPKFlushExplode => assets
            .read_particle_tracks(archive)
            .map_err(|e| anyhow::anyhow!("Kirby's copied PK Flash particle track: {e}"))?,
        _ => {}
    }
    Ok(())
}

/// ftKb_Init_UnkMotionStates5 -> Player_80031DC8(ftKb_SpecialN_800EED50):
/// with a Kirby in the match, the copy archive of every kind in it.
pub(crate) fn kirby_copy_archives(
    kinds: impl Iterator<Item = melee_types::FighterKind> + Clone,
) -> impl Iterator<Item = &'static KirbyCopyArchive> {
    let kirby = kinds.clone().any(|kind| kind == melee_types::FighterKind::Kirby);
    KIRBY_COPY_ARCHIVES
        .iter()
        .filter(move |copy| kirby && kinds.clone().any(|kind| kind == copy.kind))
}

/// The grabbable items (Item_IsGrabbable: xDC8 x15 and a pickup callback)
/// in list order, as ftpickupitem_800942A0 walks HSD_GObj_Entities->items.
/// One item as ftpickupitem_800948A8 takes it (whatever its pickup flags).
pub(crate) fn pickup_candidate(
    items: &ItemPool,
    resources: &Resources,
    id: u32,
) -> melee_ft::fighter::item_pickup::PickupCandidate {
    let item = items.iter().find(|item| item.id == id).expect("a caught item");
    candidate(item, resources)
}

fn candidate(
    item: &melee_it::ItemCore,
    resources: &Resources,
) -> melee_ft::fighter::item_pickup::PickupCandidate {
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
        consumable: matches!(
            item.kind,
            ItemKind::Heart
                | ItemKind::Tomato
                | ItemKind::Foods
                | ItemKind::Coin
                | ItemKind::LuckyEgg
                | ItemKind::WhispyHealApple
        ),
        heal: it_heal::heal_amount(item),
        use_kind: assets.use_kind,
        hand_hold_kind: assets.hand_hold_kind,
        damage_multiplier: assets.collision_damage_multiplier,
        ammo: item.ammo,
        shot_offset: it_sword::shot_offset(item.kind, assets),
    }
}

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
        .map(|item| candidate(item, resources))
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
        // it_8027870C: an English game loads ItCo.usd (the Motion-Sensor
        // Bomb's and the Cloaking Device's attributes differ from ItCo.dat).
        let common_archive = std::sync::Arc::new(archive("ItCo.usd")?);
        let public = common_archive
            .public("itPublicData")
            .context("itPublicData")?;
        let mut common = ItemCommonData::read(&common_archive, public)?;
        common.launch = launch;
        let mut kinds = vec![
            (
                ItemKind::BombHei,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::BombHei,
                    &it_bombhei::ARTICLE_STATES,
                    it_bombhei::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Kuriboh,
                ItemAssets::from_monster(
                    &common_archive,
                    public,
                    ItemKind::Kuriboh,
                    &it_monsters::KURIBOH_ARTICLE_STATES,
                    it_monsters::KURIBOH_SPECIAL_ATTRIBUTES,
                )?,
            ),
            (ItemKind::Leadead, {
                // Its hitbox and hurt capsules ride joints below the root.
                let mut assets = ItemAssets::from_monster(
                    &common_archive,
                    public,
                    ItemKind::Leadead,
                    &it_monsters::REDEAD_ARTICLE_STATES,
                    it_monsters::REDEAD_SPECIAL_ATTRIBUTES,
                )?;
                assets
                    .read_pose(&common_archive)
                    .map_err(|e| anyhow::anyhow!("ReDead pose: {e}"))?;
                assets
            }),
            // Random items' containers.
            (
                ItemKind::Capsule,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Capsule,
                    &it_container::capsule_assets::ARTICLE_STATES,
                    it_container::capsule_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Taru,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Taru,
                    &it_container::barrel_assets::ARTICLE_STATES,
                    it_container::barrel_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Box,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Box,
                    &it_container::box_assets::ARTICLE_STATES,
                    it_container::box_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Kusudama,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Kusudama,
                    &it_container::party_ball_assets::ARTICLE_STATES,
                    it_container::party_ball_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            // Peach's rare pulls.
            (
                ItemKind::Sword,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Sword,
                    &it_sword::ARTICLE_STATES,
                    it_sword::SPECIAL_ATTRIBUTES,
                )?,
            ),
            // Random items.
            (
                ItemKind::GShell,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::GShell,
                    &it_shell::green_assets::ARTICLE_STATES,
                    it_shell::green_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::RShell,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::RShell,
                    &it_shell::red_assets::ARTICLE_STATES,
                    it_shell::red_assets::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::MSBomb,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::MSBomb,
                    &it_msbomb::ARTICLE_STATES,
                    it_msbomb::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Heart,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Heart,
                    &it_heal::heart::ARTICLE_STATES,
                    it_heal::heart::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Tomato,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Tomato,
                    &it_heal::tomato::ARTICLE_STATES,
                    it_heal::tomato::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Foods,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Foods,
                    &it_heal::foods::ARTICLE_STATES,
                    it_heal::foods::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::WStar,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::WStar,
                    &it_wstar::ARTICLE_STATES,
                    it_wstar::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Freeze,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Freeze,
                    &it_freeze::ARTICLE_STATES,
                    it_freeze::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Bat,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Bat,
                    &it_sword::BAT_ARTICLE_STATES,
                    it_sword::BAT_SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Harisen,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Harisen,
                    &it_sword::harisen::ARTICLE_STATES,
                    it_sword::harisen::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::StarRod,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::StarRod,
                    &it_sword::starrod::ARTICLE_STATES,
                    it_sword::starrod::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::StarRodStar,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::StarRodStar,
                    &it_sword::starrod::STAR_ARTICLE_STATES,
                    it_sword::starrod::STAR_SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                ItemKind::Dosei,
                ItemAssets::from_common(
                    &common_archive,
                    public,
                    ItemKind::Dosei,
                    &it_dosei::ARTICLE_STATES,
                    it_dosei::SPECIAL_ATTRIBUTES,
                )?,
            ),
            (
                // A Coin match's coin (it_802E609C).
                ItemKind::Unk4,
                ItemAssets::from_bank(
                    &common_archive,
                    public,
                    ItemKind::Unk4,
                    &it_coin::ARTICLE_STATES,
                    it_coin::SPECIAL_ATTRIBUTES,
                )?,
            ),
        ];
        // Free monster instances use the common archive through ItemPose, not
        // fighter-owned article copies. Archive ownership is established here.
        let mut visual_archives = vec![
            (ItemKind::Kuriboh, std::sync::Arc::clone(&common_archive)),
            (ItemKind::Leadead, std::sync::Arc::clone(&common_archive)),
        ];
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
        // ftKp_Init_OnLoad: ftData.x48_items[0] is the Fire Breath's flame.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlKp.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataKoopa").context("Bowser fighter data")?;
            let mut flame = ItemAssets::from_fighter_states(
                &a,
                root,
                it_koopaflame::ARTICLE_INDEX,
                &it_koopaflame::ARTICLE_STATES,
                it_koopaflame::SPECIAL_ATTRIBUTES,
            )?;
            // itKoopaFlame_Spawn divides by its owner's full fuels
            // (ftKp_SpecialLw_80134DE0 / 80134E1C: ftKoopaAttributes x10
            // and x18, returned as s32).
            let breath = ft_koopa::attributes::read_koopa_attributes(&a)
                .map_err(|e| anyhow::anyhow!("Bowser attributes: {e:?}"))?
                .fire_breath;
            flame.special_attributes.extend(it_koopaflame::owner_fuels(
                breath.reach_max,
                breath.life_max,
            ));
            kinds.push((ItemKind::KoopaFlame, flame));
            visual_archives.push((ItemKind::KoopaFlame, a));
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
            // ftData.x48_items[1]: the Blizzard's puffs.
            let mut blizzard = ItemAssets::from_fighter_states(
                &a,
                root,
                it_climbersice::blizzard::ARTICLE_INDEX,
                &it_climbersice::blizzard::ARTICLE_STATES,
                it_climbersice::blizzard::SPECIAL_ATTRIBUTES,
            )?;
            // The puff's joint animation may carry DPtcl keys.
            blizzard
                .read_particle_tracks(&a)
                .map_err(|e| anyhow::anyhow!("Blizzard particle track: {e}"))?;
            kinds.push((ItemKind::IceClimberBlizzard, blizzard));
            visual_archives.push((ItemKind::IceClimberBlizzard, std::sync::Arc::clone(&a)));
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
        // ftMt_Init_OnLoad: ftData.x48_items[0] is Disable's projectile.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlMt.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataMewtwo").context("Mewtwo fighter data")?;
            let disable = ItemAssets::from_fighter_states(
                &a,
                root,
                it_mewtwo::disable::ARTICLE_INDEX,
                &it_mewtwo::disable::ARTICLE_STATES,
                it_mewtwo::disable::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::MewtwoDisable, disable));
            // ftData.x48_items[1]: the Shadow Ball, whose hitbox rides its
            // model's grandchild.
            let mut ball = ItemAssets::from_fighter_states(
                &a,
                root,
                it_mewtwo::shadow_ball::ARTICLE_INDEX,
                &it_mewtwo::shadow_ball::ARTICLE_STATES,
                it_mewtwo::shadow_ball::SPECIAL_ATTRIBUTES,
            )?;
            ball.read_pose(&a)
                .map_err(|e| anyhow::anyhow!("Shadow Ball pose: {e}"))?;
            // The ball's joint animations may carry DPtcl keys.
            ball.read_particle_tracks(&a)
                .map_err(|e| anyhow::anyhow!("Shadow Ball particle track: {e}"))?;
            kinds.push((ItemKind::MewtwoShadowBall, ball));
            visual_archives.push((ItemKind::MewtwoShadowBall, std::sync::Arc::clone(&a)));
            visual_archives.push((ItemKind::MewtwoDisable, a));
        }
        // ftKb_Init_OnLoad: ftData.x48_items[0] is Final Cutter's beam.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlKb.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataKirby").context("Kirby fighter data")?;
            let beam = ItemAssets::from_fighter_states(
                &a,
                root,
                it_kirby::cutter_beam::ARTICLE_INDEX,
                &it_kirby::cutter_beam::ARTICLE_STATES,
                it_kirby::cutter_beam::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::KirbyCBeam, beam));
            visual_archives.push((ItemKind::KirbyCBeam, std::sync::Arc::clone(&a)));
            // ftData.x48_items[1]: the hammer, whose effect follows its
            // attach joint.
            let hammer = ItemAssets::from_fighter_states(
                &a,
                root,
                it_kirby::hammer::ARTICLE_INDEX,
                &it_kirby::hammer::ARTICLE_STATES,
                0,
            )?;
            article_skeletons.push(crate::article_pose::ArticleSkeleton::load(
                ItemKind::KirbyHammer,
                &a,
                &hammer.visual,
            )?);
            kinds.push((ItemKind::KirbyHammer, hammer));
            visual_archives.push((ItemKind::KirbyHammer, std::sync::Arc::clone(&a)));
            // ftData.x48_items[2]: the star of a lost copy ability.
            let star = ItemAssets::from_fighter_states(
                &a,
                root,
                it_kirby::ability_star::ARTICLE_INDEX,
                &it_kirby::ability_star::ARTICLE_STATES,
                it_kirby::ability_star::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::Unk1, star));
            visual_archives.push((ItemKind::Unk1, a));
        }
        for copy in kirby_copy_archives(characters.iter().map(|c| c.descriptor.kind)) {
            let a = std::sync::Arc::new(archive(copy.file)?);
            let hat = a.public(copy.symbol).context("Kirby copy archive")?;
            for &(kind, slot, states) in copy.articles {
                // KirbyHatStruct.hat_dynamics (+C).
                let at = hat + 0xC + slot * 4;
                let mut assets = match kirby_copy_article_layout(kind) {
                    Some((article_states, attributes)) => {
                        ItemAssets::from_article_slot_states(&a, at, article_states, attributes)?
                    }
                    None => ItemAssets::from_article_slot(&a, at, states)?,
                };
                kirby_copy_article_finish(kind, &mut assets, &a, &common)?;
                // The copied arrows' release sweep (it_80275D5C), as Link's.
                if matches!(kind, ItemKind::KirbyLinkArrow | ItemKind::KirbyCLinkArrow) {
                    assets.read_common_release(&common_archive, public)?;
                }
                if kind == ItemKind::KirbyKoopaFlame {
                    // itKoopaFlame_Spawn divides by Kirby's full fuels
                    // (ftKb_SpecialNKp_800FA83C / 800FA878: ftKb_DatAttrs
                    // +124 and +12C, returned as s32).
                    let kirby = characters
                        .iter()
                        .find(|c| c.descriptor.data_file == "PlKb.dat")
                        .context("Kirby fighter data")?;
                    let breath = ft_kirby::attributes::read_kirby_attributes(&kirby.data)
                        .map_err(|e| anyhow::anyhow!("Kirby attributes: {e:?}"))?
                        .fire_breath;
                    assets.special_attributes.extend(it_koopaflame::owner_fuels(
                        breath.reach_max,
                        breath.life_max,
                    ));
                }
                kinds.push((kind, assets));
                visual_archives.push((kind, std::sync::Arc::clone(&a)));
            }
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
        // ftGw_Init_OnLoad: ftData.x48_items[0..=5] are the attacks' articles
        // (it_gamewatch::article_index).
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlGw.dat")
        {
            use it_gamewatch::{article_index as index, attack, attack_air, special};
            let a = std::sync::Arc::clone(&character.data);
            let root = a
                .public("ftDataGamewatch")
                .context("Mr. Game & Watch fighter data")?;
            for (kind, index, states) in [
                (
                    ItemKind::GameWatchGreenhouse,
                    index::GREENHOUSE,
                    &attack::GREENHOUSE_STATES[..],
                ),
                (
                    ItemKind::GameWatchManhole,
                    index::MANHOLE,
                    &attack::MANHOLE_STATES[..],
                ),
                (ItemKind::GameWatchFire, index::FIRE, &attack::FIRE_STATES[..]),
                (
                    ItemKind::GameWatchParachute,
                    index::PARACHUTE,
                    &attack_air::ARTICLE_STATES[..],
                ),
                (
                    ItemKind::GameWatchTurtle,
                    index::TURTLE,
                    &attack_air::ARTICLE_STATES[..],
                ),
                (
                    ItemKind::GameWatchBreath,
                    index::BREATH,
                    &attack_air::ARTICLE_STATES[..],
                ),
                // [6], [7] and [9]: Judgment's sign, Oil Panic's oil and
                // Fire's trampoline.
                (
                    ItemKind::GameWatchJudge,
                    index::JUDGE,
                    &special::JUDGE_STATES[..],
                ),
                (
                    ItemKind::GameWatchPanic,
                    index::PANIC,
                    &special::PANIC_STATES[..],
                ),
                (
                    ItemKind::GameWatchRescue,
                    index::RESCUE,
                    &special::RESCUE_STATES[..],
                ),
            ] {
                let assets = ItemAssets::from_fighter_states(&a, root, index, states, 0)?;
                // The Manhole's gust blows from its cover bone.
                if kind == ItemKind::GameWatchManhole {
                    article_skeletons.push(crate::article_pose::ArticleSkeleton::load(
                        kind,
                        &a,
                        &assets.visual,
                    )?);
                }
                kinds.push((kind, assets));
                visual_archives.push((kind, std::sync::Arc::clone(&a)));
            }
            // [8]: Chef's food, which flies free.
            let chef = ItemAssets::from_fighter_states(
                &a,
                root,
                index::CHEF,
                &it_gamewatch::chef::ARTICLE_STATES,
                it_gamewatch::chef::SPECIAL_ATTRIBUTES,
            )?;
            kinds.push((ItemKind::GameWatchChef, chef));
            visual_archives.push((ItemKind::GameWatchChef, std::sync::Arc::clone(&a)));
        }
        // ftNs_Init_OnLoad: ftData.x48_items[0..=10], Ness's articles.
        if let Some(character) = characters
            .iter()
            .find(|c| c.descriptor.data_file == "PlNs.dat")
        {
            let a = std::sync::Arc::clone(&character.data);
            let root = a.public("ftDataNess").context("Ness fighter data")?;
            for (kind, index, states, attributes) in [
                (
                    ItemKind::NessBat,
                    it_ness::bat::ARTICLE_INDEX,
                    &it_ness::bat::ARTICLE_STATES[..],
                    0,
                ),
                // [10]: the yo-yo, whose special attributes Ness reads.
                (
                    ItemKind::NessYoyo,
                    it_ness::yoyo::ARTICLE_INDEX,
                    &it_ness::yoyo::ARTICLE_STATES[..],
                    0,
                ),
                (
                    ItemKind::NessPKFire,
                    it_ness::pk_fire::BOLT_ARTICLE_INDEX,
                    &it_ness::pk_fire::BOLT_ARTICLE_STATES[..],
                    it_ness::pk_fire::BOLT_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKFireFlame,
                    it_ness::pk_fire::PILLAR_ARTICLE_INDEX,
                    &it_ness::pk_fire::PILLAR_ARTICLE_STATES[..],
                    it_ness::pk_fire::PILLAR_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKFlush,
                    it_ness::pk_flash::FLASH_ARTICLE_INDEX,
                    &it_ness::pk_flash::FLASH_ARTICLE_STATES[..],
                    it_ness::pk_flash::FLASH_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKFlushExplode,
                    it_ness::pk_flash::EXPLODE_ARTICLE_INDEX,
                    &it_ness::pk_flash::EXPLODE_ARTICLE_STATES[..],
                    it_ness::pk_flash::EXPLODE_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKThunder,
                    it_ness::pk_thunder::HEAD_ARTICLE_INDEX,
                    &it_ness::pk_thunder::HEAD_ARTICLE_STATES[..],
                    it_ness::pk_thunder::HEAD_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKThunder1,
                    it_ness::pk_thunder::TRAIL_ARTICLE_INDICES[0],
                    &it_ness::pk_thunder::TRAIL_ARTICLE_STATES[..],
                    it_ness::pk_thunder::TRAIL_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKThunder2,
                    it_ness::pk_thunder::TRAIL_ARTICLE_INDICES[1],
                    &it_ness::pk_thunder::TRAIL_ARTICLE_STATES[..],
                    it_ness::pk_thunder::TRAIL_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKThunder3,
                    it_ness::pk_thunder::TRAIL_ARTICLE_INDICES[2],
                    &it_ness::pk_thunder::TRAIL_ARTICLE_STATES[..],
                    it_ness::pk_thunder::TRAIL_SPECIAL_ATTRIBUTES,
                ),
                (
                    ItemKind::NessPKThunder4,
                    it_ness::pk_thunder::TRAIL_ARTICLE_INDICES[3],
                    &it_ness::pk_thunder::TRAIL_ARTICLE_STATES[..],
                    it_ness::pk_thunder::TRAIL_SPECIAL_ATTRIBUTES,
                ),
            ] {
                let mut assets =
                    ItemAssets::from_fighter_states(&a, root, index, states, attributes)?;
                // The articles' joint animations may carry DPtcl keys (the
                // pillar's flames).
                assets
                    .read_particle_tracks(&a)
                    .map_err(|e| anyhow::anyhow!("Ness article particle track: {e}"))?;
                kinds.push((kind, assets));
                visual_archives.push((kind, std::sync::Arc::clone(&a)));
            }
        }
        if let Some(mut klap) =
            ItemAssets::from_stage_item(stage, ItemKind::Klap, &it_klap::ARTICLE_STATES, 0, &[])?
        {
            klap.read_pose(stage)
                .map_err(|e| anyhow::anyhow!("Klaptrap pose: {e}"))?;
            klap.read_particle_tracks(stage)
                .map_err(|e| anyhow::anyhow!("Klaptrap particle tracks: {e}"))?;
            kinds.push((ItemKind::Klap, klap));
            visual_archives.push((ItemKind::Klap, std::sync::Arc::new(stage.clone())));
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
        // Flat Zone's tools (it_802EEFA8), from GrFz.dat's itemdata.
        if let Some(tools) = ItemAssets::from_stage_item(
            stage,
            ItemKind::Tools,
            &it_tools::ARTICLE_STATES,
            it_tools::SPECIAL_ATTRIBUTES,
            &[],
        )? {
            kinds.push((ItemKind::Tools, tools));
            visual_archives.push((ItemKind::Tools, std::sync::Arc::new(stage.clone())));
        }
        // Ground_801C0800 (ground.c:493-498): a stage with ALDYakuAll
        // scripts its material item (it_802E6AEC, Jungle Japes' Klaptrap).
        if let Some(yaku) = ItemAssets::from_yaku(&common_archive, public, stage)? {
            kinds.push((ItemKind::PokemonRandom, yaku));
        }
        // it_80277040 reads it_804D6D28 for every kind.
        for (_, assets) in &mut kinds {
            assets.slide_rule = common.slide_rule;
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
    /// Read-only: the kind's assets when this scene registered it.
    pub fn try_get(&self, kind: ItemKind) -> Option<&ItemAssets> {
        self.kinds.iter().find(|(k, _)| *k == kind).map(|(_, a)| a)
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
            !item.destroyed
                && (SceneItems::logic(item.kind).tracker)(item) == Some((owner, secondary))
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
    gobjs: &mut hsd_gobj::GObjAddresses,
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
            let member = request_one_of_chain(
                pool, gobjs, resources, map, world, objects, spawn, &owner, rng,
            );
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
        ItemRequest::Spawn(spawn) | ItemRequest::SpawnInGroup(spawn) => (spawn, None, None),
        ItemRequest::NewHitGroup => {
            let slot = owner.slot.expect("Item_8026AE60 from a fighter");
            pool.begin_owner_hit_group(slot, owner.secondary);
            return None;
        }
        ItemRequest::SpawnHeld(spawn) => (spawn, None, owner.held_item),
        ItemRequest::SpawnLaser {
            spawn,
            angle,
            speed,
            motion,
        } => (spawn, Some((angle, speed, motion)), None),
        ItemRequest::ControlCurrent { .. } => {
            unreachable!("current article control must resolve after creation")
        }
        ItemRequest::ControlInstance { item, control } => {
            let kind = pool.get_mut(item).expect("article instance control").kind;
            pool.control_instance::<SceneItems>(
                item,
                control,
                resources.get(kind),
                owner_context.efasync_immediate,
            );
            return None;
        }
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
            dropped,
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
            // Item_8026AC74 (a drop) runs the kind's dropped callback.
            let callback = if dropped {
                SceneItems::logic(thrown.kind).dropped
            } else {
                SceneItems::logic(thrown.kind).thrown
            };
            callback(
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
            // Item_8026AC74 only: a dropped stage enemy is nobody's.
            if dropped {
                release_stage_enemy(thrown);
            }
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
            // Freed with this proc's other articles, in request order.
            pool.get_mut(item).expect("a held item").destroyed = true;
            return None;
        }
        ItemRequest::SpawnInHand { spawn, .. } => (spawn, None, None),
        ItemRequest::SpawnChain { .. } => unreachable!("handled above"),
        // The fighter's own loop runs these with its state (redead_request).
        ItemRequest::ReDeadCling { .. } | ItemRequest::ReDeadRelease { .. } => {
            unreachable!("ReDead requests run with their fighter")
        }
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
            // (it_80273F34), then it_802754D4; a stage enemy (it_8026B6C8)
            // is then nobody's and not grabbable (it_80275390).
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
            release_stage_enemy(dropped);
            return None;
        }
    };
    let assets = resources.get(spawn.kind);
    let id = pool.spawn_with_stale::<SceneItems>(spawn, assets, owner.stale_multiplier, gobjs)?;
    finish_spawn_request(pool, resources, map, world, objects, id, spawn, request,
        owner, rng, ray, held_owner, false);
    Some(id)
}

/// The common work after Item_8026A810, shared by ordinary and live-facing constructors.
#[allow(clippy::too_many_arguments)]
fn finish_spawn_request(
    pool: &mut ItemPool,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    id: u32,
    spawn: melee_it::SpawnItem,
    request: ItemRequest,
    owner: RequestOwner<'_>,
    rng: &mut gekko_math::HsdRng,
    ray: Option<(f32, f32, u16)>,
    held_owner: Option<&melee_it::ItemOwner>,
    collision_initialized: bool,
) {
    let assets = resources.get(spawn.kind);
    let item = pool.get_mut(id).unwrap();
    item.owner_secondary = owner.secondary && spawn.owner == owner.slot;
    if SceneItems::logic(spawn.kind).new_hit_group {
        // SpawnItem.x40 = Item_8026AE60(): xAC4 before the first script.
        let group = pool.allocate_hit_group();
        pool.get_mut(id).unwrap().hit_group = group;
    }
    let item = pool.get_mut(id).unwrap();
    if !collision_initialized {
        item.initialize_collision(spawn, assets, map);
    }
    item.past_hitbox_refresh = owner.after_hitbox_refresh;
    // A spawner at s_link 9 or later (an owner's accessory4): the new
    // item's efAsync_Spawn dispatches at once, ahead of the next
    // fighter's link 9 flush (efasync.c:1458, retail 800679B0).
    item.efasync_immediate = owner.efasync_immediate;
    if let Some((angle, speed, motion)) = ray {
        it_foxlaser::initialize_laser(pool.get_mut(id).unwrap(), assets, angle, speed, motion);
    }
    // The spawner's own set-up after Item_80268B18 (it_802BE2E8,
    // it_802BD4AC's turnip fields before its Item_8026AB54).
    let common = pool.common().clone();
    // A launch whose draws the spawner reserved (SpawnItem::launch_seed)
    // makes them from the stream's position at the request.
    let mut reserved = spawn.launch_seed.map(|seed| gekko_math::HsdRng { seed });
    (SceneItems::logic(spawn.kind).launched)(
        pool.get_mut(id).unwrap(),
        assets,
        &common,
        &spawn,
        reserved.as_mut().unwrap_or(rng),
    );
    (SceneItems::logic(spawn.kind).spawned_with_map)(
        pool.get_mut(id).unwrap(),
        assets,
        &common,
        &spawn,
        map,
    );
    if let ItemRequest::SpawnInGroup(_) = request {
        // Item_80268B18: xAC4_ignoreItemID = SpawnItem.x40, the id the
        // spawning fighter holds. The hitboxes the first script step
        // made take the group's history (it_8026FCF8), as a chain
        // member's do.
        let slot = owner.slot.expect("a grouped spawn from a fighter");
        let group = pool.owner_hit_group(slot, owner.secondary);
        pool.join_hit_group(id, group);
    }
    if SceneItems::logic(spawn.kind).procs_at_spawn {
        let bounds = owner.bounds.expect("it_802750F8 from a fighter's spawn");
        // it_802750F8: physics and collision now, without Item_802696CC's
        // blast-zone test (xDCC b3 cleared, then set again).
        pool.get_mut(id).unwrap().blast_zone_checked = false;
        let cell = std::cell::Cell::new(*rng);
        pool.physics::<SceneItems>(id, None, &Default::default(), bounds, assets, map, &cell);
        *rng = cell.get();
        let contact = pool.stage_contact(id, map);
        pool.collide::<SceneItems>(id, None, contact, map, bounds, assets, Some(&cell));
        *rng = cell.get();
        if let Some(item) = pool.get_mut(id) {
            item.blast_zone_checked = true;
        }
    }
    if let ItemRequest::SpawnInHand {
        part,
        scale_by_owner,
        ..
    } = request
    {
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
                // Sampled only for kinds with PICKUP_READS_OWNER.
                owner: owner.held_item,
                holder: None,
                map,
                assets,
                partner: None,
                rng: None,
            },
        );
        if scale_by_owner {
            // it_80274594: scl times ftLib_80086A0C, the owner's model
            // scale, through the model, hitbox radii, grab range and
            // boxes again.
            let scale = item.scale * item.owner_model_scale();
            item.rescale(scale);
        }
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
    let item = pool.get_mut(id).unwrap();
    item.past_hitbox_refresh = false;
    item.efasync_immediate = false;
    let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
    for phase in melee_it::ITEM_PROCESS_LINKS {
        world.add_tagged_proc(object, phase, tag(id, phase));
    }
    objects.push((id, object));
}

/// One member of an [`ItemRequest::SpawnChain`], spawned as an ordinary
/// request for the same owner.
#[allow(clippy::too_many_arguments)] // Item pool, scene objects and the shared RNG stay separate.
fn request_one_of_chain(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
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
        gobjs,
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
    gobjs: &mut hsd_gobj::GObjAddresses,
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
        gobjs,
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
        let item = pool.iter_mut().last().expect("spawned Bob-omb");
        it_bombhei::light(item, resources.get(ItemKind::BombHei));
    }
}

/// fn_8026C88C's spawn (8026C8FC..8026C96C): a common item of `kind` from
/// thin air at `position`, falling (Item_80268B18) with its first collision
/// (x44_flag.b0). Returns the new item, if the pool admitted it.
#[allow(clippy::too_many_arguments)]
pub fn spawn_random_item(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    kind: ItemKind,
    position: hsd_types::Vec3,
    facing: f32,
    rng: &mut gekko_math::HsdRng,
) -> Option<u32> {
    if resources.try_get(kind).is_none() {
        // The kind's it/kinds/*.c file: each unported random item fails
        // closed here, named, rather than inside its first callback.
        unimplemented!("random item {kind:?}: its it/kinds file is not ported");
    }
    let spawn = melee_it::SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind,
        // Item_802674AC: Food is hold kind 2, the other common items
        // (below It_Kind_L_Gun_Ray) 0.
        hold_kind: if kind == ItemKind::Foods { 2 } else { 0 },
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: hsd_types::Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: true,
        auxiliary_flags: [0; 3],
        ground_or_air: melee_types::GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    };
    request(
        pool,
        gobjs,
        resources,
        map,
        world,
        objects,
        ItemRequest::Spawn(spawn),
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
    )
}

/// What it_8026F5C8 gives an item a container lets out.
pub struct Released {
    pub kind: ItemKind,
    /// it_8026BB88: the container's ECB centre.
    pub position: hsd_types::Vec3,
    /// The container's position on the stage plane.
    pub previous_position: hsd_types::Vec3,
    pub facing: f32,
    /// The container as SpawnItem.x4_parent_gobj2: its attack (it_8027B0C4,
    /// 0x8027B164). SpawnItem.x0_parent_gobj is NULL: a thrown container's
    /// thrower does not own what it lets out.
    pub attack: Option<melee_types::combat::AttackInstance>,
}

/// it_8026F5C8 (8026F5C8): a released item, falling (Item_80268B18) with
/// its first collision, then spinning at ItCo +70 (it_80274658).
#[allow(clippy::too_many_arguments)]
pub fn spawn_released_item(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    released: Released,
    rng: &mut gekko_math::HsdRng,
) -> Option<u32> {
    if resources.try_get(released.kind).is_none() {
        unimplemented!(
            "item {:?} released from a container: its it/kinds file is not ported",
            released.kind
        );
    }
    let spawn = released_spawn(released, 0);
    let id = request(
        pool,
        gobjs,
        resources,
        map,
        world,
        objects,
        ItemRequest::Spawn(spawn),
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
    )?;
    let spin = pool.common().random_items.release_spin_degrees;
    pool.get_mut(id).expect("the released item").update_spin(spin);
    Some(id)
}

fn released_spawn(released: Released, hold_kind: u8) -> melee_it::SpawnItem {
    melee_it::SpawnItem {
        owner: None,
        stale_source: released.attack,
        attack_from_spawner: true,
        link_gobjs: 0,
        secondary_owner: None,
        kind: released.kind,
        hold_kind,
        spawn_variant: 0,
        position: released.position,
        previous_position: released.previous_position,
        velocity: hsd_types::Vec3::ZERO,
        facing: released.facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: true,
        auxiliary_flags: [0; 3],
        ground_or_air: melee_types::GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// it_8026F5C8 for a monster: two independent facing queries surround the
/// ordinary initial collision and the constructor's shared monster setup.
#[allow(clippy::too_many_arguments)]
pub fn spawn_released_monster(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    released: Released,
    fighters: &mut [crate::scene_fighter::SceneFighter],
    rng: &mut gekko_math::HsdRng,
) -> Option<u32> {
    let spawn = released_spawn(released, 6);
    let assets = resources.try_get(spawn.kind).unwrap_or_else(|| {
        unimplemented!(
            "it_8026F6BC: unwitnessed monster constructor {:?}",
            spawn.kind
        )
    });
    let id =
        pool.spawn_with_facing_constructor::<SceneItems>(spawn, assets, gobjs, map, |position| {
            facing_toward_fighters(position, fighters, rng)
        })?;
    finish_spawn_request(
        pool,
        resources,
        map,
        world,
        objects,
        id,
        spawn,
        ItemRequest::Spawn(spawn),
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
        None,
        None,
        true,
    );
    let spin = pool.common().random_items.release_spin_degrees;
    pool.get_mut(id)
        .expect("the released monster")
        .update_spin(spin);
    Some(id)
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

/// A free item's model bone as a generator kept for its particles reads it
/// after the item's tree is torn down
/// ([`melee_it::pose::ItemPose::detached_bone_matrix`]): the bone's local
/// SRT alone. `None` for a kind without a sampled pose.
pub fn free_item_detached_bone_matrix(
    resources: &Resources,
    item: &melee_it::ItemCore,
    bone: usize,
) -> Option<hsd_types::Mtx> {
    let pose = resources.get(item.kind).pose.as_ref()?;
    let animated = SceneItems::logic(item.kind).states[usize::from(item.motion)].animation_id >= 0;
    if animated {
        return Some(pose.detached_bone_matrix(
            Some((item.article_state, item.pose_steps)),
            bone,
            None,
        ));
    }
    let scale = match &item.scratch {
        melee_it::ItemScratch::ClimbersIce(ice) => Some((1, ice.scale)),
        _ => None,
    };
    Some(pose.detached_bone_matrix(None, bone, scale))
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
    let logic = SceneItems::logic(item.kind);
    let Some((owner, secondary)) = (logic.tracker)(item) else {
        return;
    };
    if item.owner_notified || !(logic.notifies_owner)(item) {
        return;
    }
    for fighter in fighters.iter_mut() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            if f.player.id == owner && f.player.secondary == secondary {
                f.article_destroyed(item.kind);
                f.article_retired(item.kind, item.id);
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
    gobjs: &mut hsd_gobj::GObjAddresses,
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
        gobjs,
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

/// grMaterial_801C8CFC -> it_802E6AEC during stage creation: the stage
/// material item on a JObj at `anchor`. Its scheduler object is made with
/// the scene's ([`register_existing`]).
pub(crate) fn spawn_yaku(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    anchor: melee_it::YakuAnchor,
) -> u32 {
    let spawn = it_yaku::spawn(anchor.position);
    let assets = resources.get(ItemKind::PokemonRandom);
    let id = pool
        .spawn::<SceneItems>(spawn, assets, gobjs)
        .expect("the stage material item");
    let item = pool.get_mut(id).unwrap();
    item.initialize_collision(spawn, assets, map);
    it_yaku::set_anchor(item, anchor);
    id
}

/// grMaterial_801C8CFC during the match (Yoshi's Island's blocks, at the
/// stage's start): [`spawn_yaku`] with its scheduler object.
pub(crate) fn spawn_yaku_live(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &Resources,
    map: &mut melee_mp::CollMap,
    world: &mut World,
    objects: &mut Objects,
    anchor: melee_it::YakuAnchor,
) -> u32 {
    let id = spawn_yaku(pool, gobjs, resources, map, anchor);
    let item = pool.get_mut(id).unwrap();
    item.past_hitbox_refresh = false;
    item.efasync_immediate = false;
    let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
    for phase in melee_it::ITEM_PROCESS_LINKS {
        world.add_tagged_proc(object, phase, tag(id, phase));
    }
    objects.push((id, object));
    id
}

/// The scheduler objects of the items a scene began with (a stage's
/// material item), in pool order.
pub(crate) fn register_existing(pool: &ItemPool, world: &mut World, objects: &mut Objects) {
    for item in pool.iter() {
        let object = world.create(6, melee_it::ITEM_GOBJ_LINK, melee_it::ITEM_GOBJ_PRIORITY);
        for phase in melee_it::ITEM_PROCESS_LINKS {
            world.add_tagged_proc(object, phase, tag(item.id, phase));
        }
        objects.push((item.id, object));
    }
}

/// Jungle Japes' one stage material item (kind 0xA0).
fn yaku_mut(pool: &mut ItemPool) -> &mut melee_it::ItemCore {
    pool.iter_mut()
        .find(|item| item.kind == ItemKind::PokemonRandom && !item.destroyed)
        .expect("the stage material item")
}

/// Item_80268E5C(item, state, ITEM_ANIM_UPDATE) from the stage's proc.
pub(crate) fn change_yaku_state(state: &mut crate::initial_state::InitialState, motion: u16) {
    let assets = state.assets.items.get(ItemKind::PokemonRandom);
    it_yaku::change_state(yaku_mut(&mut state.items), motion, assets);
}

/// The stage moved the material item's JObj.
pub(crate) fn anchor_yaku(
    state: &mut crate::initial_state::InitialState,
    anchor: melee_it::YakuAnchor,
) {
    it_yaku::set_anchor(yaku_mut(&mut state.items), anchor);
}

/// Item_8026A8EC's end for `item`, marked destroyed: its GObjs and its
/// scene object go.
pub fn cleanup(
    pool: &mut ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    world: &mut World,
    objects: &mut Objects,
    item: u32,
) {
    destroy_object(world, objects, item);
    pool.remove_destroyed::<SceneItems>(item, gobjs);
}

/// HSD_GObjPLink_80390228 for an item: its scene object goes too.
fn destroy_object(world: &mut World, objects: &mut Objects, item: u32) {
    let index = objects
        .iter()
        .position(|(id, _)| *id == item)
        .expect("item GObj");
    world.destroy(objects.remove(index).1);
}

/// it_802E18B4 -> it_8027B5B0, normal stage-item construction and proc registration.
pub(crate) fn spawn_klaptrap(
    state: &mut crate::initial_state::InitialState,
    world: &mut World,
    objects: &mut Objects,
    position: hsd_types::Vec3,
    facing: f32,
    parent: melee_it::StageParent,
) {
    let spawn = melee_it::SpawnItem {
        kind: ItemKind::Klap,
        hold_kind: 4,
        owner: None,
        secondary_owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        spawn_variant: 0,
        position,
        previous_position: position,
        velocity: hsd_types::Vec3::ZERO,
        facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: true,
        auxiliary_flags: [0; 3],
        ground_or_air: melee_types::GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    };
    let id = state
        .items
        .spawn_with_initial_collision::<SceneItems>(
            spawn,
            state.assets.items.get(ItemKind::Klap),
            &mut state.effects.gobjs,
            &mut state.map,
        )
        .expect("Klaptrap birth pool capacity");
    finish_spawn_request(
        &mut state.items,
        &state.assets.items,
        &mut state.map,
        world,
        objects,
        id,
        spawn,
        ItemRequest::Spawn(spawn),
        RequestOwner {
            slot: None,
            secondary: false,
            held_item: None,
            after_hitbox_refresh: false,
            efasync_immediate: false,
            stale_multiplier: 1.0,
            bounds: None,
        },
        &mut state.rng,
        None,
        None,
        true,
    );
    it_klap::attach_parent(state.items.get_mut(id).unwrap(), parent);
}

/// grFlatzone_80217EF0 -> it_802EEFA8 (0x802EEFA8): a falling tool, then
/// it_802EF098 starts tool `kind` and the stage limits its collision to the
/// ground floor (`x378_itemColl.joint_id_only = 0x32`).
pub(crate) fn spawn_tool(
    state: &mut crate::initial_state::InitialState,
    world: &mut World,
    objects: &mut Objects,
    position: hsd_types::Vec3,
    kind: i32,
    facing: f32,
) {
    let before = state.items.len();
    request(
        &mut state.items,
        &mut state.effects.gobjs,
        &state.assets.items,
        &mut state.map,
        world,
        objects,
        ItemRequest::Spawn(it_tools::spawn(position, facing)),
        RequestOwner {
            slot: None,
            secondary: false,
            held_item: None,
            after_hitbox_refresh: false,
            efasync_immediate: false,
            stale_multiplier: 1.0,
            bounds: None,
        },
        &mut state.rng,
    );
    if state.items.len() == before {
        unimplemented!("it_802EEFA8: no item slot for a tool");
    }
    let assets = state.assets.items.get(ItemKind::Tools);
    let id = state.items.iter().last().expect("the new tool").id;
    let half_life_scale = state.items.common().half_life_scale;
    let item = state.items.get_mut(id).expect("the new tool");
    it_tools::start(item, kind, assets, half_life_scale);
    item.collision
        .as_mut()
        .expect("tool collision")
        .joint_id_only = crate::scene_stage::flatzone::GROUND_JOINT;
}

/// ftCo_800C74F4's view of the ReDead behind a fighter's pending hit
/// (dmg.x1868_source): which item, and whether it already holds someone
/// (it_802EAF28), as it stands when the fighter's ProcessHit runs.
pub(crate) fn offer_redead_source(state: &mut crate::initial_state::InitialState, player: usize) {
    use melee_coll::damage_log::HitSource;
    let items = &state.items;
    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
        let combat = &mut f.core.combat;
        combat.redead_source = match (combat.pending.as_ref(), combat.pending_source) {
            (Some(hit), Some(HitSource::Item(source)))
                if hit.descriptor.element == melee_types::HitElement::ReDead =>
            {
                items
                    .iter()
                    .find(|item| {
                        !item.destroyed
                            && item.kind == ItemKind::Leadead
                            && item.gobj.index() == source.gobj
                    })
                    .map(|item| melee_ft::fighter::capture_redead::ReDeadSource {
                        item: item.id,
                        holding: it_monsters::redead::holds_fighter(item),
                    })
            }
            _ => None,
        };
    });
}

/// A fighter's request of the ReDead that holds it, run at once with the
/// fighter's own state: it_802EAAEC (`ReDeadCling`), it_802EAE80
/// (`ReDeadRelease`). False for every other request.
pub(crate) fn redead_request(
    items: &mut melee_it::ItemPool,
    assets: &Resources,
    map: &mut melee_mp::CollMap,
    request: &melee_it::ItemRequest,
    fighter: &mut melee_ft::fighter::Fighter,
    s_link: u8,
) -> bool {
    let depth = items.common().release_floor_depth;
    let redead = assets.get(ItemKind::Leadead);
    let id = match *request {
        melee_it::ItemRequest::ReDeadCling { item, .. }
        | melee_it::ItemRequest::ReDeadRelease { item, .. } => item,
        _ => return false,
    };
    // As every fighter request of an item: a script's hitbox made past
    // item link 11 is placed at once, and efAsync dispatches at once from
    // link 9 on.
    let item = items.get_mut(id).expect("the ReDead a fighter names");
    item.past_hitbox_refresh = s_link > 11;
    item.efasync_immediate = s_link >= 9;
    let handled = match *request {
        melee_it::ItemRequest::ReDeadCling { item, part } => {
            let position = fighter.core.physics.position;
            let view = fighter.core.redead_view();
            let captive = melee_it::ReDeadCaptive {
                player: fighter.player.id,
                secondary: fighter.player.secondary,
                part,
            };
            let item = items.get_mut(item).expect("the ReDead that caught a fighter");
            it_monsters::redead::cling(item, captive, position, map, redead);
            hold_root(item, &view);
            true
        }
        melee_it::ItemRequest::ReDeadRelease { item, view } => {
            let item = items.get_mut(item).expect("the ReDead holding a fighter");
            it_monsters::redead::set_captive_view(item, view);
            it_monsters::redead::let_go(item, map, redead, depth);
            true
        }
        _ => false,
    };
    let item = items.get_mut(id).expect("the ReDead a fighter names");
    item.past_hitbox_refresh = false;
    item.efasync_immediate = false;
    handled
}

fn part_position(view: &melee_it::CaptiveView) -> hsd_types::Vec3 {
    hsd_types::Vec3::new(view.part.0[0][3], view.part.0[1][3], view.part.0[2][3])
}

/// lb_8000C1C0's constraint, evaluated when a clinging ReDead's bones are
/// read: its root at its fighter's part, and that fighter as it stands.
pub(crate) fn refresh_redead_hold(state: &mut crate::initial_state::InitialState, id: u32) {
    let Some(item) = state.items.get_mut(id) else {
        return;
    };
    let melee_it::ItemScratch::Monster(monster) = &item.scratch else {
        return;
    };
    let melee_it::MonsterBehavior::ReDead(redead) = &monster.behavior else {
        return;
    };
    let Some(captive) = redead.captive else {
        return;
    };
    let index = owner_index(&state.fighters, Some(captive.player), captive.secondary)
        .expect("the fighter a ReDead holds");
    assert_eq!(
        captive.part,
        melee_types::FtPart::TopN as u8,
        "it_802EAAEC: a ReDead holds TopN"
    );
    let view = crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
        .core
        .redead_view());
    let item = state.items.get_mut(id).expect("live ReDead");
    hold_root(item, &view);
    it_monsters::redead::set_captive_view(item, view);
}

/// The constraint's resolution (HSD_RObjUpdateAll -> JObjUpdateFunc 0x35,
/// then 0x38): the root's world translation is the fighter part's, and
/// the root's own translate is read back from it, so the item push
/// (it_80271D2C's HSD_JObjGetTranslation) sees the fighter's part too.
/// Every matrix setup does it, the display pass among them.
fn hold_root(item: &mut melee_it::ItemCore, view: &melee_it::CaptiveView) {
    let position = part_position(view);
    item.constrained_root = Some(position);
    item.root_translation = position;
}

/// it_802E8CD8 (itleadead.c:173-176): a hit on a ReDead holding a fighter
/// lets the fighter go before the rest of the damage callback, whose
/// Item_80269C5C proc this precedes.
pub(crate) fn release_redead_on_hit(state: &mut crate::initial_state::InitialState, id: u32) {
    let depth = state.items.common().release_floor_depth;
    let Some(item) = state.items.get_mut(id) else {
        return;
    };
    if item.kind != ItemKind::Leadead
        || item.destroyed
        || (item.pending_knockback == 0.0 && item.pending_damage_taken == 0)
    {
        return;
    }
    it_monsters::redead::release_on_hit(
        item,
        &mut state.map,
        state.assets.items.get(ItemKind::Leadead),
        depth,
    );
}

/// it_8026B6C8 (8026B6C8): a stage enemy (the crate monsters and the
/// adventure maps' kinds, but for Kyasarin's egg and Whispy's apple), which
/// a drop leaves nobody's and ungrabbable (it_80275390).
fn release_stage_enemy(item: &mut melee_it::ItemCore) {
    let kind = item.kind as u32;
    let monster = (ItemKind::Kuriboh as u32..ItemKind::OctarockStone as u32).contains(&kind)
        || (ItemKind::OldKuri as u32..ItemKind::ArwingLaser as u32).contains(&kind);
    if monster && !matches!(item.kind, ItemKind::KyasarinEgg | ItemKind::WhispyApple) {
        item.owner = None;
        item.grabbable = false;
    }
}
