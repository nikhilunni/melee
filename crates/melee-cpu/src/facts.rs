//! Fighter flags the CPU reads, by their retail names, from the port's
//! owners of the same state.
use melee_ft::fighter::{damage::HitlagCallbacks, grab::GrabLink, Fighter};

/// x221A_b3: the fighter is in the hitlag of a hit it took (set with
/// Fighter_ProcessHit's damage hitlag, cleared when that hitlag ends, as
/// the damage hitlag callbacks are).
pub fn in_damage_hitlag(fp: &Fighter) -> bool {
    fp.core.combat.hitlag_remaining > 0.0
        && fp.core.combat.hitlag_callbacks == HitlagCallbacks::Damage
}

/// x221B_b5: the fighter holds another in a grab.
pub fn holding_victim(fp: &Fighter) -> bool {
    matches!(fp.core.combat.grab, Some(GrabLink::Holding { .. }))
}

/// fp->item_gobj != NULL: an item, or the fighter's own article, in hand.
pub fn has_item(fp: &Fighter) -> bool {
    fp.core.held_item.is_some() || fp.core.article_in_hand.is_some()
}
