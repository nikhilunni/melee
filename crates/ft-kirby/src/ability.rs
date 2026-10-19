//! The copied ability as Kirby carries it: taking it (ftKb_SpecialN_800F1BAC,
//! ftkirby.c:4049) and losing it (ftKb_SpecialN_800F5D04,
//! ftkirbyspecialn.c:266) to a taunt, to a hit's draw, or to death.
use crate::init::Kirby;
use hsd_types::Vec3;
use melee_ft::fighter::{
    commands::{FootstepSound, SoundChannel},
    Fighter,
};
use melee_ef::request::EffectRequest;
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{FighterKind, ItemKind};

/// ft_PlaySFX ids of ftKb_SpecialN_800F1BAC: a new ability, and a swallow
/// that gives none.
const COPY_SOUND: u32 = 0x222F3;
const SWALLOW_SOUND: u32 = 0x222F6;
/// ft_PlaySFX id of ftKb_SpecialN_800F5D04: the ability leaves as a star.
const LOSS_SOUND: u32 = 0x22305;

fn play(f: &mut Fighter, id: u32) {
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ftKb_SpecialN_800F190C (800F190C): the copied specials' words at rest.
/// The ones with a reader so far are xC4 and xCC.
fn reset_copied_scratch(state: &mut Kirby) {
    state.ice_shot_lift_used = false;
    state.toad_lift_used = false;
}

/// ftKb_SpecialN_800F1BAC (800F1BAC) with `arg2` true: a new kind replaces
/// the copied one with its hat (ftKb_Init_803C9CC8[kind * 2], drawn only:
/// `crate::hat`) and its articles (ftKb_SpecialN_800F16D0, item
/// registrations), and either way death1_cb and death3_cb go in.
pub fn take(f: &mut Fighter, kind: FighterKind) {
    let state = f.character.get_mut::<Kirby>();
    let sound = if state.copied != kind {
        state.copied = kind;
        reset_copied_scratch(state);
        COPY_SOUND
    } else {
        SWALLOW_SOUND
    };
    state.ability_callbacks = crate::damage_callbacks::AbilityCallbacks {
        hit: true,
        death: true,
    };
    play(f, sound);
}

/// efLib_DestroyAll(gobj): Kirby's effects go.
fn destroy_effects(f: &mut Fighter) {
    f.effects.push(EffectRequest::DestroyOwned);
}

/// ftKb_SpecialNMt_8010709C's and ftKb_SpecialNMt_80107130's first half:
/// the Shadow Ball article (u.kb.x98, it_802C573C) and its charge effect
/// (u.kb.xA0). No copied special is ported, so neither is ever out; the
/// effects go regardless.
fn put_away_shadow_ball(f: &mut Fighter) {
    destroy_effects(f);
}

/// The copied Shadow Ball's charge (u.kb.x9C) is dropped: its flash
/// (ftCo_800BFFAC clears the secondary color slot) and effects go.
fn drop_shadow_ball_charge(f: &mut Fighter) {
    f.core.clear_secondary_color_slot();
    destroy_effects(f);
}

/// The arms of ftKb_SpecialN_800F19AC / 800F1A8C that are the same in
/// both: Donkey Kong's ftKb_SpecialNPr_80100DE0 (the swing count u.kb.xBC
/// goes, with Kirby's effects) and Samus's ftKb_SpecialNSs_800FCD60 (the
/// Charge Shot article u.kb.xA4 and its effects; u.kb.xAC, xA8). Returns
/// whether the kind has one.
fn put_away_charge(f: &mut Fighter, kind: FighterKind) -> bool {
    match kind {
        FighterKind::Donkey | FighterKind::Samus => {
            destroy_effects(f);
            true
        }
        _ => false,
    }
}

/// ftKb_SpecialN_800F19AC (800F19AC): the copied special is put away, from
/// the death callback and ftKb_SpecialN_800F5D04. The kinds whose arm only
/// releases an article or a held fighter (Ice Climbers, Peach, Fox, Falco,
/// Link, Young Link, Ness, Sheik, Yoshi, Mr. Game & Watch; the Yoshi arm's
/// hat animation is drawn) do nothing while no copied special is ported:
/// their scratch is always at rest.
fn put_away(f: &mut Fighter) {
    let kind = f.character.get::<Kirby>().copied;
    if put_away_charge(f, kind) {
        return;
    }
    if kind == FighterKind::Mewtwo {
        // ftKb_SpecialNMt_8010709C (8010709C).
        put_away_shadow_ball(f);
        drop_shadow_ball_charge(f);
    }
}

/// ftKb_SpecialN_800F1A8C (800F1A8C): the same from the take-damage
/// callback, except that a fully charged Shadow Ball keeps its charge
/// (ftKb_SpecialNMt_80107130: `(float) x9C == specialn_mt_charge_time`,
/// with x9C at rest 0).
pub fn put_away_on_hit(f: &mut Fighter) {
    let state = f.character.get::<Kirby>();
    let kind = state.copied;
    let full_charge = 0.0 == state.attributes.mewtwo_charge_time;
    if put_away_charge(f, kind) {
        return;
    }
    if kind == FighterKind::Mewtwo {
        put_away_shadow_ball(f);
        if !full_charge {
            drop_shadow_ball_charge(f);
        }
    }
}

/// ftKb_SpecialN_800F5D04 (800F5D04): the copied special is put away
/// (ftKb_SpecialN_800F19AC), its scratch reset (800F190C), the hat unloaded (800EEEC4: drawn only,
/// `crate::hat`), and with `star` the ability leaves as a star from +A4
/// above Kirby at (+A8 ahead, +AC up) with its sound (retail 0x800F5D90:
/// fmuls for the facing).
pub fn lose(f: &mut Fighter, star: bool) {
    put_away(f);
    let state = f.character.get_mut::<Kirby>();
    reset_copied_scratch(state);
    if state.copied != FighterKind::Kirby && star {
        let a = state.attributes.inhale.ability_star;
        let facing = f.physics.facing;
        let position = Vec3::new(f.physics.position.x, f.physics.position.y + a.height, 0.0);
        // it_802ADA1C: no parent on either side, the initial map pass on.
        let spawn = SpawnItem {
            owner: None,
            secondary_owner: None,
            velocity: Vec3::new(a.velocity.x * facing, a.velocity.y, 0.0),
            ..SpawnItem::ray(ItemKind::Unk1, f.core.player.id, position, facing)
        };
        f.core.item_requests.push(ItemRequest::Spawn(spawn));
        play(f, LOSS_SOUND);
    }
    f.character.get_mut::<Kirby>().copied = FighterKind::Kirby;
}
