//! Link: ft/kinds/ftLink. Common states live in melee-ft; the states and
//! specials Young Link shares live in ft-link-family.
pub mod init;

/// ftLk_DatAttrs, read from `ftDataLink`.
pub mod attributes {
    pub use ft_link_family::attributes::*;

    /// ftLk_Init_OnLoad's PUSH_ATTRS source: ftDataLink's ext_attr.
    pub fn read_link_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<LinkAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataLink")
    }
}
