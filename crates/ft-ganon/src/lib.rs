//! Ganondorf: ft/kinds/ftGanon. Common states live in melee-ft; his
//! specials are Captain Falcon's code, in ft-captain-family.
pub mod init;

/// ftCaptain_DatAttrs, read from `ftDataGanon`.
pub mod attributes {
    pub use ft_captain_family::attributes::*;

    /// ftGn_Init_OnLoad's PUSH_ATTRS source (ftCa_Init_OnLoadForGanon):
    /// ftDataGanon's ext_attr.
    pub fn read_ganon_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<CaptainAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataGanon")
    }
}
