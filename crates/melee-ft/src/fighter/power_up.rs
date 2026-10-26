//! Power-up items a touch gives (it_8026B30C == 4, ftColl_80077C60's first
//! branch): the Starman's invincibility (ftColl_8007B7FC). The Super and
//! Poison Mushrooms' growth (x200C / x2010, Fighter_SuperMushroomApply)
//! fails closed.
use super::assets::FighterAssets;
use super::FighterCore;
use melee_types::ItemKind;

/// ftColl_8007B7FC's colour animation, and ftCo_800C0408's fallback while
/// the Starman lasts.
pub(super) const STARMAN_COLOR: u8 = 0x6B;
/// ftColl_80077C60's sound for a Starman's touch.
const STARMAN_SOUND: u32 = 0xF9;

/// Whether `kind` is a touch power-up (ItemAttr x0 bits 0x78 == 4).
pub fn is_power_up(kind: ItemKind) -> bool {
    matches!(kind, ItemKind::Star | ItemKind::Kinoko | ItemKind::DKinoko)
}

impl FighterCore {
    /// ftColl_80077C60 (ftcoll.c:1110-1141): a power-up's touch, unless the
    /// fighter's Stamina hit points ran out (x2224_b2). `frames` is the
    /// Starman's invincibility (it_80272818). Returns whether the touch
    /// took (the item's hitbox goes and it has dealt its "damage").
    pub fn touch_power_up(&mut self, kind: ItemKind, frames: i32, assets: &FighterAssets) -> bool {
        if self.stamina_out {
            return false;
        }
        match kind {
            ItemKind::Star => {
                self.grant_starman(frames, assets);
                self.commands
                    .footstep_sounds
                    .push(super::commands::FootstepSound {
                        channel: super::commands::SoundChannel::Ordinary,
                        id: STARMAN_SOUND,
                        volume: 127,
                        pan: 64,
                    });
            }
            ItemKind::Kinoko => unimplemented!(
                "ftColl_80077C60 (ftcoll.c:1127-1131): a Super Mushroom's growth (x200C)"
            ),
            ItemKind::DKinoko => unimplemented!(
                "ftColl_80077C60 (ftcoll.c:1132-1136): a Poison Mushroom's shrinking (x2010)"
            ),
            _ => unreachable!("{kind:?} is no power-up"),
        }
        true
    }

    /// ftColl_8007B7FC (8007B7FC): invincible (x221D_b6) for `frames`
    /// (x2004) under the Starman's colour. ft_80088080's music is not
    /// simulated.
    fn grant_starman(&mut self, frames: i32, assets: &FighterAssets) {
        self.status.star_invincible = true;
        self.status.star_frames = frames;
        self.install_color_overlay_now(STARMAN_COLOR, assets);
    }

    /// ftCo_800C88D4 (800C88D4) from the Cloaking Device's pickup
    /// (`from_item`, PlCo +7CC frames): x2030 the frames; a fighter not yet
    /// cloaked (x2226_b4), or showing the end's warning (x2226_b7), is
    /// cloaked again (x2226_b5, the refraction and its colour animations
    /// are presentation). A second cloak only renews the frames.
    pub(super) fn cloak(&mut self, frames: i32) {
        self.status.cloak_frames = frames;
        self.status.cloaked = true;
    }

    /// ftCo_800C8A64 (800C8A64): the frames gone; unless the Special Melee
    /// keeps the fighter invisible (x2226_b3) the cloak is off.
    pub(super) fn end_cloak(&mut self) {
        self.status.cloak_frames = 0;
        if !self.player.invisible {
            self.status.cloaked = false;
        }
    }

    /// Fighter_8006A360 (fighter.c:1562-1574): the cloak's countdown. At
    /// its end the cloak goes and the rest of the proc is skipped (false);
    /// at PlCo +7D0 frames left its warning colour (x2226_b7) is only shown.
    pub(super) fn tick_cloak(&mut self) -> bool {
        if !self.status.cloaked || self.status.cloak_frames == 0 {
            return true;
        }
        self.status.cloak_frames -= 1;
        if self.status.cloak_frames == 0 {
            self.end_cloak();
            return false;
        }
        true
    }

    /// Fighter_8006A360 (fighter.c:1486-1498): the Starman's countdown; at
    /// its end the invincibility goes and its colour if it still owns the
    /// slot (ftCo_800C0200, deferred like the flash's). ft_800880D8's music
    /// is not simulated.
    pub(super) fn tick_starman(&mut self) {
        if !self.status.star_invincible || self.status.star_frames == 0 {
            return;
        }
        self.status.star_frames -= 1;
        if self.status.star_frames == 0 {
            self.status.star_invincible = false;
            self.combat.color_overlay.starman_expired = true;
        }
    }
}
