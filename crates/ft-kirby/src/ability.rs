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
/// The ones with a reader so far are x68, x6C, x74, x78, x7C (ns_flash_gobj), xC4,
/// xCC, xB0, x8C, xA4..xAC and xD4..xDC. With every charge word at 0,
/// ftKb_Init_UnkMotionStates4 has no colour for any kind
/// (`crate::charge_flash`).
fn reset_copied_scratch(f: &mut Fighter) {
    let state = f.character.get_mut::<Kirby>();
    state.ice_shot_lift_used = false;
    state.toad_lift_used = false;
    state.blaster = Default::default();
    state.rollout.scale = crate::copied::rollout::SCALE_AT_REST;
    state.chef = crate::copied::chef::KirbyChef::AT_REST;
    // x68, x6C: the copied Megavitamin's colours.
    state.vitamin_current = 9;
    state.vitamin_previous = 9;
    state.charge_shot = Default::default();
    state.giant_punch_swings = 0;
    state.shadow_ball_held = false;
    state.shadow_ball_charge = 0;
    // xD0.
    state.toad.toad_out = false;
    // x7C (Fighter +22A8): the copied PK Flash's pointer.
    state.pk_flash.flash_out = false;
    // x74, x78 (Fighter +22A0, +22A4): the copied bow's arrow and bow.
    state.bow.arrow_out = false;
    state.bow.bow_out = false;
    crate::charge_flash::refresh(f);
}

/// ftKb_SpecialN_800F1BAC (800F1BAC) with `arg2` true: a new kind replaces
/// the copied one with its hat (ftKb_Init_803C9CC8[kind * 2], drawn only:
/// `crate::hat`) and its articles (ftKb_SpecialN_800F16D0, item
/// registrations), and either way death1_cb and death3_cb go in.
pub fn take(f: &mut Fighter, kind: FighterKind) {
    let state = f.character.get_mut::<Kirby>();
    let sound = if state.copied != kind {
        state.copied = kind;
        reset_copied_scratch(f);
        // ftKb_Init_803C9CC8[kind * 2]: the hat's loader.
        crate::hat::load(f, kind);
        COPY_SOUND
    } else {
        SWALLOW_SOUND
    };
    let state = f.character.get_mut::<Kirby>();
    state.ability_callbacks = crate::damage_callbacks::AbilityCallbacks {
        hit: true,
        death: true,
    };
    play(f, sound);
}

/// The arms of ftKb_SpecialN_800F19AC / 800F1A8C that are the same in
/// both: Donkey Kong's ftKb_SpecialNPr_80100DE0 (the swing count u.kb.xBC
/// goes, with Kirby's effects) and Samus's ftKb_SpecialNSs_800FCD60 (the
/// Charge Shot article u.kb.xA4 and its effects; u.kb.xAC, xA8). Returns
/// whether the kind has one.
fn put_away_charge(f: &mut Fighter, kind: FighterKind) -> bool {
    match kind {
        FighterKind::Donkey => {
            crate::copied::donkey::put_away(f);
            true
        }
        FighterKind::Samus => {
            crate::copied::samus::put_away(f);
            true
        }
        _ => false,
    }
}

/// The arms of ftKb_SpecialN_800F19AC / 800F1A8C that release an article:
/// Fox's and Falco's destroy the blaster, Mr. Game & Watch's the pan,
/// Peach's Toad; Ness's lets the flash fly on, Link's and Young Link's
/// take the arrow and the bow.
fn put_away_article(f: &mut Fighter, kind: FighterKind) {
    crate::copied::blaster::put_away(f, kind);
    match kind {
        // ftKb_SpecialNGw_8010D0A8.
        FighterKind::GameWatch => crate::copied::chef::put_away(f),
        // ftKb_SpecialNPe_8010C3F4.
        FighterKind::Peach => crate::copied::toad::put_away(f),
        // ftKb_SpecialNNs_800FECE8.
        FighterKind::Ness => crate::copied::pk_flash::put_away(f),
        // ftKb_SpecialNLk800FB800, 800FB840.
        FighterKind::Link | FighterKind::CLink => crate::copied::bow::put_away(f, kind),
        _ => {}
    }
}

/// ftKb_SpecialN_800F19AC (800F19AC): the copied special is put away, from
/// the death callback and ftKb_SpecialN_800F5D04 (`put_away_article`).
/// The other kinds whose arm only releases an article or a held fighter
/// (Ice Climbers, Sheik) do nothing while their copied special is not
/// ported: their scratch is always at rest. Yoshi's
/// (801093B4: the hat's animation, drawn only; 8010A8BC: an item on the
/// tongue, which the port never catches) has nothing to do.
fn put_away(f: &mut Fighter) {
    let kind = f.character.get::<Kirby>().copied;
    put_away_article(f, kind);
    if put_away_charge(f, kind) {
        return;
    }
    if kind == FighterKind::Mewtwo {
        crate::copied::mewtwo::put_away(f);
    }
}

/// ftKb_SpecialN_800F1A8C (800F1A8C): the same from the take-damage
/// callback, except that a fully charged Shadow Ball keeps its charge
/// (ftKb_SpecialNMt_80107130).
pub fn put_away_on_hit(f: &mut Fighter) {
    let kind = f.character.get::<Kirby>().copied;
    put_away_article(f, kind);
    if put_away_charge(f, kind) {
        return;
    }
    if kind == FighterKind::Mewtwo {
        crate::copied::mewtwo::put_away_on_hit(f);
    }
}

/// ftKb_SpecialN_800F5D04 (800F5D04): the copied special is put away
/// (ftKb_SpecialN_800F19AC), its scratch reset (800F190C), the hat unloaded (800EEEC4: drawn only,
/// `crate::hat`), and with `star` the ability leaves as a star from +A4
/// above Kirby at (+A8 ahead, +AC up) with its sound (retail 0x800F5D90:
/// fmuls for the facing).
pub fn lose(f: &mut Fighter, star: bool) {
    put_away(f);
    reset_copied_scratch(f);
    let state = f.character.get_mut::<Kirby>();
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
    // ftKb_SpecialN_800EEEC4: the hat's unloader.
    let copied = f.character.get::<Kirby>().copied;
    crate::hat::unload(f, copied);
    f.character.get_mut::<Kirby>().copied = FighterKind::Kirby;
    crate::charge_flash::refresh(f);
}
