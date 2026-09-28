//! Pichu: ft/kinds/ftPichu. Common states live in melee-ft; the specials
//! are Pikachu's (ft-pikachu-family), which ftPc_Init_MotionStateTable
//! points at. Pichu's own differences are data (PlPc.dat's attributes,
//! articles and subaction scripts, whose self-damage commands hurt Pichu)
//! and the family trait's constants.
pub mod init;

/// ftPikachuAttributes, read from `ftDataPichu` (ftPk_Init_OnLoadForPichu).
pub mod attributes {
    pub use ft_pikachu_family::attributes::*;

    /// ftPk_Init_OnLoadForPichu's PUSH_ATTRS source: ftDataPichu's ext_attr.
    pub fn read_pichu_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<PikachuAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataPichu")
    }
}
