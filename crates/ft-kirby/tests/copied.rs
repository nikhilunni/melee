use ft_fox_family::special_n::Blaster;
use ft_kirby::{
    attributes::read_kirby_attributes,
    copied::{
        blaster::{KirbyFalco, KirbyFox},
        shield_breaker::{KirbyMarth, KirbyRoy},
        rollout::KirbyPurin,
        CopiedNeutral, NEUTRAL,
    },
    hat,
};
use ft_mars_family::special_n::{ShieldBreaker, ShieldBreakerRow};
use ft_purin_family::{RolloutAttributes, RolloutKind, RolloutRow};
use hsd_archive::Archive;
use melee_ft::fighter::ActionId;
use melee_types::FighterKind;
use std::path::{Path, PathBuf};

#[test]
fn the_copied_neutral_table_ports_its_kinds_and_names_every_other_entry() {
    let inhale = [
        FighterKind::Kirby,
        FighterKind::Nana,
        FighterKind::MasterH,
        FighterKind::CrezyH,
        FighterKind::Boy,
        FighterKind::Girl,
        FighterKind::Sandbag,
    ];
    for (index, entry) in NEUTRAL.iter().enumerate() {
        let kind = FighterKind::try_from(index as i32).unwrap();
        match entry {
            CopiedNeutral::Inhale => assert!(inhale.contains(&kind), "{kind:?}"),
            CopiedNeutral::Ported(_) => {
                assert!(
                    matches!(
                        kind,
                        FighterKind::Fox
                            | FighterKind::Falco
                            | FighterKind::Captain
                            | FighterKind::Ganon
                            | FighterKind::Mars
                            | FighterKind::Emblem
                            | FighterKind::Purin
                            | FighterKind::GameWatch
                            | FighterKind::Mario
                            | FighterKind::DrMario
                            | FighterKind::Luigi
                            | FighterKind::Samus
                            | FighterKind::Donkey
                            | FighterKind::Mewtwo
                            | FighterKind::Pikachu
                            | FighterKind::Pichu
                            | FighterKind::Peach
                            | FighterKind::Ness
                            | FighterKind::Yoshi
                            | FighterKind::Link
                            | FighterKind::CLink
                            | FighterKind::Zelda
                            | FighterKind::Koopa
                            | FighterKind::Popo
                            | FighterKind::Seak
                    ),
                    "{kind:?}"
                )
            }
            CopiedNeutral::Unported { ground, air } => {
                assert!(ground.starts_with("ftKb_SpecialN"), "{kind:?}");
                assert!(air.starts_with("ftKb_SpecialN"), "{kind:?}");
            }
        }
    }
}

#[test]
fn the_blaster_rows_follow_kirbys_table() {
    for (rows, start, animation) in [(KirbyFox::ROWS, 423, 358), (KirbyFalco::ROWS, 520, 455)] {
        assert_eq!(rows.first_action, start);
        assert_eq!(rows.first_animation, animation);
        assert_eq!(rows.index(start + 1), Some(1));
        assert_eq!(rows.index(start + 6), None);
    }
}

#[test]
fn the_punch_rows_follow_kirbys_table() {
    let rows = ft_kirby::copied::captain::rows();
    let expected = [(433, 368), (434, 369), (528, 463), (529, 464)];
    for (row, (action, animation)) in rows.iter().zip(expected) {
        assert_eq!(row.action.0, action);
        assert_eq!(row.animation, animation);
    }
}

#[test]
fn the_egg_lay_rows_follow_kirbys_table() {
    // ftKb_Init_MotionStateTable 413..422: the item rows (415, 417, 420,
    // 422) fail closed; the others play ftKb_SM_YsSpecialN1 and on.
    let rows = ft_kirby::copied::egg_lay::rows();
    let expected = [
        (413, Some(352)),
        (414, Some(353)),
        (415, None),
        (416, Some(354)),
        (417, None),
        (418, Some(355)),
        (419, Some(356)),
        (420, None),
        (421, Some(357)),
        (422, None),
    ];
    for (row, (action, animation)) in rows.iter().zip(expected) {
        assert_eq!(row.action.0, action);
        assert_eq!(row.implemented, animation.is_some());
        if let Some(animation) = animation {
            assert_eq!(row.animation, animation);
        }
    }
}

