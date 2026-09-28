//! Roy load/reset hooks, ft/kinds/ftEmblem/ftemblem.c.
use crate::attributes::{read_roy_attributes, MarsAttributes};
use ft_mars_family::{MarsFamily, Specials};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    ActionId, Capabilities, CharacterCallbacks, CharacterState, DefenseContact, DefenseHit, Fighter,
    ItemDefenseContact, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Roy {
    pub attributes: MarsAttributes,
    /// mv.ms, +222C and the retained mv+4 word of the shared specials.
    pub specials: Specials,
    /// ftFe_Init_OnDeath resets model groups 0 and 1 to 0 and group 2 to -1.
    pub model_groups: [i32; 3],
}
impl Roy {
    pub fn new(attributes: MarsAttributes) -> Self {
        Self {
            attributes,
            specials: Specials::default(),
            model_groups: DEATH_MODEL_GROUPS,
        }
    }
}
/// ftFe_Init_OnDeath (8014EEF8): ftParts_80074A4C(gobj, 0, 0), (1, 0), (2, -1).
const DEATH_MODEL_GROUPS: [i32; 3] = [0, 0, -1];

impl MarsFamily for Roy {
    /// ftMs_SpecialN_801365A8 / 8013666C, kind 26 arms.
    const RELEASE_EFFECTS: [u16; 2] = [0x511, 0x512];
    /// ftMs_SpecialNStart_Anim: any kind but Marth.
    const CHARGE_COLOR_ANIMATION: u8 = 100;
    /// ftMs_SpecialLw_80139140, FTKIND_EMBLEM arm.
    const COUNTER_EFFECT: u16 = 0x510;
    const COUNTER_SETS_HIT_DAMAGE: bool = true;
    fn attributes(&self) -> &MarsAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [MotionRow; ft_mars_family::SPECIAL_COUNT] = ft_mars_family::rows::<Roy>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Roy>();

impl CharacterCallbacks for Roy {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftFe_Init_OnKnockbackEnter (8014F168): Fighter_OnKnockbackEnter(gobj, 1).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftFe_Init_OnKnockbackExit (8014F1AC): Fighter_OnKnockbackExit(gobj, 1).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_mars_family::special_moves();
    /// ftFe_Init_MotionStateTable (803D2E80): Marth's rows.
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    /// ftData_SpecialN/S/Hi/Lw[Roy]: the ftMs_ entries.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, air: bool, assets: &FighterAssets) {
        ft_mars_family::enter_special::<Self>(f, slot, air, assets);
    }
    /// Fighter_8006C80C: Flare Blade's release flash.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        ft_mars_family::special_n::accessory::<Self>(f, assets);
    }
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        ft_mars_family::retained_scratch_word::<Self>;
    const DEFENSE_CONTACT: Option<DefenseContact> =
        Some(ft_mars_family::special_lw::contact::<Self>);
    const PROCESS_DEFENSE_HIT: Option<DefenseHit> =
        Some(ft_mars_family::special_lw::process_hit::<Self>);
    /// No special reads fp->item_gobj; a held item stays in hand.
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const ITEM_DEFENSE_CONTACT: Option<ItemDefenseContact> =
        Some(ft_mars_family::special_lw::item_contact::<Self>);

    fn kind(&self) -> FighterKind {
        FighterKind::Emblem
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_roy_attributes(data)?))
    }
    /// Roy +222C (u.ms.x222C): the side-special boost already spent.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.specials.side_special_boost_used =
            u32::from_be_bytes(raw[0x222C..0x2230].try_into().unwrap()) != 0;
    }
    /// ftFe_Init_OnLoad (8014F124) -> ftMs_Init_OnLoadForRoy (80136474):
    /// PUSH_ATTRS only. ftdata.c supplies all four specials.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftFe_Init_OnDeath (8014EEF8): three model groups and Fighter +222C.
    fn on_reset(&mut self) {
        self.specials.reset();
        self.model_groups = DEATH_MODEL_GROUPS;
    }
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:64-67.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.specials.side_special_boost_used = false;
    }
}

/// ftFe_Init_* strings; ftData_Table_Unk0[26] has 327 animation rows
/// (ftCo_SM_Count plus Marth's 32 submotions).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Emblem,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Emblem),
    data_file: "PlFe.dat",
    data_symbol: "ftDataEmblem",
    animation_file: "PlFeAJ.dat",
    animation_count: 327,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlFeNr.dat",
            joint_symbol: "PlyEmblem5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlFeRe.dat",
            joint_symbol: "PlyEmblem5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlFeBu.dat",
            joint_symbol: "PlyEmblem5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlFeGr.dat",
            joint_symbol: "PlyEmblem5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlFeYe.dat",
            joint_symbol: "PlyEmblem5KYe_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[26]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftDataEmblem.x1C part-animation groups (three, as Marth).
const PART_ANIMATION_COUNT: usize = 3;
