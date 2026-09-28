//! Marth: ft/kinds/ftMars. Common states live in melee-ft; the specials
//! Roy shares live in ft-mars-family.
pub mod init;

/// MarsAttributes, read from `ftDataMars`.
pub mod attributes {
    pub use ft_mars_family::attributes::*;

    /// ftMs_Init_OnLoad's PUSH_ATTRS source: ftDataMars's ext_attr.
    pub fn read_mars_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<MarsAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataMars")
    }
}
