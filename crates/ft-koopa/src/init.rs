//! Bowser load/reset hooks, ft/kinds/ftKoopa/ftkoopa.c.
use crate::attributes::{read_koopa_attributes, KoopaAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Koopa {
    pub attributes: KoopaAttributes,
    /// ftKp_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +222C, u.kp.x222C: the Fire Breath's reach left.
    pub breath_reach: f32,
    /// Fighter +2230, u.kp.x2230: the Fire Breath's lifetime left.
    pub breath_life: f32,
}
impl Koopa {
    pub fn new(attributes: KoopaAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            breath_reach: 0.0,
            breath_life: 0.0,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Koopa>();

impl CharacterCallbacks for Koopa {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftKp_Init_OnLoad (80132ABC): fp->x2226_b1 = true.
    const DOWN_BOUND_INVERTED: bool = true;
    /// ftKp_Init_OnKnockbackEnter (80132D38) / Exit (80132D7C):
    /// Fighter_OnKnockbackEnter/Exit(gobj, 1).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(3.0);
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| fighter.set_knockback_texture_frames(0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;

    fn kind(&self) -> FighterKind {
        FighterKind::Koopa
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_koopa_attributes(data)?))
    }
    /// ftKoopa_FighterVars +222C..+2234.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.breath_reach = f32::from_bits(word(0x222C));
        self.breath_life = f32::from_bits(word(0x2230));
    }
    /// ftKp_Init_OnLoad (80132ABC): PUSH_ATTRS, the flame's item
    /// registration (it_8026B3F8, the item scene's) and x2226_b1.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.grounded_down_bound = true;
        // ftKp_Init_OnDeath (80132A0C): dmg.armor0 = attributes +0.
        capabilities.armor = self.attributes.armor;
    }
    /// ftKp_Init_OnDeath (80132A0C): ftParts_80074A4C(gobj, 0, 0), armor0
    /// (see `on_load`) and both breath fuels refilled.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.breath_reach = self.attributes.fire_breath.reach_max;
        self.breath_life = self.attributes.fire_breath.life_max;
    }
}

/// ftKp_Init_* strings (ftkoopa.c); ftData_Table_Unk0[5] has 316 animation
/// rows. ftData.x1C holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Koopa,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Koopa),
    data_file: "PlKp.dat",
    data_symbol: "ftDataKoopa",
    animation_file: "PlKpAJ.dat",
    animation_count: 316,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlKpNr.dat",
            joint_symbol: "PlyKoopa5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKpRe.dat",
            joint_symbol: "PlyKoopa5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKpBu.dat",
            joint_symbol: "PlyKoopa5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKpBk.dat",
            joint_symbol: "PlyKoopa5KBk_Share_joint",
        },
    ],
};
