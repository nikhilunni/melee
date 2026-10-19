//! The copied Blaster, ftkirbyspecialfox.c: Fox's and Falco's states
//! (`ft_fox_family::special_n`) over Kirby's rows 423..428 and 520..525,
//! with Kirby's articles, hand and callbacks.
use crate::init::Kirby;
use ft_fox_family::{
    special_n::{self, Blaster, BlasterRows, HoldJoint, HoldPoints},
    SpecialNeutral,
};
use melee_ft::{
    desc::fox_attributes::BlasterAttributes,
    fighter::{assets::FighterAssets, Fighter, MotionRow},
};
use melee_types::{FighterKind, ItemKind};

/// Kirby with Fox's ability.
pub struct KirbyFox;
/// Kirby with Falco's ability.
pub struct KirbyFalco;

/// ftKb_SpecialNFx_800FDC00 / 800FDC70 / 800FDF30: lb_8000B1CC on
/// `fp->parts[44]` (FtPart_R3rdNa's value used as a bone index; PlCo's
/// table maps Kirby's bone 44 to his RThumbNb).
const HOLD: HoldPoints = HoldPoints {
    joint: HoldJoint::Bone(44),
    y: 1.45,
    muzzle_z: 5.016,
    grip_z: 0.016,
};

impl Blaster for KirbyFox {
    /// ftKb_MS_FxSpecialNStart (423), ftKb_SM_FxSpecialNStart (358).
    const ROWS: BlasterRows = BlasterRows {
        first_action: 423,
        first_animation: 358,
    };
    const LASER: ItemKind = ItemKind::KirbyFoxLaser;
    const BLASTER: ItemKind = ItemKind::KirbyFoxBlaster;
    /// ftKb_Init_804D3DB8.
    const FIRE_SOUNDS: [u32; 2] = [0x1AE17, 0x1AE1A];
    const HOLD: HoldPoints = HOLD;
    /// ftKb_DatAttrs +220 (`specialn_fx_*`).
    fn attributes(f: &Fighter) -> BlasterAttributes {
        f.character.get::<Kirby>().attributes.fox_blaster
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialNeutral {
        &mut f.character.get_mut::<Kirby>().blaster
    }
    /// ftKb_SpecialN_set_cbs.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
}

impl Blaster for KirbyFalco {
    /// ftKb_MS_FcSpecialNStart (520), ftKb_SM_FcSpecialNStart (455).
    const ROWS: BlasterRows = BlasterRows {
        first_action: 520,
        first_animation: 455,
    };
    const LASER: ItemKind = ItemKind::KirbyFalcoLaser;
    const BLASTER: ItemKind = ItemKind::KirbyFalcoBlaster;
    /// ftKb_Init_804D3DC0.
    const FIRE_SOUNDS: [u32; 2] = [0x18703, 0x18706];
    const HOLD: HoldPoints = HOLD;
    /// ftKb_DatAttrs +244 (`specialn_fc_*`).
    fn attributes(f: &Fighter) -> BlasterAttributes {
        f.character.get::<Kirby>().attributes.falco_blaster
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialNeutral {
        &mut f.character.get_mut::<Kirby>().blaster
    }
    /// ftKb_SpecialN_set_cbs.
    fn arm_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
}

/// ftKb_SpecialNFx_800FE100 / 800FE240 (the grounded and aerial entries).
pub fn enter<C: Blaster>(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    special_n::enter::<C>(f, airborne, assets);
}

/// Rows 423..428 (ftKb_MS_FxSpecialN*) and 520..525 (ftKb_MS_FcSpecialN*):
/// the same callbacks, ftKb_FxSpecialN*.
pub const fn rows() -> [MotionRow; 12] {
    let fox = special_n::rows::<KirbyFox>();
    let falco = special_n::rows::<KirbyFalco>();
    [
        fox[0], fox[1], fox[2], fox[3], fox[4], fox[5], falco[0], falco[1], falco[2], falco[3],
        falco[4], falco[5],
    ]
}

/// accessory4_cb = ftKb_SpecialNFx_CreateBlasterShot, from the Loop rows.
pub fn accessory(f: &mut Fighter, assets: &FighterAssets) {
    match f.character.get::<Kirby>().copied {
        FighterKind::Fox => special_n::accessory::<KirbyFox>(f, assets),
        FighterKind::Falco => special_n::accessory::<KirbyFalco>(f, assets),
        _ => {}
    }
}

/// ftKb_SpecialNFx_800FDD4C and 800FDD14 switch on the hat's kind; any
/// other kind reads as no blaster.
pub fn item_owner(f: &mut Fighter) -> Option<melee_it::ItemOwner> {
    match f.character.get::<Kirby>().copied {
        FighterKind::Fox => Some(special_n::item_owner::<KirbyFox>(f)),
        FighterKind::Falco => Some(special_n::item_owner::<KirbyFalco>(f)),
        _ => None,
    }
}

/// ftKb_SpecialNFx_800FDC00 (muzzle) and 800FDC70 (grip) for it_802ADF10.
pub fn item_muzzle(f: &mut Fighter, assets: &FighterAssets) -> Option<(hsd_types::Vec3, f32)> {
    match f.character.get::<Kirby>().copied {
        FighterKind::Fox => Some(special_n::item_muzzle::<KirbyFox>(&mut f.core, assets)),
        FighterKind::Falco => Some(special_n::item_muzzle::<KirbyFalco>(&mut f.core, assets)),
        _ => None,
    }
}

/// ftKb_SpecialNFx_800FDEE0, the Fox and Falco arm of 800F19AC and
/// 800F1A8C: the blaster, if out, is destroyed.
pub fn put_away(f: &mut Fighter, kind: FighterKind) {
    match kind {
        FighterKind::Fox => special_n::destroy_blaster::<KirbyFox>(f),
        FighterKind::Falco => special_n::destroy_blaster::<KirbyFalco>(f),
        _ => {}
    }
}
