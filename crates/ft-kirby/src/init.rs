//! Kirby load/reset hooks, ft/kinds/ftKirby/ftkirby.c.
use crate::attributes::{read_kirby_attributes, KirbyAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    ActionId, AerialJumpStyle, Capabilities, CharacterCallbacks, Fighter, MotionRow,
};
use melee_types::FighterKind;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Kirby {
    /// ftKb_DatAttrs, shared: 0x424 bytes do not fit the inline payload.
    pub attributes: Arc<KirbyAttributes>,
    /// ftKb_Init_OnDeath resets model groups 0 and 1 to selection 0.
    pub model_groups: [i32; 2],
    /// Fighter +2230 (u.kb.hat.x4): the Stone form, 1..=5, drawn at every
    /// reset and again by each Stone.
    pub stone_form: i32,
    /// Fighter +2238 (u.kb.hat.kind): the copied fighter; Kirby himself
    /// while he has no ability.
    pub copied: FighterKind,
    /// Fighter +2290 (u.kb.x64): the aerial Hammer's lift is spent until
    /// the next landing.
    pub hammer_lift_used: bool,
    /// Fighter +22F0 (u.kb.xC4): the copied Ice Shot's aerial lift is spent.
    pub ice_shot_lift_used: bool,
    /// Fighter +22F8 (u.kb.xCC): the copied Toad's aerial lift is spent.
    pub toad_lift_used: bool,
    /// Fighter +228C (u.kb.x60): ticks before another bump star.
    pub bump_star_cooldown: i32,
    /// Fighter +2310 (u.kb.xE4): frames the Inhale's loop still has to last.
    pub inhale_hold: i16,
    /// x2222_b2, set by the Inhale's rows until a motion change that does
    /// not keep it: a cape hit does not turn Kirby round.
    pub cape_turn_blocked: bool,
}
impl Kirby {
    pub fn new(attributes: KirbyAttributes) -> Self {
        Self {
            attributes: Arc::new(attributes),
            model_groups: [0; 2],
            stone_form: 0,
            copied: FighterKind::Kirby,
            hammer_lift_used: false,
            ice_shot_lift_used: false,
            toad_lift_used: false,
            bump_star_cooldown: 0,
            inhale_hold: 0,
            cape_turn_blocked: false,
        }
    }
}

static SPECIAL_ROWS: [MotionRow; crate::SPECIAL_ROW_COUNT] = crate::special_rows();
pub static TABLE: melee_ft::fighter::CharacterTable =
    melee_ft::fighter::CharacterTable::new::<Kirby>();

/// Stone has five forms (ftKb_Init_OnDeath: HSD_Randi(5) + 1).
const STONE_FORMS: i32 = 5;

/// ftKb_Init_OnKnockbackEnter (800EEB38): Fighter_OnKnockbackEnter(gobj, 1),
/// texture slots 1 then 0.
fn knockback_enter(fighter: &mut Fighter, _assets: &FighterAssets) {
    fighter.set_knockback_texture_frames(3.0);
}

/// ftKb_Init_OnKnockbackExit (800EEB7C): the same slots back to frame 0.
fn knockback_exit(fighter: &mut Fighter, _assets: &FighterAssets) {
    fighter.set_knockback_texture_frames(0.0);
}

/// ftCo_AppealS_CheckInput: ftKb_SpecialN_800F5D04(gobj, true) drops a
/// copied ability first; without one it is the common AppealS entry.
fn enter_taunt(
    fighter: &mut Fighter,
    assets: &FighterAssets,
) -> melee_ft::fighter::assets::Result<()> {
    if fighter.character.get::<Kirby>().copied != FighterKind::Kirby {
        unimplemented!(
            "ftKb_SpecialN_800F5D04 (ftkirbyspecialn.c:266): a taunt drops the copied ability"
        );
    }
    fighter.enter_common_taunt(assets)
}

/// ftKb_Init_OnDeath (800EE5A0): `hat.x4 = HSD_Randi(5) + 1`.
fn draw_stone_form(state: &mut melee_ft::fighter::CharacterState, rng: &mut gekko_math::HsdRng) {
    state.get_mut::<Kirby>().stone_form = rng.randi(STONE_FORMS) + 1;
}

