//! Mewtwo load/reset hooks, ft/kinds/ftMewtwo/ftmewtwo.c.
use crate::attributes::{read_mewtwo_attributes, MewtwoAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    AerialJumpStyle, Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;

#[derive(Clone, Debug)]
pub struct Mewtwo {
    pub attributes: MewtwoAttributes,
    /// ftMt_Init_OnDeath resets model group 0 to selection 0.
    pub model_group: i32,
    /// Fighter +2234, u.mt.x2234_shadowBallCharge: Shadow Ball's stored
    /// charge, kept between states until the shot, a hit taken or a death.
    pub shadow_ball_charge: i32,
    /// Fighter +223C, u.mt.x223C_isConfusionBoost: the aerial Confusion
    /// already lifted Mewtwo this airtime.
    pub confusion_boost_used: bool,
}
impl Mewtwo {
    pub fn new(attributes: MewtwoAttributes) -> Self {
        Self {
            attributes,
            model_group: 0,
            shadow_ball_charge: 0,
            confusion_boost_used: false,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Mewtwo>();

/// Fighter_OnKnockbackEnter/Exit(gobj, 2): texture slots 2 then 0
/// (ftAnim_800704F0).
fn set_knockback_texture_frames(fighter: &mut Fighter, frame: f32) {
    for index in [2, 0] {
        fighter.commands.set_texture_frame(index, frame);
    }
}

impl CharacterCallbacks for Mewtwo {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
    /// ftMt_Init_OnKnockbackEnter (80145018).
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| set_knockback_texture_frames(fighter, 3.0);
    /// ftMt_Init_OnKnockbackExit (8014505C).
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) =
        |fighter, _assets| set_knockback_texture_frames(fighter, 0.0);
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;

    fn kind(&self) -> FighterKind {
        FighterKind::Mewtwo
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_mewtwo_attributes(data)?))
    }
    /// ftMewtwo_FighterVars +222C..+223F. The three article GObjs (+222C
    /// Disable, +2230 the held Shadow Ball, +2238 the shot one) are not
    /// restored; a save with one set fails closed.
    fn restore_saved(&mut self, raw: &[u8]) {
        let word = |offset: usize| u32::from_be_bytes(raw[offset..offset + 4].try_into().unwrap());
        self.shadow_ball_charge = word(0x2234) as i32;
        self.confusion_boost_used = word(0x223C) != 0;
        if word(0x222C) != 0 || word(0x2230) != 0 || word(0x2238) != 0 {
            unimplemented!("ftMewtwo_FighterVars: a saved Disable or Shadow Ball GObj");
        }
        if self.shadow_ball_charge as f32 == self.attributes.shadow_ball.full_charge {
            // The glow is the fighter's secondary colour slot, which the
            // character payload cannot restore.
            unimplemented!("ftMt_Init_UnkMotionStates4: a savestate with a full Shadow Ball");
        }
    }
    /// ftMt_Init_OnLoad (80144E48): PUSH_ATTRS, parts[FtPart_TransN].flags_b4
    /// and x2221_b2 (the model follows TransN's extracted motion). The two
    /// item registrations (it_8026B3F8: Disable, Shadow Ball) belong to the
    /// item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
        capabilities.compensates_root_motion = true;
    }
    /// ftMt_Init_OnDeath (80144DFC): ftParts_80074A4C(gobj, 0, 0) and the
    /// FighterVars reset.
    fn on_reset(&mut self) {
        self.model_group = 0;
        self.shadow_ball_charge = 0;
        self.confusion_boost_used = false;
    }
    /// ftCo_Landing_Enter's FTKIND_MEWTWO arm (ftCo_Landing.c:77-79).
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.confusion_boost_used = false;
    }
    /// ftMt_JumpAerial_Enter (800CC238): the animation carries the rise.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Mewtwo
    }
    /// ftCo_8009DD94's FTKIND_MEWTWO arm (ftdynamics.c:404-408): every
    /// chain's forces start at the ninth bone.
    fn dynamics_first_force_bone(&self, _set: usize, _count: usize) -> usize {
        8
    }
}

/// ftMt_Init_* strings (ftmewtwo.c); ftData_Table_Unk0[16] has 314 animation
/// rows. PlCo ftPartsTable maps Mewtwo's joints to 54 parts; ftData.x1C
/// holds three part-animation groups.
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Mewtwo,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Mewtwo),
    data_file: "PlMt.dat",
    data_symbol: "ftDataMewtwo",
    animation_file: "PlMtAJ.dat",
    animation_count: 314,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlMtNr.dat",
            joint_symbol: "PlyMewtwo5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtRe.dat",
            joint_symbol: "PlyMewtwo5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtBu.dat",
            joint_symbol: "PlyMewtwo5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlMtGr.dat",
            joint_symbol: "PlyMewtwo5KGr_Share_joint",
        },
    ],
};