#[test]
fn the_bow_rows_follow_kirbys_table() {
    // ftKb_MS_LkSpecialNStart..LkSpecialAirNEnd (401..406) play
    // ftKb_SM_LkSpecialNStart (340) and on; ftKb_MS_ClSpecialNStart.. (514..519)
    // ftKb_SM_ClSpecialNStart (449) and on.
    let rows = ft_kirby::copied::bow::rows();
    for (i, row) in rows.iter().enumerate() {
        let (action, animation) = if i < 6 { (401, 340) } else { (514 - 6, 449 - 6) };
        assert_eq!(row.action.0, action + i as u16);
        assert_eq!(row.animation, animation + i as i32);
    }
}

#[test]
fn the_pk_flash_rows_follow_kirbys_table() {
    // ftKb_MS_NsSpecialNStart..NsSpecialAirNEnd play ftKb_SM_NsSpecialNStart
    // and the seven after it.
    for (i, row) in ft_kirby::copied::pk_flash::rows().iter().enumerate() {
        assert_eq!(row.action.0, 435 + i as u16);
        assert_eq!(row.animation, 370 + i as i32);
    }
}

#[test]
fn the_nayrus_love_rows_follow_kirbys_table() {
    let rows = ft_kirby::copied::nayrus_love::rows();
    for (row, (action, animation)) in rows.iter().zip([(465, 400), (466, 401)]) {
        assert_eq!(row.action.0, action);
        assert_eq!(row.animation, animation);
    }
}

#[test]
fn the_fire_breath_rows_follow_kirbys_table() {
    let rows = ft_kirby::copied::fire_breath::rows();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.action.0, 443 + i as u16);
        assert_eq!(row.animation, 378 + i as i32);
    }
}

#[test]
fn the_ice_shot_rows_follow_kirbys_table() {
    let rows = ft_kirby::copied::ice_shot::rows();
    for (row, (action, animation)) in rows.iter().zip([(453, 388), (454, 389)]) {
        assert_eq!(row.action.0, action);
        assert_eq!(row.animation, animation);
    }
}

#[test]
fn the_fireball_rows_follow_kirbys_table() {
    let rows = ft_kirby::copied::fireball::rows();
    let expected = [
        (399, 338),
        (400, 339),
        (512, 447),
        (513, 448),
        (431, 366),
        (432, 367),
    ];
    for (row, (action, animation)) in rows.iter().zip(expected) {
        assert_eq!(row.action.0, action);
        assert_eq!(row.animation, animation);
    }
}

#[test]
fn the_charge_shot_rows_follow_kirbys_table() {
    // ftKb_Init_MotionStateTable rows 407..412 (0x803C8368): anim ids
    // 346..351.
    let rows = ft_kirby::copied::samus::rows();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.action.0, 407 + i as u16);
        assert_eq!(row.animation, 346 + i as i32);
    }
}

#[test]
fn the_giant_punch_rows_follow_kirbys_table() {
    // ftKb_Init_MotionStateTable rows 455..464 (0x803C8368): anim ids
    // 390..399.
    let rows = ft_kirby::copied::donkey::rows();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.action.0, 455 + i as u16);
        assert_eq!(row.animation, 390 + i as i32);
    }
}

#[test]
fn the_shadow_ball_rows_follow_kirbys_table() {
    // ftKb_Init_MotionStateTable rows 500..509 (0x803C8368): anim ids
    // 435..444.
    let rows = ft_kirby::copied::mewtwo::rows();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.action.0, 500 + i as u16);
        assert_eq!(row.animation, 435 + i as i32);
    }
}

#[test]
fn the_shield_breaker_rows_follow_kirbys_table() {
    for (rows, start, animation) in [(KirbyMarth::ROWS, 492, 427), (KirbyRoy::ROWS, 530, 465)] {
        assert_eq!(rows.first_action, start);
        assert_eq!(rows.first_animation, animation);
        assert_eq!(rows.action(ShieldBreakerRow::AirEnd1).0, start + 7);
    }
}

#[test]
fn the_rollout_rows_follow_kirbys_table() {
    let rows = KirbyPurin::ROWS;
    assert_eq!(rows.first_action, 475);
    assert_eq!(rows.first_animation, 410);
    assert_eq!(rows.row(ActionId(479)), Some(RolloutRow::Release));
    assert_eq!(rows.row(ActionId(491)), Some(RolloutRow::Hit));
    assert_eq!(rows.row(ActionId(492)), None);
    assert_eq!(rows.row(ActionId(474)), None);
}

