//! Pikachu: ft/kinds/ftPikachu. Common states live in melee-ft; the
//! specials Pichu shares live in ft-pikachu-family.
pub mod init;

/// ftPikachuAttributes, read from `ftDataPikachu`.
pub mod attributes {
    pub use ft_pikachu_family::attributes::*;

    /// ftPk_Init_OnLoad's PUSH_ATTRS source: ftDataPikachu's ext_attr.
    pub fn read_pikachu_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<PikachuAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataPikachu")
    }
}
