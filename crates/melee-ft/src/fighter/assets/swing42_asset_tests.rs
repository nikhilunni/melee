//! Complete original common swing graphs, using the production archive/command loader.
use super::*;
use std::path::PathBuf;

#[test]
fn original_swing_command_owners_only_captain_requests_followup() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let files = root.join("harness/roms/files");
    assert!(melee_test_support::require_files([files.join("PlCa.dat")]));
    let mut paths: Vec<_> = std::fs::read_dir(&files)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_str().unwrap();
            name.starts_with("Pl") && (name.ends_with(".dat") || name.ends_with(".usd"))
        })
        .collect();
    paths.sort();
    let (mut owners, mut copies, mut scripts, mut nonnull) = (0, 0, 0, 0);
    let mut writers = Vec::new();
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        // AJ files concatenate archive banks. Their first public header is
        // sufficient for identifying ftData owners, not for decoding AJ data.
        let header = hsd_archive::ArchiveHeader::parse(&bytes).unwrap();
        let archive = Archive::parse(&bytes[..header.file_size as usize]).unwrap();
        for public in archive
            .publics()
            .iter()
            .filter(|p| p.name.starts_with("ftData"))
        {
            if public.name.starts_with("ftDataKirbyCopy") {
                copies += 1;
                continue;
            }
            owners += 1;
            assert_eq!(path.extension().unwrap(), "dat");
            let table = crate::desc::read_fighter_animations(&archive, public.offset, 132).unwrap();
            for index in 108..132 {
                scripts += 1;
                let Some(offset) = table.script_pointer(&archive, index).unwrap() else {
                    continue;
                };
                nonnull += 1;
                let mut commands = BTreeMap::new();
                // Includes every Call and Goto body, even beyond waits; loop
                // bodies are read in full. Repeated execution cannot add writers.
                read_script(&archive, offset, &mut commands).unwrap();
                if commands.values().any(|c| matches!(c, Command::GrabRelease)) {
                    writers.push((
                        path.file_name().unwrap().to_str().unwrap().to_owned(),
                        index,
                    ));
                }
            }
        }
    }
    assert_eq!((owners, copies, scripts, nonnull), (33, 25, 792, 744));
    assert_eq!(
        writers,
        [110, 114, 118, 122, 126, 130].map(|i| ("PlCa.dat".to_owned(), i))
    );
    eprintln!("Swing42 original owners={owners}, copy roots={copies}, scripts={scripts}, nonnull={nonnull}; six Captain-only writers");
}
