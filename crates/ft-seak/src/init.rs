//! Sheik load/reset hooks, ft/kinds/ftSeak/ftseak.c.
use crate::attributes::{read_seak_attributes, SeakAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

/// The accessory4 callback a special installed; a motion change removes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// fn_80114034: the transformation's start sparkle.
    TransformStart,
    /// fn_8011412C: hand the match to Zelda (ftCommon_8007EFC8).
    TransformHandOver,
    /// fn_801140B0: the arrival sparkle.
    TransformArrival,
    /// fn_80112ED8: Vanish's smoke, puff and sound.
    VanishDisappear,
    /// fn_80113038: Vanish's reappearance puff.
    VanishReappear,
}

#[derive(Clone, Debug)]
pub struct Sheik {
    pub attributes: SeakAttributes,
    /// ftSk_Init_OnDeath: model group 0 shows selection 0, group 1 none.
    pub model_groups: [i32; 2],
    /// ftSeak_FighterVars +222C: needles charged (0..=6).
    pub needles: i32,
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// Vanish's motion scratch.
    pub vanish: crate::special_hi::Vanish,
}
impl Sheik {
    pub fn new(attributes: SeakAttributes) -> Self {
        Self {
            attributes,
            model_groups: MODEL_GROUPS_RESET,
            needles: 0,
            accessory: Accessory::None,
            vanish: Default::default(),
        }
    }
}

/// ftSk_Init_OnDeath (80110044): ftParts_80074A4C(gobj, 0, 0) and (1, -1).
const MODEL_GROUPS_RESET: [i32; 2] = [0, -1];

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Sheik>();

impl CharacterCallbacks for Sheik {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftSk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftSk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Seak] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            _ => unimplemented!(
                "ftData_Special{slot:?}[Seak] (airborne: {airborne}): character special entry"
            ),
        }
    }
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Sheik>().accessory {
            Accessory::TransformStart => crate::special_lw::sparkle(f, false),
            Accessory::TransformHandOver => crate::special_lw::hand_over(f),
            Accessory::TransformArrival => crate::special_lw::sparkle(f, true),
            Accessory::VanishDisappear => crate::special_hi::disappear(f),
            Accessory::VanishReappear => crate::special_hi::reappear(f),
            Accessory::None => {}
        }
    }
    /// ftSk_SpecialLw_80114758: Sheik arrives from Zelda's transformation.
    const TRANSFORMATION_ARRIVAL: fn(
        &mut Fighter,
        &FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = crate::special_lw::arrive;

    fn kind(&self) -> FighterKind {
        FighterKind::Seak
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_seak_attributes(data)?))
    }
    /// ftSeak_FighterVars +222C..+2237: a saved needle charge, held needle
    /// or chain is not modelled yet.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.needles = word(0x222C) as i32;
        if word(0x2230) != 0 || word(0x2234) != 0 {
            unimplemented!("ftSeak_FighterVars: a saved needle or chain GObj (+2230/+2234)");
        }
    }
    /// ftSk_Init_OnLoad (80110090): can_walljump and PUSH_ATTRS. The four
    /// article registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftSk_Init_OnDeath (80110044).
    fn on_reset(&mut self) {
        self.needles = 0;
        self.model_groups = MODEL_GROUPS_RESET;
    }
}

/// ftSk_Init_* strings; ftData_Table_Unk0[FTKIND_SEAK] has 317 animation
/// rows (ftCo_SM_Count plus 22 of Sheik's).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Seak,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Seak),
    data_file: "PlSk.dat",
    data_symbol: "ftDataSeak",
    animation_file: "PlSkAJ.dat",
    animation_count: 317,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlSkNr.dat",
            joint_symbol: "PlySeak5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSkRe.dat",
            joint_symbol: "PlySeak5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSkBu.dat",
            joint_symbol: "PlySeak5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSkGr.dat",
            joint_symbol: "PlySeak5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlSkWh.dat",
            joint_symbol: "PlySeak5KWh_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[FTKIND_SEAK]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 2;
