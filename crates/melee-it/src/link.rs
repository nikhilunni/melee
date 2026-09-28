//! Articles that hold a pointer to another item (Item.xDD4_itemVar's gobj
//! fields): Pikachu's Thunder Jolt ball and the crawler it rides, and the
//! links of a Thunder bolt chain. Retail reads and writes the partner in
//! place from inside a proc; here the kind asks, and the scene delivers the
//! request once the proc returns, before any other item's proc runs.
use crate::SpawnItem;
use hsd_types::Vec3;

/// What one linked article tells another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkMessage {
    /// The sender lets go: the receiver's partner pointer clears
    /// (it_802B43B0, it_802B3544).
    Unlinked,
    /// A surface contact for the receiver to settle on (it_802B3F88): the
    /// point and the surface's normal.
    Settle { position: Vec3, normal: Vec3 },
    /// The position the sender stopped at (it_802B22B8's update of the
    /// next bolt link).
    Reached { position: Vec3 },
    /// A chain member's place, from the spawner's loop (it_802B1DF8):
    /// its index, its delay and the velocity it will fall at.
    Chain {
        index: i32,
        delay: i32,
        velocity: Vec3,
    },
    /// Nothing beyond the spawn itself: its setup rode in the SpawnItem
    /// (Din's Fire's explosion, it_802C4580).
    Spawned,
}

/// Whom a request is for.
#[derive(Clone, Copy, Debug)]
pub enum LinkTarget {
    /// An existing item.
    Item(u32),
    /// A new item, spawned for the sender and linked both ways
    /// (Item_80268B18 inside it_802B4224).
    Spawn(SpawnItem),
}

#[derive(Clone, Copy, Debug)]
pub struct LinkRequest {
    pub target: LinkTarget,
    pub message: LinkMessage,
}

/// What an article's proc asks of its owner, delivered once the proc
/// returns (retail calls into the fighter from inside it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerRequest {
    /// The returning boomerang reached its thrower, who catches it
    /// (ftLk_SpecialS2_Enter); the article then hangs from the part the
    /// owner names (Item_8026AB54).
    Catch,
    /// The article no longer belongs to its thrower (a reflected boomerang:
    /// ftLk_SpecialS_RemoveBoomerang0).
    Released,
}

/// The partner as a reader sees it during its own proc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerView {
    /// The partner's own partner (it_802B3368 / it_802B3EFC).
    pub partner: Option<u32>,
    /// The reader's `PARTNER_BONE` of the partner's model, in world space
    /// (lb_8000B1CC on xBBC_dynamicBoneTable->bones[i]).
    pub bone_position: Option<Vec3>,
}

/// An owner's view of its tracked article, sampled before the owner's
/// animation proc (Pikachu's Thunder reads its lead bolt: it_802B1FE8's
/// point and it_802B1DEC's flag).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArticleReport {
    pub point: Vec3,
    /// The article already struck (it_802B1DEC).
    pub struck: bool,
}
