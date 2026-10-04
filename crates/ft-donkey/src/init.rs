//! Donkey Kong load/reset hooks, ft/kinds/ftDonkey/ftdonkey.c.
use crate::attributes::{read_donkey_attributes, DonkeyAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct DonkeyKong {
    pub attributes: DonkeyAttributes,
    /// ftDk_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C, u.dk.x222C: Giant Punch's stored arm swings, kept
    /// between states until the punch, a hit taken or a death.
    pub punch_swings: i32,
    /// Fighter +2230, u.dk.x2230. Fighter_Create leaves it as stale heap; a
    /// savestate carries retail's.
    // TODO(meaning): no retail reader found.
    pub unknown_2230: u32,
}
impl DonkeyKong {
    pub fn new(attributes: DonkeyAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            punch_swings: 0,
            unknown_2230: 0,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<DonkeyKong>();

impl CharacterCallbacks for DonkeyKong {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftDk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftDk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;

    fn kind(&self) -> FighterKind {
        FighterKind::Donkey
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_donkey_attributes(data)?))
    }
    /// ftDonkey_FighterVars +222C..+2233.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.punch_swings = word(0x222C) as i32;
        self.unknown_2230 = word(0x2230);
    }
    /// ftDk_Init_OnLoad (8010D9AC): the carry walks' animation lengths,
    /// PUSH_ATTRS, x2222_b0 and x2CC (the cargo carry's attributes). Donkey
    /// Kong has no aerial down special (ftData_SpecialAirLw[3] is NULL).
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.air_specials = Some([true, true, true, false]);
    }
    /// ftDk_Init_OnDeath (8010D740): the stored punch goes and
    /// ftParts_80074A4C(gobj, 0, 0).
    fn on_reset(&mut self) {
        self.punch_swings = 0;
        self.model_group = 0;
    }
}

/// ftDk_Init_* strings (ftdonkey.c); ftData_Table_Unk0[3] has 337 animation
/// rows. PlCo ftPartsTable maps Donkey Kong's joints to 54 parts;
/// ftData.x1C holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Donkey,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Donkey),
    data_file: "PlDk.dat",
    data_symbol: "ftDataDonkey",
    animation_file: "PlDkAJ.dat",
    animation_count: 337,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlDkNr.dat",
            joint_symbol: "PlyDonkey5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkBk.dat",
            joint_symbol: "PlyDonkey5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkRe.dat",
            joint_symbol: "PlyDonkey5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkBu.dat",
            joint_symbol: "PlyDonkey5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkGr.dat",
            joint_symbol: "PlyDonkey5KGr_Share_joint",
        },
    ],
};
