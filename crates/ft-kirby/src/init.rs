//! Kirby load/reset hooks, ft/kinds/ftKirby/ftkirby.c.
use crate::attributes::{read_kirby_attributes, KirbyAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    throw_flight::ThrowTraits,
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
    /// Fighter +230C (u.kb.xE0) as the swallow sets it: the ability the
    /// fighter in Kirby's mouth gives (ftCo_800BD9E0), read at the catch.
    pub mouthful_ability: FighterKind,
    /// The mouthful's jump (mv.co.kneebend and mv.co.jump in rows 364, 365).
    pub mouthful_jump: crate::mouthful::MouthfulJump,
    /// fp->mv.kb.specialhi: Final Cutter's scratch.
    pub final_cutter: crate::special_hi::FinalCutter,
    /// fp->mv.kb.speciallw: Stone's scratch.
    pub stone: crate::special_lw::Stone,
    /// take_dmg_cb = ftKb_Init_800EE7B8 and death2_cb = ftKb_Init_800EE74C
    /// (ftKb_SpecialN_set_cbs), until the next motion change.
    pub damage_callbacks: bool,
    /// death1_cb and death3_cb, which the copied ability installs.
    pub ability_callbacks: crate::damage_callbacks::AbilityCallbacks,
    /// What accessory4_cb does while it is installed.
    pub accessory: Accessory,
    /// Fighter +222C (u.kb.hat.x0): the hammer article is out.
    pub hammer_out: bool,
    /// The copied Blaster: u.kb.xB0 (the gun is out) and mv.fx.SpecialN.
    pub blaster: ft_fox_family::SpecialNeutral,
    /// The copied Shield Breaker / Flare Blade: mv.kb.specialn_ms.
    pub shield_breaker: ft_mars_family::special_n::SpecialN,
    /// The copied Rollout: mv.pr.specialn, the model scale it squashes
    /// (u.kb.x8C) and deal_dmg_cb (fn_80100E0C).
    pub rollout: ft_purin_family::RolloutState,
    /// The copied Chef: u.kb.xD4..xDC and mv.kb.specialn_gw.
    pub chef: crate::copied::chef::KirbyChef,
    /// The copied PK Flash: mv.kb.specialn_ns and u.kb.ns_flash_gobj
    /// (Fighter +22A8).
    pub pk_flash: ft_ness_family::pk_flash::PkFlash,
    /// Fighter +2294 (u.kb.x68): the copied Megavitamin's last colour.
    pub vitamin_current: i32,
    /// Fighter +2298 (u.kb.x6C): the colour before it.
    pub vitamin_previous: i32,
    /// The copied Charge Shot: u.kb.xA4 (the shot in hand), xA8 (the kept
    /// level), xAC and mv.kb.specialhi.
    pub charge_shot: ft_samus_family::charge_shot::ChargeState,
    /// The copied Giant Punch: u.kb.xBC (the stored swings) and
    /// mv.dk.specialn.
    pub giant_punch_swings: i32,
    pub giant_punch: ft_donkey_family::giant_punch::GiantPunch,
    /// The copied Shadow Ball: u.kb.x98 (the ball in hand), x9C (the
    /// stored charge) and mv.kb.specialhi.
    pub shadow_ball_held: bool,
    pub shadow_ball_charge: i32,
    pub shadow_ball: ft_mewtwo_family::shadow_ball::ShadowBall,
    /// The copied Toad: u.kb.xD0 and its counter volume.
    pub toad: crate::copied::toad::KirbyToad,
}