fn cape_turn_blocked(f: &mut Fighter) -> bool {
    f.character.get::<Kirby>().cape_turn_blocked
}

/// The down throw is the common grounded one. The other three leave the
/// ground from their scripts and are not ported.
fn throw_entry(_fighter: &mut Fighter, throw: melee_types::CommonMotionState) {
    use melee_types::CommonMotionState as S;
    match throw {
        S::ThrowF | S::ThrowB => unimplemented!(
            "ftCo_ThrowF_Phys / ftCo_ThrowF_Coll (ftCo_Throw.c:311-340): Kirby's airborne forward \
             and back throws (ft_80085134, ft_80083C00, ft_80083CE4), whose victim keeps its grab \
             timer (ftCo_800DE3FC, ftCo_Thrown.c:36)"
        ),
        S::ThrowHi => unimplemented!(
            "ftCo_800DD398 (ftCo_Throw.c:146-150): Kirby's up throw, x2219_b2 on both fighters \
             until the script's flag (ftCo_800DD724, ftCo_Throw.c:287), and ftCo_ThrowHi_Cam's \
             ftCamera_800762F4"
        ),
        _ => {}
    }
}

impl CharacterCallbacks for Kirby {
    fn table() -> &'static melee_ft::fighter::CharacterTable {
        &TABLE
    }
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        enter_taunt;
    const KNOCKBACK_ENTER: fn(&mut Fighter, &FighterAssets) = knockback_enter;
    const KNOCKBACK_EXIT: fn(&mut Fighter, &FighterAssets) = knockback_exit;
    const SPECIAL_ROWS: &'static [MotionRow] = &SPECIAL_ROWS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    /// ftCo_AttackDash.c decideFighter: ftKb_SpecialN_800F1F68.
    const DASH_ATTACK_ROW: Option<ActionId> = Some(crate::attack_dash::GROUND);
    const RESET_DRAW: Option<fn(&mut melee_ft::fighter::CharacterState, &mut gekko_math::HsdRng)> =
        Some(draw_stone_form);
    const RESET_DRAW_COUNT: usize = 1;
    const THROW_ENTRY: Option<fn(&mut Fighter, melee_types::CommonMotionState)> = Some(throw_entry);
    /// ftCo_800C3538's x2222_b2.
    const CAPE_TURN_BLOCKED: fn(&mut Fighter) -> bool = cape_turn_blocked;
    /// ftData_SpecialN/S/Hi/Lw[Kirby] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            // ftKb_SpecialN_Enter (800F1638): ftKb_Init_803C9DD0[hat.kind],
            // or the Inhale without a hat.
            SpecialSlot::Neutral => {
                if f.character.get::<Kirby>().copied != FighterKind::Kirby {
                    unimplemented!("ftKb_Init_803C9DD0 (ftkirby.c:3777): a copied neutral special");
                }
                crate::special_n::enter(f, airborne, assets)
            }
            SpecialSlot::Side => unimplemented!(
                "ftKb_SpecialS_Enter / ftKb_SpecialAirS_Enter (ftkirbyspecials.c): Hammer"
            ),
            SpecialSlot::Up => unimplemented!(
                "ftKb_SpecialHi_Enter / ftKb_SpecialAirHi_Enter (ftkirbyspecialhi.c): Final Cutter"
            ),
            SpecialSlot::Down => unimplemented!(
                "ftKb_SpecialLw_Enter / ftKb_SpecialAirLw_Enter (ftkirbyspeciallw.c): Stone"
            ),
        }
    }
    /// Fighter_ChangeMotionState clears x2222_b2; the rows that keep it set
    /// it again.
    fn on_motion_change(&mut self) {
        self.cape_turn_blocked = false;
    }
    /// ftCo_800C7CA0: ftKb_SpecialN_800F1F1C.
    const WALL_BOUNCE_EFFECT: Option<fn(&mut Fighter, hsd_types::Vec3)> =
        Some(crate::bump_star::spawn);
    /// Fighter_procMap (fighter.c:2494): ftKb_SpecialN_800F1D24.
    const AFTER_COLLISION: Option<fn(&mut Fighter)> = Some(crate::bump_star::after_collision);

    /// ftCo_Throw.c:145-157: the FTKIND_KIRBY arm is `THROW_ENTRY`.
    fn throw_variant(&self) {}
    fn kind(&self) -> FighterKind {
        FighterKind::Kirby
    }
    fn descriptor() -> &'static CharacterDescriptor {
        &DESCRIPTOR
    }
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, melee_ft::desc::FighterDescError> {
        Ok(Self::new(read_kirby_attributes(data)?))
    }
    /// Fighter +2230, the Stone form a savestate holds.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.stone_form = i32::from_be_bytes(raw[0x2230..0x2234].try_into().unwrap());
    }
    /// ftKb_Init_OnLoad (800EE680): PUSH_ATTRS, can_multijump, x2D0 =
    /// dat_attrs. The four item registrations (it_8026B3F8: the Cutter
    /// beam, the hammer and two more) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.specials = [true; 4];
    }
    /// ftKb_Init_OnDeath (800EE5A0): ftParts_80074A4C(gobj, 0, 0) and
    /// (gobj, 1, 0); the hat and its ability go. The Stone form's draw is
    /// `RESET_DRAW`.
    fn on_reset(&mut self) {
        self.model_groups = [0; 2];
        self.copied = FighterKind::Kirby;
        // u.kb.x60 and x64.
        self.bump_star_cooldown = 0;
        self.hammer_lift_used = false;
    }
    /// ftCo_Landing_Enter's FTKIND_KIRBY arm (ftCo_Landing.c:72-76): xCC,
    /// xC4 and x64.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        self.toad_lift_used = false;
        self.ice_shot_lift_used = false;
        self.hammer_lift_used = false;
    }
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::MultiJump
    }
    fn multi_jump_attributes(&self) -> Option<&melee_ft::fighter::multi_jump::MultiJumpAttributes> {
        Some(&self.attributes.multi_jump)
    }
    /// ftCo_800D7268 -> ftKb_SpecialN_800F1CD8: the helmeted rows while a
    /// hat is worn.
    fn multi_jump_family(&self) -> usize {
        if self.copied != FighterKind::Kirby {
            unimplemented!("ftKb_SpecialN_800F1CD8 (ftkirby.c:4086): the helmeted multijump rows");
        }
        0
    }
    /// ftKb_Init_MotionStateTable: JumpAerialF1..F5 play submotions
    /// 295..299, the helmeted rows 300..304.
    fn multi_jump_animation(&self, jump: usize) -> i32 {
        const FIRST_MULTI_JUMP_ANIMATION: i32 = 295; // ftKb_SM_JumpAerialF1
        FIRST_MULTI_JUMP_ANIMATION + jump as i32
    }
}

