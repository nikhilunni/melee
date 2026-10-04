//! The aerials with an article (ftgamewatchattackair.c): neutral
//! (parachute), back (turtle) and up (Spitball Sparky). Their landings
//! enter the character's own landing rows at the full attribute lag (no
//! L-cancel: ftCo_LandingAir_EnterWithMsidLag is called directly), the up
//! aerial with the back aerial's lag, and keep the article until the row
//! ends.
use crate::articles::{self, Accessory, Aerial};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        attack::aerial,
        state::{self, callbacks, AnimationPhase, CollisionPhase},
        ActionId, Fighter, MotionRow,
    },
};
use melee_types::CommonMotionState as S;

pub const ATTACK_AIR_N: ActionId = ActionId(347);
pub const ATTACK_AIR_B: ActionId = ActionId(348);
pub const ATTACK_AIR_HI: ActionId = ActionId(349);
pub const LANDING_AIR_N: ActionId = ActionId(350);
pub const LANDING_AIR_B: ActionId = ActionId(351);
pub const LANDING_AIR_HI: ActionId = ActionId(352);

/// The motion an article takes on landing (it_gamewatch::attack_air::LANDED).
pub const ARTICLE_LANDED: u16 = 1;

impl Aerial {
    fn attack(self) -> ActionId {
        match self {
            Self::Neutral => ATTACK_AIR_N,
            Self::Back => ATTACK_AIR_B,
            Self::Up => ATTACK_AIR_HI,
        }
    }
    fn landing(self) -> ActionId {
        match self {
            Self::Neutral => LANDING_AIR_N,
            Self::Back => LANDING_AIR_B,
            Self::Up => LANDING_AIR_HI,
        }
    }
    /// ftGw_LandingAirN_Init (8014BAF8), ftGw_LandingAirB_Init (8014BBE0)
    /// and ftGw_LandingAirHi_Init (8014BCC8), which reads landingairb_lag.
    fn landing_lag(self, f: &Fighter) -> f32 {
        let landing = &f.attributes.landing;
        match self {
            Self::Neutral => landing.landingairn_lag,
            Self::Back | Self::Up => landing.landingairb_lag,
        }
    }
    fn of(action: ActionId) -> Self {
        *Self::ALL
            .iter()
            .find(|a| a.attack() == action || a.landing() == action)
            .expect("an aerial row with an article")
    }
}

pub const fn rows() -> [MotionRow; 6] {
    const fn attack(action: ActionId, common: S) -> MotionRow {
        MotionRow {
            action,
            collision: attack_collision,
            ..state::COMMON[common as usize]
        }
    }
    const fn landing(action: ActionId, common: S) -> MotionRow {
        MotionRow {
            action,
            anim: landing_animation,
            collision: landing_collision,
            ..state::COMMON[common as usize]
        }
    }
    [
        attack(ATTACK_AIR_N, S::AttackAirN),
        attack(ATTACK_AIR_B, S::AttackAirB),
        attack(ATTACK_AIR_HI, S::AttackAirHi),
        landing(LANDING_AIR_N, S::LandingAirN),
        landing(LANDING_AIR_B, S::LandingAirB),
        landing(LANDING_AIR_HI, S::LandingAirHi),
    ]
}

/// ftGw_AttackAirN_DecideAction (8014B64C): the neutral, back and up
/// aerials enter the character's rows with their article's accessory; the
/// forward and down aerials are the common ones.
pub fn enter(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = aerial::select(&f.core.input, &assets.input, f.core.physics.facing);
    let article = match state {
        S::AttackAirN => Aerial::Neutral,
        S::AttackAirB => Aerial::Back,
        S::AttackAirHi => Aerial::Up,
        _ => return aerial::enter_action(f, assets, state.into()),
    };
    // ftGw_AttackAirN_Enter (8014B6E4) and its siblings.
    aerial::enter_action(f, assets, article.attack())?;
    articles::install(f, Accessory::AerialSetup(article));
    Ok(())
}

/// ftGw_AttackAirN_Coll (8014B780): ft_80082C74 with the aerial's own
/// landing.
fn attack_collision(f: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    aerial::collision_with(f, phase, land)
}

/// ftGw_LandingAirN_Enter (8014B7A8), ftGw_LandingAirB_Enter (8014B904),
/// ftGw_LandingAirHi_Enter (8014BA54): the aerial articles thaw; while the
/// script's landing-lag flag is up the character's landing row at its full
/// lag, with the article's landed accessory; otherwise the plain landing,
/// and every article goes.
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let aerial = Aerial::of(f.motion_state.action);
    articles::thaw_aerial_articles(f);
    if f.commands.variables[0] != 0 {
        let lag = aerial.landing_lag(f);
        f.enter_aerial_landing(aerial.landing(), lag, assets)?;
        articles::install(f, Accessory::AerialLanded(aerial));
        return Ok(());
    }
    f.enter_landing(assets)?;
    articles::on_damage(f);
    Ok(())
}

/// ftGw_LandingAirN_Anim (8014BB24) and siblings: ftCo_Landing_Anim, then
/// the articles go once the row is left.
fn landing_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let row = f.motion_state.action;
    let choice = callbacks::animation::landing(f, p)?;
    if f.motion_state.action != row {
        articles::on_damage(f);
    }
    Ok(choice)
}

/// ftGw_LandingAirN_Coll (8014BB94) and siblings: ftCo_Landing_Coll, then
/// the articles go once the row is left.
fn landing_collision(f: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let row = f.motion_state.action;
    callbacks::collision::ground_wait(f, phase)?;
    if f.motion_state.action != row {
        articles::on_damage(f);
    }
    Ok(())
}
