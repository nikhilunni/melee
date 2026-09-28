//! Captain Falcon: ft/kinds/ftCaptain. Common movement lives in melee-ft;
//! the specials Ganondorf shares live in ft-captain-family.
pub mod init;

/// ftCaptain_DatAttrs, read from `ftDataCaptain`.
pub mod attributes {
    pub use ft_captain_family::attributes::*;

    /// ftCa_Init_OnLoad's PUSH_ATTRS source: ftDataCaptain's ext_attr.
    pub fn read_captain_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<CaptainAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataCaptain")
    }
}