/// ftKb_DatAttrs +2B4..+358 is Jigglypuff's block but for two words.
#[test]
fn real_kirby_data_holds_jigglypuffs_rollout_but_two_words() {
    let kirby = disc_file("PlKb.dat");
    let puff = disc_file("PlPr.dat");
    if !melee_test_support::require_files([&kirby, &puff]) {
        return;
    }
    let kirby = Archive::parse(&std::fs::read(kirby).unwrap()).unwrap();
    let puff = Archive::parse(&std::fs::read(puff).unwrap()).unwrap();
    let kirby = read_kirby_attributes(&kirby).unwrap().rollout;
    // ftDataPurin.ext_attr +34, read here without Jigglypuff's crate.
    let root = puff.public("ftDataPurin").unwrap();
    let offset = melee_ft::desc::special_attributes_offset(&puff, root).unwrap();
    let block = puff.reader().slice(offset + 0x34, 0xA8).unwrap();
    let puff = RolloutAttributes::read(&hsd_archive::Reader::new(block), 0).unwrap();
    assert_eq!(kirby.minimum_hitbox_speed, 1.0);
    assert_eq!(puff.minimum_hitbox_speed, 2.0);
    assert_eq!(kirby.turn_speed_ratio, 0.7);
    assert_eq!(puff.turn_speed_ratio, 0.25);
    let same = RolloutAttributes {
        minimum_hitbox_speed: puff.minimum_hitbox_speed,
        turn_speed_ratio: puff.turn_speed_ratio,
        ..kirby
    };
    assert_eq!(same, puff);
}

fn disc_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/roms/files")
        .join(name)
}

#[test]
fn real_kirby_data_names_the_copied_blasters_items() {
    let path = disc_file("PlKb.dat");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let archive = Archive::parse(&std::fs::read(path).unwrap()).unwrap();
    let attributes = read_kirby_attributes(&archive).unwrap();
    for (blaster, laser, gun) in [
        (attributes.fox_blaster, KirbyFox::LASER, KirbyFox::BLASTER),
        (
            attributes.falco_blaster,
            KirbyFalco::LASER,
            KirbyFalco::BLASTER,
        ),
    ] {
        assert_eq!(blaster.shot_item_kind, laser);
        assert_eq!(blaster.gun_item_kind, gun);
        // Both Blasters end in the air with Fall.
        assert_eq!(blaster.landing_lag, 0.0);
    }
}

/// KirbyHatStruct.hat_dynamics[1] (+0x10), the graft's part mask.
#[test]
fn real_copy_archives_hold_the_grafted_hats_part_masks() {
    let kinds = [
        (FighterKind::Donkey, "PlKbCpDk.dat", "ftDataKirbyCopyDonkey"),
        (FighterKind::Purin, "PlKbCpPr.dat", "ftDataKirbyCopyPurin"),
        (FighterKind::Mewtwo, "PlKbCpMt.dat", "ftDataKirbyCopyMewtwo"),
        (FighterKind::Falco, "PlKbCpFc.dat", "ftDataKirbyCopyFalco"),
        (
            FighterKind::GameWatch,
            "PlKbCpGw.dat",
            "ftDataKirbyCopyGamewatch",
        ),
    ];
    let paths: Vec<_> = kinds.iter().map(|(_, file, _)| disc_file(file)).collect();
    if !melee_test_support::require_files(&paths) {
        return;
    }
    for ((kind, _, symbol), path) in kinds.iter().zip(&paths) {
        let archive = Archive::parse(&std::fs::read(path).unwrap()).unwrap();
        let hat = archive.public(symbol).unwrap();
        let mask = archive.reader().u32(hat + 0x10).unwrap();
        assert_eq!(mask, hat::grafted_parts(*kind), "{kind:?}");
    }
}

#[test]
fn unwitnessed_giga_bowser_copy_stays_fail_closed() {
    assert!(matches!(
        ft_kirby::copied::NEUTRAL[i32::from(FighterKind::GKoops) as usize],
        ft_kirby::copied::CopiedNeutral::Unported { .. }
    ));
    for row in ft_kirby::special_rows().iter().filter(|row| (538..544).contains(&row.action.0)) {
        assert!(!row.implemented);
    }
}
