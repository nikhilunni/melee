//! Grab-pair hitlag linkage, fighter.c:2742-2804.
//!
//! Retail freezes a fighter while x2219_b5 is set; its own countdown
//! (x195c) is one way to hold it. A grab pair links both members through
//! x1A5C (the captor's victim, the victim's captor): when one member starts
//! hitlag (Fighter_UnkRecursiveFunc_8006D044), the other gets x2219_b7 and
//! is frozen too. A member whose own countdown ends while x2219_b7 is set
//! stays frozen; the partner's hitlag end (Fighter_8006D10C) releases it.
//!
//! The port's freeze flag is `FighterCore::in_hitlag`: the countdown, or
//! `HitlagLink::frozen` for a freeze held past (or without) it.
use super::{Fighter, FighterCore};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitlagLink {
    /// x2219_b7: the grab partner's hitlag entry holds this fighter frozen
    /// until that partner's hitlag ends.
    pub held: bool,
    /// x2219_b5 without a running countdown: set by the partner's entry, or
    /// kept when this fighter's own countdown ends while `held`.
    pub frozen: bool,
}

impl FighterCore {
    /// x2219_b5: frozen in hitlag, by its own countdown or by its partner.
    pub fn in_hitlag(&self) -> bool {
        self.combat.hitlag_remaining > 0.0 || self.combat.hitlag_link.frozen
    }
}

/// Fighter_UnkRecursiveFunc_8006D044's recursion (8006D044): `fighter` has
/// just started hitlag. Unless `fighter` is itself held, its grab partner is
/// held (x2219_b7) and starts hitlag too: its pre_hitlag_cb runs and x2219_b5
/// is set. Returns whether the partner's effects pause (its pre_hitlag_cb is
/// efLib_PauseAll). The partner's own recursion stops at its new x2219_b7.
pub fn hold_partner(fighter: &Fighter, partner: &mut Fighter) -> bool {
    if fighter.combat.hitlag_link.held {
        return false;
    }
    partner.core.combat.hitlag_link.held = true;
    if !partner.in_hitlag() {
        partner.core.combat.hitlag_link.frozen = true;
        partner.core.status.interaction = super::Interaction::Hitlag;
    }
    partner.effect_state.hitlag_callbacks
}

/// Fighter_8006D10C's tail (8006D10C, inline 1): `fighter`'s hitlag has
/// ended. Unless `fighter` is itself held, a partner it holds is released:
/// with no countdown left (allow_sdi clear; x1954 is unported) the partner's
/// hitlag ends now, and x2219_b7 clears either way. Returns whether the
/// partner's effects resume (its post_hitlag_cb is efLib_ResumeAll).
pub fn release_partner(fighter: &Fighter, partner: &mut Fighter) -> bool {
    if fighter.combat.hitlag_link.held || !partner.combat.hitlag_link.held {
        return false;
    }
    let mut resumed = false;
    if partner.combat.hitlag_remaining == 0.0 {
        partner.core.end_hitlag();
        resumed = partner.effect_state.hitlag_callbacks;
    }
    // Inline2 (Fighter_8006CFE0 on the partner's partner) needs the
    // partner's x2219_b7 clear, which it is not yet.
    partner.core.combat.hitlag_link.held = false;
    resumed
}

impl FighterCore {
    /// Fighter_8006A1BC's x221A_b1 -> Fighter_8006CFE0 (8006CFE0), before the
    /// countdown: ftCo_800DC920 separating the pair sets x221A_b1 on a held
    /// member (Fighter_UnkSetFlag_8006CFBC). The port reads a held fighter
    /// whose grab link is gone as that flag. With no countdown left (allow_sdi
    /// clear; x1954 is unported) its hitlag ends now; x2219_b7 clears.
    pub(super) fn release_separated_hold(&mut self) {
        if !self.combat.hitlag_link.held || self.combat.grab.is_some() {
            return;
        }
        if self.combat.hitlag_remaining == 0.0 {
            self.end_hitlag();
        }
        self.combat.hitlag_link.held = false;
    }
}
