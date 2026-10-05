//! Menu art decoded from the real disc image: every piece the menus ask for
//! exists at its native size, distinct pieces differ, and decoding is
//! deterministic. Nothing decoded is stored: the checks compare images
//! decoded in this run with each other.
use melee_lib::{Character, Stage};
use melee_platform::{
    app::App,
    art::{Image, Piece},
    catalog,
    disc::DiscFiles,
};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

fn iso() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let iso = root.join("harness/roms/GALE01.iso");
    melee_test_support::require_files([&iso]).then_some(iso)
}

fn app_with_art(iso: &Path) -> App {
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(iso).unwrap()).unwrap();
    assert!(!app.art_ready());
    app.load_art().unwrap();
    assert!(app.art_ready());
    app
}

/// Every piece the catalog can ask for, with its expected size; `None`
/// where retail has no such image.
fn pieces() -> Vec<(Piece, Option<(u32, u32)>)> {
    let mut out = Vec::new();
    for &character in catalog::characters() {
        let sheik = character == Character::Sheik;
        let unless_sheik = |size| (!sheik).then_some(size);
        for costume in 0..character.costume_count() {
            out.push((Piece::Portrait(character, costume), unless_sheik((136, 188))));
            out.push((Piece::Stock(character, costume), Some((24, 24))));
        }
        out.push((Piece::Face(character), unless_sheik((64, 56))));
        out.push((Piece::CharacterEmblem(character), Some((80, 64))));
    }
    for &stage in catalog::stages() {
        // The Past Stages row has smaller icons.
        let icon = if stage == Stage::DreamLand { (48, 48) } else { (64, 56) };
        out.push((Piece::StageIcon(stage), Some(icon)));
        out.push((Piece::StageName(stage), Some((224, 56))));
        out.push((Piece::StageEmblem(stage), Some((64, 64))));
    }
    out
}

fn fingerprint(image: &Image) -> u64 {
    let mut hasher = DefaultHasher::new();
    (image.width, image.height, &image.rgba).hash(&mut hasher);
    hasher.finish()
}

#[test]
fn every_menu_piece_decodes_at_its_native_size() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = app_with_art(&iso);
    for (piece, size) in pieces() {
        match (app.art_image(piece), size) {
            (Ok(image), Some((w, h))) => {
                assert_eq!((image.width, image.height), (w, h), "{piece:?}");
                assert_eq!(image.rgba.len(), (w * h * 4) as usize, "{piece:?}");
                assert!(
                    image.rgba.chunks_exact(4).any(|p| p[3] != 0),
                    "{piece:?} is fully transparent"
                );
            }
            (Err(error), None) => assert!(error.contains("Sheik"), "{piece:?}: {error}"),
            (result, size) => panic!("{piece:?}: expected {size:?}, got {result:?}"),
        }
    }
}

#[test]
fn distinct_pieces_show_distinct_images() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = app_with_art(&iso);
    // Portraits, stocks, faces, stage icons and names are one per pick;
    // emblems repeat across a series by design.
    let mut seen: HashMap<(u8, u64), Piece> = HashMap::new();
    for (piece, _) in pieces() {
        let kind = match piece {
            Piece::Portrait(..) => 0,
            Piece::Stock(..) => 1,
            Piece::Face(_) => 2,
            Piece::StageIcon(_) => 3,
            Piece::StageName(_) => 4,
            Piece::CharacterEmblem(_) | Piece::StageEmblem(_) | Piece::StagePreview(_) => continue,
        };
        let Ok(image) = app.art_image(piece) else {
            continue;
        };
        if let Some(other) = seen.insert((kind, fingerprint(&image)), piece) {
            panic!("{piece:?} shows the same image as {other:?}");
        }
    }
    // Series emblems: Mario and Dr. Mario share one, Fox and Marth do not.
    let mut emblem = |c| fingerprint(&app.art_image(Piece::CharacterEmblem(c)).unwrap());
    assert_eq!(emblem(Character::Mario), emblem(Character::DrMario));
    assert_eq!(emblem(Character::Zelda), emblem(Character::Sheik));
    assert_ne!(emblem(Character::Fox), emblem(Character::Marth));
}

#[test]
fn decoding_is_deterministic_and_cached() {
    let Some(iso) = iso() else {
        return;
    };
    let mut first = app_with_art(&iso);
    let mut second = app_with_art(&iso);
    for (piece, _) in pieces() {
        let (Ok(a), Ok(b)) = (first.art_image(piece), second.art_image(piece)) else {
            continue;
        };
        assert_eq!(fingerprint(&a), fingerprint(&b), "{piece:?}");
        let again = first.art_image(piece).unwrap();
        assert!(std::sync::Arc::ptr_eq(&a, &again), "{piece:?} is cached");
    }
    assert!(first
        .art_image(Piece::StageIcon(Stage::FinalDestination))
        .is_ok());
}
