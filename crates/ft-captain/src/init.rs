//! Captain Falcon load/reset hooks, ft/kinds/ftCaptain/ftcaptain.c.
use crate::attributes::CaptainAttributes;
use ft_captain_family::{
    special_hi_catch, special_lw, special_s, CaptainFamily, FamilyEffects, Specials,
};
use melee_ft::fighter::assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets};
use melee_ft::fighter::{
    ActionId, Capabilities, CharacterCallbacks, CharacterState, Fighter, MotionRow, SpecialSlot,
};
use melee_types::{FighterKind, FtPart};

#[derive(Clone, Debug)]
pub struct CaptainFalcon {
    pub attributes: CaptainAttributes,
    /// ftCa_Init_OnDeath resets model group 0 to its default variant.
    pub model_group: i32,
    /// The FighterVars flags and mv.ca scratch of the shared specials.
    pub specials: Specials,
}
impl CaptainFalcon {
    pub fn new(attributes: CaptainAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            specials: Specials::default(),
        }
    }
}
impl CaptainFamily for CaptainFalcon {
    /// The FTKIND_CAPTAIN arms.
    const EFFECTS: FamilyEffects = FamilyEffects {
        // efSync_Spawn(1167, gobj, parts[FtPart_TopN], parts[57]): efAlt
        // 0x48F's two models.
        punch: 1167,
        punch_bones: [0, 57],
        punch_wind: false,
        // efSync_Spawn(1169, gobj, HeadN): efAlt 0x491, model 0xFA4.
        boost_start: 1169,
        boost_start_part: FtPart::HeadN,
        // efSync_Spawn(1170 / 1171, gobj, TransN, &facing_dir): efAlt 0x492
        // / 0x493 (0xFA3 / 0xFA5), scaled and turned to the facing.
        ground_lunge: 1170,
        air_lunge: 1171,
        // efAsync_Spawn(gobj, &fp->x60C, 3, 0x490, foot, &angle): efAlt
        // 0x490, model 0xFA2 rotated by the angle each update.
        kick_flame: 0x490,
    };
    fn attributes(&self) -> &CaptainAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [MotionRow; ft_captain_family::ROW_COUNT] =
    ft_captain_family::special_rows::<CaptainFalcon>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<CaptainFalcon>();

impl CharacterCallbacks for CaptainFalcon {
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_captain_family::special_moves();
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, air: bool, a: &FighterAssets) {
        ft_captain_family::enter_special::<Self>(f, slot, air, a);
    }
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> =
        Some(special_lw::deal_damage::<Self>);
    const HURTBOX_DETECT: Option<
        fn(&mut Fighter, &FighterAssets, melee_ft::fighter::damage::InertTouch),
    > = Some(special_s::detect::<Self>);
    /// ftCommon_8007E2D0's grab_cb (ftCa_SpecialLw_800E5128) and grabbed_cb
    /// (ftCo_8009CA0C).
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = special_hi_catch::grab;
    /// ftCo_800DDDE4 / ftCo_800DE7C0 once the catch's animation ends.
    const SPECIAL_RELEASE: melee_ft::fighter::SpecialRelease =
        melee_ft::fighter::capture_captain::release;
    /// accessory4: Falcon Dive's ftCa_SpecialLw_800E550C.
    fn accessory(fighter: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        special_hi_catch::follow_victim(fighter);
    }
    /// ftCa_Init_800E28C8: Raptor Boost's take_dmg_cb.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(special_s::remove_effects::<Self>);
    /// ftCa_Init_800E28C8: Raptor Boost's death2_cb.
    const DEATH: Option<fn(&mut Fighter)> = Some(special_s::remove_effects::<Self>);
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        ft_captain_family::retained_scratch_word::<Self>;
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
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
        self.specials.restore_saved(raw);
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
        self.specials.reset();
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
