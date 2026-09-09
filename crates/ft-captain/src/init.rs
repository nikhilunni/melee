//! Captain Falcon load/reset hooks, ft/kinds/ftCaptain/ftcaptain.c.
use crate::attributes::CaptainAttributes;
use melee_ft::fighter::assets::{CharacterDescriptor, CostumeDescriptor};
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct CaptainFalcon {
    pub attributes: CaptainAttributes,
    /// Fighter +222C, types.dox: Raptor Boost startup GFX is active.
    pub raptor_boost_start_effect_active: bool,
    /// Fighter +2230, types.dox: Raptor Boost lunge GFX is active.
    pub raptor_boost_lunge_effect_active: bool,
    /// ftCa_Init_OnDeath resets model group 0 to its default variant.
    pub model_group: i32,
}
impl CaptainFalcon {
    pub fn new(attributes: CaptainAttributes) -> Self {
        Self {
            attributes,
            raptor_boost_start_effect_active: false,
            raptor_boost_lunge_effect_active: false,
            model_group: 0,
        }
    }
}
impl CharacterCallbacks for CaptainFalcon {
    fn kind(&self) -> FighterKind {
        FighterKind::Captain
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_captain_attributes(data)?))
    }
    /// ftCaptain_FighterVars (types.h): saved startup and lunge effect flags.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.raptor_boost_start_effect_active =
            u32::from_be_bytes(raw[0x222C..0x2230].try_into().unwrap()) != 0;
        self.raptor_boost_lunge_effect_active =
            u32::from_be_bytes(raw[0x2230..0x2234].try_into().unwrap()) != 0;
    }
    /// ftCa_Init_OnLoad (800E2AEC): walljump flag and PUSH_ATTRS only; no
    /// item registrations. ftdata.c supplies all four ground/air specials.
    /// Dynamic chains/colliders are empty in ftData.x2C, read by melee-ft.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftCa_Init_OnDeath (800E2888): model group 0, then +2230 and +222C.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.raptor_boost_lunge_effect_active = false;
        self.raptor_boost_start_effect_active = false;
    }
}

/// ftCa_Init_* strings; ftData_Table_Unk0[2] has 318 animation rows.
/// PlCo ftPartsTable[2] maps 63 joints to 54 semantic parts; ftData.x1C
/// contains three part-animation groups. Dynamic sets are archive-owned.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Captain,
    data_file: "PlCa.dat",
    data_symbol: "ftDataCaptain",
    animation_file: "PlCaAJ.dat",
    animation_count: 318,
    part_count: 54,
    part_animation_count: 3,
    additional_motions: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlCaNr.dat",
            joint_symbol: "PlyCaptain5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlCaGy.dat",
            joint_symbol: "PlyCaptain5KGy_Share_joint",
        },
        // Retail "PlCaRe." resolves to .usd with lbLang_IsSettingUS
        // (lbfile.c:74-81); this descriptor targets NTSC-U's English setting.
        CostumeDescriptor {
            file: "PlCaRe.usd",
            joint_symbol: "PlyCaptain5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlCaWh.dat",
            joint_symbol: "PlyCaptain5KWh_Share_joint",
        },
        CostumeDescriptor {
            file: "PlCaGr.dat",
            joint_symbol: "PlyCaptain5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlCaBu.dat",
            joint_symbol: "PlyCaptain5KBu_Share_joint",
        },
    ],
};
