//! Fighter ownership and scheduler callbacks for grounded, item-free Wait.
//! Retail addresses and unsupported paths are documented at each entry point.
mod character;
pub use character::{Accessory2, CharacterState, CharacterTable, VerticalSmash};

pub mod absorb;
pub mod air_catch;
pub mod air_dodge;
pub mod assets;
pub mod attack;
pub mod caches;
pub mod cape_turn;
pub mod capture_captain;
pub mod capture_mewtwo;
pub mod barrel;
pub mod bury;
pub mod capture_redead;
pub mod capture_kirby;
pub mod capture_koopa;
pub mod capture_yoshi;
pub mod cargo;
pub mod coins;
pub mod clank;
pub mod color_overlay;
pub mod commands;
pub mod cpu;
pub mod cpu_profile;
pub mod damage;
mod damage_bind;
mod damage_song;
pub mod dash;
pub mod down;
mod dynamic_commands;
pub mod effects;
pub mod entry;
pub mod escape;
pub mod fall;
pub mod fly_reflect;
pub mod frozen;
pub mod heal;
pub mod grab;
pub mod grab_damage;
pub mod grab_escape;
pub mod grab_throw;
mod hit_log;
pub mod hazard;
pub mod hitbox;
pub mod hitlag_link;
pub mod item_hits;
pub mod item_pickup;
pub mod item_flower;
pub mod power_up;
pub mod item_scope;
pub mod item_screw;
pub mod metal;
pub mod item_shoot;
pub mod item_swing;
pub mod item_throw;
pub mod jump;
pub mod ko_source;
pub mod landing;
pub mod ledge;
pub mod life;
pub mod lift;
pub mod model_scale;
pub mod multi_jump;
pub mod offscreen;
pub mod overlap;
pub mod parasol;
pub mod part_rotation;
pub mod partner;
mod pass;
pub mod passive_ceil;
mod procs;
pub mod reflection;
pub mod run;
pub mod scratch_tail;
pub mod shield;
mod shield_break;
mod sleep;
pub mod smash;
mod snapshot;
mod spawn;
pub mod squat;
pub mod stage_hazard;
mod stage_wind;
pub mod stamina;
pub mod state;
pub mod stop_ceil;
pub mod stop_wall;
pub mod teeter;
pub mod throw_flight;
pub mod transform;
pub mod tether;
pub mod turn;
pub mod turn_run;
pub mod walk;
pub mod wall_jump;

use crate::{
    anim::FighterAnimation,
    collision::{ground::EnvironmentCollision, pose::GroundPoseFlags},
    desc::{FighterAttributes, FighterBones},
    input::FighterInput,
    physics::FighterPhysics,
};
pub use cpu::CpuState;
use hsd_anim::jobj::JObjTree;
use hsd_types::{Vec2, Vec3};
use melee_types::{FighterKind, PlayerKind};
pub use spawn::{
    MotionColorPolicy, MotionEntryFlags, MotionPreservation, PlayerSlot, SpawnContext, SpawnCounter,
};
pub use state::{
    common_table, interleaved_order, ActionId, FighterProc, MotionRow, MotionState, SpecialSlot,
    COMMON_COUNT,
};

/// Per-candidate character defense callback, before ordinary hurtbox contact.
pub type DefenseContact = fn(&mut Fighter, &mut Fighter, &assets::FighterAssets, usize) -> bool;
/// A special's grab callbacks: (captor, victim, captor assets, victim assets).
pub type SpecialGrab = fn(
    &mut Fighter,
    &mut Fighter,
    &assets::FighterAssets,
    &assets::FighterAssets,
) -> assets::Result<()>;
/// A kind's own entry for a ground attack the common code selects.
pub type GroundAttackEntry = fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>;
/// A special's release of the fighter it caught, run on the pair once the
/// captor's animation callback asks for it (`combat.special_throw_release`):
/// (victim, captor, victim assets, captor assets, map, rng).
pub type SpecialRelease = fn(
    &mut Fighter,
    &mut Fighter,
    &assets::FighterAssets,
    &assets::FighterAssets,
    &mut melee_mp::CollMap,
    &mut gekko_math::HsdRng,
) -> assets::Result<()>;
/// Deferred character defense reaction at Fighter_ProcessHit.
pub type DefenseHit = fn(&mut Fighter, &assets::FighterAssets);
/// A character shield volume against item hitbox `id` (ftColl_8007925C's
/// shield step, ftColl_80077688). Returns true when it caught the hit.
pub type ItemDefenseContact =
    fn(&mut Fighter, &mut melee_it::ItemCore, usize, &assets::FighterAssets) -> bool;

/// Character-owned load/reset hooks (`ftData_OnLoad`/`ftData_OnDeath`).
/// Implementations live in ft-<character>; common fighter code never loads a
/// character crate. The implementation owns its typed special attributes.
/// A character motion entry that draws from the RNG (ftPe_AttackS4_Enter).
pub type RngEntry =
    fn(&mut Fighter, &assets::FighterAssets, &mut gekko_math::HsdRng) -> assets::Result<()>;
/// `CharacterCallbacks::ACT_ON_PARTNER`: the fighter, its partner, their
/// assets in that order, and the stage.
pub type PartnerAction = fn(
    &mut Fighter,
    &mut Fighter,
    &assets::FighterAssets,
    &assets::FighterAssets,
    &mut melee_mp::CollMap,
) -> assets::Result<PartnerMotionChanges>;

/// `CharacterCallbacks::OBSERVE_PARTNER`: the fighter, its partner and the
/// proc about to run. The partner is mutable only so that a part's world
/// position can be read from it (lb_8000B1CC sets up the joint's matrix,
/// as retail's callbacks do on the other fighter).
pub type PartnerObserver = fn(&mut Fighter, &mut Fighter, state::FighterProc);

/// Which of the pair `ACT_ON_PARTNER` moved to a new motion; the scene
/// flushes that fighter's efAsync queue, as its Fighter_ChangeMotionState
/// did within the proc (fighter.c:951).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartnerMotionChanges {
    pub fighter: bool,
    pub partner: bool,
}

/// The captor's side of ftCo_CaptureWaitKirby.c's calls into it.
#[derive(Clone, Copy)]
pub struct MouthHold {
    /// ftKb_SpecialN_800F5A38: the captor stands or falls with its mouth
    /// full, where the held fighter's stick moves it.
    pub steerable: fn(&Fighter) -> bool,
    /// ftKb_SpecialS_800F6BB0: the captor's aerial row when the held
    /// fighter hops it off the floor.
    pub leave_floor: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>,
}

/// What the CPU reads of a chargeable neutral special.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeutralCharge {
    /// u.dk.x222C, u.ss.x2230, u.mt.x2234; Kirby's u.kb.xBC, xA8, x9C.
    pub stored: i32,
    /// Mewtwo's full Shadow Ball (attributes +0, compared with the stored
    /// charge as a float; Kirby's specialn_mt_charge_time).
    pub full: f32,
}

