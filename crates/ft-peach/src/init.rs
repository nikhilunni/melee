//! Peach load/reset hooks, ft/kinds/ftPeach/ftpeach.c.
use crate::attributes::PeachAttributes;
use melee_ft::fighter::assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks, PlayerSlot};
use melee_types::{FighterKind, ItemKind};

#[derive(Clone, Debug, Default)]
pub struct PeachItems {
    /// take_dmg_cb = ftPe_Init_OnDeath2, cleared by every motion change.
    pub take_damage_armed: bool,
    /// death2_cb (Toad), cleared by every motion change, and death3_cb
    /// (the parasol), which survives them.
    pub death2_armed: bool,
    pub death3_armed: bool,
    /// Fighter +2238 / +223C: two parasol references.
    pub parasol: [bool; 2],
    /// The parasol item's motion (1 opening, 2 open), which Peach drives
    /// and ftGetParasolStatus reads through fp->item_gobj.
    pub parasol_motion: u16,
    /// Fighter +2244.
    pub toad: bool,
    /// Fighter +2248.
    pub vegetable: bool,
}

/// The one-shot accessory4 callback a special installed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// onAccessory4 (8011E174): Toad hangs from joint 109.
    DrawToad,
    /// onHitAccessory4 (8011E230): one of Toad's spores.
    ReleaseSpore,
    /// ftPe_SpecialHi_8011D424: the parasol hangs from joint 109.
    DrawParasol,
    /// spawnVeg (8011D018): the pull, each frame until the motion changes.
    PullVegetable,
}
#[derive(Clone, Debug)]
pub struct Peach {
    pub attributes: PeachAttributes,
    /// Fighter +222C: once-per-airtime float availability.
    pub has_float: bool,
    /// Fighter +2230: remaining float frames.
    pub float_remaining: f32,
    /// Fighter +2234: previous forward-smash motion, -1 when reset.
    pub smash_motion: i32,
    /// Fighter +2240: Toad's once-per-airtime vertical boost.
    pub aerial_toad_used: bool,
    pub items: PeachItems,
    /// fp->mv.pe.floatattack, the float aerials' motion scratch.
    pub float_attack: crate::float_attack::FloatAttack,
    /// Fighter accessory4_cb while a special owns it.
    pub accessory: Accessory,
    /// Toad's counter volume and caught hit.
    pub special_n: crate::special_n::SpecialN,
    /// fp->mv.pe.specialhi.
    pub special_hi: crate::special_hi::SpecialHi,
    /// fp->mv.pe.specials.
    pub bomber: crate::special_s::Bomber,
    /// mv+4 as the state before a special left it: Toad and the parasol
    /// start write only mv+0.
    pub retained_word: Option<f32>,
    pub registered_items: Vec<ItemKind>,
    pub model_groups: [i32; 7],
    costume: u8,
}
impl Peach {
    pub fn new(attributes: PeachAttributes) -> Self {
        Self {
            attributes,
            has_float: true,
            float_remaining: 0.0,
            smash_motion: -1,
            aerial_toad_used: false,
            items: PeachItems::default(),
            float_attack: Default::default(),
            accessory: Accessory::None,
            special_n: Default::default(),
            special_hi: Default::default(),
            bomber: Default::default(),
            retained_word: None,
            registered_items: Vec::new(),
            model_groups: [0; 7],
            costume: 0,
        }
    }
}
static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; crate::SPECIAL_ROW_COUNT] =
    crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Peach>();

