//! Pikachu load/reset hooks, ft/kinds/ftPikachu/ftpikachu.c.
use crate::attributes::{read_pikachu_attributes, PikachuAttributes};
use ft_pikachu_family::{PikachuFamily, Specials};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    ActionId, Capabilities, CharacterCallbacks, CharacterState, Fighter, ForwardSmashVariant,
    JabVariant, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Pikachu {
    pub attributes: PikachuAttributes,
    /// ftPk_Init_OnDeath resets model groups 0 and 1 to selection 0.
    pub model_groups: [i32; 2],
    /// mv.pk and the callbacks the specials install.
    pub specials: Specials,
}
impl Pikachu {
    pub fn new(attributes: PikachuAttributes) -> Self {
        Self {
            attributes,
            model_groups: [0; 2],
            specials: Specials::default(),
        }
    }
}
impl PikachuFamily for Pikachu {
    const QUICK_ATTACK_TRAIL: bool = true;
    const JOLT_SOUND: u32 = ft_pikachu_family::special_n::PIKACHU_JOLT_SOUND;
    fn attributes(&self) -> &PikachuAttributes {
        &self.attributes
    }
    fn specials(&mut self) -> &mut Specials {
        &mut self.specials
    }
    fn specials_ref(&self) -> &Specials {
        &self.specials
    }
}

static SPECIAL_ROWS: [MotionRow; ft_pikachu_family::FamilyState::COUNT] =
    ft_pikachu_family::rows::<Pikachu>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Pikachu>();

impl CharacterCallbacks for Pikachu {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftPk_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftPk_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_pikachu_family::special_moves();
    /// ftData_SpecialN/S/Hi/Lw[Pikachu] and the aerial tables.
    fn enter_special(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
        ft_pikachu_family::enter_special::<Self>(f, slot, airborne, assets);
    }
    /// Fighter_8006C80C: the special's one-shot accessory4.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, _rng: &mut gekko_math::HsdRng) {
        ft_pikachu_family::accessory::<Self>(f, assets);
    }
    /// deal_dmg_cb = ftPk_SpecialS_ZeroVelocity while Skull Bash's launch
    /// installed it.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> = Some(|f, assets| {
        if ft_pikachu_family::special_s::deals_damage(f.motion_state.action) {
            ft_pikachu_family::special_s::deal_damage::<Self>(f, assets);
        }
    });
    /// take_dmg_cb = ftPk_SpecialLw_SetState_Unk1 while Thunder's loop
    /// installed it.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> =
        Some(ft_pikachu_family::special_lw::take_damage::<Self>);
    /// it_2725_Logic39_Destroyed: the lead bolt's end.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = |f, kind| {
        if kind == melee_types::ItemKind::PikachuThunder {
            ft_pikachu_family::special_lw::lead_gone::<Self>(f);
        }
    };
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        ft_pikachu_family::retained_scratch_word::<Self>;
    /// ftCo_Attack1.c getMotionFlags and doAttack12: FTKIND_PIKACHU arms.
    fn jab_variant(&self) -> JabVariant {
        JabVariant::Repeating
    }
    /// ftCo_AttackS4.c decideFighter: FTKIND_PIKACHU arm.
    fn forward_smash_variant(&self) -> ForwardSmashVariant {
        ForwardSmashVariant::EffectHitlagCallbacks
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Pikachu
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_pikachu_attributes(data)?))
    }
    /// ftPk_Init_OnLoad (80124528): PUSH_ATTRS and three item registrations
    /// (the bolt and both jolts, it_8026B3F8); ftdata.c supplies all four
    /// specials. The attached-item model hooks (ftPk_Init_UnkMotionStates1/2,
    /// through ftCommon_8007F8E8/8007F948) need a head item, which a match
    /// without items never attaches.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftPk_Init_OnDeath (80124620): ftParts_80074A4C(gobj, 0, 0) and (1, 0).
    fn on_reset(&mut self) {
        self.model_groups = [0; 2];
    }
}

/// ftPk_Init_* strings; ftData_Table_Unk0[12] has 320 animation rows
/// (ftCo_SM_Count plus ftPk_SM_SelfCount).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Pikachu,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Pikachu),
    data_file: "PlPk.dat",
    data_symbol: "ftDataPikachu",
    animation_file: "PlPkAJ.dat",
    animation_count: 320,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPkNr.dat",
            joint_symbol: "PlyPikachu5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkRe.dat",
            joint_symbol: "PlyPikachu5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkBu.dat",
            joint_symbol: "PlyPikachu5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPkGr.dat",
            joint_symbol: "PlyPikachu5KGr_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[12]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 2;