pub trait CharacterCallbacks: Clone + Sized + Send + Sync + 'static {
    /// Character choices in ftCo_0A01.c / ftcpuattack.c, consumed as static data.
    const CPU_PROFILE: cpu_profile::CpuProfile = cpu_profile::CpuProfile::DEFAULT;
    /// A copied charged special whose CPU branch still needs a retail witness.
    const CPU_COPIED_CHARGE: fn(&CharacterState) -> bool = character::no_cpu_copied_charge;

    const TABLE: CharacterTable = CharacterTable::new::<Self>();

    #[inline(always)]
    fn table() -> &'static CharacterTable {
        &Self::TABLE
    }

    fn into_state(self) -> CharacterState {
        CharacterState::new(self)
    }

    /// ftCo_AttackAir.c: decideFighter. Link/Young Link and Game & Watch
    /// supply their character entry here when their aerials are ported.
    const ENTER_AERIAL: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()> =
        attack::aerial::enter;

    /// Whether ordinary common Down Air keeps the common Animation owner.
    /// Link's family replaces it; held-entry proof must not admit that body.
    const USES_COMMON_DOWN_AIR_ANIMATION: bool = true;

    /// Character-owned table (retail's per-kind MotionState table), indexed
    /// from action 341. Specials will form its bulk; existing multijumps and
    /// character shield states also live here.
    const SPECIAL_ROWS: &'static [MotionRow] = &[];
    /// The character table's x4_flags column (Fighter.x2070 on entry),
    /// indexed from action 341; empty until a caller reads it.
    const MOTION_FLAGS: &'static [u32] = &[];
    /// ftData special-row move IDs, indexed from action 341.
    const SPECIAL_MOVES: &'static [Option<melee_types::combat::StaleMove>] = &[];
    /// MotionState.x9_b0 of the special rows, indexed from action 341 (read
    /// only for a partner fighter; see `state::partner_sync`).
    const SPECIAL_PARTNER_SYNC: &'static [bool] = &[];

    fn special_rows() -> &'static [MotionRow] {
        Self::SPECIAL_ROWS
    }

    /// ftData_SpecialN/S/Hi/Lw[kind] (and the Air tables). Every kind has
    /// its own row in each table, so every character supplies its entry
    /// (a special it has not ported fails closed in its own crate).
    fn enter_special(
        fighter: &mut Fighter,
        slot: SpecialSlot,
        airborne: bool,
        assets: &assets::FighterAssets,
    );

    /// Fighter_UnkProcessGrab (8006CA5C) for a special armed by
    /// ftCommon_8007E2D0: the character's grab_cb on `captor`, then the
    /// grabbed_cb it installed on `victim` (a common capture entry).
    const SPECIAL_GRAB: SpecialGrab = character::unsupported_special_grab;
    /// What the special's release does to the caught pair (Falcon Dive's
    /// ftCo_800DE7C0 launch, Confusion's ftCo_80090780 drop).
    const SPECIAL_RELEASE: SpecialRelease = character::unsupported_special_release;

    /// Fighter_8006C80C: character-owned accessory4, after the deferred effect flush.
    fn accessory(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        _rng: &mut gekko_math::HsdRng,
    ) {
    }
    /// Fighter_8006C80C's accessory4 for a callback that reads the stage
    /// (Donkey Kong's Hand Slap lays its hitboxes along the floor,
    /// ftDk_Init_8010DB3C): called right after `accessory`.
    const MAP_ACCESSORY: Option<fn(&mut Fighter, &assets::FighterAssets, &mut melee_mp::CollMap)> =
        None;
    /// ftData_UnkMotionStates3[kind] (fighter.c:1652): the kind's own work
    /// every frame of Fighter_8006A360, in hitlag too, ahead of the
    /// animation step (Bowser's breath refilling, ftKp_SpecialLw_80134D78).
    const EVERY_FRAME: Option<fn(&mut Fighter)> = None;
    /// ftCommon_8007DB58: character take-damage callback before damage entry.
    const TAKE_DAMAGE: Option<fn(&mut Fighter)> = None;
    /// Fighter_ProcessHit's take_dmg_2_cb (fighter.c:2862): a hit with
    /// knockback landed, before the reaction is chosen (armour or not). The
    /// hook decides whether the current motion installed the callback.
    const HIT_TAKEN: Option<fn(&mut Fighter)> = None;
    /// Fighter_ProcessHit's kind calls once the frame's damage has reached
    /// percent: ftKb_SpecialN_800F5BA4 for a hit with knockback
    /// (fighter.c:2860, ahead of take_dmg_2_cb; the flag says the hit was a
    /// cape's) and ftKb_SpecialN_800F5C34 for damage without one
    /// (fighter.c:2960). Kirby draws there for the loss of his ability.
    const DAMAGE_DRAW: Option<DamageDrawFn> = None;
    /// ftData_UnkMotionStates4[kind] as the kind's state stands once
    /// OnDeath has run: the colour animation the secondary slot takes
    /// whenever it empties (`CombatState::secondary_color_fallback`), for a
    /// kind whose state survives the reset (Mr. Game & Watch's full bucket).
    const COLOR_FALLBACK_AFTER_RESET: fn(&CharacterState) -> Option<u8> =
        character::no_color_fallback;
    /// ftData_OnAbsorb[kind] (Fighter_ProcessHit, fighter.c:2947-2950): the
    /// absorbing bubble took an item's hitbox this frame and nothing the
    /// fighter received or dealt came first.
    const ON_ABSORB: Option<fn(&mut Fighter, &assets::FighterAssets, absorb::Absorbed)> = None;
    /// Fighter.deal_dmg_cb (fighter.c:2929): Fighter_ProcessHit when this
    /// fighter's hit landed and nothing it received took precedence. The hook
    /// decides whether the current motion installed the callback.
    const DEAL_DAMAGE: Option<fn(&mut Fighter, &assets::FighterAssets)> = None;
    /// Whether every special row was audited to leave a held light item in
    /// hand (no item branch in the character's special code); a special
    /// entered while holding one otherwise fails closed.
    const SPECIALS_KEEP_HELD_ITEM: bool = false;
    /// The character rows with MotionState.x9_b1 (bit 22 of the word at
    /// +8), whose grounded entry starts the KO credit's countdown
    /// (fighter.c:1185). In the retail tables only Donkey Kong's, Kirby's,
    /// Mr. Game & Watch's and Sandbag's have any.
    const KO_COUNTDOWN_ROWS: &'static [u16] = &[];
    /// Fighter x2226_b1, set by the kind's OnLoad (Yoshi, ftYs_Init_OnLoad):
    /// the down bound's face-up/face-down choice from HipN is inverted
    /// (ftCo_8009794C, ftCo_DownBound.c:129-131).
    const DOWN_BOUND_INVERTED: bool = false;
    /// The kind's attributes follow a model scale other than 1 beyond
    /// ftCo_800CF6E8's common ones: its LoadSpecialAttrs scales special
    /// attributes (Link, Mewtwo, Pikachu, Samus), its multi-jump table
    /// (x2D0, Kirby and Jigglypuff: ftCo_800D0CBC) or its cargo
    /// attributes (x2222_b0, Donkey Kong), none of them ported.
    const SCALES_SPECIAL_ATTRIBUTES: bool = false;
    /// ftCo_800D331C: death2/death3/death1 callbacks before a death entry.
    const DEATH: Option<fn(&mut Fighter)> = None;
    /// The character's own parasol while it hangs on fp->item_gobj
    /// (ftGetParasolStatus, ftCo_ItemParasolGetFallMotionId). A table
    /// constant rather than a default method, so characters without one
    /// share one concrete default.
    const SPECIAL_PARASOL: fn(&CharacterState) -> Option<parasol::SpecialParasol> =
        character::no_special_parasol;
    /// ftCommon_8007E83C (8007E83C): parasol animation `index` over
    /// `frames` (zero: at the fighter's rate).
    const SET_PARASOL_ANIMATION: fn(&mut Fighter, usize, f32) =
        character::unsupported_parasol_animation;
    /// An article this fighter owns was destroyed (the kind's Destroyed
    /// logic callback reaching back to its owner).
    const ARTICLE_DESTROYED: fn(&mut Fighter, melee_types::ItemKind) = character::no_article;
    /// An owned article's accessory offers this fighter its blast (Samus's
    /// bomb, it_802B5478).
    const OWNER_BLAST: fn(&mut Fighter, &melee_it::OwnerBlast, &assets::FighterAssets) =
        character::unsupported_owner_blast;
    /// The on_accessory of an article whose kind leaves it to its owner
    /// (melee_it::ItemLogic::OWNER_ACCESSORY), at item link 9: the owner's
    /// work on it, returning the article's new motion state if it changes.
    const ARTICLE_ACCESSORY: fn(
        &mut Fighter,
        &assets::FighterAssets,
        &mut melee_mp::CollMap,
    ) -> Option<u16> = character::no_article_accessory;
    /// An article this fighter requested was created: the concrete Item
    /// GObj its creation call returned (Ness stores it as `yoyo_gobj`,
    /// ftNs_AttackHi4_YoyoCreateItem 80115A4C).
    const ARTICLE_CREATED: fn(&mut Fighter, melee_types::ItemKind, u32) = |_, _, _| {};
    /// An article this fighter owns ended (after `ARTICLE_DESTROYED`): the
    /// instance id, for owners keeping per-instance state.
    const ARTICLE_RETIRED: fn(&mut Fighter, melee_types::ItemKind, u32) = |_, _, _| {};
    /// Which article a control of `kind` reaches: every owned article of
    /// the kind (the default), or the one instance the fighter's pointer
    /// names (Ness's `yoyo_gobj`).
    const ARTICLE_CONTROL_TARGET: fn(
        &CharacterState,
        melee_types::ItemKind,
    ) -> melee_it::ArticleControlTarget = |_, _| melee_it::ArticleControlTarget::Owned;
    /// The phys_cb of an article whose kind leaves it to its owner
    /// (melee_it::ItemLogic::OWNER_PHYSICS), at item link 4: the owner's
    /// work on that instance (Ness's yo-yo string, itnessyoyo.c).
    const ARTICLE_PHYSICS: fn(&mut Fighter, u32, &assets::FighterAssets, &mut melee_mp::CollMap) =
        character::no_article_physics;
    /// The rest of an article's post-hitlag callback
    /// (`effect_state.article_hitlag`), after the article thaws.
    const ARTICLE_HITLAG_END: fn(&mut Fighter) = character::no_article_hitlag_end;
    /// The rest of an article's pre-hitlag callback, after the article of
    /// `effect_state.article_hitlag` freezes (ftGw_AttackAirN_EnterItemHitlag
    /// freezes every aerial article that is out).
    const ARTICLE_HITLAG_BEGIN: fn(&mut Fighter) = character::no_article_hitlag_end;
    /// An article this fighter owns asked something of it from its proc
    /// (the returning boomerang's catch, ftLk_SpecialS2_Enter). Returns the
    /// part a caught article hangs from.
    const ARTICLE_REQUEST: fn(
        &mut Fighter,
        &assets::FighterAssets,
        melee_types::ItemKind,
        melee_it::OwnerRequest,
    ) -> Option<u8> = character::unsupported_article_request;
    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:54-58: articles the
    /// character puts away on landing, after `on_landing`.
    const LANDING_ARTICLES: fn(&mut Fighter, bool) = character::no_landing_articles;
    /// ft_8008A348's kind branch on entering Wait (ft_08A1.c:85-91).
    const WAIT_ARTICLES: fn(&mut Fighter) = character::no_wait_articles;
    /// ft_8008A348's and ftCo_SquatWait_Enter_inline's kind switch after
    /// the motion change (Link's Hylian shield, ft_08A1.c:99-106,
    /// ftCo_SquatWait.c:67-75).
    const WAIT_ENTERED: fn(&mut Fighter) = character::no_wait_articles;
    /// ftCo_800C3538's x2222_b2: the character's current state takes a cape
    /// hit without the turnaround (Fox's Illusion).
    const CAPE_TURN_BLOCKED: fn(&mut Fighter) -> bool = character::cape_turn_allowed;
    /// ftCo_800C37A0's fp->x21F8 when the state installed
    /// `CapeTurnEnd::Character` (Yoshi's and Jigglypuff's rolls).
    const CAPE_TURN_END: fn(&mut Fighter) = character::no_cape_turn_end;
    /// ftCommon_8007EFC8's last step, the callback the form in play passes:
    /// this form's arrival once it takes over (ftZd_SpecialLw_8013B4D8,
    /// ftSk_SpecialLw_80114758). Only transforming characters have one.
    const TRANSFORMATION_ARRIVAL: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()> =
        character::no_transformation;
    /// Fighter_ProcessHit's hurtbox_detect_cb (fighter.c:2950-2954): an
    /// inert hitbox of this fighter touched another fighter (`unk_gobj`
    /// and x221C_b5). Per-motion in retail, like DEAL_DAMAGE.
    const HURTBOX_DETECT: Option<fn(&mut Fighter, &assets::FighterAssets, damage::InertTouch)> =
        None;
    fn item_muzzle(_fighter: &mut Fighter, _assets: &assets::FighterAssets) -> Option<(Vec3, f32)> {
        None
    }

    fn item_owner(fighter: &mut Fighter, _assets: &assets::FighterAssets) -> melee_it::ItemOwner {
        default_item_owner(fighter)
    }

    fn kind(&self) -> FighterKind;
    /// ftCo_800BD9E0 (800BD9E0): the kind a Kirby that swallows this
    /// fighter copies, where it is not the fighter's own: Nana gives
    /// Popo's, Kirby his own hat's.
    const SWALLOWED_ABILITY: Option<fn(&CharacterState) -> FighterKind> = None;
    /// decideFighter's character arm (ftCo_AttackS4.c:150-156), after the
    /// facing is set: Peach's ftPe_AttackS4_Enter. It draws from the RNG,
    /// so it runs once the IASA returns (`Fighter::finish_input`).
    const FORWARD_SMASH: Option<RngEntry> = None;
    /// ftCo_AttackHi4_CheckInput's and ftCo_AttackLw4_CheckInput's
    /// `switch (fp->kind)` (ftCo_AttackHi4.c:43, 76; ftCo_AttackLw4.c:42):
    /// Ness's yo-yo smashes enter their own rows.
    const VERTICAL_SMASH: Option<character::VerticalSmash> = None;
    /// RNG draws a special's entry makes inside its motion change (an x21EC
    /// such as Luigi's ftLg_SpecialS_SetVars), run once the entering IASA
    /// returns (`Fighter::finish_input`): nothing in between draws.
    const INPUT_RNG: Option<fn(&mut Fighter, &mut gekko_math::HsdRng)> = None;
    /// A special whose entry draws from the RNG before its motion change
    /// (ftGw_SpecialS_GetRandomInt chooses Judgment's row): the entry asked
    /// for during the IASA runs once it returns (`Fighter::finish_input`).
    /// The hook decides whether one is pending.
    const INPUT_RNG_ENTRY: Option<RngEntry> = None;
    /// ftCo_800CED30 (800CED30): the kind's smash42 row (ftLk_MS_AttackS42
    /// for Link and Young Link); any other kind asserts "don't have smash42
    /// motion!!!".
    const FORWARD_SMASH_COMBO: Option<ActionId> = None;
    /// get_anim_id (ftswing.c:89-99): the kind's first Swing42 row (the
    /// sword's; the other swing items follow in fn_800CCEC4's order), only
    /// FTKIND_CAPTAIN's ftCa_MS_SwordSwing4 (341); any other kind asserts
    /// "don't have swing42 motion!!!".
    const SWING42_FIRST_ROW: Option<ActionId> = None;
    /// ftCo_AttackS4.c:145-166, decideFighter (8008C348): the default arm.
    /// Ness, Peach and Mr. Game & Watch take their own entries
    /// (`FORWARD_SMASH`); Pikachu and Pichu override this to install the
    /// effect hitlag callbacks.
    fn forward_smash_variant(&self) -> ForwardSmashVariant {
        ForwardSmashVariant::Standard
    }
    /// ftCo_Catch.c / CatchPull.c: ordinary body grab by default. Tether and
    /// character-specific capture variants override this boundary.
    fn catch_variant(&mut self) {}
    /// ftYs_Init_8012BAC0: a captor that holds its victim in its mouth (the
    /// FTKIND_YOSHI arms of ftCo_CaptureWait.c and ftCo_Thrown.c:33) returns
    /// the scale of the victim's single stand-in hurt capsule. A table
    /// constant, so characters without one share one concrete default.
    const MOUTH_CAPTURE_SCALE: fn(&CharacterState) -> Option<f32> =
        character::no_mouth_capture;
    /// Kirby's copied fighter (u.kb.hat.kind, Fighter +2238), which the
    /// CPU's attack choice reads (ftCo_800B77E8's FTKIND_KIRBY arm); None
    /// for every other kind.
    const COPIED_ABILITY: fn(&CharacterState) -> Option<melee_types::FighterKind> =
        character::no_copied_ability;
    /// The stored charge of a neutral special the CPU charges and releases
    /// (ftCo_800ADC28, ftCo_800ACD5C): Donkey Kong's swings, Samus's shot,
    /// Mewtwo's Shadow Ball, Kirby's copies of them. Zero for every other
    /// kind.
    const NEUTRAL_CHARGE: fn(&CharacterState) -> NeutralCharge = character::no_neutral_charge;
    /// ftCo_Throw.c:145-157,346-353: the kind switch's default arm (no
    /// callback). Its arms are Fox's and Falco's laser, Samus's grapple
    /// model, Kirby's and Yoshi's: each of those crates overrides this.
    fn throw_variant(&self) {}

    /// ftCo_800DD398's kind switch (ftCo_Throw.c:145-157), ftCo_800DD724's
    /// and ftCo_ThrowHi_Cam's kind tests (ftCo_Throw.c:287, 441) and
    /// ftCo_800DE3FC's thrower test (ftCo_Thrown.c:36), by the throw's
    /// state.
    const THROW_TRAITS: fn(melee_types::CommonMotionState) -> throw_flight::ThrowTraits =
        throw_flight::ordinary_throw;

    /// ftCommon_8007F824 / 8007F86C, after damage entry and on hitstun expiry.
    const KNOCKBACK_ENTER: fn(&mut Fighter, &assets::FighterAssets) = character::no_animation;
    const KNOCKBACK_EXIT: fn(&mut Fighter, &assets::FighterAssets) = character::no_animation;

    /// Character article work before the common throw release/end processing.
    const THROW_ANIMATION: fn(&mut Fighter, &assets::FighterAssets) = character::no_animation;

    /// fn_800D9CE8 (ftCo_CatchPull.c:28-37): the frame a standing grab's
    /// CatchPull starts at, given Catch's current frame. Default: that frame.
    const CATCH_PULL_START: fn(&mut Fighter, &assets::FighterAssets, f32) -> f32 =
        character::catch_frame;

    /// ftColl candidate boundary: special defense may consume an eligible hit.
    const DEFENSE_CONTACT: Option<DefenseContact> = None;
    const REFLECTOR_CONTACT: Option<reflection::CharacterContact> = None;
    const REFLECT_HIT: Option<reflection::CharacterResponse> = None;
    /// x2218_b4 while the character's reflector is up: a reflected item
    /// keeps its owner (ftColl_80077464 sets the item's xDCC b1; Mewtwo's
    /// Confusion, ftMt_SpecialS_ReflectThink).
    const REFLECTOR_KEEPS_OWNER: bool = false;
    /// Fighter_ProcessHit: deferred special defense reaction, before hitlag.
    const PROCESS_DEFENSE_HIT: Option<DefenseHit> = None;
    const ITEM_DEFENSE_CONTACT: Option<ItemDefenseContact> = None;

    /// ftCo_Attack1.c:89-110 and 202-213: decideAttack11, getMotionFlags
    /// (8008AB84 / 8008ABC0) and doAttack12's kind switch.
    /// The default arm; Pikachu and Pichu override it (Repeating), and
    /// Mr. Game & Watch's first jab is his own (`ENTER_JAB`).
    fn jab_variant(&self) -> JabVariant {
        JabVariant::Standard
    }
    /// decideAttack11 and doAttack12Rapid's kind arm (ftCo_Attack1.c:89-98,
    /// 171-181): the kind's own first jab (ftGw_Attack11_Enter) in place of
    /// checkAttack11.
    const ENTER_JAB: Option<GroundAttackEntry> = None;
    /// fn_800D6AC4's kind arm (ftCo_Attack100.c:133-143): the kind's own
    /// rapid-jab start (ftGw_Attack100Start_Enter) in place of fn_800D6B8C.
    const ENTER_RAPID_JAB: Option<GroundAttackEntry> = None;
    /// ftCo_AttackLw3.c decideFighter's kind arm (:67-76): the kind's own
    /// down tilt (ftGw_AttackLw3_Enter) in place of doEnter.
    const ENTER_DOWN_TILT: Option<GroundAttackEntry> = None;
    /// ftCo_AttackDash.c decideFighter: the kind's own dash attack row,
    /// entered as the common one is (Kirby's ftKb_MS_AttackDash).
    const DASH_ATTACK_ROW: Option<ActionId> = None;
    /// ftCo_Attack1 doAttack13 (8008B194): Marth restarts Attack11.
    fn third_jab_state(&self) -> melee_types::CommonMotionState {
        melee_types::CommonMotionState::Attack13
    }
    /// ftParts_800753D4 in OnLoad (ftLk_Init_OnLoad, ftCl_Init_OnLoad): the
    /// ftData.x48_items entry whose joint fills the kind's first
    /// Fighter_804D6540 conditional part (`desc::graft_conditional_joint`).
    const ONLOAD_ITEM_JOINT: Option<u32> = None;
    /// The character's on-disc resources (`ft<Char>_Init_*` strings, part and
    /// animation counts). The scene loads archives through this.
    fn descriptor() -> &'static assets::CharacterDescriptor
    where
        Self: Sized;
    /// Build the character from its data archive (`ftData.ext_attr` and
    /// whatever else its `OnLoad` reads). Mirrors `ft<Char>_Init_OnLoad`'s
    /// PUSH_ATTRS without the runtime allocation.
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, crate::desc::FighterDescError>
    where
        Self: Sized;
    /// Restore character-owned fields from a retail Fighter dump when a scene
    /// starts from a savestate (the shared fields are restored by the scene).
    fn restore_saved(&mut self, _raw_fighter: &[u8]) {}
    /// Take, from a retail Fighter dump of a match's first frame, the
    /// character variables Fighter_Create (0x80068E98) never writes and this
    /// character reads before writing: HSD_ObjAlloc hands the block back
    /// uncleared, so they hold whatever the heap held (docs/PORT_NOTES/
    /// STALE_HEAP.md). A scene built from parameters carries them from its
    /// boundary's recording; without one they keep their defaults.
    fn restore_stale_heap(&mut self, _raw_fighter: &[u8]) {}
    /// ftCo_800DEA28 (0x800DEA28): taunt entry. Retail switches on kind:
    /// Young Link (ftCl_Init_80149318), Dr. Mario (ftDr_Init_80149910) and
    /// Ganondorf (lb_800119DC effect, then ftCo_800DEBD0 twice) have their
    /// own arms; every other kind takes ftCo_800DEBD0, which also calls
    /// ftKb_SpecialN_800F5D04 for Kirby (Peach/Zelda arms need DbLevel >= 3,
    /// never true in retail). Characters on the default arm bind
    /// `Fighter::enter_common_taunt`; the pl_80040120 taunt stat is not
    /// modelled. Every kind names its arm.
    const ENTER_TAUNT: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()>;
    fn on_load(&mut self, capabilities: &mut Capabilities);
    fn on_reset(&mut self);
    /// The random draws of the kind's OnDeath, after `on_reset` (Kirby's
    /// Stone form, ftKb_Init_OnDeath).
    const RESET_DRAW: Option<fn(&mut CharacterState, &mut gekko_math::HsdRng)> = None;
    /// Fighter_procMap (fighter.c:2492-2495): after the state's collision
    /// callback, ftKb_SpecialN_800F1D24 (Kirby's bump star). Retail runs it
    /// only for a state with a collision callback; the port's rows all have
    /// one.
    const AFTER_COLLISION: Option<fn(&mut Fighter)> = None;
    /// ftCo_800C7CA0's ftKb_SpecialN_800F1F1C (ftCo_800C7CA0.c:37, 50): the
    /// kind's effect at the ECB corner a wall bounce starts from, before
    /// the bounce's own motion change.
    const WALL_BOUNCE_EFFECT: Option<fn(&mut Fighter, Vec3)> = None;
    /// How many values `RESET_DRAW` takes: creation's are part of the fixed
    /// interval between a Game Start seed and the pre-music boundary seed.
    const RESET_DRAW_COUNT: usize = 0;
    /// Costume-dependent OnLoad work (e.g. material animation end frames).
    fn on_costume_loaded(
        &mut self,
        _archive: &hsd_archive::Archive,
        _costume: u8,
    ) -> assets::Result<()> {
        Ok(())
    }
    /// OnLoad work requiring decoded animation resources and costume identity.
    fn on_resources_loaded(&mut self, _assets: &assets::FighterAssets, _player: &PlayerSlot) {}
    /// Fighter_ChangeMotionState, fighter.c:1120-1123: restore ground resources.
    fn on_grounded_motion(&mut self) {}
    /// Fighter_ChangeMotionState, fighter.c:1376-1389: the per-motion
    /// callbacks (take_dmg_cb, death2_cb, ...) a character installed go.
    fn on_motion_change(&mut self) {}
    /// ftCo_8009DD94, ftdynamics.c:397-419: first joint affected by forces.
    fn dynamics_first_force_bone(&self, _set: usize, _count: usize) -> usize {
        0
    }

    /// ftCo_800C3B10 (800C3B10), ftCo_AirCatch.c:54-79: the kind's tether
    /// once the common tests pass (unused this airtime, no held item, LR held
    /// and A pressed). The hook checks its own article (accessory2/death1/
    /// accessory3 installed) and enters the tether (ftCo_800C3BE8), returning
    /// whether it did. `None`: the kind has no tether.
    const AIR_TETHER: Option<fn(&mut Fighter, &assets::FighterAssets) -> bool> = None;
    /// The grabs' tether article (the FTKIND_LINK / FTKIND_CLINK arms of
    /// ftCo_Catch.c, ftCo_CatchPull.c, ftCo_CatchWait.c and the accessory
    /// callbacks); `None` for an ordinary grab.
    const TETHER: Option<tether::Tether> = None;
    /// What a fighter held in this character's mouth asks of it from its
    /// own callbacks (ftCo_CaptureWaitKirby.c reaching into
    /// ftkirbyspecialn.c); `None` for a kind that inhales nothing.
    const MOUTH_HOLD: Option<MouthHold> = None;

    /// The second motion scratch word (mv+4) while `action`, one of this
    /// character's special rows, is current. Common states that leave that
    /// word untouched (JumpAerial, Landing) carry it on. `None` for rows
    /// whose word is not modelled. A table constant rather than a default
    /// method, so characters without such rows share one concrete default.
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        character::no_retained_scratch_word;

    /// The motion scratch words at mv+0x14 and mv+0x18 (fp+0x2354,
    /// fp+0x2358) that `action`, one of this character's special rows, has
    /// written: `None` for a word the row leaves as it found it
    /// (`scratch_tail`). A table constant, as `RETAINED_SCRATCH_WORD` is.
    const SCRATCH_TAIL: fn(&CharacterState, ActionId) -> [Option<u32>; 2] =
        character::no_scratch_tail;

    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:51-83: the kind
    /// switch's default arm resets nothing. Its arms (Mario, Dr. Mario,
    /// Peach, Roy, Mr. Game & Watch, Popo, Nana, Kirby, Mewtwo) reset their
    /// airborne special resources in their own crates.
    fn on_landing(&mut self, _allow_interrupt: bool) {}

    /// ftCo_Guard.c:335-350, 917-934: egg shield and sword model hooks.
    fn guard_variant(&self, _commands: &mut commands::CommandState) {}
    /// ftCo_800992A8's kind arm before ftCo_80099314's motion change for a
    /// roll (Samus's ftCo_80099390 clears cmd_vars[0] and mv+4 first).
    const PREPARE_ROLL: Option<fn(&mut Fighter)> = None;
    /// ftCo_800D8C54's grab entry, once the Catch or CatchDash row is in:
    /// a kind whose grab throws an article takes over its callbacks.
    const CATCH_ENTERED: Option<fn(&mut Fighter)> = None;
    /// fn_800D9CE8's kind arm, once the captor's pull row is in (the
    /// tethers' pull reels their article in).
    const CATCH_PULLED: Option<fn(&mut Fighter)> = None;
    /// accessory2_cb (Fighter_CallAcessoryCallbacks_8006C624) outside
    /// hitlag, ahead of accessory1: an article the fighter drives from its
    /// own proc (Samus's grapple beam, it_802BAC80).
    const ACCESSORY2: Option<Accessory2> = None;
    /// accessory3_cb, which runs instead in hitlag (it_802BACC4).
    const HITLAG_ACCESSORY: Option<fn(&mut Fighter, &mut gekko_math::HsdRng)> = None;
    /// A fighter that shares its player with a partner fighter (Popo and
    /// Nana) reads it: the scene calls this before each of the fighter's
    /// procs with the partner as it stands then, since retail's callbacks
    /// dereference the other fighter directly (Player_GetEntityAtIndex).
    const OBSERVE_PARTNER: Option<PartnerObserver> = None;
    /// What a proc of this fighter left for its partner (a special that
    /// changes the other fighter's motion or link): the scene calls this
    /// after each proc with both fighters.
    const ACT_ON_PARTNER: Option<PartnerAction> = None;
    /// ftCo_Escape.c: per-character setup at its retail motion-entry boundary.
    /// The default arm; Samus's morph-ball roll (ftCo_Escape.c:83-85) and
    /// Yoshi's egg-shield escapes are their crates' overrides.
    fn escape_variant(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        _rolling: bool,
    ) -> assets::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }
    /// ftPe_8011BA54 / ftPe_8011BAD8: float selection surrounding the
    /// aerial-jump predicate. Characters without float never match; a match
    /// ends the IASA chain with `ENTER_FLOAT`.
    fn check_float_input(
        &self,
        _input: &crate::input::FighterInput,
        _assets: &assets::FighterAssets,
        _vertical_velocity: f32,
        _phase: FloatInputPhase,
    ) -> bool {
        false
    }
    /// ftPe_8011BB6C: the float entry selected by `check_float_input`.
    const ENTER_FLOAT: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()> =
        character::unsupported_float;

    /// Which double-jump entry the character uses
    /// (ftCo_JumpAerial.c:103-119 `switch (fp->kind)`). The default arm is
    /// the ordinary `ftCo_JumpAerial_Enter_Basic`; Ness, Yoshi, Peach and
    /// Mewtwo override this in their own crates.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Basic
    }
    /// ftCo_800CB870 / ftCo_800D730C: shared multijump attributes, when enabled.
    fn multi_jump_attributes(&self) -> Option<&multi_jump::MultiJumpAttributes> {
        None
    }
    /// ftCo_800D7268: ordinary family by default; Kirby's copied helmet uses
    /// the second family and must override this hook when that path is ported.
    fn multi_jump_family(&self) -> usize {
        0
    }
    /// The multijump's animation for `jump`. Read only for a kind whose
    /// `multi_jump_attributes` is `Some` (Kirby and Jigglypuff, the kinds
    /// with can_multijump, x2222_b1), and each of those overrides it.
    fn multi_jump_animation(&self, _jump: usize) -> i32 {
        unreachable!("ftCo_JumpAerialF1.c: multijump animation without multijump attributes")
    }
    /// Character-owned setup after the common aerial-jump entry and Anim.
    fn aerial_jump_entered(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }
    fn aerial_jump_animated(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }

    /// Retail action identity for a shared semantic state. Character tables
    /// may reuse common callbacks with their own action numbers.
    fn action_id(&self, state: melee_types::CommonMotionState) -> i32 {
        state.into()
    }
    /// GuardOn/Reflect usually skip animation and blend the guard pose.
    /// Animated shields instead run the actual GuardOn motion and script.
    fn animated_shield(&self) -> bool {
        false
    }
    /// Complete character-owned shield callbacks; None selects shared behavior.
    fn enter_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        _reflect: bool,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn animate_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    /// Character-owned Guard IASA; None selects the shared input order.
    fn input_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn enter_guard_hold(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn enter_guard_off(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    /// ftCo_80092E50's fp->kind branch: the character's shield-stun entry
    /// in place of ftCo_80092F2C; None selects the shared GuardSetOff. A
    /// table constant, so characters without one share one concrete default.
    const ENTER_SHIELD_STUN: fn(
        &mut Fighter,
        &shield::ShieldImpact,
        &assets::FighterAssets,
    ) -> Option<assets::Result<()>> = character::common_shield_stun;
    /// ftCo_Escape.c: character setup after motion entry, and completion.
    fn escape_finished(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn escape_animated(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }
}

/// One fighter kind as a scene builds it: the archives it loads and the
/// typed payload it runs. Every character type is its own form; a kind that
/// shares another kind's payload type (Nana, with Popo: one code path in
/// retail) is a form of its own, so the shared hooks compile once.
pub trait CharacterForm {
    type Character: CharacterCallbacks;
    fn form_descriptor() -> &'static assets::CharacterDescriptor;
    fn read_form(
        data: &hsd_archive::Archive,
    ) -> Result<Self::Character, crate::desc::FighterDescError>;
}
// Forwarders: inlined so each form adds no instantiation of its own.
impl<C: CharacterCallbacks> CharacterForm for C {
    type Character = C;
    #[inline(always)]
    fn form_descriptor() -> &'static assets::CharacterDescriptor {
        <C as CharacterCallbacks>::descriptor()
    }
    #[inline(always)]
    fn read_form(data: &hsd_archive::Archive) -> Result<C, crate::desc::FighterDescError> {
        <C as CharacterCallbacks>::from_archive(data)
    }
}

/// decideFighter's arms (ftCo_AttackS4.c:145-166) that enter through doEnter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForwardSmashVariant {
    /// The default arm: doEnter only.
    Standard,
    /// Pikachu and Pichu: doEnter, then Fighter_SetEffectHitlagCallbacks.
    EffectHitlagCallbacks,
}

