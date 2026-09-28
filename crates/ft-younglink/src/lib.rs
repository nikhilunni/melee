//! Young Link: ft/kinds/ftCLink. Common states live in melee-ft; the
//! states and specials Link shares live in ft-link-family.
pub mod init;

/// ftLk_DatAttrs, read from `ftDataClink`.
pub mod attributes {
    pub use ft_link_family::attributes::*;

    /// ftLk_Init_OnLoadForCLink's PUSH_ATTRS source: ftDataClink's ext_attr.
    pub fn read_young_link_attributes(
        archive: &hsd_archive::Archive,
    ) -> Result<LinkAttributes, melee_ft::desc::FighterDescError> {
        read(archive, "ftDataClink")
    }
}
