//! Marth load/reset hooks, ft/kinds/ftMars/ftmars.c.
use crate::attributes::MarsAttributes;
use melee_ft::fighter::{Capabilities, CharacterCallbacks};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Marth {
    pub attributes: MarsAttributes,
    pub special_n: crate::special_n::SpecialN,
    pub special_side: crate::special_s::SpecialSide,
    pub special_lw: crate::special_lw::SpecialLw,
    pub special_hi: crate::special_hi::SpecialHi,
    /// Fighter +222C, ftmarsspecials.c:56-60: once-per-airtime vertical boost.
    pub side_special_boost_used: bool,
    /// ftMs_Init_OnDeath resets model groups 0 and 1.
    pub model_groups: [i32; 2],
}
impl Marth {
    pub fn new(attributes: MarsAttributes) -> Self {
        Self {
            attributes,
            special_hi: Default::default(),
            special_lw: Default::default(),
            special_side: Default::default(),
            special_n: Default::default(),
            side_special_boost_used: false,
            model_groups: [0; 2],
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Marth>();

impl CharacterCallbacks for Marth {
    fn enter_taunt(
        fighter: &mut melee_ft::fighter::Fighter,
        assets: &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> {
        fighter.enter_common_taunt(assets)
    }
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &crate::special_moves();
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &crate::special_rows();
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        air: bool,
        a: &melee_ft::fighter::assets::FighterAssets,
    ) {
        match slot {
            melee_ft::fighter::SpecialSlot::Neutral => crate::special_n::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Side => crate::special_s::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Up => crate::special_hi::enter(f, air, a),
            melee_ft::fighter::SpecialSlot::Down => crate::special_lw::enter(f, air, a),
        }
    }

    fn accessory(f: &mut melee_ft::fighter::Fighter, a: &melee_ft::fighter::assets::FighterAssets) {
        crate::special_n::accessory(f, a);
    }
    const DEFENSE_CONTACT: Option<melee_ft::fighter::DefenseContact> =
        Some(crate::special_lw::contact);
    const PROCESS_DEFENSE_HIT: Option<melee_ft::fighter::DefenseHit> =
        Some(crate::special_lw::process_hit);
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
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
        self.special_hi = Default::default();
        self.special_lw = Default::default();
        self.special_side = Default::default();
        self.special_n = Default::default();
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
            255, 263, 239, 240, 295, 296, 297, 298, 299, 300, 301, 302, 303, 304, 305, 306, 307,
            308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322, 323, 324,
            325, 326,
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
