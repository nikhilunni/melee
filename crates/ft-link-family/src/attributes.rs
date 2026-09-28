//! ftLk_DatAttrs, ft/kinds/ftLink/types.h; ftData.ext_attr (+4). Young
//! Link shares the layout (ftLk_Init_OnLoadForCLink). ftLk_Init_OnLoad
//! (800EAE44) writes the down aerial's hit end frame into the archive's
//! copy before PUSH_ATTRS, so `down_air.hit_frame_end` is filled in from
//! the loaded animation (`LinkAttributes::with_down_air_end`).
//! ftLk_Init_LoadSpecialAttrs (800EB250) scales `hookshot_height` by the
//! player's y scale when it is not 1, which a plain versus match never sets.
use hsd_archive::{Archive, Reader};
use melee_ft::desc::{special_attributes_offset, FighterDescError};
type Result<T> = std::result::Result<T, FighterDescError>;

pub const LINK_ATTRIBUTES_SIZE: u32 = 0xDC;

#[derive(Clone, Debug, PartialEq)]
pub struct LinkAttributes {
    pub bow: BowAttributes,
    pub boomerang: BoomerangAttributes,
    /// +0x28: specialhi_pos_y_offset, the height above the fighter's
    /// position ftLk_SpecialHi_GetPosWithAdjustedY reports.
    pub hookshot_height: f32,
    pub spin_attack: SpinAttackAttributes,
    /// +0x48: the bomb item kind (It_Kind_Link_Bomb / CLink_Bomb).
    pub bomb_item: u32,
    pub down_air: DownAirAttributes,
    /// +0x64: SwordAttrs, the sword afterimage's shape and colours.
    pub sword_trail: [u32; 8],
    /// +0x84..+0xBB: the hookshot's chain parameters (itlinkhookshot.c).
    pub hookshot: [u32; 14],
    /// +0xBC: the hookshot item kind.
    pub hookshot_item: u32,
    /// +0xC0. TODO(meaning)
    pub unknown_c0: u32,
    pub shield: ShieldAttributes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BowAttributes {
    /// +0x00: the charge (frames held) that stops growing.
    pub max_charge: f32,
    /// +0x04: specialn_anim_rate, the draw animation's rate.
    pub draw_animation_rate: f32,
    /// +0x08: landing lag of the aerial bow's ftCo_80096900 (zero: Fall).
    pub air_landing_lag: f32,
    /// +0x0C: the arrow item kind.
    pub arrow_item: u32,
    /// +0x10: the bow item kind.
    pub bow_item: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoomerangAttributes {
    /// +0x14: the stick's |y| beyond which the throw angles.
    pub angle_stick_threshold: f32,
    /// +0x18: the throw angle's limit (radians), scaled by +0x1C.
    pub max_angle: f32,
    /// +0x1C: multiplies `max_angle`.
    pub max_angle_scale: f32,
    /// +0x20: launch speed of a smash throw.
    pub smash_speed: f32,
    /// +0x24: launch speed of an ordinary throw.
    pub speed: f32,
    /// +0x2C: the boomerang item kind.
    pub item: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpinAttackAttributes {
    /// +0x30: landing lag of the aerial spin's ftCo_80096900.
    pub landing_lag: f32,
    /// +0x34: multiplies the horizontal velocity entering the aerial spin.
    pub air_horizontal_scale: f32,
    /// +0x38: specialairhi_drift_stick_mul.
    pub air_drift_stick_scale: f32,
    /// +0x3C: specialairhi_drift_max_mul.
    pub air_drift_max_scale: f32,
    /// +0x40: the aerial spin's vertical velocity.
    pub air_vertical_speed: f32,
    /// +0x44: specialhi_grav_mul.
    pub gravity_scale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownAirAttributes {
    /// +0x4C: attackairlw_hit_vel_y, the bounce on a hit.
    pub hit_vertical_speed: f32,
    /// +0x50: attackairlw_hit_anim_frame_start.
    pub hit_frame_start: f32,
    /// +0x54: attackairlw_hit_anim_frame_end: OnLoad's end frame of
    /// animation 72 (the archive holds zero).
    pub hit_frame_end: f32,
    /// +0x58: attackairlw_anim_flags, each hitbox's re-armed flags.
    pub hitbox_flags: [u32; 3],
}

/// +0xC4: the AbsorbDesc ftColl_8007B1B8 reads as a ShieldDesc (its
/// damage multiplier is +0xD8).
#[derive(Clone, Debug, PartialEq)]
pub struct ShieldAttributes {
    pub bone: i32,
    pub offset: hsd_types::Vec3,
    pub radius: f32,
    /// +0xD8: the ShieldDesc damage multiplier, and the pushback scale of
    /// ftLk_800EB334 (ftCo_80092ED8's second argument).
    pub damage_scale: f32,
}

/// ftLk_Init_OnLoad's PUSH_ATTRS source: the kind's ftData ext_attr.
pub fn read(archive: &Archive, data_symbol: &str) -> Result<LinkAttributes> {
    let root = archive.public(data_symbol).ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: data_symbol.into(),
        })
    })?;
    LinkAttributes::read(archive, special_attributes_offset(archive, root)?)
}