/// ftCo_Attack1.c's per-kind jab arms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JabVariant {
    /// checkAttack11 with Ft_MF_None; the combo continues into Attack12.
    Standard,
    /// Pikachu and Pichu: getMotionFlags installs onPkPc21EC (a new attack
    /// instance inside the state change) with Ft_MF_SkipAttackCount, and
    /// doAttack12 restarts Attack11 through doAttack12Rapid.
    Repeating,
}

/// Float predicates run on either side of the ordinary aerial-jump check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatInputPhase {
    BeforeAerialJump,
    AfterAerialJump,
}

/// Double-jump entry variants of ftCo_JumpAerial.c:103-119. Basic, Ness, Peach and
/// Yoshi are ported; Mewtwo retains an explicit unsupported boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AerialJumpStyle {
    /// ftCo_JumpAerialF1: shared Kirby/Jigglypuff multijumps.
    MultiJump,
    /// ftCo_JumpAerial_Enter_Basic (0x800CBBC0).
    Basic,
    /// ftNs_JumpAerial_Enter: Ness's multi-frame double jump.
    Ness,
    /// ftYs_JumpAerial_Enter: Yoshi's armoured double jump.
    Yoshi,
    /// ftPe_JumpAerial_Enter: Peach's float-capable double jump.
    Peach,
    /// ftMt_JumpAerial_Enter: Mewtwo's teleport-style double jump.
    Mewtwo,
}

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    /// Fox or Falco on Corneria or Venom (grCorneria_801E2CE8): the Wait
    /// IASA offers the Star Fox taunt (ftFx_AppealS_CheckInput).
    pub star_fox_taunt: bool,
    /// Fighter +2226 bit 1: ftCo_DownBound selects the grounded-damage variant.
    pub grounded_down_bound: bool,
    /// can_walljump, Fighter +2224 bit 0; Fox OnLoad sets this.
    pub can_walljump: bool,
    /// ftData special callback presence, S/Hi/N/Lw.
    pub specials: [bool; 4],
    /// ftData_SpecialAirS/Hi/N/Lw[kind] presence where it differs from the
    /// ground tables (Donkey Kong has no aerial down special). `None`: the
    /// same as `specials`.
    pub air_specials: Option<[bool; 4]>,
    /// fp->x40, which OnLoad sets (the Ice Climbers only): the spawn and
    /// revival offset along the facing (ftCommon_800804EC).
    pub spawn_offset: f32,
    /// dmg.armor0 as every reset leaves it: zero (fighter.c:292), then the
    /// kind's OnDeath (Nana, Bowser, Giga Bowser).
    pub armor: f32,
    /// x2222_b5: this fighter's player has a partner fighter that goes when
    /// it does (Popo; ftCo_800BFD9C).
    pub leads_partner: bool,
    /// x2222_b4: healing items are never picked up (Nana; ftpickupitem.c:47).
    pub refuses_healing_items: bool,
    /// ft_GetGroundFrictionMultiplier's FTKIND_POPO / FTKIND_NANA arm
    /// (0x80084A50..5C): 1.0 on every floor, slippery or sticky.
    pub fixed_traction: bool,
    /// ftCo_800A101C's FTKIND_NANA arm: the CPU follows the player's own
    /// fighter (CpuFighter.xC 6).
    pub cpu_partner: bool,
    /// x2222_b0 and x2CC: the kind's forward throw ends in the cargo carry
    /// (Donkey Kong; ftCo_ThrowF_Anim).
    pub cargo: Option<cargo::CargoCarry>,
    /// fp->x34_scale.z when OnLoad sets it (Mr. Game & Watch's
    /// x0_GAMEWATCH_WIDTH): the model root's x scale in place of the model
    /// scale (Fighter_UpdateModelScale, fighter.c:220-224), which flattens
    /// the fighter along the depth axis. None: x34_scale.z is 1.
    pub model_width: Option<f32>,
    /// `stage_info.grkind == Gr_Kind_Flatzone`: ftCo_8009DD94 solves the
    /// dynamic bones with the root's x scale at x34_scale.y.
    pub flat_zone: bool,
    /// x2221_b2 (ftMt_Init_OnLoad): ftAnim_8006E054 takes TransN's extracted
    /// motion back out of the model's translation joint, so the model does
    /// not move with an animation whose motion the physics applies.
    pub compensates_root_motion: bool,
    /// ftCo_8009DD94's FTKIND_MEWTWO arm (ftdynamics.c:383-386): the dynamic
    /// chains test the floor in the captured rows (motions 223..232) even
    /// in the air.
    pub captured_dynamics_meet_floor: bool,
}

