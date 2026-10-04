//! Fighter ownership and scheduler callbacks for grounded, item-free Wait.
//! Retail addresses and unsupported paths are documented at each entry point.
mod character;
pub use character::{Accessory2, CharacterState, CharacterTable};

pub mod absorb;
pub mod air_catch;
pub mod air_dodge;
pub mod assets;
pub mod attack;
pub mod caches;
pub mod cape_turn;
pub mod capture_captain;
pub mod bury;
pub mod capture_koopa;
pub mod capture_yoshi;
pub mod cargo;
pub mod clank;
pub mod color_overlay;
pub mod commands;
pub mod cpu;
pub mod damage;
mod damage_song;
pub mod dash;
pub mod down;
mod dynamic_commands;
pub mod effects;
pub mod entry;
pub mod escape;
pub mod fall;
pub mod fly_reflect;
pub mod grab;
pub mod grab_damage;
pub mod grab_escape;
pub mod grab_throw;
mod hit_log;
pub mod hitbox;
pub mod hitlag_link;
pub mod item_hits;
pub mod item_pickup;
pub mod item_swing;
pub mod item_throw;
pub mod jump;
pub mod ko_source;
pub mod landing;
pub mod ledge;
pub mod life;
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
pub mod shield;
mod shield_break;
mod sleep;
pub mod smash;
mod snapshot;
mod spawn;
pub mod squat;
mod stage_wind;
pub mod state;
pub mod stop_ceil;
pub mod stop_wall;
pub mod teeter;
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

