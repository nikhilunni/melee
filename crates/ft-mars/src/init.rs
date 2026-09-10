//! Marth load/reset hooks, ft/kinds/ftMars/ftmars.c.
use crate::attributes::MarsAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Marth {
    pub attributes: MarsAttributes,
    /// Fighter +222C, ftmarsspecials.c:56-60: once-per-airtime vertical boost.
    pub side_special_boost_used: bool,
    /// ftMs_Init_OnDeath resets model groups 0 and 1.
    pub model_groups: [i32; 2],
}
impl Marth {
    pub fn new(attributes: MarsAttributes) -> Self {
        Self {
            attributes,
            side_special_boost_used: false,
            model_groups: [0; 2],
        }
    }
}
impl CharacterCallbacks for Marth {
    fn third_jab_state(&self) -> melee_types::CommonMotionState {
        melee_types::CommonMotionState::Attack11
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Mars
    }
    fn descriptor() -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_mars_attributes(data)?))
    }
    /// Marth +222C: side-special boost already spent (ftmars.c reset state).
    fn restore_saved(&mut self, raw: &[u8]) {
        self.side_special_boost_used =
            u32::from_be_bytes(raw[0x222C..0x2230].try_into().unwrap()) != 0;
    }
    /// ftMs_Init_OnLoad (801364AC): PUSH_ATTRS only; no item registrations
    /// or walljump flag. Four specials are present in ftdata.c's callback tables.
    /// OnLoadForRoy (80136474) uses this same attribute type, without scaling.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftMs_Init_OnDeath (80136258): two model groups and Fighter +222C.
    fn on_reset(&mut self) {
        self.model_groups = [0; 2];
        self.side_special_boost_used = false;
    }
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:64-67.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.side_special_boost_used = false;
    }
    /// ftCo_800923B4 (800923B4), ftCo_800939B4 (800939B4),
    /// ftCo_Guard.c:342-346,924-928: select the sword model after shield setup.
    /// Shield joint/neutral pose still come from PlMs.dat's shared descriptors.
    fn guard_variant(&self, commands: &mut melee_ft::fighter::commands::CommandState) {
        use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
        const SWORD_MODEL_GROUP: i32 = 1;
        const SHIELDED_SWORD_MODEL: i32 = 1;
        const SHIELD_SOUND: u32 = 190115;
        commands
            .model_selections
            .insert(SWORD_MODEL_GROUP, SHIELDED_SWORD_MODEL);
        commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: SHIELD_SOUND,
            volume: 127,
            pan: 64,
        });
    }
}

/// ftMs_Init_* strings, ftData_Table_Unk0[18] (327 rows), and
/// PlCo ftPartsTable[18]: 90 joints mapped to the 54 semantic parts.
pub const DESCRIPTOR: melee_ft::fighter::assets::CharacterDescriptor =
    melee_ft::fighter::assets::CharacterDescriptor {
        kind: FighterKind::Mars,
        common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mars),
        data_file: "PlMs.dat",
        data_symbol: "ftDataMars",
        animation_file: "PlMsAJ.dat",
        animation_count: 327,
        part_count: 54,
        part_animation_count: 3,
        // A3 combat scripts: smash, tumble, prone recovery, tech and linked throws.
        additional_motions: &[
            47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 65, 66, 67, 165, 166, 170, 171, 172,
            173, 174, 175, 176, 177, 179, 180, 183, 184, 29, 62, 178, 191, 192, 201, 244, 248, 254,
            255, 263,
        ],
        costumes: &[
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsNr.dat",
                joint_symbol: "PlyMars5K_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsRe.dat",
                joint_symbol: "PlyMars5KRe_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsGr.dat",
                joint_symbol: "PlyMars5KGr_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsBk.dat",
                joint_symbol: "PlyMars5KBk_Share_joint",
            },
            melee_ft::fighter::assets::CostumeDescriptor {
                file: "PlMsWh.dat",
                joint_symbol: "PlyMars5KWh_Share_joint",
            },
        ],
    };
