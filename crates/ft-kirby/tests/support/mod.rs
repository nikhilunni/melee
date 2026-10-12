use hsd_archive::{Archive, ArchiveHeader};

pub fn word(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

// Minimal independently encoded archive, following melee-mp/tests/desc.rs.
pub fn archive(data: &[u8], relocs: &[u32], public: Option<(&str, u32)>) -> Archive {
    let mut strings = Vec::new();
    if let Some((name, _)) = public {
        strings.extend_from_slice(name.as_bytes());
        strings.push(0);
    }
    let public_bytes = if public.is_some() { 8 } else { 0 };
    let header = ArchiveHeader {
        file_size: (ArchiveHeader::SIZE
            + data.len()
            + relocs.len() * 4
            + public_bytes
            + strings.len()) as u32,
        data_size: data.len() as u32,
        nb_reloc: relocs.len() as u32,
        nb_public: u32::from(public.is_some()),
        nb_extern: 0,
        version: *b"001B",
    };
    let mut bytes = header.to_bytes().to_vec();
    bytes.extend_from_slice(data);
    for slot in relocs {
        bytes.extend_from_slice(&slot.to_be_bytes());
    }
    if let Some((_, root)) = public {
        bytes.extend_from_slice(&root.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
    }
    bytes.extend_from_slice(&strings);
    Archive::parse(&bytes).unwrap()
}