/// Kirby's accessory4 callbacks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// fn_800F21E8: Final Cutter's beam on the script's flag.
    CutterBeam,
    /// fn_800F53AC: the hammer article on the script's flags.
    Hammer,
    /// fn_8010CFB0: the copied Chef's pan.
    ChefPan,
    /// fn_8010CE5C: the copied Chef's food on the script's flag.
    ChefFood,
    /// fn_800F9260: the copied Fireball or Megavitamin on the script's flag.
    Fireball,
    /// fn_800F98F4: the copied green Fireball on the script's flag.
    GreenFireball,
    /// fn_8010C288: the copied Toad drawn out.
    ToadDraw,
    /// fn_8010C344: one of the copied Toad's spores.
    ToadSpore,
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
            mouthful_ability: FighterKind::Kirby,
            mouthful_jump: Default::default(),
            final_cutter: Default::default(),
            stone: Default::default(),
            damage_callbacks: false,
            ability_callbacks: Default::default(),
            accessory: Accessory::None,
            hammer_out: false,
            blaster: Default::default(),
            shield_breaker: Default::default(),
            rollout: crate::copied::rollout::at_rest(),
            chef: Default::default(),
            // ftKb_SpecialN_800F190C's 9, which every new ability sets
            // before the first draw reads them.
            vitamin_current: 9,
            vitamin_previous: 9,
            charge_shot: Default::default(),
            giant_punch_swings: 0,
            giant_punch: Default::default(),
            shadow_ball_held: false,
            shadow_ball_charge: 0,
            shadow_ball: Default::default(),
            toad: Default::default(),
            pk_flash: Default::default(),
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

/// ftCo_800DEBD0 (ftCo_AppealS.c:100): ftKb_SpecialN_800F5D04(gobj, true)
/// drops a copied ability as a star, then the common AppealS entry.
fn enter_taunt(
    fighter: &mut Fighter,
    assets: &FighterAssets,
) -> melee_ft::fighter::assets::Result<()> {
    crate::ability::lose(fighter, true);
    fighter.enter_common_taunt(assets)
}

/// ftKb_Init_OnDeath (800EE5A0): `hat.x4 = HSD_Randi(5) + 1`.
fn draw_stone_form(state: &mut melee_ft::fighter::CharacterState, rng: &mut gekko_math::HsdRng) {
    state.get_mut::<Kirby>().stone_form = rng.randi(STONE_FORMS) + 1;
}

/// ftCo_800BD9E0's FTKIND_KIRBY arm: a swallowed Kirby gives the ability
/// he has copied. With one, ftCo_800BDA50 marks him to lose it as he leaves
/// the copy star (ftCo_ThrownKirby.c:283-289), which is not ported.
fn swallowed_ability(state: &melee_ft::fighter::CharacterState) -> FighterKind {
    let copied = state.get::<Kirby>().copied;
    if copied != FighterKind::Kirby {
        unimplemented!(
            "ftCo_800BDA50 (ftCo_ThrownKirby.c:63-69): a Kirby with a copied ability inhaled"
        );
    }
    copied
}

fn cape_turn_blocked(f: &mut Fighter) -> bool {
    f.character.get::<Kirby>().cape_turn_blocked
}