/// Which proc guard the current state asked for: `proc_status` checks that
/// a hitlag, damage or attack state still holds the data it needs. Held
/// items, grabs, death, status effects, accessory callbacks, the effect
/// queue, stage collision lists and the overlap nudge each live in their own
/// owner now, not behind an interaction gate (docs/COVERAGE_AUDIT.md).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Interaction {
    #[default]
    Idle,
    Hitlag,
    Shield,
    Damage,
    Attack,
}

/// A parts slot an article's model fills (see `FighterCore::grafted_part`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraftedPart {
    /// The fp->parts index.
    pub part: usize,
    /// The model's world translation.
    pub position: Vec3,
}

/// Reset sentinels used by the ordinary Wait proc path.
#[derive(Clone, Debug)]
pub struct Status {
    pub wall_jump: wall_jump::WallJump,
    /// used_tether (+2228 bit 6): ftCo_800C3B10 already tethered this
    /// airtime; a grounded motion change or death clears it.
    pub used_tether: bool,
    /// x221F_b3 (+221F mask 10).
    pub disabled: bool,
    /// Fighter +221C mask2: damage owns hitstun completion until cleared.
    pub in_hitstun: bool,
    /// x221D_b4: reset input during match startup.
    pub input_frozen: bool,
    /// x221D_b5: skip fighter-overlap nudge while dodging (ftcommon.c:850).
    pub ignore_fighter_nudge: bool,
    /// x2220_b3: a hit deals its damage without a reaction (ftCo_8008EC90's
    /// inlineB2), set while buried.
    pub no_hit_reaction: bool,
    /// x2224_b4: buried; Fighter_procUpdate skips the stage's wind.
    pub buried: bool,
    /// x2227_b6: frozen in ice (ftCo_DamageIce); every dynamic bone follows
    /// the animation (ftCo_8009E7B4) and a Ground hit does not bury. No
    /// motion change clears it: thawing, shattering and a take-damage
    /// interrupt do.
    pub frozen: bool,
    pub interaction: Interaction,
    /// dmg.x18ac_time_since_hit (+18AC), reset -1.
    pub time_since_hit: i32,
    /// smash_attrs.x2138_smashSinceHitbox (+2138), reset -1.
    pub time_since_smash: f32,
    /// shield_health (+1998), reset PlCo.x260.
    pub shield_health: f32,
    /// x2064_ledgeCooldown (+2064).
    pub ledge_cooldown: i32,
    /// Fighter2227.b1: ledge-hang timeout provenance, cleared on grounded entry.
    pub ledge_timed_out: bool,
    /// x2222_b3: count a top exit without upward knockback.
    pub unconditional_top_exit: bool,
    /// x2219_b2: ftCo_800D3158 does not test this fighter against the blast
    /// zone. Every motion change clears it (fighter.c:1040).
    pub blast_zone_exempt: bool,
    /// x2228_b2: suppress grabs while another state owns ledge detection.
    pub ledge_grab_disabled: bool,
    /// x221D_b7: hanging/ledge-option state, cleared on motion entry.
    pub on_ledge: bool,
    /// x1A6A, ftCommon_8007E2F4: excluded grab categories.
    pub grab_exclusions: ledge::GrabExclusions,
    /// x221E_b6 and x1A68 for a character special's grab (ftCommon_8007E2D0
    /// outside Catch): the grab category its catch hitboxes take. Every
    /// motion change disarms it (fighter.c:1068).
    pub special_grab: Option<ledge::GrabExclusions>,
    /// x1990: timed intangibility, independent of subaction hurt status.
    pub ledge_intangibility: i32,
    /// Fighter +1994: revival protection allows contact sparks but no damage.
    pub revival_invincibility: i32,