/// ftKb_Init_* strings (ftkirbydata.c); ftData_Table_Unk0[4] has 479
/// animation rows. PlCo's ftPartsTable maps Kirby's joints to 54 parts;
/// ftData.x1C holds three part-animation groups (roots 39, 44 and 12).
pub const DESCRIPTOR: CharacterDescriptor = CharacterDescriptor {
    kind: FighterKind::Kirby,
    common_behavior: melee_ft::fighter::assets::CommonBehavior::for_kind(FighterKind::Kirby),
    data_file: "PlKb.dat",
    data_symbol: "ftDataKirby",
    animation_file: "PlKbAJ.dat",
    animation_count: 479,
    part_count: 54,
    part_animation_count: 3,
    additional_part_animations: &[],
    costumes: &[
        CostumeDescriptor {
            file: "PlKbNr.dat",
            joint_symbol: "PlyKirby5K_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKbYe.dat",
            joint_symbol: "PlyKirby5KYe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKbBu.dat",
            joint_symbol: "PlyKirby5KBu_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKbRe.dat",
            joint_symbol: "PlyKirby5KRe_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKbGr.dat",
            joint_symbol: "PlyKirby5KGr_Share_joint",
        },
        CostumeDescriptor {
            file: "PlKbWh.dat",
            joint_symbol: "PlyKirby5KWh_Share_joint",
        },
    ],
};
