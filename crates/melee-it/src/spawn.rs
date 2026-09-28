use hsd_types::Vec3;
use melee_types::{GroundOrAir, ItemKind};

/// Owned counterpart of `it/types.h:689`, retail `SpawnItem` (0x4C bytes).
/// Parent GObjs become player slots; memory offsets stay out of the tick path.
#[derive(Clone, Copy, Debug)]
pub struct SpawnItem {
    pub owner: Option<u8>,
    pub stale_source: Option<melee_types::combat::AttackInstance>,
    pub secondary_owner: Option<u8>,
    pub kind: ItemKind,
    pub hold_kind: u8,
    pub spawn_variant: i32,
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub facing: f32,
    pub damage: i16,
    pub auxiliary_damage: i16,
    pub spawn_argument: i32,
    pub initial_collision: bool,
    pub auxiliary_flags: [u8; 3],
    pub ground_or_air: GroundOrAir,
}
impl SpawnItem {
    /// `Item_InitSpawnOnPlaneNoInitialCollision`, it/kinds/inlines.h.
    pub fn held(kind: ItemKind, owner: u8, mut position: Vec3, facing: f32) -> Self {
        position.z = 0.0;
        Self {
            owner: Some(owner),
            stale_source: None,
            secondary_owner: Some(owner),
            kind,
            hold_kind: 8,
            spawn_variant: 0,
            position,
            previous_position: position,
            velocity: Vec3::ZERO,
            facing,
            damage: 0,
            auxiliary_damage: 0,
            spawn_argument: 0,
            initial_collision: false,
            auxiliary_flags: [0; 3],
            ground_or_air: GroundOrAir::Air,
        }
    }
    /// `Item_InitSpawn` (it/kinds/inlines.h), the attached-article spawn:
    /// the position keeps its Z and the item takes its initial collision.
    /// x48_ground_or_air is left unset by retail callers (it_802BDA64,
    /// it_802BDE18); every recorded spawn resolves as airborne.
    pub fn attached(kind: ItemKind, owner: u8, position: Vec3, facing: f32) -> Self {
        Self {
            position,
            previous_position: position,
            initial_collision: true,
            ..Self::held(kind, owner, position, facing)
        }
    }
    /// `Item_InitRaySpawnPosition` / `Item_InitRaySpawnFields`.
    pub fn ray(kind: ItemKind, owner: u8, position: Vec3, facing: f32) -> Self {
        Self {
            initial_collision: true,
            ..Self::held(kind, owner, position, facing)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ItemControl {
    Visibility(i32),
    Open,
    Close,
    Fire,
    Remove,
    /// it_802BE100: Toad's counter, motion 1 with its animation ten frames in.
    Counter,
    /// it_802BDD40 / it_802BDDB4: the Peach parasol's opening (motion 1) or
    /// open (motion 2) animation at the given rate (ftCommon_8007E83C).
    ParasolOpening(f32),
    ParasolOpen(f32),
    /// it_8026B724 / it_8026B73C: the owner's pre/post-hitlag callbacks.
    OwnerHitlag(bool),
    /// it_802B1FC8: the owner struck by its lead Thunder bolt.
    Strike,
}
#[derive(Clone, Copy, Debug)]
pub enum ItemRequest {
    Spawn(SpawnItem),
    SpawnHeld(SpawnItem),
    SpawnLaser {
        spawn: SpawnItem,
        angle: f32,
        speed: f32,
        motion: u16,
    },
    Control {
        owner: u8,
        kind: ItemKind,
        control: ItemControl,
    },
    /// ftpickupitem_800948A8 -> Item_8026AB54: the requesting fighter takes
    /// `item` into its hand at `part`.
    PickUp {
        item: u32,
        /// A fp->parts index (ftData x8 +0x10 or +0x11).
        part: u8,
    },
    /// ftCo_80095EFC -> Item_8026AD20: the holder throws `item` from
    /// `position` with `velocity`; `speed` scales the thrown hitboxes.
    /// `center` and `attack` are the holder's (ftLib_80086990, it_8027B070).
    Throw {
        item: u32,
        position: Vec3,
        /// The world matrix of the hand joint holding it (ftLib_80086630).
        hand: hsd_types::Mtx,
        velocity: Vec3,
        speed: f32,
        center: Vec3,
        attack: Option<melee_types::combat::AttackInstance>,
    },
    /// Item_8026ABD8: the holder lets go of `item` at the hand (`position`)
    /// without a push; `speed` becomes xC44.
    Drop {
        item: u32,
        position: Vec3,
        /// The world matrix of the hand joint holding it (ftLib_80086630).
        hand: hsd_types::Mtx,
        speed: f32,
        center: Vec3,
        attack: Option<melee_types::combat::AttackInstance>,
    },
    /// Item_8026A8EC: `item` is destroyed at once (a holder's death).
    Destroy {
        item: u32,
    },
    /// it_802B1DF8: `count` copies of `spawn` in list order, each linked
    /// to the next (its partner), set up by a [`crate::LinkMessage::Chain`]
    /// with its index, `index * delay` and `velocity`.
    SpawnChain {
        spawn: SpawnItem,
        count: i32,
        delay: i32,
        velocity: Vec3,
    },
    /// Item_80268B18 then Item_8026AB54: `spawn` is created and taken at
    /// once into its owner's hand at `part`, a fp->parts index
    /// (it_802B2A10, Yoshi's Egg Throw egg). With `hold`, the owner's
    /// fp->item_gobj becomes the new item (Peach's setupVeg).
    SpawnInHand {
        spawn: SpawnItem,
        part: u8,
        hold: bool,
    },
    /// The owner sends its held article of `kind` out of the hand
    /// (it_802B28C8, the Egg Throw): see [`Launch`].
    Launch {
        owner: u8,
        kind: ItemKind,
        launch: Launch,
    },
    /// Item_8026ABD8 on `owner`'s `kind` article (hold kind 8): it leaves
    /// the joint whose world matrix is `hold`, offset back by its own attach
    /// joint (it_80273B50), with no push.
    DropArticle {
        owner: u8,
        kind: ItemKind,
        hold: hsd_types::Mtx,
        center: Vec3,
        attack: Option<melee_types::combat::AttackInstance>,
    },
}

/// A held article's launch, sampled from its holder when the holder asks.
#[derive(Clone, Copy, Debug)]
pub struct Launch {
    /// The release velocity it_8027429C receives.
    pub velocity: Vec3,
    /// Added to the released position afterwards.
    pub offset: Vec3,
    /// it_80274658's spin, in degrees per frame.
    pub spin_degrees: f32,
    /// The world matrix of the holder's hand joint (it_80273B50 transforms
    /// the article's attachment offset by it), the holder's body centre
    /// (ftLib_80086990), current attack and its stale multiplier (it_8027B070).
    pub hand: hsd_types::Mtx,
    pub center: Vec3,
    pub attack: Option<melee_types::combat::AttackInstance>,
    pub attack_stale: f32,
}

/// The holding fighter, lent to a held item's callbacks. A held item
/// released from the hand starts at the hand (it_80273B50) and sweeps the
/// map from the holder's body (it_80275BC8).
pub struct ItemHolder<'a> {
    /// The holder's skeleton and the held part's joint (ftLib_80086630:
    /// fp->parts[xDC4].joint).
    pub skeleton: &'a mut hsd_anim::jobj::JObjTree,
    pub part: hsd_anim::jobj::JObjId,
    /// ftLib_80086990: the holder's position raised to its ECB centre.
    pub center: Vec3,
    /// it_8027B070: the holder's current attack, which the item takes on release.
    pub attack: Option<melee_types::combat::AttackInstance>,
    /// The holder's stale multiplier for `attack`: hitboxes the item makes
    /// after it lets go are restaled for it (it_80272460 -> ft_80089228).
    pub attack_stale: f32,
}
impl ItemHolder<'_> {
    /// The part's world matrix, set up on demand (HSD_JObjSetupMatrix).
    pub fn part_matrix(&mut self) -> hsd_types::Mtx {
        *self.skeleton.get_mtx(self.part)
    }
    /// lb_8000B1CC(part, 0): the part's world translation, set up on demand
    /// (HSD_JObjSetupMatrix) as retail does only when it asks.
    pub fn part_position(&mut self) -> Vec3 {
        if self.skeleton.parent(self.part).is_none() {
            return self.skeleton.translation(self.part);
        }
        let matrix = self.skeleton.get_mtx(self.part);
        Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
    }
}

/// Fighter-owned inputs sampled for the item callback; no fighter dependency.
#[derive(Clone, Copy, Debug)]
pub struct IllusionOwner {
    pub create_secondary: bool,
    pub positions: [Vec3; 4],
    pub rotations: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct ItemOwner {
    pub illusion: Option<IllusionOwner>,
    pub position: Vec3,
    pub facing: f32,
    pub hold_position: Vec3,
    pub blaster_action: u16,
    pub remove_blaster: bool,
    /// fp->motion_id, read by articles whose lifetime follows the owner's
    /// motion (ftPe_SpecialN_IsActive, ftPe_SpecialHi_NotActive).
    pub motion: u16,
}
