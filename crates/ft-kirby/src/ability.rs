//! The copied ability as Kirby carries it: taking it (ftKb_SpecialN_800F1BAC,
//! ftkirby.c:4049) and losing it (ftKb_SpecialN_800F5D04,
//! ftkirbyspecialn.c:266) to a taunt, to a hit's draw, or to death.
use crate::init::Kirby;
use hsd_types::Vec3;
use melee_ft::fighter::{
    commands::{FootstepSound, SoundChannel},
    Fighter,
};
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
/// the copied one, and either way the take-damage and death callbacks go
/// in. The hat itself (ftKb_Init_803C9CC8[kind * 2]) is `crate::hat`.
pub fn take(f: &mut Fighter, kind: FighterKind) {
    let state = f.character.get_mut::<Kirby>();
    let sound = if state.copied != kind {
        crate::hat::require_supported(kind);
        state.copied = kind;
        reset_copied_scratch(state);
        COPY_SOUND
    } else {
        SWALLOW_SOUND
    };
    state.damage_callbacks = true;
    play(f, sound);
}

/// ftKb_SpecialN_800F5D04 (800F5D04): the copied special is put away
/// (ftKb_SpecialN_800F19AC: nothing while no copied special is ported),
/// its scratch reset (800F190C), the hat unloaded (800EEEC4: drawn only,
/// `crate::hat`), and with `star` the ability leaves as a star from +A4
/// above Kirby at (+A8 ahead, +AC up) with its sound (retail 0x800F5D90:
/// fmuls for the facing).
pub fn lose(f: &mut Fighter, star: bool) {
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
