//! Popo and Nana load/reset hooks: ftpopo.c and ftnana.c.
use crate::attributes::{self, IceClimberAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, CharacterForm, CharacterState, Fighter, MotionRow,
};
use melee_types::FighterKind;

/// ftPopo_FighterVars (+222C..+2253), which both climbers carry. OnDeath
/// clears every field (ftPp_Init_OnDeath, ftNn_Init_OnDeath).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClimberVars {
    /// +222C: the ice block the Ice Shot holds until it launches it.
    pub ice: bool,
    /// death2_cb and take_dmg_cb are ftPp_Init_8011F060 (installed with the
    /// ice block, removed by a motion change or its release).
    pub ice_callbacks: bool,
    /// accessory4_cb.
    pub accessory: crate::climber::Accessory,
    /// ftParts_80074A4C(gobj, 0 | 1, 0): model groups 0 and 1.
    pub model_groups: [i32; 2],
    /// +2234: x2234.
    // TODO(meaning): written by the specials.
    pub x2234: u32,
    /// +2230 bit 0.
    // TODO(meaning): written by the specials.
    pub x2230_b0: bool,
    /// +224C: an aerial Ice Shot lifted the climber since it last landed
    /// (cleared by ftCo_Landing_Enter too, ftCo_Landing.c:71).
    pub air_ice_shot_used: bool,
    /// +2250: how far below the usual height a repeated aerial Ice Shot
    /// makes its block.
    pub ice_drop: f32,
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
        self.air_ice_shot_used = word(0x224C) != 0;
        self.ice_drop = f32::from_bits(word(0x2250));
    }
}

/// Which climber a fighter is. Retail runs both on one code path
/// (ftNn_Init_MotionStateTable repeats Popo's); only load, reset and a few
/// voice clips look at the kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Climber {
    /// FTKIND_POPO, the climber the player controls.
    Popo,
    /// FTKIND_NANA, the partner fighter (x221F_b4) whose inputs melee-cpu
    /// supplies.
    Nana,
}

/// Either climber's payload: one type, so the shared hooks and specials
/// compile once; `POPO_TABLE` and `NANA_TABLE` differ only in descriptor.
#[derive(Clone, Debug)]
pub struct IceClimber {
    pub climber: Climber,
    pub attributes: IceClimberAttributes,
    pub vars: ClimberVars,
}

/// Nana's form: her own archive and descriptor, Popo's payload type.
pub struct Nana;

impl CharacterForm for Nana {
    type Character = IceClimber;
    fn form_descriptor() -> &'static CharacterDescriptor {
        &NANA_DESCRIPTOR
    }
    fn read_form(
        data: &hsd_archive::Archive,
    ) -> Result<IceClimber, melee_ft::desc::FighterDescError> {
        IceClimber::read(data, Climber::Nana)
    }
}

impl IceClimber {
    fn read(
        data: &hsd_archive::Archive,
        climber: Climber,
    ) -> Result<Self, melee_ft::desc::FighterDescError> {
        let descriptor = match climber {
            Climber::Popo => &POPO_DESCRIPTOR,
            Climber::Nana => &NANA_DESCRIPTOR,
        };
        Ok(Self {
            climber,
            attributes: attributes::read(data, descriptor.data_symbol)?,
            vars: ClimberVars::default(),
        })
    }
}

pub static POPO_TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<IceClimber>();
pub static NANA_TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<IceClimber>().with_descriptor(nana_descriptor);

fn nana_descriptor() -> &'static CharacterDescriptor {
    &NANA_DESCRIPTOR
}

impl CharacterCallbacks for IceClimber {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &POPO_TABLE
    }
    /// Nana runs Popo's hooks under her own descriptor. Kept out of line so
    /// scene composition calls this crate's one copy.
    #[inline(never)]
    fn into_state(self) -> CharacterState {
        let nana = self.climber == Climber::Nana;
        let mut state = CharacterState::new(self);
        if nana {
            state.use_table(&NANA_TABLE);
        }
        state
    }
    fn kind(&self) -> FighterKind {
        match self.climber {
            Climber::Popo => FighterKind::Popo,
            Climber::Nana => FighterKind::Nana,
        }
    }
    /// Popo's: the player's own climber. Nana's form is `Nana`.
    fn descriptor() -> &'static CharacterDescriptor {
        &POPO_DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Self::read(data, Climber::Popo)
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [MotionRow] = &crate::SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    const SPECIAL_PARTNER_SYNC: &'static [bool] = &crate::SPECIAL_PARTNER_SYNC;
    /// ftPp_Init_OnKnockbackEnter: Fighter_OnKnockbackEnter(gobj, 1).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftPp_Init_OnKnockbackExit: Fighter_OnKnockbackExit(gobj, 1).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    fn restore_saved(&mut self, raw: &[u8]) {
        self.vars.restore(raw);
    }
    /// ftCo_Landing_Enter, ftCo_Landing.c:69-71: FTKIND_POPO/NANA.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.vars.air_ice_shot_used = false;
    }
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: death2_cb and
    /// take_dmg_cb go.
    fn on_motion_change(&mut self) {
        self.vars.ice_callbacks = false;
    }
    /// ftCommon_8007DB58: take_dmg_cb, ftPp_Init_8011F060.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::special::lose_articles);
    /// ftCo_800D331C: death2_cb, ftPp_Init_8011F060.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::special::lose_articles);
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match crate::climber::vars(f).accessory {
            crate::climber::Accessory::IceShot => crate::special_n::accessory(f, assets),
            crate::climber::Accessory::None => {}
        }
    }
    /// ftData_SpecialN/S/Hi/Lw[kind] and their aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        crate::special::enter(f, slot, airborne, assets);
    }
    /// ftPp_Init_OnLoad (8011EF14): x2222_b5, PUSH_ATTRS, x40 = attributes
    /// +0; the three item registrations (it_8026B3F8) belong to the item
    /// scene. ftdata.c supplies every special.
    ///
    /// ftNn_Init_OnLoad (80122EB4): x2222_b4, PUSH_ATTRS (through
    /// ftPp_Init_OnLoadForNana), x40 = attributes +C4. ftData_SpecialS and
    /// ftData_SpecialHi (and their aerial tables) are NULL for Nana.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        match self.climber {
            Climber::Popo => {
                capabilities.leads_partner = true;
                capabilities.spawn_offset = self.attributes.leader_spawn_offset;
                capabilities.specials = [true; 4];
            }
            Climber::Nana => {
                capabilities.refuses_healing_items = true;
                capabilities.cpu_partner = true;
                capabilities.spawn_offset = self.attributes.partner_spawn_offset;
                capabilities.armor = self.attributes.partner_armor;
                capabilities.specials = [false, false, true, true];
            }
        }
    }
    /// ftPp_Init_OnDeath (8011EFE8); ftNn_Init_OnDeath (80122F28):
    /// dmg.armor0 = attributes +C8 (see `on_load`), then the shared reset.
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
