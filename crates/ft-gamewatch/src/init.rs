//! Mr. Game & Watch load/reset hooks, ft/kinds/ftGameWatch/ftgamewatch.c.
use crate::attributes::{read_gamewatch_attributes, GameWatchAttributes};
use melee_ft::fighter::{
    assets::{CharacterDescriptor, CostumeDescriptor, FighterAssets},
    Capabilities, CharacterCallbacks, Fighter, MotionRow,
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
    /// Fighter +2248..+226C: the articles that are out, and the damage
    /// callbacks their motions install.
    pub articles: crate::articles::Articles,
    /// accessory4_cb while a character row owns it.
    pub accessory: crate::articles::Accessory,
    /// fp->mv.gw.SpecialN.
    pub chef: crate::special_n::Chef,
    /// A Judgment asked for in this Input proc's IASA (its airborne flag),
    /// entered once the IASA returns.
    pub pending_judgement: Option<bool>,
    /// Fighter +6BC lstick_angle, which Fighter_ChangeMotionState zeroes
    /// (fighter.c:1171): Fire's lean.
    pub rescue_angle: f32,
    /// fp->mv.gw.SpecialLw.
    pub oil_panic: crate::special_lw::OilPanic,
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
            articles: Default::default(),
            accessory: Default::default(),
            chef: Default::default(),
            pending_judgement: None,
            rescue_angle: 0.0,
            oil_panic: Default::default(),
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
    /// ftGw_Init_MotionStateTable's rows with x9_b1 (bit 22 of the word at
    /// +8, read from the DOL at 0x803D23E8): the neutral, back and up
    /// aerials' landings (350..352).
    const KO_COUNTDOWN_ROWS: &'static [u16] = &[350, 351, 352];
    const MOTION_FLAGS: &'static [u32] = &crate::MOTION_FLAGS;
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &crate::SPECIAL_MOVES;
    const SPECIAL_PARTNER_SYNC: &'static [bool] = &crate::SPECIAL_PARTNER_SYNC;
    /// decideAttack11's FTKIND_GAMEWATCH arm (ftCo_Attack1.c:92).
    const ENTER_JAB: Option<melee_ft::fighter::GroundAttackEntry> = Some(crate::attack::enter_jab);
    /// fn_800D6AC4's FTKIND_GAMEWATCH arm (ftCo_Attack100.c:137).
    const ENTER_RAPID_JAB: Option<melee_ft::fighter::GroundAttackEntry> =
        Some(crate::attack::enter_rapid_jab);
    /// ftCo_AttackLw3.c decideFighter's FTKIND_GAMEWATCH arm (:70).
    const ENTER_DOWN_TILT: Option<melee_ft::fighter::GroundAttackEntry> =
        Some(crate::attack::enter_down_tilt);
    /// ftCo_AttackS4.c decideFighter's FTKIND_GAMEWATCH arm (:156).
    const FORWARD_SMASH: Option<melee_ft::fighter::RngEntry> =
        Some(crate::attack::enter_forward_smash);
    /// ftCo_AttackAir.c decideFighter's FTKIND_GAMEWATCH arm (:81).
    const ENTER_AERIAL: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        crate::attack_air::enter;
    /// Fighter_8006C80C: the row's accessory4.
    fn accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
        crate::articles::accessory(f, assets, rng);
    }
    /// ftData_SpecialN/S/Hi/Lw[GameWatch] and the aerial tables.
    fn enter_special(
        f: &mut Fighter,
        slot: melee_ft::fighter::SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        use melee_ft::fighter::SpecialSlot;
        match slot {
            SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
            SpecialSlot::Side => crate::special_s::request(f, airborne),
            SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
            SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
        }
    }
    /// ftGw_SpecialS_Enter / ftGw_SpecialAirS_Enter draw the face first.
    const INPUT_RNG_ENTRY: Option<melee_ft::fighter::RngEntry> =
        Some(crate::special_s::enter_pending);
    /// ftGw_Init_UnkMotionStates4 (8014A77C): OnDeath keeps the bucket's
    /// level, so a full bucket glows again after a stock is lost.
    const COLOR_FALLBACK_AFTER_RESET: fn(&melee_ft::fighter::CharacterState) -> Option<u8> =
        |state| {
            (state.get::<GameWatch>().panic_charge >= PANIC_FULL)
                .then_some(crate::special_lw::FULL_BUCKET_COLOR)
        };
    /// ftData_OnAbsorb: ftGw_Init_OnAbsorb (8014A828).
    const ON_ABSORB: Option<fn(&mut Fighter, &FighterAssets, melee_ft::fighter::absorb::Absorbed)> =
        Some(crate::special_lw::on_absorb);
    /// ftCommon_8007DB58: take_dmg_cb (ftGw_Init_OnDamage) when installed.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = Some(crate::articles::damage_callback);
    /// ftCo_800D331C: death2_cb, the same callback.
    const DEATH: Option<fn(&mut Fighter)> = Some(crate::articles::damage_callback);
    /// The articles' Destroyed callbacks.
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = crate::articles::destroyed;
    /// ftGw_AttackAirN_EnterItemHitlag / ExitItemHitlag: every aerial
    /// article that is out.
    const ARTICLE_HITLAG_BEGIN: fn(&mut Fighter) = crate::articles::hitlag_begin;
    const ARTICLE_HITLAG_END: fn(&mut Fighter) = crate::articles::hitlag_end;
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks go.
    fn on_motion_change(&mut self) {
        self.articles.damage_callbacks = false;
        self.accessory = crate::articles::Accessory::None;
        self.rescue_angle = 0.0;
    }
    /// The Fire torch holds its animation while the forward smash charges
    /// (ftLib_800876D4: smash_attrs.state == 2), reported as stage 1.
    fn item_owner(f: &mut Fighter, _assets: &FighterAssets) -> melee_it::ItemOwner {
        let charging = f
            .commands
            .smash_charge
            .is_some_and(|c| matches!(c.phase, melee_cmd::ChargePhase::Charging));
        melee_it::ItemOwner {
            illusion: None,
            position: f.physics.position,
            facing: f.physics.facing,
            hold_position: f.physics.position,
            blaster_action: 9,
            remove_blaster: true,
            motion: f.motion_state.action.0,
            articles_fired: 0,
            charge: None,
            holds_needles: false,
            stick: hsd_types::Vec2::new(f.input.current.stick.x, f.input.current.stick.y),
            steering_article: false,
            detonating_article: false,
            motion_flags: f.motion_flags(),
            in_hitlag: f.core.in_hitlag(),
            anchor: f.physics.position,
            article_stage: charging.then_some(1),
            model_scale: f.player.scale * f.attributes.size.model_scaling,
        }
    }
    /// ftCo_800DEA28 default arm (`ftCo_800DEBD0`): the common AppealS entry.
    const ENTER_TAUNT: fn(&mut Fighter, &FighterAssets) -> melee_ft::fighter::assets::Result<()> =
        Fighter::enter_common_taunt;
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
        self.judge_last = saved_word(raw, 0x222C) as i32;
        self.judge_previous = saved_word(raw, 0x2230) as i32;
        self.x2234 = saved_word(raw, 0x2234);
        self.panic_charge = saved_word(raw, 0x2238) as i32;
        self.panic_damage = saved_word(raw, 0x223C) as i32;
        self.chef_last = saved_word(raw, 0x2240) as i32;
        self.chef_previous = saved_word(raw, 0x2244) as i32;
        if (0x2248..0x2270).step_by(4).any(|at| saved_word(raw, at) != 0) {
            unimplemented!("ftGameWatch_FighterVars: a saved article GObj");
        }
    }
    /// ftGw_Init_OnLoad (8014A404): x2222_b6 clear, x2223_b1 and
    /// can_walljump set, PUSH_ATTRS, the empty bucket. The ten item
    /// registrations (it_8026B3F8) belong to the item scene.
    fn on_load(&mut self, capabilities: &mut Capabilities) {
        capabilities.can_walljump = true;
        capabilities.specials = [true; 4];
        // fp->x34_scale.z = da->x0_GAMEWATCH_WIDTH.
        capabilities.model_width = Some(self.attributes.width);
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
        self.articles = Default::default();
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

/// A big-endian word of a saved fighter struct.
fn saved_word(raw: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([raw[offset], raw[offset + 1], raw[offset + 2], raw[offset + 3]])
}
