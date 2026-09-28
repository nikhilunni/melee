//! Peach load/reset hooks, ft/kinds/ftPeach/ftpeach.c.
use crate::attributes::PeachAttributes;
use melee_ft::fighter::assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks, PlayerSlot};
use melee_types::{FighterKind, ItemKind};

#[derive(Clone, Debug, Default)]
pub struct PeachItems {
    /// Fighter +2238 / +223C: two parasol references.
    pub parasol: [bool; 2],
    /// Fighter +2244.
    pub toad: bool,
    /// Fighter +2248.
    pub vegetable: bool,
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
            registered_items: Vec::new(),
            model_groups: [0; 7],
            costume: 0,
        }
    }
}
static SPECIAL_ROWS: [melee_ft::fighter::MotionRow; 8] = crate::special_rows();
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
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    fn kind(&self) -> FighterKind {
        FighterKind::Peach
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(crate::attributes::read_peach_attributes(data)?))
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
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:58-62.
    fn on_landing(&mut self, allow_interrupt: bool) {
        self.aerial_toad_used = false;
        if allow_interrupt && self.items.parasol[0] {
            unimplemented!("ftPe_8011D598: remove active parasol item on landing");
        }
    }
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
