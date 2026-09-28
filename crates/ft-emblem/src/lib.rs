//! Roy: ft/kinds/ftEmblem. Common states live in melee-ft; Roy runs
//! Marth's specials (ft-mars-family) with his own attributes and effects.
pub mod init;

/// MarsAttributes, read from `ftDataEmblem`.
pub mod attributes {
    pub use ft_mars_family::attributes::*;

    /// ftFe_Init_OnLoad -> ftMs_Init_OnLoadForRoy (80136474): PUSH_ATTRS of
    /// ftDataEmblem's ext_attr, in Marth's layout.
    pub fn read_roy_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<MarsAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataEmblem")
    }
}
