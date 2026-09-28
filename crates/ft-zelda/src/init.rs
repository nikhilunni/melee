//! Zelda load/reset hooks, ft/kinds/ftZelda/ftzelda.c.
use crate::attributes::{read_zelda_attributes, ZeldaAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

/// The accessory4 callback a special installed; a motion change removes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftZd_SpecialLw_8013ADB4: the transformation's start sparkle.
    TransformStart,
    /// ftZd_SpecialLw_8013AEAC: hand the match to Sheik (ftCommon_8007EFC8).
    TransformHandOver,
    /// ftZd_SpecialLw_8013AE30: the arrival sparkle.
    TransformArrival,
    /// ftZd_SpecialN_8013A830: Nayru's Love's crystal on the ground.
    NayrusLoveCrystal,
    /// ftZd_SpecialN_8013A8AC: the crystal in the air.
    NayrusLoveAirCrystal,
    /// ftZd_SpecialHi_801396AC: Farore's Wind's wind model.
    FaroresWindStart,
    /// ftZd_SpecialHi_8013979C: Farore's Wind's reappearance puff.
    FaroresWindReappear,
}

#[derive(Clone, Debug)]
pub struct Zelda {
    pub attributes: ZeldaAttributes,
    /// ftZd_Init_OnDeath resets model groups 0 and 1 to selection 0.
    pub model_groups: [i32; 2],
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// Farore's Wind's motion scratch.
    pub farores_wind: crate::special_hi::FaroresWind,
    /// Nayru's Love's hang and reflector.
    pub nayrus_love: crate::special_n::NayrusLove,
    /// Din's Fire's counters and Zelda's hold on the fire.
    pub dins_fire: crate::special_s::DinsFire,
}
impl Zelda {
    pub fn new(attributes: ZeldaAttributes) -> Self {
        Self {
            attributes,
            model_groups: [0; 2],
            accessory: Accessory::None,
            farores_wind: Default::default(),
            nayrus_love: Default::default(),
            dins_fire: Default::default(),
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Zelda>();

impl CharacterCallbacks for Zelda {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS
    /// entry (its Zelda arm needs DbLevel >= 3, never true in retail).
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftZd_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftZd_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Zelda] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
        }
    }
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, _assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Zelda>().accessory {
            Accessory::TransformStart => crate::special_lw::sparkle(f, false),
            Accessory::TransformHandOver => crate::special_lw::hand_over(f),
            Accessory::TransformArrival => crate::special_lw::sparkle(f, true),
            Accessory::FaroresWindStart => crate::special_hi::wind(f),
            Accessory::NayrusLoveCrystal => crate::special_n::crystal(f, false),
            Accessory::NayrusLoveAirCrystal => crate::special_n::crystal(f, true),
            Accessory::FaroresWindReappear => crate::special_hi::reappear(f),
            Accessory::None => {}
        }
    }
    /// Nayru's Love's reflector (ftColl_CreateReflectHit, an empty hit
    /// callback).
    const REFLECTOR_CONTACT: Option<melee_ft::fighter::reflection::CharacterContact> =
        Some(crate::special_n::reflector_contact);
    const REFLECT_HIT: Option<melee_ft::fighter::reflection::CharacterResponse> =
        Some(crate::special_n::reflect_hit);
    /// ftZd_SpecialLw_8013B4D8: Zelda arrives from Sheik's transformation.
    const TRANSFORMATION_ARRIVAL: fn(
        &mut Fighter,
        &FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = crate::special_lw::arrive;

    fn kind(&self) -> FighterKind {
        FighterKind::Zelda
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_zelda_attributes(data)?))
    }
    /// ftZelda_FighterVars +222C: a saved Din's Fire is not modelled yet.
    fn restore_saved(&mut self, raw: &[u8]) {
        if raw[0x222C..0x2230] != [0; 4] {
            unimplemented!("ftZelda_FighterVars: a saved Din's Fire GObj (+222C)");
        }
    }
    /// ftZd_Init_OnLoad (80139334): PUSH_ATTRS. The two Din's Fire item
    /// registrations (it_8026B3F8) belong to the item scene; Zelda cannot
    /// walljump.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftZd_Init_OnDeath (801392E8): ftParts_80074A4C(gobj, 0, 0), (1, 0).
    fn on_reset(&mut self) {
        self.model_groups = [0; 2];
        self.dins_fire.fire_out = false;
    }
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go (Din's Fire's take_dmg_cb and death2_cb).
    fn on_motion_change(&mut self) {
        self.dins_fire.damage_callbacks = false;
    }
    /// ftCommon_8007DB58: take_dmg_cb, ftZd_Init_801393AC while Din's
    /// Fire installed it.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::special_s::damage_callback);
    /// ftCo_800D331C: death2_cb, the same callback.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::special_s::damage_callback);
    /// itZeldaDinFire_Logic65_Destroyed: the fire lets Zelda go
    /// (ftZd_SpecialLw_8013B5C4) while it is hers.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = |f, kind| {
        if kind == melee_types::ItemKind::ZeldaDinFire {
            crate::special_s::fire_gone(f);
        }
    };
    /// The common owner view plus what Din's Fire reads of Zelda.
    fn item_owner(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
        let mut owner = melee_it::ItemOwner {
            illusion: None,
            position: f.physics.position,
            facing: f.physics.facing,
            hold_position: f.physics.position,
            blaster_action: 9,
            remove_blaster: true,
            motion: f.motion_state.action.0,
            stick: hsd_types::Vec2::new(f.input.current.stick.x, f.input.current.stick.y),
            steering_article: false,
            detonating_article: false,
            holds_needles: false,
            articles_fired: 0,
            charge: None,
        };
        crate::special_s::item_owner(f, &mut owner);
        owner
    }
}

/// ftZd_Init_* strings; ftData_Table_Unk0[FTKIND_ZELDA] has 311 animation
/// rows (ftCo_SM_Count plus 16 of Zelda's).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Zelda,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Zelda),
    data_file: "PlZd.dat",
    data_symbol: "ftDataZelda",
    animation_file: "PlZdAJ.dat",
    animation_count: 311,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlZdNr.dat",
            joint_symbol: "PlyZelda5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlZdRe.dat",
            joint_symbol: "PlyZelda5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlZdBu.dat",
            joint_symbol: "PlyZelda5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlZdGr.dat",
            joint_symbol: "PlyZelda5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlZdWh.dat",
            joint_symbol: "PlyZelda5KWh_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[FTKIND_ZELDA]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 3;
