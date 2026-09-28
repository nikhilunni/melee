//! Pichu load/reset hooks, ft/kinds/ftPichu/ftpichu.c.
use crate::attributes::{read_pichu_attributes, PikachuAttributes};
use ft_pikachu_family::{PikachuFamily, Specials};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    ActionId, Capabilities, CharacterCallbacks, CharacterState, Fighter, ForwardSmashVariant,
    JabVariant, MotionRow, SpecialSlot,
};
use melee_types::FighterKind;

/// ftParts_80074A4C's "hidden" selection.
const HIDDEN: i32 = -1;

#[derive(Clone, Debug)]
pub struct Pichu {
    pub attributes: PikachuAttributes,
    /// fp->x619_costume_id: which accessory group ftPc_Init_OnDeath shows.
    pub costume: u8,
    /// Model groups 0..=3 as ftPc_Init_OnDeath leaves them: 0 the body,
    /// 1..=3 the costume accessories (one shown for costumes 1..=3).
    pub model_groups: [i32; 4],
    /// mv.pk and the callbacks the specials install.
    pub specials: Specials,
}
impl Pichu {
    pub fn new(attributes: PikachuAttributes) -> Self {
        let mut pichu = Self {
            attributes,
            costume: 0,
            model_groups: [0; 4],
            specials: Specials::default(),
        };
        pichu.reset_model_groups();
        pichu
    }
    /// ftPc_Init_OnDeath (80149EAC): group 0 selection 0; the costume's
    /// accessory group (costume 1..=3 shows group 1..=3) selection 0 and the
    /// others hidden.
    fn reset_model_groups(&mut self) {
        self.model_groups[0] = 0;
        for group in 1..=3 {
            self.model_groups[group] = if usize::from(self.costume) == group {
                0
            } else {
                HIDDEN
            };
        }
    }
}
impl PikachuFamily for Pichu {
    /// ftPk_SpecialHiStart1_Anim: `fp->kind != FTKIND_PICHU` skips the trail.
    const QUICK_ATTACK_TRAIL: bool = false;
    const JOLT_SOUND: u32 = PICHU_JOLT_SOUND;
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

/// ftPk_SpecialN_Anim / ftPk_SpecialAirN_Anim: ft_PlaySFX(fp, 230067, 127,
/// 64) for FTKIND_PICHU.
pub const PICHU_JOLT_SOUND: u32 = 230067;

static SPECIAL_ROWS: [MotionRow; ft_pikachu_family::FamilyState::COUNT] =
    ft_pikachu_family::rows::<Pichu>();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Pichu>();

impl CharacterCallbacks for Pichu {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftPc_Init_OnKnockbackEnter (Fighter_OnKnockbackEnter(gobj, 1)).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    /// ftPc_Init_OnKnockbackExit (Fighter_OnKnockbackExit(gobj, 1)).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] =
        &ft_pikachu_family::special_moves();
    /// ftData_SpecialN/S/Hi/Lw[Pichu] and the aerial tables: Pikachu's.
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
        if kind == melee_types::ItemKind::PichuThunder {
            ft_pikachu_family::special_lw::lead_gone::<Self>(f);
        }
    };
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        ft_pikachu_family::retained_scratch_word::<Self>;
    /// ftCo_Attack1.c getMotionFlags and doAttack12: FTKIND_PICHU arms.
    fn jab_variant(&self) -> JabVariant {
        JabVariant::Repeating
    }
    /// ftCo_AttackS4.c decideFighter: FTKIND_PICHU arm.
    fn forward_smash_variant(&self) -> ForwardSmashVariant {
        ForwardSmashVariant::EffectHitlagCallbacks
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Pichu
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_pichu_attributes(data)?))
    }
    fn on_costume_loaded(
        &mut self,
        _archive: &hsd_archive::Archive,
        costume: u8,
    ) -> melee_ft::fighter::assets::Result<()> {
        self.costume = costume;
        self.reset_model_groups();
        Ok(())
    }
    /// ftPc_Init_OnLoad (80149E34): can_walljump, ftPk_Init_OnLoadForPichu's
    /// PUSH_ATTRS and three item registrations (the bolt and both jolts,
    /// it_8026B3F8); ftdata.c supplies all four specials.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
    }
    /// ftPc_Init_OnDeath (80149EAC).
    fn on_reset(&mut self) {
        self.reset_model_groups();
    }
}

/// ftPc_Init_* strings (ftpichu.c); ftData_Table_Unk0[23] has 320
/// animation rows (ftCo_SM_Count plus ftPk_SM_SelfCount). PlCo
/// ftPartsTable maps Pichu's 46 joints to 54 parts; ftData.x1C holds two
/// part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Pichu,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Pichu),
    data_file: "PlPc.dat",
    data_symbol: "ftDataPichu",
    animation_file: "PlPcAJ.dat",
    animation_count: 320,
    part_count: PART_COUNT,
    part_animation_count: PART_ANIMATION_COUNT,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPcNr.dat",
            joint_symbol: "PlyPichu5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPcRe.dat",
            joint_symbol: "PlyPichu5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPcBu.dat",
            joint_symbol: "PlyPichu5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPcGr.dat",
            joint_symbol: "PlyPichu5KGr_Share_joint",
        },
    ],
};
/// PlCo ftPartsTable[23]: semantic parts.
const PART_COUNT: u32 = 54;
/// ftData.x1C part-animation groups.
const PART_ANIMATION_COUNT: usize = 2;
