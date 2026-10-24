//! Fighter flags the CPU reads, by their retail names, from the port's
//! owners of the same state.
use melee_ft::fighter::{grab::GrabLink, Fighter};

/// x221A_b3: the fighter is in the hitlag of a hit it took with knockback
/// (Fighter_ProcessHit, fighter.c:2980), launched or not: a thrown fighter
/// struck by its captor's throw hitbox counts.
pub fn in_damage_hitlag(fp: &Fighter) -> bool {
    fp.core.combat.struck_hitlag
}

/// x221B_b5: the fighter holds another in a grab.
pub fn holding_victim(fp: &Fighter) -> bool {
    matches!(fp.core.combat.grab, Some(GrabLink::Holding { .. }))
}

/// fp->item_gobj != NULL: an item, or the fighter's own article, in hand.
pub fn has_item(fp: &Fighter) -> bool {
    fp.core.held_item.is_some() || fp.core.article_in_hand.is_some()
}

/// The kind of fp->item_gobj: a held item, or the fighter's own article.
pub fn held_kind(fp: &Fighter) -> Option<melee_types::ItemKind> {
    match (&fp.core.held_item, &fp.core.article_in_hand) {
        (Some(item), _) => Some(item.kind),
        (None, Some(article)) => Some(article.kind),
        (None, None) => None,
    }
}
