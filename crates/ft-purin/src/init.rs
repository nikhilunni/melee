//! Jigglypuff load/reset hooks, ft/kinds/ftPurin/ftpurin.c.
use crate::attributes::{read_purin_attributes, PurinAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor},
    AerialJumpStyle, Capabilities, CharacterCallbacks,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Jigglypuff {
    pub attributes: PurinAttributes,
    /// ftPr_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
}
impl Jigglypuff {
    pub fn new(attributes: PurinAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
        }
    }
}
impl CharacterCallbacks for Jigglypuff {
    fn kind(&self) -> FighterKind {
        FighterKind::Purin
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_purin_attributes(data)?))
    }
    /// ftPr_Init_OnLoad (8013C67C): PUSH_ATTRS, can_multijump, x2D0 = dat_attrs.
    /// No item registrations or walljump flag; all four specials exist.
    /// Dynamic chains are read from ftData.x2C by the shared asset loader.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftPr_Init_8013C360: neutral costume has no accessory joint. Other
    /// costumes load a hat with separate parts visibility and renderer callbacks.
    fn on_costume_loaded(
        &mut self,
        _archive: &hsd_archive::Archive,
        costume: u8,
    ) -> melee_ft::fighter::assets::Result<()> {
        if costume != 0 {
            unimplemented!("ftpurin.c:447-478: costume hat loading and visibility");
        }
        Ok(())
    }
    /// ftPr_Init_OnDeath (8013C318): ftParts_80074A4C(gobj, 0, 0).
    fn on_reset(&mut self) {
        self.model_group = 0;
    }
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::MultiJump
    }
    fn multi_jump_attributes(&self) -> Option<&melee_ft::fighter::multi_jump::MultiJumpAttributes> {
        Some(&self.attributes.multi_jump)
    }
    /// ftPr_Init_MotionStateTable: F1..F5 use submotions 295..299.
    fn multi_jump_animation(&self, jump: usize) -> i32 {
        const FIRST_MULTI_JUMP_ANIMATION: i32 = 295; // ftPr_SM_JumpAerialF1
        FIRST_MULTI_JUMP_ANIMATION + jump as i32
    }
}

/// ftPr_Init_* strings; ftData_Table_Unk0[15] has 327 animation rows.
/// ftparts.c's PlCo table maps 50 joints to 54 parts; ftData.x1C has two groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Purin,
    data_file: "PlPr.dat",
    data_symbol: "ftDataPurin",
    animation_file: "PlPrAJ.dat",
    animation_count: 327,
    part_count: 54,
    part_animation_count: 2,
    additional_motions: &[295, 296, 297, 298, 299],
    costumes: &[
        CostumeDescriptor {
            file: "PlPrNr.dat",
            joint_symbol: "PlyPurin5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrRe.dat",
            joint_symbol: "PlyPurin5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrBu.dat",
            joint_symbol: "PlyPurin5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrGr.dat",
            joint_symbol: "PlyPurin5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrYe.dat",
            joint_symbol: "PlyPurin5KYe_Share_joint",
        },
    ],
};