    /// x2100 (+2100), -1 disables sword afterimages.
    pub sword_trail: i32,
    /// dmg.x18B8/x18BC: camera damage displacement.
    pub camera_shift: Vec2,
    /// x209A, nametag display countdown; Wait entry uses PlCo.x5F0.
    pub name_tag_timer: u16,
    /// bury_timer_2 (+232C): after a barrel's shot, frames before a barrel
    /// may catch the fighter again (ftCo_800C0A98 counts it down on the
    /// Kongo stages; ftCo_800C09B4 clears it at creation and each death).
    pub barrel_cooldown: u32,
    /// bury_timer_1 (+2328): frames after a stage line's hit before the
    /// next (ftCo_800C0B20 sets it, ftCo_800C0A98 counts it down,
    /// ftCo_800C09B4 clears it at creation and each death).
    pub hazard_cooldown: u32,
    /// capture_timer (+2108): frames after a ReDead lets go before one may
    /// catch the fighter again (ftCo_800C7800 sets it, Fighter_procUpdate
    /// counts it down, fighter.c:486 clears it).
    pub capture_timer: i32,
    /// dmg.x18F0 (+18F0): points of healing still to come, one a frame
    /// (Fighter_8006CF5C adds, Fighter_8006A360 spends); reset 0
    /// (fighter.c:313).
    pub heal_remaining: i32,
    /// x221D_b6: a Starman's invincibility (ftColl_8007B7FC); reset 0
    /// (fighter.c:391).
    pub star_invincible: bool,
    /// x2004: the Starman's frames left.
    pub star_frames: i32,
    /// x2226_b4: cloaked by a Cloaking Device (or the Invisible Special
    /// Melee); percent damage is not taken (Fighter_TakeDamage_8006CC7C).
    pub cloaked: bool,
    /// x2030: the cloak's frames left.
    pub cloak_frames: i32,
    /// is_metal, metal_timer, metal_health (a Metal Box).
    pub metal: metal::MetalState,
}
impl Status {
    pub fn reset(shield_health: f32) -> Self {
        Self {
            wall_jump: wall_jump::WallJump::default(),
            used_tether: false,
            disabled: false,
            in_hitstun: false,
            input_frozen: false,
            ignore_fighter_nudge: false,
            no_hit_reaction: false,
            buried: false,
            frozen: false,
            interaction: Interaction::Idle,
            time_since_hit: -1,
            time_since_smash: -1.0,
            shield_health,
            ledge_cooldown: 0,
            ledge_timed_out: false,
            unconditional_top_exit: false,
            blast_zone_exempt: false,
            ledge_grab_disabled: false,
            on_ledge: false,
            grab_exclusions: ledge::GrabExclusions::NONE,
            special_grab: None,
            ledge_intangibility: 0,
            revival_invincibility: 0,
            sword_trail: -1,
            camera_shift: Vec2::ZERO,
            name_tag_timer: 0,
            barrel_cooldown: 0,
            hazard_cooldown: 0,
            capture_timer: 0,
            heal_remaining: 0,
            star_invincible: false,
            star_frames: 0,
            cloaked: false,
            cloak_frames: 0,
            metal: metal::MetalState::default(),
        }
    }
}