/// The FTKIND_KIRBY arms of the common throw code. The forward and back
/// throws leave the thrown fighter its grab timer (ftCo_800DE3FC,
/// ftCo_Thrown.c:36: `msid - ftCo_MS_ThrownF > 1` fails for those two). The
/// up throw carries both fighters above the screen: no blast-zone test
/// until its script's flag (ftCo_800DD398 and ftCo_800DD724,
/// ftCo_Throw.c:146-150, 287-292) and no camera box (ftCo_ThrowHi_Cam,
/// ftCo_ThrownHi_Cam). The down throw is an ordinary one.
fn throw_traits(throw: melee_types::CommonMotionState) -> ThrowTraits {
    use melee_types::CommonMotionState as S;
    ThrowTraits {
        victim_keeps_grab_timer: matches!(throw, S::ThrowF | S::ThrowB),
        leaves_the_screen: throw == S::ThrowHi,
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
    const THROW_TRAITS: fn(melee_types::CommonMotionState) -> ThrowTraits = throw_traits;
    /// ftCo_800C3538's x2222_b2.
    const CAPE_TURN_BLOCKED: fn(&mut Fighter) -> bool = cape_turn_blocked;
    /// fn_800F6210 / fn_800F6318 and ftCo_800BD1DC: the Inhale's catch.
    const SPECIAL_GRAB: melee_ft::fighter::SpecialGrab = crate::mouthful::grab;
    /// ftKb_SpecialN_800F5A38 and ftKb_SpecialS_800F6BB0, for the fighter
    /// in his mouth.
    const MOUTH_HOLD: Option<melee_ft::fighter::MouthHold> = Some(melee_ft::fighter::MouthHold {
        steerable: crate::mouthful::steerable,
        leave_floor: crate::mouthful::leave_floor,
    });
    const SWALLOWED_ABILITY: Option<fn(&melee_ft::fighter::CharacterState) -> FighterKind> =
        Some(swallowed_ability);
    /// ftKb_SpecialN_800F5BA4 / ftKb_SpecialN_800F5C34.
    const DAMAGE_DRAW: Option<melee_ft::fighter::DamageDrawFn> =
        Some(crate::swallow::ability_loss_draw);
    /// EatWait (359) and EatLanding (366) carry MotionState.x9_b1.
    const KO_COUNTDOWN_ROWS: &'static [u16] = &[359, 366];
    /// ftData_SpecialN/S/Hi/Lw[Kirby] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            // ftKb_SpecialN_Enter (800F1638) / ftKb_SpecialAirN_Enter:
            // ftKb_Init_803C9DD0[hat.kind], or the Inhale without a hat.
            SpecialSlot::Neutral => crate::copied::enter_neutral(f, airborne, assets),
            // ftKb_SpecialS_Enter (800F5550) / ftKb_SpecialAirS_Enter
            // (800F55C0).
            SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
            // ftKb_SpecialHi_Enter (800F2300) / ftKb_SpecialAirHi_Enter
            // (800F23B8).
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            // ftKb_SpecialLw_Enter (800F3D50) / ftKb_SpecialAirLw_Enter
            // (800F3E68).
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// Fighter_ChangeMotionState clears x2222_b2 and the per-motion
    /// callbacks (fighter.c:1376-1389); the rows that keep them set them
    /// again.
    fn on_motion_change(&mut self) {
        self.cape_turn_blocked = false;
        self.damage_callbacks = false;
        self.accessory = Accessory::None;
        self.rollout.bounce = false;
    }
    /// Fighter_8006C80C: the special's accessory4, installed until the next
    /// motion change.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        crate::copied::accessory(f, assets);
        if !f.core.accessory4_armed {
            return;
        }
        match f.character.get::<Kirby>().accessory {
            Accessory::CutterBeam => crate::special_hi::beam_accessory(f),
            Accessory::Hammer => crate::special_s::hammer_accessory(f),
            Accessory::ChefPan => crate::copied::chef::pan_accessory(f, assets),
            Accessory::ChefFood => crate::copied::chef::throw_accessory(f, assets, rng),
            Accessory::Fireball => crate::copied::fireball::mario_accessory(f, assets, rng),
            Accessory::GreenFireball => crate::copied::fireball::luigi_accessory(f, assets),
            toad @ (Accessory::ToadDraw | Accessory::ToadSpore) => {
                crate::copied::toad::accessory(f, toad)
            }
            Accessory::None => {}
        }
    }
    fn item_owner(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
        crate::copied::item_owner(f)
    }
    fn item_muzzle(f: &mut Fighter, assets: &FighterAssets) -> Option<(hsd_types::Vec3, f32)> {
        crate::copied::item_muzzle(f, assets)
    }
    /// deal_dmg_cb of a copied special's rows.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &FighterAssets)> = Some(crate::copied::deal_damage);
    /// x21F8 of a copied special's rows, after a cape turnaround.
    const CAPE_TURN_END: fn(&mut Fighter) = crate::copied::cape_turn_end;
    /// ftCommon_8007DB58: take_dmg_cb, then death1_cb.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::damage_callbacks::take_damage);
    /// ftCo_800D331C: death2_cb, death3_cb, then death1_cb.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::damage_callbacks::death);
    /// ftCo_800C7CA0: ftKb_SpecialN_800F1F1C.
    const WALL_BOUNCE_EFFECT: Option<fn(&mut Fighter, hsd_types::Vec3)> =
        Some(crate::bump_star::spawn);
    /// Fighter_procMap (fighter.c:2494): ftKb_SpecialN_800F1D24.
    const AFTER_COLLISION: Option<fn(&mut Fighter)> = Some(crate::bump_star::after_collision);
    /// itKirbyHammer_Logic8_Destroyed (ftKb_SpecialAirLw_800F539C) and the
    /// copied specials' articles.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) =
        crate::copied::article_destroyed;
    /// A copied special's shield volume (the copied Toad's,
    /// ftColl_8007B1B8 from doPeAnim).
    const DEFENSE_CONTACT: Option<melee_ft::fighter::DefenseContact> =
        Some(crate::copied::defense_contact);
    const PROCESS_DEFENSE_HIT: Option<melee_ft::fighter::DefenseHit> =
        Some(crate::copied::process_defense_hit);
    const ITEM_DEFENSE_CONTACT: Option<melee_ft::fighter::ItemDefenseContact> =
        Some(crate::copied::item_defense_contact);

    /// ftCo_Throw.c:145-157: the FTKIND_KIRBY arm is `THROW_TRAITS`.
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
    /// Fighter +2230, the Stone form a savestate holds; +222C is the hammer
    /// article, which no start boundary has out.
    fn restore_saved(&mut self, raw: &[u8]) {
        self.stone_form = i32::from_be_bytes(raw[0x2230..0x2234].try_into().unwrap());
        assert_eq!(raw[0x222C..0x2230], [0; 4], "a saved hammer article");
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
        // Fighter_UnkInitReset (fighter.c:467-468): death1_cb, death3_cb.
        self.ability_callbacks = Default::default();
        self.model_groups = [0; 2];
        self.copied = FighterKind::Kirby;
        self.hammer_out = false;
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
    /// ftCo_800D7268 -> ftKb_SpecialN_800F1CD8 -> fn_800F1CA0 (800F1CA0):
    /// the helmeted rows 346..350 (+30) under the five hats that cover
    /// Kirby's face, the bare rows 341..345 (+2C) otherwise.
    fn multi_jump_family(&self) -> usize {
        usize::from(crate::hat::is_helmet(self.copied))
    }
    /// ftKb_Init_MotionStateTable: JumpAerialF1..F5 play submotions
    /// 295..299, the helmeted rows 300..304.
    fn multi_jump_animation(&self, jump: usize) -> i32 {
        const FIRST_MULTI_JUMP_ANIMATION: i32 = 295; // ftKb_SM_JumpAerialF1
        const JUMPS: usize = 5;
        FIRST_MULTI_JUMP_ANIMATION + (self.multi_jump_family() * JUMPS + jump) as i32
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

#[cfg(test)]
mod tests {
    use super::*;
    use melee_types::CommonMotionState as S;

    #[test]
    fn the_forward_and_back_throws_leave_the_grab_timer_and_the_up_throw_leaves_the_screen() {
        for (throw, keeps_timer, leaves_the_screen) in [
            (S::ThrowF, true, false),
            (S::ThrowB, true, false),
            (S::ThrowHi, false, true),
            (S::ThrowLw, false, false),
        ] {
            let expected = ThrowTraits {
                victim_keeps_grab_timer: keeps_timer,
                leaves_the_screen,
            };
            assert_eq!(throw_traits(throw), expected, "{throw:?}");
        }
    }
}
