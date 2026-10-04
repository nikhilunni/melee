//! Mr. Game & Watch load/reset hooks, ft/kinds/ftGameWatch/ftgamewatch.c.
use crate::attributes::{read_gamewatch_attributes, GameWatchAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor},
    Capabilities, CharacterCallbacks, MotionRow,
};
use melee_types::FighterKind;

/// ftGameWatch_PanicLevel: Oil Panic's bucket.
pub const PANIC_EMPTY: i32 = 0;
pub const PANIC_FULL: i32 = 3;

#[derive(Clone, Debug)]
pub struct GameWatch {
    pub attributes: GameWatchAttributes,
    /// Fighter +222C x222C_judgeVar1: the last Judgment face drawn.
    pub judge_last: i32,
    /// Fighter +2230 x2230_judgeVar2: the face before it.
    pub judge_previous: i32,
    /// Fighter +2234 x2234.
    // TODO(meaning): reset by OnDeath; the spill's absorbed-damage word.
    pub x2234: u32,
    /// Fighter +2238 x2238_panicCharge: the bucket's level (0..3).
    pub panic_charge: i32,
    /// Fighter +223C x223C_panicDamage: damage absorbed so far.
    pub panic_damage: i32,
    /// Fighter +2240 x2240_chefVar1: the last sausage kind thrown.
    pub chef_last: i32,
    /// Fighter +2244 x2244_chefVar2: the kind before it.
    pub chef_previous: i32,
}

impl GameWatch {
    pub fn new(attributes: GameWatchAttributes) -> Self {
        Self {
            attributes,
            judge_last: 1,
            judge_previous: 0,
            x2234: 0,
            panic_charge: PANIC_EMPTY,
            panic_damage: 0,
            chef_last: 1,
            chef_previous: 3,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<GameWatch>();

impl CharacterCallbacks for GameWatch {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(
        &mut melee_ft::fighter::Fighter,
        &melee_ft::fighter::assets::FighterAssets,
    ) -> melee_ft::fighter::assets::Result<()> = melee_ft::fighter::Fighter::enter_common_taunt;
    /// ftCo_Landing_Enter's FTKIND_GAMEWATCH arm (ftCo_Landing.c:65-67):
    /// the aerial Judgment's hop is available again.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.x2234 = 0;
    }

    fn kind(&self) -> FighterKind {
        FighterKind::GameWatch
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_gamewatch_attributes(data)?))
    }
    /// ftGameWatch_FighterVars +222C..+2270. The ten article GObjs
    /// (+2248..+226C) are not restored; a save with one set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.judge_last = word(0x222C) as i32;
        self.judge_previous = word(0x2230) as i32;
        self.x2234 = word(0x2234);
        self.panic_charge = word(0x2238) as i32;
        self.panic_damage = word(0x223C) as i32;
        self.chef_last = word(0x2240) as i32;
        self.chef_previous = word(0x2244) as i32;
        if (0x2248..0x2270).step_by(4).any(|at| word(at) != 0) {
            unimplemented!("ftGameWatch_FighterVars: a saved article GObj");
        }
    }
    /// ftGw_Init_OnLoad (8014A404): x2222_b6 clear, x2223_b1 and
    /// can_walljump set, PUSH_ATTRS, the empty bucket. The ten item
    /// registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
        self.panic_charge = PANIC_EMPTY;
    }
    /// ftGw_Init_OnDeath (8014A37C): the model groups and the FighterVars
    /// reset; the bucket's level (x2238) survives.
    fn on_reset(&mut self) {
        self.judge_last = 1;
        self.judge_previous = 0;
        self.x2234 = 0;
        self.panic_damage = 0;
        self.chef_last = 1;
        self.chef_previous = 3;
    }
}

const COSTUME: CostumeDescriptor = CostumeDescriptor {
    file: "PlGwNr.dat",
    joint_symbol: "PlyGamewatch5K_Share_joint",
};

/// ftGw_Init_* strings (ftgamewatch.c); ftData_Table_Unk0[24] has 323
/// animation rows. ftData.x1C holds three part-animation groups. The four
/// costumes share one model file: the colour is a material override
/// (ftMaterial_800BFB4C).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::GameWatch,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::GameWatch),
    data_file: "PlGw.dat",
    data_symbol: "ftDataGamewatch",
    animation_file: "PlGwAJ.dat",
    animation_count: 323,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[COSTUME, COSTUME, COSTUME, COSTUME],
};