impl CharacterCallbacks for Peach {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    const SPECIAL_ROWS: &'static [melee_ft::fighter::MotionRow] = &SPECIAL_ROWS;
    /// ftpeach*.c read fp->item_gobj only in the down special (a held
    /// turnip is thrown), the up special (its own unimplemented branch) and
    /// the float aerials (ftPe_8011BE80, which fails closed); every other
    /// row leaves a held item in hand.
    const SPECIALS_KEEP_HELD_ITEM: bool = true;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    /// ftCo_AttackS4.c decideFighter: ftPe_AttackS4_Enter.
    const FORWARD_SMASH: Option<melee_ft::fighter::RngEntry> = Some(crate::attack_s4::enter);
    /// ftData_SpecialN/S/Hi/Lw[Peach].
    fn enter_special(
        f: &mut melee_ft::fighter::Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// Fighter_8006C80C: the special's one-shot accessory4.
    fn accessory(
        f: &mut melee_ft::fighter::Fighter,
        _assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        let pending = f.character.get::<Peach>().accessory;
        if pending == Accessory::PullVegetable {
            // spawnVeg stays installed: it runs every frame of the pull.
            let action = f.motion_state.action;
            if f.accessory4_armed
                && (action == crate::special_lw::PULL || action == crate::special_lw::AIR_PULL)
            {
                crate::special_lw::pull(f, rng);
            }
            return;
        }
        if !f.run_accessory4(pending != Accessory::None) {
            return;
        }
        f.character.get_mut::<Peach>().accessory = Accessory::None;
        match pending {
            Accessory::DrawToad => crate::special_n::draw_toad(f),
            Accessory::ReleaseSpore => crate::special_n::release_spore(f),
            Accessory::DrawParasol => crate::special_hi::draw_parasol(f),
            Accessory::None | Accessory::PullVegetable => unreachable!(),
        }
    }
    /// itPeachParasol_Logic60_Destroyed / itPeachToad_Logic91_Destroyed:
    /// the article lets go of its owner.
    const RETAINED_SCRATCH_WORD: fn(
        &melee_ft::fighter::CharacterState,
        melee_ft::fighter::ActionId,
    ) -> Option<f32> = |state, action| {
        (361..=368).contains(&action.0).then(|| {
            state.get::<Self>().retained_word.unwrap_or_else(|| {
                unimplemented!("ftPe specials: mv+4 inherited from an unmodelled scratch word")
            })
        })
    };
    const ARTICLE_DESTROYED: fn(&mut melee_ft::fighter::Fighter, ItemKind) =
        crate::articles::destroyed;
    /// hurtbox_detect_cb = doAirEnd0 in the Peach Bomber jump.
    const HURTBOX_DETECT: Option<
        fn(
            &mut melee_ft::fighter::Fighter,
            &FighterAssets,
            melee_ft::fighter::damage::InertTouch,
        ),
    > = Some(crate::special_s::inert_contact);
    const DEFENSE_CONTACT: Option<melee_ft::fighter::DefenseContact> =
        Some(crate::special_n::contact);
    const PROCESS_DEFENSE_HIT: Option<melee_ft::fighter::DefenseHit> =
        Some(crate::special_n::process_hit);
    const ITEM_DEFENSE_CONTACT: Option<melee_ft::fighter::ItemDefenseContact> =
        Some(crate::special_n::item_contact);
    fn kind(&self) -> FighterKind {
        FighterKind::Peach
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        let mut attributes = crate::attributes::read_peach_attributes(data)?;
        attributes.turnip_faces = crate::attributes::read_turnip_faces(data)?;
        Ok(Self::new(attributes))
    }
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.has_float = word(0x222C) != 0;
        self.float_remaining = f32::from_bits(word(0x2230));
        self.smash_motion = word(0x2234) as i32;
        self.items.parasol = [word(0x2238) != 0, word(0x223C) != 0];
        self.aerial_toad_used = word(0x2240) != 0;
        self.items.toad = word(0x2244) != 0;
        self.items.vegetable = word(0x2248) != 0;
        self.costume = raw[0x619];
    }
    /// ftPe_Init_OnLoad (8011B628): PUSH_ATTRS and five item definitions.
    /// ftdata.c has all four ground/air specials; OnLoad sets no walljump flag.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        self.registered_items = vec![
            ItemKind::PeachExplode,
            ItemKind::PeachTurnip,
            ItemKind::PeachParasol,
            ItemKind::PeachToad,
            ItemKind::PeachToadSpore,
        ];
    }
    /// ftPe_Init_OnLoad: lbAnim_8001E8F8 reads FigaTree.frames for motions 18/19.
    fn on_resources_loaded(&mut self, assets: &FighterAssets, player: &PlayerSlot) {
        self.attributes.float.forward_fall_start = assets.motions[&18].animation.frames;
        self.attributes.float.backward_fall_start = assets.motions[&19].animation.frames;
        self.costume = player.costume;
    }
    /// ftPe_Init_OnDeath (8011B51C): resources, smash choice and costume models.
    fn on_reset(&mut self) {
        self.has_float = true;
        self.smash_motion = -1;
        self.aerial_toad_used = false;
        self.items = PeachItems::default();
        self.model_groups = if self.costume == 1 {
            [0, -1, 0, -1, 0, 0, -1]
        } else {
            [0, 0, 0, -1, 0, -1, 0]
        };
    }
    /// Fighter_ChangeMotionState, fighter.c:1120-1123.
    fn on_grounded_motion(&mut self) {
        self.has_float = true;
    }
    /// Fighter_ChangeMotionState, fighter.c:1385/1388.
    fn on_motion_change(&mut self) {
        self.items.take_damage_armed = false;
        self.items.death2_armed = false;
    }
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:58-62.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.aerial_toad_used = false;
    }
    /// ftCo_Landing.c:56-57: ftPe_8011D598 when the landing is interruptible.
    const LANDING_ARTICLES: fn(&mut melee_ft::fighter::Fighter, bool) = crate::articles::landing;
    fn special_parasol(&self) -> Option<melee_ft::fighter::parasol::SpecialParasol> {
        crate::articles::special_parasol(self)
    }
    const SET_PARASOL_ANIMATION: fn(&mut melee_ft::fighter::Fighter, usize, f32) =
        crate::articles::set_parasol_animation;
    /// take_dmg_cb = ftPe_Init_OnDeath2 while armed.
    const TAKE_DAMAGE: Option<fn(&mut melee_ft::fighter::Fighter)> =
        Some(crate::articles::take_damage);
    /// death2_cb / death3_cb = ftPe_Init_OnDeath2 while armed.
    const DEATH: Option<fn(&mut melee_ft::fighter::Fighter)> = Some(crate::articles::death);
    /// ftCo_8009DD94, ftdynamics.c:397-405: only the last chain starts at zero.
    fn dynamics_first_force_bone(&self, set: usize, count: usize) -> usize {
        if set + 1 < count {
            3
        } else {
            0
        }
    }
    fn check_float_input(
        &self,
        input: &melee_ft::input::FighterInput,
        assets: &FighterAssets,
        vertical_velocity: f32,
        phase: melee_ft::fighter::FloatInputPhase,
    ) -> bool {
        crate::float::float_input_selected(self, input, assets, vertical_velocity, phase)
    }
    const ENTER_FLOAT: fn(
        &mut melee_ft::fighter::Fighter,
        &FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = crate::float::enter_from_input;
    /// ftPe_JumpAerial_Enter (800CC0E8): animation-driven vertical velocity.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Peach
    }
}

/// ftPe_Init_* strings; ftData_Table_Unk0[9]: 318 motions. PlCo's part table
/// maps 114 joints to 54 semantic parts. ftData.x1C has three animation groups;
/// ftData.x2C supplies nine five-joint dynamic chains, with no colliders.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Peach,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Peach),
    data_file: "PlPe.dat",
    data_symbol: "ftDataPeach",
    animation_file: "PlPeAJ.dat",
    animation_count: 318,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlPeNr.dat",
            joint_symbol: "PlyPeach5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPeYe.dat",
            joint_symbol: "PlyPeach5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPeWh.dat",
            joint_symbol: "PlyPeach5KWh_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPeBu.dat",
            joint_symbol: "PlyPeach5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlPeGr.dat",
            joint_symbol: "PlyPeach5KGr_Share_joint",
        },
    ],
};