impl LinkAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, LINK_ATTRIBUTES_SIZE)?);
        let words = |at: u32, out: &mut [u32]| -> Result<()> {
            for (i, word) in out.iter_mut().enumerate() {
                *word = r.u32(at + 4 * i as u32)?;
            }
            Ok(())
        };
        let mut sword_trail = [0; 8];
        words(0x64, &mut sword_trail)?;
        let mut hookshot = [0; 14];
        words(0x84, &mut hookshot)?;
        Ok(Self {
            bow: BowAttributes {
                max_charge: r.f32(0x0)?,
                draw_animation_rate: r.f32(0x4)?,
                air_landing_lag: r.f32(0x8)?,
                arrow_item: r.u32(0xC)?,
                bow_item: r.u32(0x10)?,
            },
            boomerang: BoomerangAttributes {
                angle_stick_threshold: r.f32(0x14)?,
                max_angle: r.f32(0x18)?,
                max_angle_scale: r.f32(0x1C)?,
                smash_speed: r.f32(0x20)?,
                speed: r.f32(0x24)?,
                item: r.u32(0x2C)?,
            },
            hookshot_height: r.f32(0x28)?,
            spin_attack: SpinAttackAttributes {
                landing_lag: r.f32(0x30)?,
                air_horizontal_scale: r.f32(0x34)?,
                air_drift_stick_scale: r.f32(0x38)?,
                air_drift_max_scale: r.f32(0x3C)?,
                air_vertical_speed: r.f32(0x40)?,
                gravity_scale: r.f32(0x44)?,
            },
            bomb_item: r.u32(0x48)?,
            down_air: DownAirAttributes {
                hit_vertical_speed: r.f32(0x4C)?,
                hit_frame_start: r.f32(0x50)?,
                hit_frame_end: r.f32(0x54)?,
                hitbox_flags: [r.u32(0x58)?, r.u32(0x5C)?, r.u32(0x60)?],
            },
            sword_trail,
            hookshot,
            hookshot_item: r.u32(0xBC)?,
            unknown_c0: r.u32(0xC0)?,
            shield: ShieldAttributes {
                bone: r.u32(0xC4)? as i32,
                offset: hsd_types::Vec3::new(r.f32(0xC8)?, r.f32(0xCC)?, r.f32(0xD0)?),
                radius: r.f32(0xD4)?,
                damage_scale: r.f32(0xD8)?,
            },
        })
    }

    /// ftLk_Init_OnLoad / ftCl_Init_OnLoad: `da->attackairlw_hit_anim_frame_end
    /// = lbAnim_8001E8F8(ftData_80085E50(fp, 72))`, the frame count of
    /// animation 72, before PUSH_ATTRS copies the block.
    pub fn with_down_air_end(mut self, frames: f32) -> Self {
        self.down_air.hit_frame_end = frames;
        self
    }
}

/// ftData_80085E50's motion index for the down aerial's end frame.
pub const DOWN_AIR_ANIMATION: i32 = 72;