/// Character-owned payload and live callback row (ft/types.h).
/// Calculation owners live in the concrete core.
pub struct Fighter {
    pub core: FighterCore,
    pub character: CharacterState,
    pub motion_row: MotionRow,
}
impl Clone for Fighter {
    // Keep the complete ownership operation in melee-ft. An inlined derived
    // clone duplicates all nested allocation/cleanup code in every consumer.
    #[inline(never)]
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            character: self.character.clone(),
            motion_row: self.motion_row,
        }
    }
}
impl std::ops::Deref for Fighter {
    type Target = FighterCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
impl std::ops::DerefMut for Fighter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}
impl Fighter {
    /// Destroy a boxed fighter in this crate, so a scene that owns fighters
    /// does not repeat the fighter graph's drop glue.
    #[inline(never)]
    pub fn destroy(self: Box<Self>) {}
    pub fn character_accessory(
        &mut self,
        assets: &assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        (self.character.table().accessory)(self, assets, rng);
    }
    /// `CharacterCallbacks::MAP_ACCESSORY`, when the character has one.
    pub fn character_map_accessory(
        &mut self,
        assets: &assets::FighterAssets,
        map: &mut melee_mp::CollMap,
    ) {
        if let Some(accessory) = self.character.table().map_accessory {
            accessory(self, assets, map);
        }
    }
    pub fn item_muzzle(&mut self, assets: &assets::FighterAssets) -> Option<(Vec3, f32)> {
        (self.character.table().item_muzzle)(self, assets)
    }
    /// A destroyed article's Destroyed callback reaching this owner.
    pub fn article_destroyed(&mut self, kind: melee_types::ItemKind) {
        (self.character.table().article_destroyed)(self, kind)
    }
    /// An owned article's blast reaching this owner (its accessory).
    pub fn owner_blast(&mut self, blast: &melee_it::OwnerBlast, assets: &assets::FighterAssets) {
        (self.character.table().owner_blast)(self, blast, assets)
    }
    /// An owner-driven article's on_accessory (`ARTICLE_ACCESSORY`).
    pub fn article_accessory(
        &mut self,
        assets: &assets::FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Option<u16> {
        (self.character.table().article_accessory)(self, assets, map)
    }
    /// An owner-driven article's phys_cb (`ARTICLE_PHYSICS`).
    pub fn article_physics(
        &mut self,
        item: u32,
        assets: &assets::FighterAssets,
        map: &mut melee_mp::CollMap,
    ) {
        (self.character.table().article_physics)(self, item, assets, map)
    }
    /// `ARTICLE_CREATED`.
    pub fn article_created(&mut self, kind: melee_types::ItemKind, item: u32) {
        (self.character.table().article_created)(self, kind, item)
    }
    /// `ARTICLE_RETIRED`.
    pub fn article_retired(&mut self, kind: melee_types::ItemKind, item: u32) {
        (self.character.table().article_retired)(self, kind, item)
    }
    /// Queue a control of this fighter's article of `kind`, routed by
    /// `ARTICLE_CONTROL_TARGET`. A NULL instance pointer controls nothing;
    /// it never widens to every article of the kind.
    pub fn control_article(&mut self, kind: melee_types::ItemKind, control: melee_it::ItemControl) {
        use melee_it::{ArticleControlTarget, ItemRequest};
        let request = match (self.character.table().article_control_target)(&self.character, kind)
        {
            ArticleControlTarget::Owned => ItemRequest::Control {
                owner: self.player.id,
                kind,
                control,
            },
            ArticleControlTarget::Instance(Some(item)) => {
                ItemRequest::ControlInstance { item, control }
            }
            ArticleControlTarget::Instance(None) => return,
            ArticleControlTarget::PendingCreation => ItemRequest::ControlCurrent { kind, control },
        };
        self.core.item_requests.push(request);
    }
    /// The installed article post-hitlag callback's own work.
    pub fn article_hitlag_begin(&mut self) {
        (self.character.table().article_hitlag_begin)(self)
    }
    pub fn article_hitlag_end(&mut self) {
        (self.character.table().article_hitlag_end)(self)
    }
    /// An owned article's request from its proc (`ARTICLE_REQUEST`).
    pub fn article_request(
        &mut self,
        assets: &assets::FighterAssets,
        kind: melee_types::ItemKind,
        request: melee_it::OwnerRequest,
    ) -> Option<u8> {
        (self.character.table().article_request)(self, assets, kind, request)
    }
    pub fn item_owner(&mut self, assets: &assets::FighterAssets) -> melee_it::ItemOwner {
        (self.character.table().item_owner)(self, assets)
    }
    /// Install callback and scalar state together, including during savestate import.
    pub fn install_motion_row(&mut self, row: MotionRow) {
        self.core.motion_state = MotionState::new(row);
        self.motion_row = row;
    }
}

/// Shared fighter state and calculations, compiled independently of character.
/// Position and facing have one owner: `physics`.
#[derive(Clone)]
pub struct FighterCore {
    /// kind (+004), initialized by Fighter_UnkInitLoad_80068914.
    pub kind: FighterKind,
    /// player_id (+00C) and Player slot metadata (pl/player.c).
    pub player: PlayerSlot,
    /// x8_spawnNum (+008); allocated only by cold spawning, never import.
    pub spawn_number: u32,
    pub physics: FighterPhysics,
    pub animation: FighterAnimation,
    pub input: FighterInput,
    pub collision: EnvironmentCollision,
    /// co_attrs (+110); copied from ftData.x0.
    pub attributes: FighterAttributes,
    pub bones: FighterBones,
    /// GObj.hsd_obj: main skeleton; animation owns the secondary tree.
    pub skeleton: JObjTree,
    pub revival_platform: life::RevivalPlatform,
    pub revival_platform_active: bool,
    /// smash_attrs.x2135: the revival marker ftCo_800D4FF4 took from the
    /// player (Player_80032F30), `None` for -1 (Fighter_UnkInitLoad,
    /// fighter.c:804). A tracked marker skips Rebirth's collision, re-aims
    /// the platform every tick and, from the next revival on, the spawn's
    /// floor probe (fighter.c:530).
    pub revival_marker: Option<i8>,
    /// The player's other fighter (Popo's Nana, Nana's Popo) as the scene
    /// last showed it, before this proc; None for a player's only fighter.
    pub partner: Option<partner::PartnerView>,
    pub motion_state: MotionState,
    pub state_data: MotionData,
    /// Retail's motion scratch words at mv+0x14 and mv+0x18 as the states
    /// before this one left them (`scratch_tail`).
    pub scratch_tail_image: [u32; 2],
    pub combat: damage::CombatState,
    pub shield: shield::ShieldState,
    pub effect_state: effects::FighterEffects,
    pub effects: melee_ef::request::EffectQueue,
    /// Ordered item operations drained by the scene after each fighter phase.
    pub item_requests: melee_types::fixed::FixedVec<melee_it::ItemRequest, 64>,
    /// Synchronous effects a proc spawned after creating an item it
    /// requested (ftMr_SpecialN_ItemFireSpawn: the fireball, then its
    /// efSync flash): the scene dispatches them once those requests ran.
    pub effects_after_items: melee_types::fixed::FixedVec<melee_ef::request::EffectRequest, 8>,
    pub capabilities: Capabilities,
    pub cpu: CpuState,
    pub status: Status,
    pub commands: commands::CommandState,
    /// x221C_u16_y, three ground-IK enable bits.
    pub ground_pose: GroundPoseFlags,
    /// Owned DynamicsDesc chains from dynamic_bone_sets[].dyn_desc.
    pub dynamics: Vec<melee_lb::dynamics::DynamicBoneSet>,
    /// Fighter +2228 bit 1; use the fighter-height plane instead of mpCheckFloor.
    pub dynamics_use_floor_plane: bool,
    /// dynamic_bone_sets[].bone_id (+2F0, stride 0x18); 0x100 disables solving.
    pub dynamics_first_bone: Vec<u32>,
    /// lb_80011ABC as of the last dynamics-field pool tick (lb_800115F4),
    /// published by the scene.
    pub stage_wind: melee_lb::radial_force::WindState,
    /// Player_80032828 / Player_SetFacingDirectionConditional mirror.
    pub player_position: Vec3,
    pub player_facing: f32,
    /// Player_UpdateJoystickCountByIndex events.
    pub joystick_count: u64,
    /// dmg.x1930.x0: swept collision bounds sampled at s_link 0.
    pub previous_collision_bounds: Vec3,
    /// x890_cameraBox: target subject updated at s_link 18.
    pub camera: melee_cm::Subject,
    /// The render-time off-screen state and its magnifier damage.
    pub offscreen: Offscreen,
    /// Camera_RequestQuake from this tick's procs, for the scene to forward.
    pub quake_request: Option<melee_cm::QuakeKind>,
    /// BarrelWait's IASA asked the barrel to fire (Ground_801C4DD0), for
    /// the scene to forward to the stage.
    pub barrel_fire_request: bool,
    /// ftCo_800D34E0 ran in this proc: the scene hands the player's stock
    /// count and emptied stale table to the player's other fighter.
    pub fell: bool,
    /// With `fell`: the player ftCo_800D34E0 credits with the KO
    /// (Player_UpdateKOsBySlot), for the scene's KO counts.
    pub fall_credit: Option<u8>,
    /// x2224_b2: a Stamina fighter whose hit points ran out
    /// (fn_800C8E74), reset at a revival (fighter.c:483).
    pub stamina_out: bool,
    /// x2224_b3: ftCo_800C8C84 found the hit points gone; the next
    /// ProcessHit's ftCo_800C8D00 drops the fighter (`stamina`).
    pub stamina_ko_pending: bool,
    /// fn_800C8_inline ran in this proc: the scene's gm_80167470
    /// (gm_801B97C4 in a Stamina match) slows the game for this player.
    pub stamina_knocked_out: bool,
    /// The motion a death interrupted, which ftCo_800D331C (0x800D34B4)
    /// writes into the motion scratch at fp+236C before the Dead entry.
    pub fatal_action: ActionId,
    /// A grab link this fighter dropped by dying (ftCo_800DD100); the scene
    /// releases the partner.
    pub released_link: Option<grab::GrabLink>,
    /// item_gobj (+1974): the item in hand.
    pub held_item: Option<item_pickup::HeldItem>,
    /// A parts slot past the model that an article's model fills (Samus's
    /// grapple tip, fp->parts[0x8B]): its world translation as last posed.
    pub grafted_part: Option<GraftedPart>,
    /// mv.co.capturedamage.x18 is the grafted part (fn_800D9CE8's tether
    /// arms) rather than the hold bone, until CatchWait (fn_800DA1D8).
    pub holds_by_graft: bool,
    /// Link/Young Link u.lk.xC or Samus u.ss.x223C != NULL: a tether
    /// article is out, so a grab is refused (fn_800D8E94, fn_800D952C).
    pub tether_article: bool,
    /// The accessory4 of a transforming form's ended transformation motion
    /// asked the scene to swap in its partner (ftCommon_8007EFC8).
    pub transformation_requested: bool,
    /// x2221_b4..b7 and x2104: the parasol's open timer.
    pub parasol: parasol::ParasolTimer,
    /// A character forward smash chosen this IASA, entered by
    /// `Fighter::finish_input` with the RNG.
    pub pending_forward_smash: bool,
    /// fp->item_gobj while it holds one of the fighter's own articles.
    pub article_in_hand: Option<item_pickup::ArticleInHand>,
    /// An item still attached to the hand, frozen and hidden, while an
    /// article holds fp->item_gobj (Peach's u.pe.parasol_gobj_1).
    pub stowed_item: Option<item_pickup::HeldItem>,
    /// Fighter_OnItemPickup(gobj, true) owed to a stowed item brought back
    /// by a callback without the fighter's assets; the scene applies it
    /// before the proc's item requests.
    pub pickup_pose_pending: bool,
    /// Fighter_OnItemDrop(gobj, true) owed to a held item a callback without
    /// the fighter's assets destroyed (see `release_held_item_now`).
    pub drop_pose_pending: bool,
    /// The grabbable items the scene offered to the running proc.
    pub pickup_candidates: item_pickup::PickupCandidates,
    /// The article this fighter tracks, as the scene sampled it before the
    /// animation proc (see [`melee_it::ArticleReport`]).
    pub owned_article: Option<melee_it::ArticleReport>,
    /// ft_80082E3C's view of the other fighters on ledges, offered to Map.
    pub ledge_holders: ledge::LedgeHolders,
    /// The held fighter's position, offered to the accessory proc of a
    /// captor that follows its victim (Falcon Dive's accessory4).
    pub partner_position: Option<Vec3>,
    /// ftCo_800A4A40's fighter (its cur_pos), offered to the accessory proc
    /// while a tether article is out: the grapple beam's button code steers
    /// its tip at it.
    pub nearest_fighter: Option<Vec3>,
    /// accessory4_cb != NULL for a character's one-shot accessory (effects
    /// created after the deferred flush). The character keeps what it will
    /// do; every Fighter_ChangeMotionState disarms it (fighter.c:1377).
    pub accessory4_armed: bool,
    /// take_dmg_cb = ft_800CD31C while a swing runs; every
    /// Fighter_ChangeMotionState disarms it (fighter.c:1376-1389).
    pub swing_hand_armed: bool,
    /// deal_dmg_cb = ft_800CD65C and take_dmg_cb = ft_800CD604 while a
    /// Fan swing runs; every Fighter_ChangeMotionState disarms them.
    pub fan_callbacks_armed: bool,
    /// take_dmg_cb = ftCo_800CE620 while the Fire Flower fires: the flower
    /// closes (it_80292EAC). Every Fighter_ChangeMotionState disarms it.
    pub flower_close_armed: bool,
    /// take_dmg_cb = fn_800D7938 while the Super Scope charges: the charge
    /// is spent. Every Fighter_ChangeMotionState disarms it.
    pub scope_charge_armed: bool,
    /// take_dmg_cb = ftCo_800974C4 after the entry of HeavyGet, a lift
    /// state or a heavy throw; every motion change clears this common
    /// callback before entry commands run.
    pub heavy_item_damage_armed: bool,
    /// x2224_b1: an item was just dropped or caught in the air, so the
    /// aerial catch waits for the next grounded motion entry.
    pub item_catch_locked: bool,
    /// x209C: frames left in which an empty hand catches a light item in
    /// reach (armed by ftCo_800D705C, run by ftCo_800D71D8); every motion
    /// change closes it (fighter.c:1072).
    pub catch_window: u16,
    /// Set by the animation proc: ftCo_800D71D8 runs the catch window at
    /// the end of the state's next animation step (ftAnim_8006EBA4).
    pub catch_after_step: bool,
    /// hitlag_mul (+196C): the jab combo window. Every motion change except
    /// into Wait or Walk closes it (fighter.c:1142-1144).
    pub jab_countdown: f32,
    /// Fighter+x1A54: A edges counted by ftCo_Attack_800D6A50, surviving
    /// intervening motions until checkAttack11 starts a fresh jab.
    pub rapid_jab_edges: i32,
    /// unk_msid: the jab a combo continues from (Attack11 or Attack12).
    pub last_jab: Option<melee_types::CommonMotionState>,
    pub hurtboxes: Vec<melee_coll::hurtbox::HurtCapsule>,
    /// x221A_b6: a capsule was overwritten by ftColl_HurtboxInit; the next
    /// motion change restores the data table (ftColl_8007B4E0).
    pub hurtboxes_replaced: bool,
    pub dynamic_colliders: Vec<caches::DynamicCollider>,
    /// x1064_thrownHitbox: its pose advances even without a throw.
    pub thrown_hitbox: caches::ThrownHitbox,
    /// Player_GetHandicap; initialized by match setup or the saved player boundary.
    pub grab_handicap: u8,
    /// Player_80033BB8: this player's stock standing (0 leads; ties share
    /// a rank), refreshed by the scene before Fighter_ProcessHit.
    pub standing_rank: u8,
    /// A Coin match's rules (gm_8016B0B4); `None` in any other match.
    pub coin_rules: Option<coins::CoinRules>,
    /// x1614: the two coin pickup spheres.
    pub coin_spheres: [coins::CoinSphere; 2],
    /// The spills this proc's hits made, for the scene to create.
    pub coin_spills: melee_types::fixed::FixedVec<coins::CoinSpill, 2>,
    /// A fall's payout this proc asked for, for the scene.
    pub fall_payout: Option<coins::FallPayout>,
}

/// Retail inverse trig adapter used by HSD animation.
pub struct RetailTrig;
impl hsd_anim::mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}

