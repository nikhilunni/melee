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

/// The partner as a reader sees it during its own proc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerView {
    /// The partner's own partner (it_802B3368 / it_802B3EFC).
    pub partner: Option<u32>,
    /// The reader's `PARTNER_BONE` of the partner's model, in world space
    /// (lb_8000B1CC on xBBC_dynamicBoneTable->bones[i]).
    pub bone_position: Option<Vec3>,
}
