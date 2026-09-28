//! ftPe_DatAttrs, ft/kinds/ftPeach/types.h; ftData.ext_attr (+4).
use hsd_archive::{Archive, Reader};
use hsd_types::Vec3;
use melee_ft::desc::{special_attributes_offset, FighterDescError};
use melee_types::ItemKind;
type Result<T> = std::result::Result<T, FighterDescError>;
pub const PEACH_ATTRIBUTES_SIZE: u32 = 0xC0;
#[derive(Clone, Debug, PartialEq)]
pub struct PeachAttributes {
    pub float: FloatAttributes,
    pub vegetable: VegetableAttributes,
    pub bomber: BomberAttributes,
    pub parasol: ParasolAttributes,
    pub toad: ToadAttributes,
    /// +AC..BC: AbsorbDesc, used as Toad's counter volume.
    pub toad_volume: CounterVolume,
    /// The turnip article's face weights (ftData.x48_items[1]
    /// itPeachTurnipAttributes x8[].x0_odds), which it_802BD32C draws from
    /// right after the pull chose a turnip. Filled by `read_turnip_faces`.
    pub turnip_faces: TurnipFaces,
}
/// itPeachTurnipAttributes x4_length and x8[].x0_odds.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnipFaces {
    pub count: usize,
    pub weights: [i32; TURNIP_FACE_CAPACITY],
}
/// Retail PlPe.dat has eight turnip faces.
pub const TURNIP_FACE_CAPACITY: usize = 8;
#[derive(Clone, Debug, PartialEq)]
pub struct FloatAttributes {
    /// +0x00, ftPeach/types.h.
    pub forward_fall_start: f32,
    /// +0x04, ftPeach/types.h.
    pub backward_fall_start: f32,
    /// +0x08, ftPeach/types.h.
    pub fall_start_offset: f32,
    /// +0x0C, ftPeach/types.h.
    pub duration: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VegetableAttributes {
    /// +0x10, ftPeach/types.h.
    pub rare_item_count: i32,
    /// +0x14, ftPeach/types.h.
    pub rare_item_odds: i32,
    /// +18..2C: weighted item choices. The header's +1C table comment is stale.
    pub rare_items: [ItemChance; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct BomberAttributes {
    /// +0x30, ftPeach/types.h.
    pub smash_input_window: i32,
    /// +0x34, ftPeach/types.h.
    pub ground_start_speed: f32,
    /// +0x38, ftPeach/types.h.
    pub ground_start_acceleration: f32,
    /// +0x3C, ftPeach/types.h.
    pub ground_start_max_speed: f32,
    /// +0x40, ftPeach/types.h.
    pub air_start_vertical_speed: f32,
    /// +0x44, ftPeach/types.h.
    pub travel_speed: f32,
    /// +0x48, ftPeach/types.h.
    pub smash_travel_speed: f32,
    /// +0x4C, ftPeach/types.h.
    pub travel_vertical_speed: f32,
    /// +0x50, ftPeach/types.h.
    pub start_gravity: f32,
    /// +0x54, ftPeach/types.h.
    pub air_friction: f32,
    /// +0x58, ftPeach/types.h.
    pub travel_gravity: f32,
    /// +0x5C, ftPeach/types.h.
    pub terminal_velocity: f32,
    /// +0x60, ftPeach/types.h.
    pub rebound_horizontal_speed: f32,
    /// +0x64, ftPeach/types.h.
    pub rebound_vertical_speed: f32,
    /// +0x68, ftPeach/types.h.
    pub hit_horizontal_divisor: f32,
    /// +0x6C, ftPeach/types.h.
    pub hit_vertical_divisor: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParasolAttributes {
    /// +0x70, ftPeach/types.h.
    pub freefall_mobility: f32,
    /// +0x74, ftPeach/types.h.
    pub landing_lag: f32,
    /// +0x78, ftPeach/types.h.
    pub reverse_stick_threshold: f32,
    /// +0x7C, ftPeach/types.h.
    pub angle_stick_threshold: f32,
    /// +0x80, ftPeach/types.h.
    pub maximum_angle_degrees: f32,
    /// +0x84, ftPeach/types.h.
    pub startup_momentum_multiplier: f32,
    /// +0x88, ftPeach/types.h.
    pub gravity: f32,
    /// +0x8C, ftPeach/types.h.
    pub ending_momentum_multiplier: f32,
    /// +0x90, ftPeach/types.h.
    pub open_duration: i32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ToadAttributes {
    /// +0x94, ftPeach/types.h.
    pub horizontal_momentum_divisor: f32,
    /// +0x98, ftPeach/types.h.
    pub air_friction: f32,
    /// +0x9C, ftPeach/types.h.
    pub first_air_vertical_speed: f32,
    /// +0xA0, ftPeach/types.h.
    pub gravity: f32,
    /// +0xA4, ftPeach/types.h.
    pub terminal_velocity: f32,
    /// +0xA8, ftPeach/types.h.
    pub collision_parameter: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ItemChance {
    /// ItemChance +0: weight supplied to HSD_Randi.
    pub weight: i32,
    /// ItemChance +4: ItemKind.
    pub kind: ItemKind,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CounterVolume {
    /// +AC: signed joint index.
    pub bone: i32,
    /// +B0: local collision offset.
    pub offset: Vec3,
    /// +BC: collision radius.
    pub radius: f32,
}
pub fn read_peach_attributes(archive: &Archive) -> Result<PeachAttributes> {
    let root = archive.public("ftDataPeach").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataPeach".into(),
        })
    })?;
    PeachAttributes::read(archive, special_attributes_offset(archive, root)?)
}
/// ftData.x48_items[1] (the turnip Article) +4: itPeachTurnipAttributes.
pub fn read_turnip_faces(archive: &Archive) -> Result<TurnipFaces> {
    let root = archive.public("ftDataPeach").ok_or_else(|| {
        FighterDescError::Archive(hsd_archive::desc::DescError::MissingSymbol {
            name: "ftDataPeach".into(),
        })
    })?;
    let r = archive.reader();
    let items = r.u32(root + 0x48)?;
    let article = r.u32(items + 4)?;
    let special = r.u32(article + 4)?;
    let count = r.s32(special + 4)? as usize;
    assert!(count <= TURNIP_FACE_CAPACITY, "itPeachTurnipAttributes x4_length");
    let mut weights = [0; TURNIP_FACE_CAPACITY];
    for (i, weight) in weights.iter_mut().enumerate().take(count) {
        *weight = r.s32(special + 8 + 8 * i as u32)?;
    }
    Ok(TurnipFaces { count, weights })
}
impl PeachAttributes {
    pub fn read(archive: &Archive, offset: u32) -> Result<Self> {
        let r = Reader::new(archive.reader().slice(offset, PEACH_ATTRIBUTES_SIZE)?);
        Ok(Self {
            float: FloatAttributes {
                forward_fall_start: r.f32(0x0)?,
                backward_fall_start: r.f32(0x4)?,
                fall_start_offset: r.f32(0x8)?,
                duration: r.f32(0xC)?,
            },
            vegetable: VegetableAttributes {
                rare_item_count: r.s32(0x10)?,
                rare_item_odds: r.s32(0x14)?,
                rare_items: [
                    ItemChance {
                        weight: r.s32(0x18)?,
                        kind: ItemKind::try_from(r.s32(0x1C)?)?,
                    },
                    ItemChance {
                        weight: r.s32(0x20)?,
                        kind: ItemKind::try_from(r.s32(0x24)?)?,
                    },
                    ItemChance {
                        weight: r.s32(0x28)?,
                        kind: ItemKind::try_from(r.s32(0x2C)?)?,
                    },
                ],
            },
            bomber: BomberAttributes {
                smash_input_window: r.s32(0x30)?,
                ground_start_speed: r.f32(0x34)?,
                ground_start_acceleration: r.f32(0x38)?,
                ground_start_max_speed: r.f32(0x3C)?,
                air_start_vertical_speed: r.f32(0x40)?,
                travel_speed: r.f32(0x44)?,
                smash_travel_speed: r.f32(0x48)?,
                travel_vertical_speed: r.f32(0x4C)?,
                start_gravity: r.f32(0x50)?,
                air_friction: r.f32(0x54)?,
                travel_gravity: r.f32(0x58)?,
                terminal_velocity: r.f32(0x5C)?,
                rebound_horizontal_speed: r.f32(0x60)?,
                rebound_vertical_speed: r.f32(0x64)?,
                hit_horizontal_divisor: r.f32(0x68)?,
                hit_vertical_divisor: r.f32(0x6C)?,
            },
            parasol: ParasolAttributes {
                freefall_mobility: r.f32(0x70)?,
                landing_lag: r.f32(0x74)?,
                reverse_stick_threshold: r.f32(0x78)?,
                angle_stick_threshold: r.f32(0x7C)?,
                maximum_angle_degrees: r.f32(0x80)?,
                startup_momentum_multiplier: r.f32(0x84)?,
                gravity: r.f32(0x88)?,
                ending_momentum_multiplier: r.f32(0x8C)?,
                open_duration: r.s32(0x90)?,
            },
            toad: ToadAttributes {
                horizontal_momentum_divisor: r.f32(0x94)?,
                air_friction: r.f32(0x98)?,
                first_air_vertical_speed: r.f32(0x9C)?,
                gravity: r.f32(0xA0)?,
                terminal_velocity: r.f32(0xA4)?,
                collision_parameter: r.f32(0xA8)?,
            },
            toad_volume: CounterVolume {
                bone: r.s32(0xAC)?,
                offset: Vec3::new(r.f32(0xB0)?, r.f32(0xB4)?, r.f32(0xB8)?),
                radius: r.f32(0xBC)?,
            },
            turnip_faces: TurnipFaces {
                count: 0,
                weights: [0; TURNIP_FACE_CAPACITY],
            },
        })
    }
}