/// Whether the fighter's camera bone left the screen, and the magnifier
/// damage it causes (fighter.c:1595-1610).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offscreen {
    /// x221F_b0: set by the main camera's render (ftLib_80086A8C) when the
    /// camera bone projects outside the scissor.
    pub outside_camera: bool,
    /// ifMagnify player `is_offscreen`: the magnifier bubble drew this
    /// fighter on the last display pass.
    pub magnified: bool,
    /// dmg.x1910: consecutive magnified ticks toward the next point of damage.
    pub magnified_ticks: i32,
    /// Camera_80031144() == 1.0, published by the camera each tick: only an
    /// unzoomed camera counts magnified ticks.
    pub camera_unzoomed: bool,
    /// Player_GetMoreFlagsBit3: match rules allow off-screen damage.
    pub damage_enabled: bool,
}

impl Default for Offscreen {
    fn default() -> Self {
        Self {
            outside_camera: false,
            magnified: false,
            magnified_ticks: 0,
            camera_unzoomed: true,
            damage_enabled: true,
        }
    }
}

/// State-local data; the retail union starts at Fighter +2340.
#[derive(Clone, Debug, Default)]
pub enum MotionData {
    Life(life::LifeState),
    /// The smash attacks: mv+4 as their predecessor left it, which they do
    /// not write (`None` where the port does not model it).
    Smash {
        retained_word: Option<f32>,
    },
    /// DownBound, DownWait and the states out of them (mv.co.downwait.x0).
    Down {
        wait_remaining: f32,
        /// mv+4: the predecessor's word with its top byte cleared by
        /// ftCo_80097D40 (`None` where the port does not model it).
        retained_word: Option<f32>,
        /// mv.co.downreflect.x4, that word's top byte: the wall of the last
        /// DownReflect (1 and 2 in retail), which cannot bounce the fighter
        /// again until a landing out of a tumble clears it.
        last_reflect: Option<fly_reflect::BounceSurface>,
    },
    /// Catch, CatchDash and the holding states. ftCo_800D8C54 writes only
    /// mv.co.catch.x0 (+2340), so mv+4 is the predecessor's word (`None`
    /// where the port does not model it).
    Catch {
        retained_word: Option<f32>,
    },
    /// mv.co.itemget: LightGet (false) or HeavyGet (true).
    ItemGet {
        heavy: bool,
        /// mv+4 as the predecessor left it (`None` where not modelled).
        retained_word: Option<f32>,
    },
    /// mv.co.throw.x4 is owned by combat.throw_flight, including its
    /// frozen flag retained through a following Wait.
    Throw,
    ItemThrow(item_throw::ItemThrowState),
    /// mv.co.lift: LiftWait, LiftWalk1/2 and LiftTurn.
    Lift(lift::LiftState),
    /// SwordSwing1..SwordSwingDash (mv.co.swing).
    Swing(item_swing::SwingState),
    /// mv.co.itemthrow as ftCo_800CE650 uses it: the Fire Flower's firing.
    FlowerShoot(item_flower::FlowerShootState),
    /// mv at fp+2340 while the Super Scope charges or fires.
    Scope(item_scope::ScopeState),
    Capture(grab_escape::CaptureState),
    /// Bury and BuryWait (mv.co.bury and the grab timer).
    Bury(bury::BuryState),
    /// CaptureLeadead (mv.co.captureleadead and the grab timer).
    CaptureReDead(capture_redead::CaptureReDeadState),
    /// BarrelWait: no scratch (the barrel holds the state).
    Barrel,
    /// mv.co.buryjump.x0: frames since the jump out of the ground; mv+4
    /// is still the buried collision box's top.
    BuryJump {
        frames: f32,
        retained_word: f32,
    },
    /// DamageIce (mv.co.damageice and the grab timer).
    Frozen(frozen::FrozenState),
    /// mv.co.damageicejump.escape_timer: frames of DamageIceJump left; mv+4
    /// is still the block's spin.
    FrozenJump {
        frames_left: f32,
        retained_word: f32,
    },
    /// The cargo carrier's rows with scratch of their own.
    Cargo(cargo::CargoState),
    /// ShoulderedWait..ShoulderedTurn: the carried fighter's grab timer.
    Shouldered(cargo::ShoulderedState),
    YoshiEgg(capture_yoshi::YoshiEggState),
    /// The Koopa Klaw's hold and throw rows (278..287).
    CaptureKoopa(capture_koopa::CaptureKoopaState),
    /// CaptureKirby and CaptureWaitKirby (288, 289): pulled into Kirby's
    /// mouth and held there.
    CaptureKirby(capture_kirby::CaptureKirbyState),
    /// ThrownKirbyStar, ThrownCopyStar and ThrownKirby (290..292).
    KirbyStar(capture_kirby::StarState),
    CaptureJump(grab_escape::CaptureJumpState),
    #[default]
    None,
    /// A state that writes no scratch of its own holds only the second word
    /// (mv+4) its predecessor left, where the port has no typed predecessor
    /// to keep: a savestate's fighter in Wait (fp+0x2344 imported), or a
    /// write from outside while such a state is current
    /// (`Fighter::overwrite_common_scratch_word`).
    Inherited(f32),
    Entry(entry::EntryState),
    Jab(attack::JabState),
    RapidJab(attack::RapidJabState),
    /// AttackLw3: mv.co.attacklw3.x0 (+2340) is the repeat latch; the state
    /// writes nothing else, so mv+4 is the predecessor's word (`None` where
    /// the port does not model it).
    DownTilt {
        repeat_pressed: bool,
        retained_word: Option<f32>,
    },
    /// AttackHi4, AttackLw4 and AttackHi3: mv+4 as their predecessor left
    /// it (`None` where the port does not model it).
    Tilt {
        retained_word: Option<f32>,
    },
    /// mv.co.attackdash.x0 (+2340): frames left in which holding shield
    /// cancels the dash attack into a dash grab.
    DashAttack {
        grab_window: i32,
        /// mv+4 as the predecessor left it (`None` where not modelled).
        retained_word: Option<f32>,
    },
    Aerial {
        retained_drop_timer: f32,
    },
    Damage(damage::DamageState),
    Rebound(clank::State),
    Guard(shield::GuardState),
    /// ShieldBreakFly through ShieldBreakStand: mv+4 as the guard state
    /// left it (`None` where not modelled).
    ShieldBreak {
        retained_word: Option<f32>,
    },
    Dizzy(shield_break::DizzyState),
    Escape(escape::EscapeState),
    EscapeAir(air_dodge::AirDodgeState),
    AirCatchHit(air_catch::AirCatchHitState),
    WallJump(wall_jump::State),
    Cliff(ledge::CliffState),
    CliffJump(ledge::CliffJumpState),
    Squat(squat::SquatState),
    Turn(turn::TurnState),
    Walk(walk::WalkState),
    Dash(dash::DashState),
    Run(run::RunState),
    RunBrake(run::RunBrakeState),
    TurnRun(turn_run::TurnRunState),
    KneeBend(jump::KneeBendState),
    Jump(jump::JumpState),
    MultiJump(multi_jump::MultiJumpState),
    /// mv.co.parasol_open (ItemParasolOpen and its special-fall sequel).
    Parasol(parasol::ParasolState),
    JumpAerial {
        retained_drop_timer: f32,
    },
    Pass {
        retained_drop_timer: f32,
    },
    Fall(fall::FallState),
    FallSpecial(fall::SpecialFallState),
    Landing {
        allow_interrupt: bool,
        retained_drop_timer: f32,
    },
}

