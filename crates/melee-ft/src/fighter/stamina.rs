//! Stamina's KO when the hit points run out (ft_0C8C.c).
//!
//! Every Fighter_TakeDamage_8006CC7C ends in ftCo_800C8C84: a Stamina
//! fighter (x2225_b7) whose Player_GetRemainingHPByIndex reaches 0 is
//! marked (x2224_b3, `stamina_ko_pending`). When a knockback hit is being
//! applied (dmg.kb_applied) it is knocked out at once (fn_800C8_inline:
//! x2224_b2, `stamina_out`); otherwise the next ProcessHit's ftCo_800C8D00
//! does it. ftCo_800C8D00 then drops the fighter: DamageFall in the air,
//! DownSpot on the ground. From then on the fighter takes no input
//! (Fighter_Spaghetti_8006AD10), knockback reads PlCo +6D8, prone states
//! never count down, and the scene slows the game (gm_801B97C4).
use super::assets::{FighterAssets, Result};
use super::commands::{FootstepSound, SoundChannel};
use super::{Fighter, FighterCore};
use melee_types::GroundOrAir;

/// ftCo_800BFFD0(fp, 0x7A, 0): the knocked-out fighter's color animation,
/// which ftCo_800C0408 reinstalls whenever the primary slot empties.
pub(super) const STAMINA_OUT_COLOR: u8 = 0x7A;

impl FighterCore {
    /// ftCo_800C8C84 (0x800C8C84), the tail of Fighter_TakeDamage_8006CC7C:
    /// the hit points are gone. `launched` is dmg.kb_applied != 0.
    pub(crate) fn check_stamina_knockout(&mut self, launched: bool, assets: &FighterAssets) {
        if self.stamina_spent() {
            self.mark_stamina_knockout(launched, assets);
        }
    }

    /// Fighter_ProcessHit's Fighter_UnkTakeDamage_8006CC30 for a knockback
    /// hit (fighter.c:2856), which the reaction applies later in the port:
    /// ftCo_800C8C84 on the percent the hit will leave, before the
    /// knockback and the reaction.
    pub(super) fn check_stamina_knockout_before_launch(
        &mut self,
        damage: f32,
        assets: &FighterAssets,
    ) {
        if self.combat.pending_percent_taken {
            // Taken already (`take_percent_damage`), and checked there.
            return;
        }
        let percent = super::offscreen::capped_percent(self.physics.percent + damage);
        if self.stamina_spent_at(percent) {
            self.mark_stamina_knockout(true, assets);
        }
    }

    /// fn_800C8E74 (0x800C8E74). Master Hand and Crazy Hand, whose branch
    /// this skips, are not playable fighters.
    fn mark_stamina_knockout(&mut self, launched: bool, assets: &FighterAssets) {
        self.stamina_ko_pending = true;
        if launched {
            self.knock_out_stamina(assets);
        }
    }

    /// fn_800C8_inline (ft_0C8C.c:42-53).
    fn knock_out_stamina(&mut self, assets: &FighterAssets) {
        self.stamina_out = true;
        // Fighter_ResetInputData_80068854: the current and previous samples
        // and every input timer; the saved sample stays.
        self.input.clear_current_and_buffers();
        self.install_color_overlay_now(STAMINA_OUT_COLOR, assets);
        // fn_800C8E40: ft_PlaySFX(fp, x4C_sfx->xC, 127, 64), the star KO's
        // cry, on the ordinary channel (no pitch draw: a voice id).
        self.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: assets.life.death_sounds.star,
            volume: 127,
            pan: 64,
        });
        // x2225_b6 then mutes the fighter's voices (ft_800881D8,
        // ft_80088328), which play no draws: not modelled. lbBgFlash_80021C48
        // (0xE, 0) tints the background (its callback fn_80021C18 is empty).
        // gm_80167470: the scene's part.
        self.stamina_knocked_out = true;
    }
}

impl Fighter {
    /// ftCo_800C8D00 (0x800C8D00): a marked fighter is knocked out, its
    /// actions interrupted, a heavy item dropped, and it drops into
    /// DamageFall (ftCo_80090780) or, grounded, DownSpot.
    pub(super) fn complete_stamina_knockout(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.stamina_ko_pending {
            return Ok(());
        }
        if !self.core.stamina_out {
            self.core.knock_out_stamina(assets);
        }
        // ftCommon_8007DB58.
        self.interrupt_actions(assets);
        // ftCo_8009750C (ftCo_Lift.c:201): Item_8026ABD8(item, {0}, 0).
        if let Some(held) = self.core.held_item {
            if held.heavy {
                self.core.drop_held_item(held, 0.0, assets);
            }
        }
        if self.core.combat.grab.is_some() {
            unimplemented!(
                "ftCo_800C8D00 (ft_0C8C.c:66-79): a Stamina KO with a grab link \
                 (CaptureCut, ftCo_800DA698)"
            );
        }
        if self.core.physics.ground_or_air == GroundOrAir::Air {
            self.enter_damage_fall(assets)?;
        } else {
            unimplemented!("ftCo_DownSpot_Enter (ftCo_DownSpot.c:14): a grounded Stamina KO");
        }
        self.core.stamina_ko_pending = false;
        Ok(())
    }
}
