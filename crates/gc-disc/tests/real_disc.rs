//! The real NTSC-U 1.02 image, read in place and never written.
use gc_disc::Disc;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

/// `MELEE_DATA_ROOT` points a worktree at the main checkout's data.
fn roms() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .join("harness/roms")
}

#[test]
fn the_retail_disc_opens_and_its_files_match_the_extraction() {
    let roms = roms();
    let iso = roms.join("GALE01.iso");
    let files = roms.join("files");
    if !melee_test_support::require_files([&iso, &files.join("PlCo.dat")]) {
        return;
    }
    let mut image = File::open(&iso).unwrap();
    let len = image.metadata().unwrap().len();
    let disc = Disc::open(len, |range| {
        image.seek(SeekFrom::Start(range.start))?;
        let mut bytes = vec![0; (range.end - range.start) as usize];
        image.read_exact(&mut bytes)?;
        Ok::<_, std::io::Error>(bytes)
    })
    .unwrap();
    // docs/DISC.md: 1,209 files; audio/us/ is the nested directory.
    assert_eq!(disc.header.game_id, "GALE01");
    assert_eq!(disc.header.revision, 2);
    assert_eq!(disc.header.fst_offset, 0x456E00);
    assert_eq!(disc.fst.files().len(), 1209);
    for name in [
        "PlCo.dat",
        "PlFxNr.dat",
        "GrNLa.dat",
        "IfAll.usd",
        "audio/us/1padv.ssm",
    ] {
        let entry = disc
            .fst
            .file(name)
            .unwrap_or_else(|| panic!("{name} missing"));
        image.seek(SeekFrom::Start(entry.range().start)).unwrap();
        let mut bytes = vec![0; entry.len as usize];
        image.read_exact(&mut bytes).unwrap();
        let extracted = std::fs::read(files.join(name)).unwrap();
        assert!(bytes == extracted, "{name} differs from the extraction");
    }
}