/// What an article reads of an owner whose character holds no article of
/// its own state: no Blaster (`blaster_action` 9), nothing charged or fired.
pub fn default_item_owner(fighter: &Fighter) -> melee_it::ItemOwner {
    melee_it::ItemOwner {
        illusion: None,
        position: fighter.physics.position,
        facing: fighter.physics.facing,
        hold_position: fighter.physics.position,
        blaster_action: 9,
        remove_blaster: true,
        motion: fighter.motion_state.action.0,
        articles_fired: 0,
        charge: None,
        holds_needles: false,
        stick: hsd_types::Vec2::new(
            fighter.input.current.stick.x,
            fighter.input.current.stick.y,
        ),
        steering_article: false,
        detonating_article: false,
        motion_flags: fighter.motion_flags(),
        in_hitlag: fighter.core.in_hitlag(),
        anchor: fighter.physics.position,
        article_stage: None,
        model_scale: fighter.player.scale * fighter.attributes.size.model_scaling,
    }
}

/// Which of Fighter_ProcessHit's two kind calls a hit reaches
/// (`CharacterCallbacks::DAMAGE_DRAW`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageDraw {
    /// fighter.c:2860, a hit with knockback; `cape`: dmg.x1860_element is
    /// a cape's.
    Knockback { cape: bool },
    /// fighter.c:2960, damage without knockback, after it reached percent;
    /// `element` is dmg.x1860_element.
    Damage {
        element: melee_types::HitElement,
    },
}

pub type DamageDrawFn = fn(
    &mut Fighter,
    DamageDraw,
    &assets::FighterAssets,
    &mut gekko_math::HsdRng,
) -> assets::Result<()>;
