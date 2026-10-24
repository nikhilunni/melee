//! The copied Nayru's Love, ftkirbyspecialzelda.c (80105A34..80105FEC):
//! Zelda's states (`ft_zelda_family::nayrus_love`) over Kirby's rows
//! 465/466, with his attributes (+2A4..+2B0, the ReflectDesc at +400) and
//! EfKbZd.dat's crystals (efAlt 0x4B6 / 0x4B7). No article, no damage
//! callbacks and no put-away arm (800F19AC / 800F1A8C have no case for
//! Zelda).
use crate::init::{Accessory, Kirby};
use ft_zelda_family::nayrus_love::{self, NayrusLove, NayrusLoveAttributes, NayrusLoveUser};
use melee_ft::{
    desc::fox_attributes::ReflectionAttributes,
    fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow},
};

/// Kirby with Zelda's ability.
pub struct KirbyZelda;

impl NayrusLoveUser for KirbyZelda {
    /// ftKb_MS_ZdSpecialN / ZdSpecialAirN (465, 466).
    const GROUND: ActionId = ActionId(465);
    const AIR: ActionId = ActionId(466);
    /// ftKb_SM_ZdSpecialN (400).
    const GROUND_ANIMATION: i32 = 400;
    /// fn_80105A34 / fn_80105AB0: efSync_Spawn(0x4B6 / 0x4B7, gobj,
    /// parts[1].joint), efAlt's efLib_Create_AttachChild 0x5208 / 0x5209.
    const CRYSTALS: [u16; 2] = [0x4B6, 0x4B7];
    /// ftKb_DatAttrs +2A4 (`specialn_zd_*`).
    fn attributes(f: &Fighter) -> &NayrusLoveAttributes {
        &f.character.get::<Kirby>().attributes.nayrus_love
    }
    /// ftKb_DatAttrs +400 (`specialn_zd_reflectdesc`).
    fn reflection(f: &Fighter) -> &ReflectionAttributes {
        &f.character.get::<Kirby>().attributes.nayrus_love_reflection
    }
    fn scratch(f: &mut Fighter) -> &mut NayrusLove {
        &mut f.character.get_mut::<Kirby>().nayrus_love
    }
    fn scratch_ref(f: &Fighter) -> &NayrusLove {
        &f.character.get::<Kirby>().nayrus_love
    }
    /// accessory4_cb = fn_80105A34 / fn_80105AB0.
    fn install_accessory(f: &mut Fighter, air: bool) {
        f.character.get_mut::<Kirby>().accessory = if air {
            Accessory::NayrusLoveAirCrystal
        } else {
            Accessory::NayrusLoveCrystal
        };
        f.core.arm_accessory4();
    }
    fn uninstall_accessory(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().accessory = Accessory::None;
    }
    /// ftKb_SpecialNSk_80105E8C / 80105F3C: with the crystal out
    /// (x2219_b0), Fighter_SetEffectHitlagCallbacks again; the spawner is
    /// not installed again, so a change before the crystal leaves none.
    fn after_ground_air_change(f: &mut Fighter, _air: bool) {
        if f.effect_state.destroy_on_state_change {
            f.effect_state.hitlag_callbacks = true;
        }
    }
}

/// ftKb_SpecialNZd_80105B2C / 80105BA8 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    nayrus_love::enter::<KirbyZelda>(f, airborne, assets);
}

/// fn_80105A34 / fn_80105AB0, from accessory4.
pub fn crystal(f: &mut Fighter, air: bool) {
    nayrus_love::crystal::<KirbyZelda>(f, air);
}

/// Rows 465, 466 (ftKb_ZdSpecial*N_*).
pub const fn rows() -> [MotionRow; 2] {
    nayrus_love::rows::<KirbyZelda>()
}
