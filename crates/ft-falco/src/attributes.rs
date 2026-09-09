//! Falco archive root for the shared ftFox_DatAttrs reader.
use hsd_archive::Archive;
use melee_ft::desc::{fox_attributes::FoxAttributes, special_attributes_offset, FighterDescError};

pub fn read_falco_attributes(archive: &Archive) -> Result<FoxAttributes, FighterDescError> {
    let root = archive.public("ftDataFalco").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataFalco".into(),
        })
    })?;
    FoxAttributes::read(archive, special_attributes_offset(archive, root)?)
}
