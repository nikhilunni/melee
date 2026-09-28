//! Samus load/reset hooks, ft/kinds/ftSamus/ftsamus.c.
use crate::attributes::{read_samus_attributes, SamusAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Samus {
    pub attributes: SamusAttributes,
    /// ftSs_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +2230, x2230: the Charge Shot level kept between shots.
    pub charge_level: i32,
    /// Fighter +2234, x2234: the charge's effects are alive.
    pub charge_effects: bool,
    /// Fighter +2238, x2238: missiles fired (ftSs_SpecialS_8012A074).
    pub missiles_fired: u32,
    /// Fighter +2244, x2244: the Screw Attack's effect is alive.
    pub screw_effect: bool,
}
impl Samus {
    pub fn new(attributes: SamusAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            charge_level: 0,
            charge_effects: false,
            missiles_fired: 0,
            screw_effect: false,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Samus>();

impl CharacterCallbacks for Samus {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftCo_Catch.c:26-136 (fn_800D952C, fn_800D9558, fn_800D9930): the
    /// grapple beam replaces the body grab.
    fn catch_variant(&mut self) {
        unimplemented!("ftCo_Catch.c:125: Samus grapple beam (fn_800D9558)");
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Samus
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_samus_attributes(data)?))
    }
    /// ftSamus_FighterVars +222C..+2248. The charge-shot and grapple GObjs
    /// (+222C, +223C) are not modelled; a save with either set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.charge_level = word(0x2230) as i32;
        self.charge_effects = word(0x2234) != 0;
        self.missiles_fired = word(0x2238);
        self.screw_effect = word(0x2244) != 0;
        if word(0x222C) != 0 || word(0x223C) != 0 {
            unimplemented!("ftSamus_FighterVars: a saved charge shot or grapple GObj");
        }
    }
    /// ftSs_Init_OnLoad (8012837C): can_walljump and PUSH_ATTRS. The four
    /// item registrations (it_8026B3F8: bomb, charge shot, missile, grapple
    /// beam) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftSs_Init_OnDeath (8012832C): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset (x2234 and x2248 keep their values).
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.charge_level = 0;
        self.missiles_fired = 0;
        self.screw_effect = false;
    }
}

/// ftSs_Init_* strings (ftsamus.c); ftData_Table_Unk0[13] has 313
/// animation rows. PlCo ftPartsTable maps Samus's joints to 54 parts;
/// ftData.x1C holds one part-animation group.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Samus,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Samus),
    data_file: "PlSs.dat",
    data_symbol: "ftDataSamus",
    animation_file: "PlSsAJ.dat",
    animation_count: 313,
    part_count: 54,
    part_animation_count: 1,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlSsNr.dat",
            joint_symbol: "PlySamus5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsPi.dat",
            joint_symbol: "PlySamus5KPi_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsBk.dat",
            joint_symbol: "PlySamus5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsGr.dat",
            joint_symbol: "PlySamus5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSsLa.dat",
            joint_symbol: "PlySamus5KLa_Share_joint",
        },
    ],
};
