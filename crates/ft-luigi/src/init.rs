//! Luigi load/reset hooks, ft/kinds/ftLuigi/ftluigi.c.
use crate::attributes::{read_luigi_attributes, LuigiAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter,
};
use melee_types::FighterKind;

/// The accessory4 callback a special installed; a motion change removes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftLg_SpecialN_FireSpawn: the fireball on the script's throw flag.
    Fireball,
    /// ftLg_SpecialS_SetGFX: Green Missile's charge keeps x2219_b0 set.
    ChargeEffects,
}

#[derive(Clone, Debug)]
pub struct Luigi {
    pub attributes: LuigiAttributes,
    /// ftLg_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C, x222C_cycloneCharge: an aerial Cyclone already rose.
    /// Only the Cyclone's own landing clears it. Fighter_Create leaves it
    /// as stale heap (fighter.c:855-858); a savestate carries retail's.
    pub cyclone_charged: bool,
    /// Fighter +2230: stale heap; nothing in retail writes or reads it.
    pub unknown_2230: u32,
    /// Fighter +2234, cleared on death.
    // TODO(meaning): no retail reader found.
    pub unknown_2234: u32,
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// Green Missile's motion scratch.
    pub green_missile: crate::special_s::GreenMissile,
    /// Super Jump Punch's steering (Fighter +6BC).
    pub super_jump_punch: crate::special_hi::SuperJumpPunch,
    /// Luigi Cyclone's motion scratch and callbacks.
    pub cyclone: crate::special_lw::Cyclone,
}
impl Luigi {
    pub fn new(attributes: LuigiAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            cyclone_charged: false,
            unknown_2230: 0,
            unknown_2234: 0,
            accessory: Accessory::None,
            green_missile: Default::default(),
            super_jump_punch: Default::default(),
            cyclone: Default::default(),
        }
    }
}

static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; crate::SPECIAL_ROW_COUNT] =
    crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Luigi>();

impl CharacterCallbacks for Luigi {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftLg_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftLg_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Luigi] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// ftLg_SpecialS_SetVars's misfire draw, deferred to the end of the
    /// entering IASA.
    const INPUT_RNG: Option<fn(&mut Fighter, &mut gekko_math::HsdRng)> =
        Some(crate::special_s::draw_misfire);
    /// ftCommon_8007DB58: take_dmg_cb, the Cyclone's ftLg_SpecialLw_UpdateRot.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::special_lw::clear_tilt);
    /// ftCo_800D331C: death2_cb, the same.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::special_lw::clear_tilt);
    /// deal_dmg_cb = ftLg_SpecialS_OnGiveDamage while Green Missile's flight
    /// installed it.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> = Some(|f, assets| {
        if crate::special_s::deals_damage(f.motion_state.action) {
            crate::special_s::deal_damage(f, assets);
        }
    });
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.cyclone.callbacks = false;
    }
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Luigi>().accessory {
            Accessory::Fireball => crate::special_n::spawn_fireball(f, assets),
            Accessory::ChargeEffects => crate::special_s::charge_effects(f),
            Accessory::None => {}
        }
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Luigi
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_luigi_attributes(data)?))
    }
    /// ftLuigi_FighterVars +222C..+2237.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.cyclone_charged = word(0x222C) != 0;
        self.unknown_2230 = word(0x2230);
        self.unknown_2234 = word(0x2234);
    }
    /// ftLg_Init_OnLoad (80142324): PUSH_ATTRS and the fireball's item
    /// registration (it_8026B3F8, the item scene's); ftdata.c supplies all
    /// four specials.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftLg_Init_OnDeath (801422E8): ftParts_80074A4C(gobj, 0, 0) and x2234.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.unknown_2234 = 0;
    }
}

/// ftLg_Init_* strings (ftluigi.c); ftData_Table_Unk0 has 312 animation
/// rows (ftCo_SM_Count plus ftLg_SM_SelfCount).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Luigi,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Luigi),
    data_file: "PlLg.dat",
    data_symbol: "ftDataLuigi",
    animation_file: "PlLgAJ.dat",
    animation_count: 312,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlLgNr.dat",
            joint_symbol: "PlyLuigi5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLgWh.dat",
            joint_symbol: "PlyLuigi5KWh_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLgAq.dat",
            joint_symbol: "PlyLuigi5KAq_Share_joint",
        },
        CostumeDescriptor {
            file: "PlLgPi.dat",
            joint_symbol: "PlyLuigi5KPi_Share_joint",
        },
    ],
};
