//! ftIceClimberAttributes (ft/kinds/ftPopo/types.h), 0x15C bytes read from
//! `ftDataPopo` or `ftDataNana`'s ext_attr (+4). Both climbers PUSH_ATTRS
//! the same layout (ftPp_Init_OnLoad, ftPp_Init_OnLoadForNana); the two
//! archives carry identical values. Fields join this struct as the code
//! that reads them is ported.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

/// sizeof(ftIceClimberAttributes).
pub const ICE_CLIMBER_ATTRIBUTES_SIZE: u32 = 0x15C;

#[derive(Clone, Debug, PartialEq)]
pub struct IceClimberAttributes {
    /// +0x00: Popo's fp->x40, the spawn and revival offset along his facing
    /// (ftCommon_800804EC).
    pub leader_spawn_offset: f32,
    /// +0x04: the lift of a first aerial Ice Shot (ftPp_SpecialAirN_Enter).
    pub air_lift: f32,
    /// +0x08: an aerial Ice Shot's landing lag (ftPp_SpecialAirN_Coll).
    pub ice_shot_landing_lag: f32,
    /// +0x0C / +0x10: where the ice block forms, along the facing and above
    /// TopN (ftPp_SpecialN_8011F500).
    pub ice_reach: f32,
    pub ice_height: f32,
    /// +0xC4: Nana's fp->x40 (ftNn_Init_OnLoad).
    pub partner_spawn_offset: f32,
    /// +0xC8: Nana's dmg.armor0, set on every reset (ftNn_Init_OnDeath).
    pub partner_armor: f32,
}

/// The attributes of `symbol`'s ftData (`ftDataPopo` or `ftDataNana`).
pub fn read(archive: &Archive, symbol: &str) -> Result<IceClimberAttributes> {
    let root = archive.public(symbol).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: symbol.into(),
        })
    })?;
    IceClimberAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl IceClimberAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(
            archive
                .reader()
                .slice(offset, ICE_CLIMBER_ATTRIBUTES_SIZE)?,
        );
        Ok(Self {
            leader_spawn_offset: r.f32(0x00)?,
            air_lift: r.f32(0x04)?,
            ice_shot_landing_lag: r.f32(0x08)?,
            ice_reach: r.f32(0x0C)?,
            ice_height: r.f32(0x10)?,
            partner_spawn_offset: r.f32(0xC4)?,
            partner_armor: r.f32(0xC8)?,
        })
    }
}
