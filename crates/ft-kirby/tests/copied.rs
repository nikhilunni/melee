use ft_fox_family::special_n::Blaster;
use ft_kirby::{
    attributes::read_kirby_attributes,
    copied::{
        blaster::{KirbyFalco, KirbyFox},
        CopiedNeutral, NEUTRAL,
    },
    hat,
};
use hsd_archive::Archive;
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
