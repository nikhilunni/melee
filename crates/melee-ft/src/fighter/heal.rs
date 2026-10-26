//! Healing: eating a Heart Container, Maxim Tomato or Food
//! (ftpickupitem_8009447C / ftpickupitem_80094B6C) adds its amount to
//! dmg.x18F0 (Fighter_8006CF5C), which takes a point of damage off each
//! frame (Fighter_8006A360, fighter.c:1612-1631) under the healing colour.
use super::assets::FighterAssets;
use super::commands::{FootstepSound, SoundChannel};
use super::item_pickup::HeldItem;
use super::{Fighter, FighterCore};
use melee_types::ItemKind;

/// ftCo_800BFFD0(fp, 8, 0): the healing colour animation.
pub(super) const HEALING_COLOR: u8 = 8;
/// ft_80088640(fp, 0x7D, 127, 64): the sound of each point healed.
const HEAL_TICK_SOUND: u32 = 0x7D;
/// it_8026B30C: the use kind of what the pickup consumes.
pub(super) const CONSUMED: u8 = 5;

impl FighterCore {
    /// Fighter_8006CF5C (8006CF5C): unless its Stamina hit points ran out
    /// (x2224_b2), `amount` more points to heal, under the healing colour.
    /// ftCommon_8007EBAC(fp, 2, 0) is the controller's rumble.
    pub fn heal(&mut self, amount: i32, assets: &FighterAssets) {
        if self.stamina_out {
            return;
        }
        self.status.heal_remaining += amount;
        self.install_color_overlay_now(HEALING_COLOR, assets);
    }

    /// Fighter_8006A360, fighter.c:1612-1631, during hitlag too: a point
    /// healed a frame (the percent falls by 1.0 with a sound) until none is
    /// left or the percent is 0; then the healing colour goes if it still
    /// owns its slot (ftCo_800C0694, ftCo_800C0200). Player_SetHPByIndex
    /// and pl_80040B8C only mirror the percent and count statistics;
    /// ftCommon_8007ECD4 stops the rumble.
    pub(super) fn tick_heal(&mut self, assets: &FighterAssets) {
        if self.status.heal_remaining == 0 {
            return;
        }
        self.status.heal_remaining -= 1;
        if self.physics.percent > 0.0 {
            self.physics.percent -= 1.0;
            self.commands.footstep_sounds.push(FootstepSound {
                channel: SoundChannel::Auxiliary,
                id: HEAL_TICK_SOUND,
                volume: 127,
                pan: 64,
            });
        }
        if self.physics.percent <= 0.0 {
            self.physics.percent = 0.0;
            self.status.heal_remaining = 0;
        }
        if self.status.heal_remaining == 0 && self.combat.color_overlay.id == HEALING_COLOR {
            self.release_charge_color(HEALING_COLOR, assets);
        }
    }

    /// ftpickupitem_80094B6C (80094B6C) and ftpickupitem_8009447C's
    /// consuming half: a use-kind-5 item in hand is eaten (Fighter_8006CF5C
    /// with it_8026B47C, then Item_8026A8EC, which empties the hand through
    /// ftLib_80086724). True when it was.
    pub(super) fn eat(&mut self, held: HeldItem, assets: &FighterAssets) -> bool {
        if held.use_kind != CONSUMED {
            return false;
        }
        if held.kind == ItemKind::RabbitC {
            // ftCommon_8007FA58: worn, not eaten.
            self.wear_bunny_hood(held, assets);
            return true;
        }
        match held.kind {
            ItemKind::Heart | ItemKind::Tomato | ItemKind::Foods => self.heal(held.heal, assets),
            // ftCo_800C88D4(gobj, PlCo +7CC, 1).
            ItemKind::Spycloak => self.cloak(assets.cloak_frames),
            // ftLib_800871A8 with it_8026B560 / it_8026B574.
            ItemKind::MetalB => {
                let (frames, health) = held.metal.expect("a Metal Box's attributes");
                self.become_metal(frames, health, assets)
            }
            kind => unimplemented!("ftpickupitem_8009447C (ftpickupitem.c:156-196): consuming {kind:?}"),
        }
        self.item_requests
            .push(melee_it::ItemRequest::Destroy { item: held.item });
        self.release_held_item(held.item, assets);
        true
    }
}

impl Fighter {
    /// ftpickupitem_80094DF8, LightGet's take_dmg_cb: whatever the hand
    /// holds is eaten if it can be (ftpickupitem_80094B6C). x1978, a second
    /// item, is never filled.
    pub(super) fn eat_on_damage(&mut self, assets: &FighterAssets) {
        if let Some(held) = self.core.held_item {
            // ftpickupitem.c:367-370: a Warp Star is let go with no push.
            if held.use_kind == CONSUMED && held.kind == ItemKind::WStar {
                self.core.drop_held_item(held, 1.0, assets);
                return;
            }
            self.core.eat(held, assets);
        }
    }
}
