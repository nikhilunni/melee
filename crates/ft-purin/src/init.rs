//! Jigglypuff load/reset hooks, ft/kinds/ftPurin/ftpurin.c.
use crate::attributes::{read_purin_attributes, PurinAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    AerialJumpStyle, Capabilities, CharacterCallbacks,
};
use melee_types::FighterKind;

/// The one-shot accessory4 callback a special installed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftPr_Init_8013C94C: Sing's notes effect.
    Notes,
    /// ftPr_SpecialHi_8013CE7C: Rest's accessory, which only uninstalls itself.
    Uninstall,
}

#[derive(Clone, Debug)]
pub struct Jigglypuff {
    pub attributes: PurinAttributes,
    /// ftPr_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C: aerial Pound lunges, counted and never read.
    pub pound_count: u32,
    /// fp->mv.pr.specialhi.x0: Sing's Pokémon Stadium sleep element.
    pub stadium_sleep: bool,
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// mv+4 as the state before a special left it: Pound, Sing and Rest
    /// never write it.
    pub retained_word: Option<f32>,
    /// fp->mv.pr.specialn, Rollout's motion scratch.
    pub rollout: crate::special_n::Rollout,
    /// Fighter +2230 (u.pr.x2230): the model scale Rollout squashes.
    pub rollout_scale: hsd_types::Vec3,
    /// The callbacks the current Rollout motion installed.
    pub rollout_callbacks: crate::special_n::Callbacks,
    /// Hitbox 0's contents when it was last disabled (retail keeps them).
    pub rollout_hitbox: Option<melee_coll::hitbox::HitCapsule>,
    /// fp->u.pr.x223C: the costume's hat, loaded once by OnLoad.
    pub hat: Option<crate::hat::CostumeHat>,
}
impl Jigglypuff {
    pub fn new(attributes: PurinAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            pound_count: 0,
            stadium_sleep: false,
            accessory: Accessory::None,
            retained_word: None,
            rollout: Default::default(),
            rollout_scale: hsd_types::Vec3::ZERO,
            rollout_callbacks: Default::default(),
            rollout_hitbox: None,
            hat: None,
        }
    }
}
static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; crate::SPECIAL_ROW_COUNT] =
    crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Jigglypuff>();

impl CharacterCallbacks for Jigglypuff {
    /// ftPr_Init_OnKnockbackEnter/Exit: Fighter_OnKnockbackEnter/Exit(gobj, 1), the damage
    /// texture frames only.
    const KNOCKBACK_ENTER: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    const KNOCKBACK_EXIT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) = |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    /// ftPr_SpecialN_8014222C, Rollout's x21F8 after a cape turnaround.
    const CAPE_TURN_END: fn(&mut melee_ft::fighter::Fighter) = |_| {
        unimplemented!(
            "ftPr_SpecialN_8014222C (ftpurinspecialn.c:1447): Rollout after a cape turnaround"
        )
    };
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftData_SpecialN/S/Hi/Lw[Purin] and the aerial tables.
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
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
    /// Fighter_8006C80C: the special's one-shot accessory4.
    fn accessory(
        f: &mut melee_ft::fighter::Fighter,
        assets: &FighterAssets,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let pending = f.character.get::<Jigglypuff>().accessory;
        if !f.run_accessory4(pending != Accessory::None) {
            return;
        }
        f.character.get_mut::<Jigglypuff>().accessory = Accessory::None;
        match pending {
            Accessory::Notes => crate::special_hi::notes(f, assets),
            Accessory::Uninstall => {}
            Accessory::None => unreachable!(),
        }
    }
    /// mv+4 while a special is current: Pound, Sing and Rest keep the word
    /// the state before them left; Rollout's setup stores the integer -1
    /// there (mv.pr.specialn.x4).
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| match action.0 {
        346..=362 => Some(f32::from_bits(u32::MAX)),
        363..=372 => Some(state.get::<Self>().retained_word.unwrap_or_else(|| {
            unimplemented!("ftPr specials: mv+4 inherited from an unmodelled scratch word")
        })),
        _ => None,
    };
    /// deal_dmg_cb = ftPr_SpecialS_8013D764 while Rollout installed it.
    const DEAL_DAMAGE: Option<fn(&mut melee_ft::fighter::Fighter, &FighterAssets)> =
        Some(crate::special_n::bounce);
    /// take_dmg_cb = ftPr_SpecialS_8013D658 while Rollout installed it.
    const TAKE_DAMAGE: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(crate::special_n::restore_on_callback);
    /// death2_cb = ftPr_SpecialS_8013D658 while Rollout installed it.
    const DEATH: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(crate::special_n::restore_on_callback);
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the callbacks go.
    fn on_motion_change(&mut self) {
        self.rollout_callbacks = Default::default();
    }

    fn kind(&self) -> FighterKind {
        FighterKind::Purin
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_purin_attributes(data)?))
    }
    /// Fighter +222C (the aerial Pound count) and +2230 (Rollout's saved
    /// model scale).
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.pound_count = word(0x222C);
        self.rollout_scale = hsd_types::Vec3::new(
            f32::from_bits(word(0x2230)),
            f32::from_bits(word(0x2234)),
            f32::from_bits(word(0x2238)),
        );
    }
    /// ftPr_Init_OnLoad (8013C67C): PUSH_ATTRS, can_multijump, x2D0 = dat_attrs.
    /// No item registrations or walljump flag; all four specials exist.
    /// Dynamic chains are read from ftData.x2C by the shared asset loader.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftPr_Init_OnLoad's ftPr_Init_8013C360: the neutral costume has no
    /// hat; the others load one (see `crate::hat`).
    fn on_costume_loaded(
        &mut self,
        archive: &hsd_archive::Archive,
        costume: u8,
    ) -> melee_ft::fighter::assets::Result<()> {
        self.hat = crate::hat::CostumeHat::load(archive, costume)?;
        Ok(())
    }
    /// ftPr_Init_OnDeath (8013C318): ftParts_80074A4C(gobj, 0, 0).
    fn on_reset(&mut self) {
        self.model_group = 0;
    }
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::MultiJump
    }
    fn multi_jump_attributes(&self) -> Option<&melee_ft::fighter::multi_jump::MultiJumpAttributes> {
        Some(&self.attributes.multi_jump)
    }
    /// ftPr_Init_MotionStateTable: F1..F5 use submotions 295..299.
    fn multi_jump_animation(&self, jump: usize) -> i32 {
        const FIRST_MULTI_JUMP_ANIMATION: i32 = 295; // ftPr_SM_JumpAerialF1
        FIRST_MULTI_JUMP_ANIMATION + jump as i32
    }
}

/// ftPr_Init_* strings; ftData_Table_Unk0[15] has 327 animation rows.
/// ftparts.c's PlCo table maps 50 joints to 54 parts; ftData.x1C has two groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Purin,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Purin),
    data_file: "PlPr.dat",
    data_symbol: "ftDataPurin",
    animation_file: "PlPrAJ.dat",
    animation_count: 327,
    part_count: 54,
    part_animation_count: 2,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPrNr.dat",
            joint_symbol: "PlyPurin5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrRe.dat",
            joint_symbol: "PlyPurin5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrBu.dat",
            joint_symbol: "PlyPurin5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrGr.dat",
            joint_symbol: "PlyPurin5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPrYe.dat",
            joint_symbol: "PlyPurin5KYe_Share_joint",
        },
    ],
};
