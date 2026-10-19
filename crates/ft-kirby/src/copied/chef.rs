//! The copied Chef, ftkirbyspecialgamewatch.c (8010CE5C..8010D740): Mr.
//! Game & Watch's states (`ft_gamewatch_family::chef`) over Kirby's rows
//! 510 and 511, with Kirby's food kind, attributes and scratch, and the pan
//! Kirby has to hold (Mr. Game & Watch's is part of his model).
//!
//! The entry's accessory (fn_8010CFB0) hangs the pan on the left thumb,
//! installs the damage callbacks if it came and the hitlag pair that freezes
//! it, then hands accessory4 to the food's thrower (fn_8010CE5C), which the
//! ground/air changes and the loop's restart install again.
use crate::init::{Accessory, Kirby};
use ft_gamewatch_family::chef::{self, Chef, ChefAttributes, ChefUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, ItemKind};

/// Kirby with Mr. Game & Watch's ability.
pub struct KirbyGameWatch;

/// ftKb_MS_GwSpecialN (510) and ftKb_MS_GwSpecialAirN (511).
pub const GROUND: ActionId = ActionId(510);
pub const AIR: ActionId = ActionId(511);

/// ftKb_MF_GwSpecialN_Coll (ftkirbyspecialgamewatch.c:27) and the loop's
/// restart (0x0C4C508C): the ground/air set with KeepColAnimHitStatus and
/// SkipHit. Unlike Mr. Game & Watch's, it does not keep the model.
const TRANSITION_FLAGS: u32 = 0x0C4C_508C;

/// The copied Chef's words: u.kb.xD4, xD8, xDC and mv.kb.specialn_gw.
#[derive(Clone, Copy, Debug, Default)]
pub struct KirbyChef {
    /// mv.kb.specialn_gw.
    pub chef: Chef,
    /// Fighter +2300 (u.kb.xD4): the last food thrown.
    pub last: i32,
    /// Fighter +2304 (u.kb.xD8): the food before it.
    pub previous: i32,
    /// Fighter +2308 (u.kb.xDC): the pan is out.
    pub pan_out: bool,
}

impl KirbyChef {
    /// ftKb_SpecialN_800F190C's words: xD4 = 1, xD8 = 3, xDC = NULL.
    pub const AT_REST: Self = Self {
        chef: Chef {
            loop_disabled: false,
            thrown: 0,
        },
        last: 1,
        previous: 3,
        pan_out: false,
    };
}

fn state(f: &mut Fighter) -> &mut KirbyChef {
    &mut f.character.get_mut::<Kirby>().chef
}

fn install(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<Kirby>().accessory = accessory;
    f.core.arm_accessory4();
}

impl ChefUser for KirbyGameWatch {
    const GROUND: ActionId = GROUND;
    const AIR: ActionId = AIR;
    /// ftKb_SM_GwSpecialN.
    const GROUND_ANIMATION: i32 = 445;
    const FOOD: ItemKind = ItemKind::KirbyGameWatchChef;
    const TRANSITION_FLAGS: u32 = TRANSITION_FLAGS;
    /// ftKb_DatAttrs +3E4 / +3E8.
    fn attributes(f: &Fighter) -> ChefAttributes {
        f.character.get::<Kirby>().attributes.gamewatch_chef
    }
    fn scratch(f: &mut Fighter) -> &mut Chef {
        &mut state(f).chef
    }
    fn foods(f: &mut Fighter) -> (&mut i32, &mut i32) {
        let s = state(f);
        (&mut s.last, &mut s.previous)
    }
    /// setGwVars: fn_8010CFB0, the pan.
    fn install_entry_accessory(f: &mut Fighter) {
        install(f, Accessory::ChefPan);
    }
    /// fn_8010CE5C.
    fn install_thrower(f: &mut Fighter) {
        install(f, Accessory::ChefFood);
    }
    fn uninstall(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().accessory = Accessory::None;
        f.core.accessory4_armed = false;
    }
}

/// ftKb_SpecialNGw_8010D188 / 8010D204 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    chef::enter::<KirbyGameWatch>(f, airborne, assets);
}

/// Rows 510 and 511: ftKb_GwSpecialN_* and ftKb_GwSpecialAirN_*.
pub const fn rows() -> [MotionRow; 2] {
    chef::rows::<KirbyGameWatch>()
}

/// fn_8010CE5C (8010CE5C): the food on the script's flag.
pub fn throw_accessory(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) {
    chef::create_sausage::<KirbyGameWatch>(f, assets, rng);
}

/// fn_8010CFB0 (8010CFB0): the pan on the left thumb (it_802C74D8 at the
/// joint's origin, lb_8000B1CC with no offset); with it, death2_cb and
/// take_dmg_cb (ftKb_Init_800EE74C / 800EE7B8). pre_hitlag_cb and
/// post_hitlag_cb (fn_8010D100 / ftKb_SpecialNGw_8010D130) freeze and thaw
/// it, and accessory4 becomes the thrower.
pub fn pan_accessory(f: &mut Fighter, assets: &FighterAssets) {
    let thumb = assets.parts.joint(FtPart::LThumbNb).expect("LThumbNb part");
    let position = crate::common::joint_position(f, usize::from(thumb));
    let c = &mut f.core;
    let spawn = SpawnItem::attached(
        ItemKind::KirbyGameWatchChefPan,
        c.player.id,
        position,
        c.physics.facing,
    );
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: thumb,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    let kirby = f.character.get_mut::<Kirby>();
    kirby.chef.pan_out = true;
    kirby.damage_callbacks = true;
    f.core.effect_state.article_hitlag = Some(ItemKind::KirbyGameWatchChefPan);
    install(f, Accessory::ChefFood);
}

/// itKirbyGameWatchChefPan_Logic113_Destroyed -> ftKb_SpecialNGw_8010D074
/// (8010D074): Kirby lets go of the pan (its thaw, 8010D130, reaches only
/// the pan going). The hitlag pair, which tests xDC, does nothing more.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyGameWatchChefPan {
        state(f).pan_out = false;
        if f.core.effect_state.article_hitlag == Some(kind) {
            f.core.effect_state.article_hitlag = None;
        }
    }
}

/// ftKb_SpecialNGw_8010D0A8 (8010D0A8), the Mr. Game & Watch arm of
/// 800F19AC and 800F1A8C: the pan, if out, is destroyed (it_802C760C: its
/// Destroyed callback, then Item_8026A8EC inside Kirby's proc).
pub fn put_away(f: &mut Fighter) {
    if !state(f).pan_out {
        return;
    }
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::KirbyGameWatchChefPan,
        control: ItemControl::Remove,
    });
    article_destroyed(f, ItemKind::KirbyGameWatchChefPan);
    // Item_8026A8EC's efLib_DestroyAll leaves the generator insertion
    // cursor on the list's tail (as ftGw_Init_OnDamage's removals).
    f.core
        .effects
        .push(melee_ef::request::EffectRequest::ArticleDestroyed);
}
