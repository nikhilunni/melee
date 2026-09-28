//! Popo and Nana load/reset hooks: ftpopo.c and ftnana.c.
use crate::attributes::{self, IceClimberAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

/// ftPopo_FighterVars (+222C..+2253), which both climbers carry. OnDeath
/// clears every field (ftPp_Init_OnDeath, ftNn_Init_OnDeath); the item
/// GObjs they hold arrive with the specials.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClimberVars {
    /// ftParts_80074A4C(gobj, 0 | 1, 0): model groups 0 and 1.
    pub model_groups: [i32; 2],
    /// +2234: x2234.
    // TODO(meaning): written by the specials.
    pub x2234: u32,
    /// +2230 bit 0.
    // TODO(meaning): written by the specials.
    pub x2230_b0: bool,
    /// +224C: cleared by ftCo_Landing_Enter too (ftCo_Landing.c:71).
    // TODO(meaning): written by the specials.
    pub x224c: u32,
    /// +2250.
    // TODO(meaning): written by the specials.
    pub x2250: f32,
}
impl ClimberVars {
    /// ftPp_Init_OnDeath / ftNn_Init_OnDeath: the shared reset.
    fn reset(&mut self) {
        *self = Self::default();
    }
    /// ftPopo_FighterVars from a retail Fighter dump. A saved item GObj
    /// (+222C, +2238) has no port owner yet.
    fn restore(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        if word(0x222C) != 0 || word(0x2238) != 0 {
            unimplemented!("ftPopo_FighterVars: a saved item GObj (+222C/+2238)");
        }
        self.x2230_b0 = raw[0x2230] & 0x80 != 0;
        self.x2234 = word(0x2234);
        self.x224c = word(0x224C);
        self.x2250 = f32::from_bits(word(0x2250));
    }
}

/// Popo, the climber the player controls.
#[derive(Clone, Debug)]
pub struct Popo {
    pub attributes: IceClimberAttributes,
    pub vars: ClimberVars,
}

/// Nana, the partner fighter (x221F_b4) whose inputs melee-cpu supplies.
#[derive(Clone, Debug)]
pub struct Nana {
    pub attributes: IceClimberAttributes,
    pub vars: ClimberVars,
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static POPO_TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Popo>();
pub static NANA_TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Nana>();

/// The hooks both climbers share; `Popo` and `Nana` differ only in load
/// and reset.
macro_rules! climber_callbacks {
    () => {
        /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
        const ENTER_TAUNT: fn(
            &mut Fighter,
            &FighterAssets,
        ) -> melee_ft::fighter::assets::Result<()> = Fighter::enter_common_taunt;
        const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
        const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
            &crate::SPECIAL_MOVES;
        const SPECIAL_PARTNER_SYNC: &'static [bool] = &crate::SPECIAL_PARTNER_SYNC;
        /// ftPp_Init_OnKnockbackEnter: Fighter_OnKnockbackEnter(gobj, 1).
        const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
            |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
        /// ftPp_Init_OnKnockbackExit: Fighter_OnKnockbackExit(gobj, 1).
        const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
            |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
        fn from_archive(
            data: &hsd_archive::Archive,
        ) -> Result<Self, melee_ft::desc::FighterDescError> {
            Ok(Self {
                attributes: attributes::read(data, Self::descriptor().data_symbol)?,
                vars: ClimberVars::default(),
            })
        }
        fn restore_saved(&mut self, raw: &[u8]) {
            self.vars.restore(raw);
        }
        /// ftCo_Landing_Enter, ftCo_Landing.c:69-71: FTKIND_POPO/NANA.
        fn on_landing(&mut self, _allow_interrupt: bool) {
            self.vars.x224c = 0;
        }
    };
}

impl CharacterCallbacks for Popo {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &POPO_TABLE
    }
    climber_callbacks!();
    fn kind(&self) -> FighterKind {
        FighterKind::Popo
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &POPO_DESCRIPTOR
    }
    /// ftPp_Init_OnLoad (8011EF14): x2222_b5, PUSH_ATTRS, x40 = attributes
    /// +0; the three item registrations (it_8026B3F8) belong to the item
    /// scene. ftdata.c supplies every special.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.leads_partner = true;
        capabilities.spawn_offset = self.attributes.leader_spawn_offset;
        capabilities.specials = [true; 4];
    }
    /// ftPp_Init_OnDeath (8011EFE8).
    fn on_reset(&mut self) {
        self.vars.reset();
    }
}

impl CharacterCallbacks for Nana {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &NANA_TABLE
    }
    climber_callbacks!();
    fn kind(&self) -> FighterKind {
        FighterKind::Nana
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &NANA_DESCRIPTOR
    }
    /// ftNn_Init_OnLoad (80122EB4): x2222_b4, PUSH_ATTRS (through
    /// ftPp_Init_OnLoadForNana), x40 = attributes +C4. ftData_SpecialS and
    /// ftData_SpecialHi (and their aerial tables) are NULL for Nana.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.refuses_healing_items = true;
        capabilities.cpu_partner = true;
        capabilities.spawn_offset = self.attributes.partner_spawn_offset;
        capabilities.armor = self.attributes.partner_armor;
        capabilities.specials = [false, false, true, true];
    }
    /// ftNn_Init_OnDeath (80122F28): dmg.armor0 = attributes +C8 (see
    /// `on_load`), then the shared reset.
    fn on_reset(&mut self) {
        self.vars.reset();
    }
}

/// ftPp_Init_* strings; ftData_Table_Unk0[10] has 321 animation rows.
pub const POPO_DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Popo,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Popo),
    data_file: "PlPp.dat",
    data_symbol: "ftDataPopo",
    animation_file: "PlPpAJ.dat",
    animation_count: 321,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPpNr.dat",
            joint_symbol: "PlyPopo5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPpGr.dat",
            joint_symbol: "PlyPopo5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPpOr.dat",
            joint_symbol: "PlyPopo5KOr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPpRe.dat",
            joint_symbol: "PlyPopo5KRe_Share_joint",
        },
    ],
};

/// ftNn_Init_* strings; ftData_Table_Unk0[11] has 321 animation rows.
pub const NANA_DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Nana,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Nana),
    data_file: "PlNn.dat",
    data_symbol: "ftDataNana",
    animation_file: "PlNnAJ.dat",
    animation_count: 321,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlNnNr.dat",
            joint_symbol: "PlyNana5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNnYe.dat",
            joint_symbol: "PlyNana5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNnAq.dat",
            joint_symbol: "PlyNana5KAq_Share_joint",
        },
        CostumeDescriptor {
            file: "PlNnWh.dat",
            joint_symbol: "PlyNana5KWh_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[10] and [11]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups (ftDataPopo +1C..+20, ftDataNana alike).
const PART_ANIMATION_COUNT: usize = 3;