pub trait CharacterCallbacks: Clone + Sized + Send + Sync + 'static {
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

    /// ftData_SpecialN/S/Hi/Lw[kind] (and the Air tables): a kind that has
    /// not ported its specials fails closed so the explorer names it,
    /// rather than ignoring the B press.
    fn enter_special(
        fighter: &mut Fighter,
        slot: SpecialSlot,
        airborne: bool,
        _assets: &assets::FighterAssets,
    ) {
        unimplemented!(
            "ftData_Special{slot:?}[{:?}] (airborne: {airborne}): character special entry",
            fighter.core.kind
        );
    }

    /// Fighter_UnkProcessGrab (8006CA5C) for a special armed by
    /// ftCommon_8007E2D0: the character's grab_cb on `captor`, then the
    /// grabbed_cb it installed on `victim` (a common capture entry).
    const SPECIAL_GRAB: SpecialGrab = character::unsupported_special_grab;

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

    fn kind(&self) -> FighterKind;
    /// decideFighter's character arm (ftCo_AttackS4.c:150-156), after the
    /// facing is set: Peach's ftPe_AttackS4_Enter. It draws from the RNG,
    /// so it runs once the IASA returns (`Fighter::finish_input`).
    const FORWARD_SMASH: Option<RngEntry> = None;
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
    /// ftCo_AttackS4.c:145-166, decideFighter (8008C348): nonstandard entry.
    fn forward_smash_variant(&self) -> ForwardSmashVariant {
        if Self::descriptor().common_behavior.forward_smash_entry {
            unimplemented!("ftCo_AttackS4: character entry hook");
        }
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
    /// ftCo_Throw.c:145-157,346-353: special capture and Fox laser callbacks.
    fn throw_variant(&self) {
        if Self::descriptor().common_behavior.throw_callback {
            unimplemented!("ftCo_Throw.c: character throw callback hook");
        }
    }

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
    /// Fighter_ProcessHit: deferred special defense reaction, before hitlag.
    const PROCESS_DEFENSE_HIT: Option<DefenseHit> = None;
    const ITEM_DEFENSE_CONTACT: Option<ItemDefenseContact> = None;

    /// ftCo_Attack1.c:89-110 and 202-213: decideAttack11, getMotionFlags
    /// (8008AB84 / 8008ABC0) and doAttack12's kind switch.
    fn jab_variant(&self) -> JabVariant {
        if Self::descriptor().common_behavior.jab_entry {
            unimplemented!("ftCo_Attack1.c:89-110: character jab entry hook");
        }
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
    /// ftCo_800DEA28 (0x800DEA28): taunt entry. Retail switches on kind:
    /// Young Link (ftCl_Init_80149318), Dr. Mario (ftDr_Init_80149910) and
    /// Ganondorf (lb_800119DC effect, then ftCo_800DEBD0 twice) have their
    /// own arms; every other kind takes ftCo_800DEBD0, which also calls
    /// ftKb_SpecialN_800F5D04 for Kirby (Peach/Zelda arms need DbLevel >= 3,
    /// never true in retail). Characters on the default arm bind
    /// `Fighter::enter_common_taunt`; the pl_80040120 taunt stat is not
    /// modelled. Unaudited kinds keep the panic.
    const ENTER_TAUNT: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()> =
        character::unsupported_taunt;
    fn on_load(&mut self, capabilities: &mut Capabilities);
    fn on_reset(&mut self);
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

    /// The second motion scratch word (mv+4) while `action`, one of this
    /// character's special rows, is current. Common states that leave that
    /// word untouched (JumpAerial, Landing) carry it on. `None` for rows
    /// whose word is not modelled. A table constant rather than a default
    /// method, so characters without such rows share one concrete default.
    const RETAINED_SCRATCH_WORD: fn(&CharacterState, ActionId) -> Option<f32> =
        character::no_retained_scratch_word;

    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:51-83.
    /// Character crates reset their airborne special resources here.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        if Self::descriptor().common_behavior.landing_reset {
            unimplemented!("ftCo_Landing.c:51-83: character landing reset hook");
        }
    }

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
    fn escape_variant(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        rolling: bool,
    ) -> assets::Result<()>
    where
        Self: Sized,
    {
        if rolling && Self::descriptor().common_behavior.morph_ball_roll {
            unimplemented!("ftCo_Escape.c:83-85: Samus morph-ball roll");
        }
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
    fn multi_jump_animation(&self, _jump: usize) -> i32 {
        unimplemented!("ftCo_JumpAerialF1.c: character multijump animation table")
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

/// Double-jump entry variants of ftCo_JumpAerial.c:103-119. Basic, Peach and
/// Yoshi are ported; the others retain explicit unsupported boundaries.
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
}

/// Unsupported interactions are represented explicitly, never inferred from
/// zero controller samples. These gates correspond to fighter.c's proc arms.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Interaction {
    #[default]
    Idle,
    Hitlag,
    HeldItem,
    Grab,
    Shield,
    Damage,
    Death,
    StatusEffect,
    Accessory,
    Attack,
    AsyncEffect,
    StageHazard,
    FighterOverlap,
    CoinMatch,
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
            interaction: Interaction::Idle,
            time_since_hit: -1,
            time_since_smash: -1.0,
            shield_health,
            ledge_cooldown: 0,
            ledge_timed_out: false,
            unconditional_top_exit: false,
            ledge_grab_disabled: false,
            on_ledge: false,
            grab_exclusions: ledge::GrabExclusions::NONE,
            special_grab: None,
            ledge_intangibility: 0,
            revival_invincibility: 0,
            sword_trail: -1,
            camera_shift: Vec2::ZERO,
            name_tag_timer: 0,
        }
    }
    fn require_supported(&self) {
        match self.interaction {
            Interaction::Idle
            | Interaction::Shield
            | Interaction::Hitlag
            | Interaction::Damage
            | Interaction::Attack => {}
            Interaction::HeldItem => unimplemented!("fighter.c:1523-1532: held-item lifetime"),
            Interaction::Grab => unimplemented!("fighter.c:1560-1578,2602-2626: capture/grab"),
            Interaction::Death => unimplemented!("fighter.c:914-929: death/entry states"),
            Interaction::StatusEffect => unimplemented!("fighter.c:1463-1641: status/item effects"),
            Interaction::Accessory => {
                unimplemented!("fighter.c:2533-2576: accessory callbacks/model")
            }
            Interaction::AsyncEffect => unimplemented!("fighter.c:2560: nonempty efAsync queue"),
            Interaction::StageHazard => {
                unimplemented!("fighter.c:2604,2631: stage collision lists")
            }
            Interaction::FighterOverlap => {
                unimplemented!("ftcommon.c:827-888: fighter overlap nudge")
            }
            Interaction::CoinMatch => {
                unimplemented!("ft_07C6.c:39: coin-match attachment collision")
            }
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
    /// The player's other fighter (Popo's Nana, Nana's Popo) as the scene
    /// last showed it, before this proc; None for a player's only fighter.
    pub partner: Option<partner::PartnerView>,
    pub motion_state: MotionState,
    pub state_data: MotionData,
    pub combat: damage::CombatState,
    pub shield: shield::ShieldState,
    pub effect_state: effects::FighterEffects,
    pub effects: melee_ef::request::EffectQueue,
    /// Ordered item operations drained by the scene after each fighter phase.
    pub item_requests: melee_types::fixed::FixedVec<melee_it::ItemRequest, 64>,
    /// Synchronous effects a proc spawned after creating an item it
    /// requested (ftMr_SpecialN_ItemFireSpawn: the fireball, then its
    /// efSync flash): the scene dispatches them once those requests ran.
    pub effects_after_items: melee_types::fixed::FixedVec<melee_ef::request::EffectRequest, 4>,
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
    /// ftCo_800D34E0 ran in this proc: the scene hands the player's stock
    /// count and emptied stale table to the player's other fighter.
    pub fell: bool,
    /// With `fell`: the player ftCo_800D34E0 credits with the KO
    /// (Player_UpdateKOsBySlot), for the scene's KO counts.
    pub fall_credit: Option<u8>,
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
    /// x2224_b1: an item was just dropped or caught in the air, so the
    /// aerial catch waits for the next grounded motion entry.
    pub item_catch_locked: bool,
    /// x209C: frames left in which an empty hand catches a light item in
    /// reach (armed by ftCo_800D705C, run by ftCo_800D71D8); every motion
    /// change closes it (fighter.c:1072).
    pub catch_window: u16,
    /// hitlag_mul (+196C): the jab combo window. Every motion change except
    /// into Wait or Walk closes it (fighter.c:1142-1144).
    pub jab_countdown: f32,
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
    ItemThrow(item_throw::ItemThrowState),
    /// SwordSwing1..SwordSwingDash (mv.co.swing).
    Swing(item_swing::SwingState),
    Capture(grab_escape::CaptureState),
    /// Bury and BuryWait (mv.co.bury and the grab timer).
    Bury(bury::BuryState),
    /// mv.co.buryjump.x0: frames since the jump out of the ground; mv+4
    /// is still the buried collision box's top.
    BuryJump {
        frames: f32,
        retained_word: f32,
    },
    /// The cargo carrier's rows with scratch of their own.
    Cargo(cargo::CargoState),
    /// ShoulderedWait..ShoulderedTurn: the carried fighter's grab timer.
    Shouldered(cargo::ShoulderedState),
    YoshiEgg(capture_yoshi::YoshiEggState),
    /// The Koopa Klaw's hold and throw rows (278..287).
    CaptureKoopa(capture_koopa::CaptureKoopaState),
    CaptureJump(grab_escape::CaptureJumpState),
    #[default]
    None,
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
