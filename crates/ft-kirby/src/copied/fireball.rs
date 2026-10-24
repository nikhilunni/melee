//! The copied Fireball and Megavitamin (ftkirbyspecialmario.c,
//! 800F9110..800F9614) and the copied green Fireball
//! (ftkirbyspecialluigi.c, 800F9614..800F99BC): Mario's states
//! (`ft_mario_family::special_n`) over Kirby's rows 399/400 (Mario's
//! ability) and 512/513 (Dr. Mario's), and Luigi's
//! (`ft_luigi_family::fireball`) over rows 431/432. Both throw from Kirby's
//! LHandN with his copy archives' articles and EfKbMr.dat / EfKbLg.dat's
//! flashes. No damage callbacks and no put-away arm (800F19AC / 800F1A8C
//! have no case for these kinds).
use crate::init::{Accessory, Kirby};
use ft_luigi_family::fireball::{self as green, GreenFireball};
use ft_mario_family::special_n::{self, Fireball};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::{FighterKind, FtPart, ItemKind};

/// Kirby with Mario's or Dr. Mario's ability: one marker, since
/// ftkirbyspecialmario.c's entries, ground/air changes and spawner pick the
/// rows and the article by the hat's kind themselves.
pub struct KirbyMario;
/// Kirby with Luigi's ability.
pub struct KirbyLuigi;

/// ftKb_MS_MrSpecialN / MrSpecialAirN (399, 400), ftKb_SM_MrSpecialN (338).
const MARIO_ROWS: [ActionId; 2] = [ActionId(399), ActionId(400)];
const MARIO_ANIMATION: i32 = 338;
/// ftKb_MS_DrSpecialN / DrSpecialAirN (512, 513), ftKb_SM_DrSpecialN (447).
const DR_MARIO_ROWS: [ActionId; 2] = [ActionId(512), ActionId(513)];
const DR_MARIO_ANIMATION: i32 = 447;

/// efSync_Spawn(0x49F, gobj, hand, &facing_dir): the fireball's flash
/// (model 0x7D00 of EfKbMr.dat, turned to the facing, then generator
/// 0x7D00).
const MARIO_FLASH: u16 = 0x49F;

fn kirby(f: &mut Fighter) -> &mut Kirby {
    f.character.get_mut::<Kirby>()
}

fn install(f: &mut Fighter, accessory: Accessory) {
    kirby(f).accessory = accessory;
    f.core.arm_accessory4();
}

impl Fireball for KirbyMario {
    /// ftKb_SpecialN_800F9110 / ftKb_SpecialNMr_800F93CC and
    /// ftKb_SpecialNLg_800F951C / 800F9598: Dr. Mario's rows for any hat
    /// but Mario's.
    fn rows(f: &Fighter) -> [ActionId; 2] {
        if f.character.get::<Kirby>().copied == FighterKind::Mario {
            MARIO_ROWS
        } else {
            DR_MARIO_ROWS
        }
    }
    /// fn_800F9260: ftParts_GetBoneIndex(fp, FtPart_LHandN) (800F92C0).
    const HAND: FtPart = FtPart::LHandN;
    /// accessory4_cb = fn_800F9260.
    fn install_accessory(f: &mut Fighter) {
        install(f, Accessory::Fireball);
    }
    /// fn_800F9260 (800F9280..8C): nothing without a copied kind, not even
    /// the throw flag's read.
    fn armed(f: &Fighter) -> bool {
        f.character.get::<Kirby>().copied != FighterKind::Kirby
    }
    /// fn_800F9260: Mario's hat throws It_Kind_Kirby_MarioFire
    /// (it_8029B6F8) with its flash; any other a Megavitamin
    /// (itDrMarioPill_Spawn, It_Kind_Kirby_DrMarioVitamin) of a colour drawn
    /// over u.kb.x68 / x6C (800F9330..A0), without one.
    fn throw(
        f: &mut Fighter,
        _assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
        hand: Vec3,
        bone: usize,
    ) {
        if kirby(f).copied == FighterKind::Mario {
            special_n::throw_fireball(f, ItemKind::KirbyMarioFire, MARIO_FLASH, hand, bone);
            return;
        }
        let state = kirby(f);
        let colour = ft_mario_family::draw_vitamin(
            &mut state.vitamin_current,
            &mut state.vitamin_previous,
            rng,
        );
        special_n::spawn_projectile(
            f,
            ItemKind::KirbyDrMarioVitamin,
            hand,
            it_drmariopill::thrown_argument(colour),
        );
    }
}

impl GreenFireball for KirbyLuigi {
    /// ftKb_MS_LgSpecialN / LgSpecialAirN (431, 432).
    const GROUND: ActionId = ActionId(431);
    const AIR: ActionId = ActionId(432);
    /// ftKb_SM_LgSpecialN.
    const GROUND_ANIMATION: i32 = 366;
    const ITEM: ItemKind = ItemKind::KirbyLuigiFire;
    /// efSync_Spawn(0x4B1, ...): model 0x9088 of EfKbLg.dat turned to the
    /// facing, then generator 0x9088.
    const FLASH: u16 = 0x4B1;
    /// fn_800F98F4: ftParts_GetBoneIndex(fp, FtPart_LHandN) (800F9944).
    const HAND: FtPart = FtPart::LHandN;
    /// accessory4_cb = fn_800F98F4.
    fn install_accessory(f: &mut Fighter) {
        install(f, Accessory::GreenFireball);
    }
}

/// ftKb_SpecialN_800F9110 / ftKb_SpecialNMr_800F93CC (Mario's and Dr.
/// Mario's entries).
pub fn enter_mario(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    special_n::enter::<KirbyMario>(f, airborne, assets);
}

/// ftKb_SpecialNLg_800F9614 / 800F9684 (Luigi's entries).
pub fn enter_luigi(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    green::enter::<KirbyLuigi>(f, airborne, assets);
}

/// fn_800F9260 (800F9260), from accessory4.
pub fn mario_accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    special_n::item_spawn::<KirbyMario>(f, assets, rng);
}

/// fn_800F98F4 (800F98F4), from accessory4.
pub fn luigi_accessory(f: &mut Fighter, assets: &FighterAssets) {
    green::spawn::<KirbyLuigi>(f, assets);
}

/// Rows 399, 400 (ftKb_MrSpecial*N_*), 512, 513 (the same callbacks) and
/// 431, 432 (ftKb_LgSpecial*N_*).
pub const fn rows() -> [MotionRow; 6] {
    let mario = special_n::rows::<KirbyMario>(MARIO_ROWS, MARIO_ANIMATION);
    let dr_mario = special_n::rows::<KirbyMario>(DR_MARIO_ROWS, DR_MARIO_ANIMATION);
    let luigi = green::rows::<KirbyLuigi>();
    [
        mario[0], mario[1], dr_mario[0], dr_mario[1], luigi[0], luigi[1],
    ]
}
