//! Complete original common swing graphs, using the production archive/command loader.
use super::*;
use std::path::PathBuf;

struct ScriptAudit {
    owners: usize,
    copies: usize,
    scripts: usize,
    nonnull: usize,
    writers: Vec<(String, usize)>,
}

fn audit_original_scripts(
    indices: std::ops::Range<usize>,
    requests_followup: impl Fn(&Command) -> bool,
) -> ScriptAudit {
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
            for index in indices.clone() {
                scripts += 1;
                let Some(offset) = table.script_pointer(&archive, index).unwrap() else {
                    continue;
                };
                nonnull += 1;
                let mut commands = BTreeMap::new();
                // Includes every Call and Goto body, even beyond waits; loop
                // bodies are read in full. Repeated execution cannot add writers.
                read_script(&archive, offset, &mut commands).unwrap();
                if commands.values().any(&requests_followup) {
                    writers.push((
                        path.file_name().unwrap().to_str().unwrap().to_owned(),
                        index,
                    ));
                }
            }
        }
    }
    ScriptAudit {
        owners,
        copies,
        scripts,
        nonnull,
        writers,
    }
}

#[test]
fn original_swing_command_owners_only_captain_requests_followup() {
    let ScriptAudit {
        owners,
        copies,
        scripts,
        nonnull,
        writers,
    } = audit_original_scripts(108..132, |c| matches!(c, Command::GrabRelease));
    assert_eq!((owners, copies, scripts, nonnull), (33, 25, 792, 744));
    assert_eq!(
        writers,
        [110, 114, 118, 122, 126, 130].map(|i| ("PlCa.dat".to_owned(), i))
    );
    eprintln!("Swing42 original owners={owners}, copy roots={copies}, scripts={scripts}, nonnull={nonnull}; six Captain-only writers");
}

#[test]
fn original_forward_smash_graphs_keep_ness_private_writer_exception() {
    let ScriptAudit {
        owners,
        copies,
        scripts,
        nonnull,
        writers,
    } = audit_original_scripts(60..65, |c| {
        if let Command::SetVariable { index: 0, value } = c {
            if *value != 0 {
                assert_eq!(*value, 1, "original common S4 window value");
                return true;
            }
        }
        false
    });
    assert_eq!((owners, copies, scripts, nonnull), (33, 25, 165, 135));
    // Ness is a real authored writer. Its private bat row, not common S4,
    // owns the IASA; the live ownership test checks that separate join.
    assert_eq!(
        writers,
        ["PlCl.dat", "PlLk.dat", "PlNs.dat"].map(|name| (name.to_owned(), 62))
    );
    eprintln!("S4 original owners={owners}, copy roots={copies}, scripts={scripts}, nonnull={nonnull}; exact Link/YoungLink/Ness writers");
}
