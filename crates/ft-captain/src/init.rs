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
    /// mv.ca.speciallw: the grounded Falcon Kick's hit slowdown.
    pub falcon_kick: crate::special_lw::FalconKick,
    /// mv.ca.specials: the aerial Raptor Boost's vertical velocity.
    pub raptor_boost: crate::special_s::RaptorBoost,
    /// mv.ca.specialhi: Falcon Dive's carried velocity and flags.
    pub falcon_dive: crate::special_hi::FalconDive,
}
impl CaptainFalcon {
    pub fn new(attributes: CaptainAttributes) -> Self {
        Self {
            attributes,
            raptor_boost_start_effect_active: false,
            raptor_boost_lunge_effect_active: false,
            model_group: 0,
            falcon_kick: Default::default(),
            raptor_boost: Default::default(),
            falcon_dive: Default::default(),
        }
    }
}
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<CaptainFalcon>();

impl CharacterCallbacks for CaptainFalcon {
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &crate::special_moves();
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &crate::special_rows();
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        air: bool,
        a: &melee_ft::fighter::assets::FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, air, a),
            SpecialSlot::Side => crate::special_s::enter(f, air, a),
            SpecialSlot::Up => crate::special_hi::enter(f, air, a),
            SpecialSlot::Down => crate::special_lw::enter(f, air, a),
        }
    }
    const DEAL_DAMAGE: Option<
        fn(&mut melee_ft::fighter::Fighter, &melee_ft::fighter::assets::FighterAssets),
    > = Some(crate::special_lw::deal_damage);
    const HURTBOX_DETECT: Option<
        fn(
            &mut melee_ft::fighter::Fighter,
            &melee_ft::fighter::assets::FighterAssets,
            melee_ft::fighter::damage::InertTouch,
        ),
    > = Some(crate::special_s::detect);
    /// ftCommon_8007E2D0's grab_cb (ftCa_SpecialLw_800E5128) and grabbed_cb
    /// (ftCo_8009CA0C).
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = crate::special_hi_catch::grab;
    /// accessory4: Falcon Dive's ftCa_SpecialLw_800E550C.
    fn accessory(
        fighter: &mut melee_ft::fighter::Fighter,
        _assets: &melee_ft::fighter::assets::FighterAssets,
        _rng: &mut gekko_math::HsdRng,
    ) {
        crate::special_hi_catch::follow_victim(fighter);
    }
    /// ftCa_Init_800E28C8: Raptor Boost's take_dmg_cb.
    const TAKE_DAMAGE: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(crate::special_s::remove_effects);
    /// ftCa_Init_800E28C8: Raptor Boost's death2_cb.
    const DEATH: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(crate::special_s::remove_effects);
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| {
        let falcon = state.get::<Self>();
        crate::special_s::retained_scratch_word(falcon, action)
            .or_else(|| crate::special_hi::retained_scratch_word(falcon, action))
    };
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
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
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Captain),
    data_file: "PlCa.dat",
    data_symbol: "ftDataCaptain",
    animation_file: "PlCaAJ.dat",
    animation_count: 318,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
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
