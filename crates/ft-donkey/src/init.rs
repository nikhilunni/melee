//! Donkey Kong load/reset hooks, ft/kinds/ftDonkey/ftdonkey.c.
use crate::attributes::{read_donkey_attributes, DonkeyAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct DonkeyKong {
    pub attributes: DonkeyAttributes,
    /// ftDk_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C, u.dk.x222C: Giant Punch's stored arm swings, kept
    /// between states until the punch, a hit taken or a death.
    pub punch_swings: i32,
    /// Fighter +2230, u.dk.x2230. Fighter_Create leaves it as stale heap; a
    /// savestate carries retail's.
    // TODO(meaning): no retail reader found.
    pub unknown_2230: u32,
    /// take_dmg_cb / death2_cb = ftDk_Init_8010D774, installed by Giant
    /// Punch and Spinning Kong until the next motion change.
    pub damage_callbacks: bool,
    /// take_dmg_2_cb = ftDk_SpecialN_DestroyAllEffects, Giant Punch's.
    pub hit_destroys_effects: bool,
    /// The accessory4 callback a special installed.
    pub accessory: Accessory,
    /// Giant Punch's motion scratch (mv.dk.specialn).
    pub giant_punch: crate::special_n::GiantPunch,
    /// mv.dk.speciallw.x0: B was pressed during the current Hand Slap.
    pub slap_again: bool,
}

/// accessory4_cb while a special owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftDk_SpecialLw_8010E0CC: the grounded Headbutt's model.
    HeadbuttEffect,
    /// ftDk_SpecialLw_8010E148: the aerial Headbutt's model.
    AirHeadbuttEffect,
    /// ftDk_Init_8010DB3C: Hand Slap's quake hitboxes.
    Quake,
}
impl DonkeyKong {
    pub fn new(attributes: DonkeyAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            punch_swings: 0,
            unknown_2230: 0,
            damage_callbacks: false,
            hit_destroys_effects: false,
            accessory: Accessory::None,
            giant_punch: Default::default(),
            slap_again: false,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<DonkeyKong>();

impl CharacterCallbacks for DonkeyKong {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftDk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftDk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Donkey] and the aerial tables (no aerial
    /// down special).
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Down => {
                assert!(!airborne, "ftData_SpecialAirLw[FTKIND_DONKEY] is NULL");
                crate::special_lw::enter(f, assets)
            }
        }
    }
    /// Fighter_8006C80C: Headbutt's one-shot accessory4.
    fn accessory(f: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<DonkeyKong>().accessory {
            Accessory::HeadbuttEffect => crate::special_s::spawn_effect(f, false),
            Accessory::AirHeadbuttEffect => crate::special_s::spawn_effect(f, true),
            Accessory::Quake | Accessory::None => {}
        }
    }
    /// Fighter_8006C80C: Hand Slap's accessory4 reads the floor.
    const MAP_ACCESSORY: Option<fn(&mut Fighter, &FighterAssets, &mut melee_mp::CollMap)> =
        Some(|f, _assets, map| {
            if f.core.accessory4_armed
                && f.character.get::<DonkeyKong>().accessory == Accessory::Quake
            {
                crate::special_lw::quake(f, map);
            }
        });
    /// ftCommon_8007DB58: take_dmg_cb (ftDk_Init_8010D774) when installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::common::damage_callback);
    /// ftCo_800D331C: death2_cb, the same callback.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::common::damage_callback);
    /// Fighter_ProcessHit: take_dmg_2_cb (ftDk_SpecialN_DestroyAllEffects)
    /// while Giant Punch installed it.
    const HIT_TAKEN: Option<fn(&mut Fighter)> = Some(|f| {
        if f.character.get::<DonkeyKong>().hit_destroys_effects {
            f.effects.push(melee_ef::request::EffectRequest::DestroyOwned);
        }
    });
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.damage_callbacks = false;
        self.hit_destroys_effects = false;
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Donkey
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_donkey_attributes(data)?))
    }
    /// ftDonkey_FighterVars +222C..+2233.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.punch_swings = word(0x222C) as i32;
        self.unknown_2230 = word(0x2230);
        if self.punch_swings == self.attributes.giant_punch.max_swings {
            // The glow is the fighter's secondary colour slot, which the
            // character payload cannot restore.
            unimplemented!("ftDk_Init_UnkMotionStates4: a savestate with a full Giant Punch");
        }
    }
    /// ftDk_Init_OnLoad (8010D9AC): the carry walks' animation lengths,
    /// PUSH_ATTRS, x2222_b0 and x2CC (the cargo carry's attributes). Donkey
    /// Kong has no aerial down special (ftData_SpecialAirLw[3] is NULL).
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.air_specials = Some([true, true, true, false]);
    }
    /// ftDk_Init_OnDeath (8010D740): the stored punch goes and
    /// ftParts_80074A4C(gobj, 0, 0).
    fn on_reset(&mut self) {
        self.punch_swings = 0;
        self.model_group = 0;
    }
}

/// ftDk_Init_* strings (ftdonkey.c); ftData_Table_Unk0[3] has 337 animation
/// rows. PlCo ftPartsTable maps Donkey Kong's joints to 54 parts;
/// ftData.x1C holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Donkey,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Donkey),
    data_file: "PlDk.dat",
    data_symbol: "ftDataDonkey",
    animation_file: "PlDkAJ.dat",
    animation_count: 337,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlDkNr.dat",
            joint_symbol: "PlyDonkey5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkBk.dat",
            joint_symbol: "PlyDonkey5KBk_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkRe.dat",
            joint_symbol: "PlyDonkey5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkBu.dat",
            joint_symbol: "PlyDonkey5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlDkGr.dat",
            joint_symbol: "PlyDonkey5KGr_Share_joint",
        },
    ],
};
