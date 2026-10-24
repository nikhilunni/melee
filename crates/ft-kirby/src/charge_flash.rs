//! ftKb_Init_UnkMotionStates4 (800EE818, ftkirby.c:2603): the colour
//! animation Kirby's secondary colour slot takes whenever it empties,
//! by the copied kind, while that kind's kept charge is full.
//!
//! The port keeps the answer in `CombatState::secondary_color_fallback`;
//! every write of a charge word, and every change of the copied kind,
//! calls [`refresh`].
use crate::init::Kirby;
use melee_ft::fighter::Fighter;
use melee_types::FighterKind;

/// ftCo_800BFFD0's colours: Samus's full charge (54), Mewtwo's full
/// Shadow Ball (93).
const SAMUS_FULL: u8 = 54;
const MEWTWO_FULL: u8 = 93;

/// The colour ftKb_Init_UnkMotionStates4 would install now.
pub fn color(kirby: &Kirby) -> Option<u8> {
    let a = &kirby.attributes;
    match kirby.copied {
        // `xBC == specialn_dk_swings_to_full_charge` (colour 58): no copied
        // Giant Punch is ported, so xBC is 0 and never full (the attribute
        // is not 0 on the disc).
        FighterKind::Donkey => None,
        // `xA8 == specialn_ss_charge_time`, the integer converted.
        FighterKind::Samus => {
            (kirby.charge_shot.level as f32 == a.charge_shot.max_charge).then_some(SAMUS_FULL)
        }
        // `x9C == specialn_mt_charge_time`: no copied Shadow Ball is
        // ported, so x9C is 0.
        FighterKind::Mewtwo => (0.0 == a.mewtwo_charge_time).then_some(MEWTWO_FULL),
        // `xB4 == 6` (colour 87): no copied Needle Storm is ported, so
        // xB4 is 0.
        FighterKind::Seak => None,
        _ => None,
    }
}

/// Keeps `secondary_color_fallback` current after a charge word or the
/// copied kind changed.
pub fn refresh(f: &mut Fighter) {
    let color = color(f.character.get::<Kirby>());
    f.core.combat.secondary_color_fallback = color;
}
