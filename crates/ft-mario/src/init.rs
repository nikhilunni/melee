//! Mario load/reset hooks, ft/kinds/ftMario/ftmario.c.
use crate::attributes::{read_mario_attributes, MarioAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor},
    Capabilities, CharacterCallbacks,
};
use melee_types::FighterKind;

/// ftMario_FighterVars (ftMario/types.h): Dr. Mario's vitamin colours share
/// the block, so Mario keeps them at their OnDeath value.
const VITAMIN_RESET: i32 = 9;

#[derive(Clone, Debug)]
pub struct Mario {
    pub attributes: MarioAttributes,
    /// ftMr_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C, x222C_vitaminCurr (Dr. Mario's current pill colour).
    pub vitamin_current: i32,
    /// Fighter +2230, x2230_vitaminPrev.
    pub vitamin_previous: i32,
    /// Fighter +2234, x2234_tornadoCharge: an aerial Tornado already rose.
    pub tornado_charged: bool,
    /// Fighter +2238, x2238_isCapeBoost: an aerial cape already boosted.
    pub cape_boosted: bool,
}
impl Mario {
    pub fn new(attributes: MarioAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            vitamin_current: VITAMIN_RESET,
            vitamin_previous: VITAMIN_RESET,
            tornado_charged: false,
            cape_boosted: false,
        }
    }
}

static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; crate::SPECIAL_ROW_COUNT] =
    crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Mario>();

impl CharacterCallbacks for Mario {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    /// Rows 341/342 (ftMr_MS_AppealSR/L) are Dr. Mario's and unused here.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;

    fn kind(&self) -> FighterKind {
        FighterKind::Mario
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_mario_attributes(data)?))
    }
    /// ftMario_FighterVars +222C..+223B. The cape GObj (+223C) and +2240
    /// are not modelled; a save with either set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.vitamin_current = word(0x222C) as i32;
        self.vitamin_previous = word(0x2230) as i32;
        self.tornado_charged = word(0x2234) != 0;
        self.cape_boosted = word(0x2238) != 0;
        if word(0x223C) != 0 || word(0x2240) != 0 {
            unimplemented!("ftMario_FighterVars: a saved cape GObj (+223C/+2240)");
        }
    }
    /// ftMr_Init_OnLoad (800E0960): can_walljump and PUSH_ATTRS. The fire
    /// and cape item registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftMr_Init_OnDeath (800E08CC): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.vitamin_current = VITAMIN_RESET;
        self.vitamin_previous = VITAMIN_RESET;
        self.tornado_charged = false;
        self.cape_boosted = false;
    }
    /// ftCo_Landing_Enter, ftCo_Landing.c:49-52.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.tornado_charged = false;
        self.cape_boosted = false;
    }
}

/// ftMr_Init_* strings (ftmariostrings.c); ftData_Table_Unk0[0] has 303
/// animation rows. PlCo ftPartsTable maps Mario's joints to 54 parts;
/// ftData.x1C holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Mario,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mario),
    data_file: "PlMr.dat",
    data_symbol: "ftDataMario",
    animation_file: "PlMrAJ.dat",
    animation_count: 303,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlMrNr.dat",
            joint_symbol: "PlyMario5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrYe.dat",
            joint_symbol: "PlyMario5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrBk.dat",
            joint_symbol: "PlyMario5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrBu.dat",
            joint_symbol: "PlyMario5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMrGr.dat",
            joint_symbol: "PlyMario5KGr_Share_joint",
        },
    ],
};
