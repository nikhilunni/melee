//! Pikachu load/reset hooks, ft/kinds/ftPikachu/ftpikachu.c.
use crate::attributes::{read_pikachu_attributes, PikachuAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Pikachu {
    pub attributes: PikachuAttributes,
    /// ftPk_Init_OnDeath resets model groups 0 and 1 to selection 0.
    pub model_groups: [i32; 2],
}
impl Pikachu {
    pub fn new(attributes: PikachuAttributes) -> Self {
        Self {
            attributes,
            model_groups: [0; 2],
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Pikachu>();

impl CharacterCallbacks for Pikachu {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftPk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftPk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);

    fn kind(&self) -> FighterKind {
        FighterKind::Pikachu
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_pikachu_attributes(data)?))
    }
    /// ftPk_Init_OnLoad (80124528): PUSH_ATTRS and three item registrations
    /// (the bolt and both jolts, it_8026B3F8); ftdata.c supplies all four
    /// specials. The attached-item model hooks (ftPk_Init_UnkMotionStates1/2,
    /// through ftCommon_8007F8E8/8007F948) need a head item, which a match
    /// without items never attaches.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftPk_Init_OnDeath (80124620): ftParts_80074A4C(gobj, 0, 0) and (1, 0).
    fn on_reset(&mut self) {
        self.model_groups = [0; 2];
    }
}

/// ftPk_Init_* strings; ftData_Table_Unk0[12] has 319 animation rows
/// (ftCo_SM_Count plus ftPk_SM_SelfCount).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Pikachu,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Pikachu),
    data_file: "PlPk.dat",
    data_symbol: "ftDataPikachu",
    animation_file: "PlPkAJ.dat",
    animation_count: 319,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPkNr.dat",
            joint_symbol: "PlyPikachu5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkRe.dat",
            joint_symbol: "PlyPikachu5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkBu.dat",
            joint_symbol: "PlyPikachu5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkGr.dat",
            joint_symbol: "PlyPikachu5KGr_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[12]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 2;
