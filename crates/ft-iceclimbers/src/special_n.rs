//! Ice Shot, ftpopospecialn.c (8011F2A4..8011F6B4), which both climbers
//! run: the states of `ft_iceclimbers_family::ice_shot` over rows 341/342.
use crate::{
    climber::{self, attributes, vars, Accessory},
    init::{Climber, IceClimber},
};
use ft_iceclimbers_family::ice_shot::{self, IceShotAttributes, IceShotUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::ItemKind;

/// ftPp_MS_SpecialN (341) and ftPp_MS_SpecialAirN (342).
pub const GROUND: ActionId = ActionId(341);
pub const AIR: ActionId = ActionId(342);

/// ft_800881D8's voice at the launch: Popo's and Nana's.
const LEADER_VOICE: u32 = 130_141;
const PARTNER_VOICE: u32 = 130_090;

impl IceShotUser for IceClimber {
    const GROUND: ActionId = GROUND;
    const AIR: ActionId = AIR;
    /// ftPp_SM_SpecialN (295).
    const GROUND_ANIMATION: i32 = 295;
    const ITEM: ItemKind = ItemKind::IceClimberIce;
    /// ftPopo_DatAttrs +0x04..+0x10.
    fn attributes(f: &Fighter) -> &IceShotAttributes {
        &attributes(f).ice_shot
    }
    fn held(f: &mut Fighter) -> &mut bool {
        &mut vars(f).ice
    }
    fn lift_used(f: &mut Fighter) -> &mut bool {
        &mut vars(f).air_ice_shot_used
    }
    fn drop(f: &mut Fighter) -> &mut f32 {
        &mut vars(f).ice_drop
    }
    /// accessory4_cb = ftPp_SpecialN_8011F500.
    fn install_accessory(f: &mut Fighter) {
        vars(f).accessory = Accessory::IceShot;
        f.core.arm_accessory4();
    }
    /// x222C, and death2_cb / take_dmg_cb = ftPp_Init_8011F060.
    fn hold(f: &mut Fighter) {
        let v = vars(f);
        v.ice = true;
        v.ice_callbacks = true;
    }
    /// ftPp_Init_8011F16C (8011F16C): x222C, death2_cb and take_dmg_cb
    /// cleared.
    fn let_go(f: &mut Fighter) {
        let v = vars(f);
        v.ice = false;
        v.ice_callbacks = false;
    }
    fn launch_voice(f: &Fighter) -> u32 {
        match climber::climber(f) {
            Climber::Popo => LEADER_VOICE,
            Climber::Nana => PARTNER_VOICE,
        }
    }
}

/// ftPp_Init_MotionStateTable rows 341 and 342.
pub const ROWS: [MotionRow; 2] = ice_shot::rows::<IceClimber>();

/// ftPp_SpecialN_Enter (8011F2A4) / ftPp_SpecialAirN_Enter (8011F318).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    ice_shot::enter::<IceClimber>(f, airborne, assets);
}

/// ftPp_Init_8011F190 (8011F190).
pub fn break_held_ice(f: &mut Fighter) {
    ice_shot::break_held_ice::<IceClimber>(f);
}

/// ftPp_SpecialN_8011F500 (8011F500).
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    ice_shot::accessory::<IceClimber>(f, assets);
}
