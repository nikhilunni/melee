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
    /// Item_80268E5C(item, state, ITEM_ANIM_UPDATE) from the owner's proc
    /// (Samus's grapple beam, which its owner drives).
    Motion(u16),
    /// it_802C3D44: the owner lets its article fly on without it (Zelda
    /// hit or dying with Din's Fire out).
    Orphan,
    /// it_802A8398: a drawn arrow follows the hands (its position is the
    /// tip, its tail the bow hand).
    Aim { tip: Vec3, tail: Vec3 },
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
    /// An item in hand stowed under an article (ftPe_SpecialHi_8011D424:
    /// it_8026BB44 hides it, it_8026B724 sets xDC8 x3, which freezes it) or
    /// brought back (ftPe_8011D518: it_8026BB20, it_8026B73C). It stays
    /// attached to the hand throughout.
    Stow {
        item: u32,
        stowed: bool,
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
    /// it_802A0534's flight angle (radians) and lifetime choice (the smash
    /// throw's longer one).
    pub angle: f32,
    pub long_lifetime: bool,
    /// itLinkArrow_802A850C's aim and draw, for a shot article.
    pub shot: Option<Shot>,
    /// A release that also aims the article (Samus's charge shot,
    /// it_802B56E4), if any.
    pub aim: Option<Aim>,
}

/// it_802B56E4's arguments: the point the article leaves from, its angle
/// in radians, the charge it carries and the full charge, as floats, and
/// the holder's facing (ftLib_800865C0).
#[derive(Clone, Copy, Debug)]
pub struct Aim {
    pub position: Vec3,
    pub angle: f32,
    pub charge: f32,
    pub full_charge: f32,
    pub facing: f32,
}

/// A drawn arrow's release (ftLk_SpecialNEnd_Coll -> itLinkArrow_802A850C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// The drawing hand's thumb (the arrow's tip) and the bow hand's (its
    /// tail), at z 0.
    pub tip: Vec3,
    pub tail: Vec3,
    /// The flight angle (radians).
    pub angle: f32,
    /// Frames drawn, and the most that count.
    pub charge: f32,
    pub max_charge: f32,
    /// The archer's facing (ftLib_800865C0).
    pub facing: f32,
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

/// A blast an item's accessory offers its owner (it_802B5478): the owner
/// tests its hurt capsules against hitbox 0 (ftSs_Init_80128A1C) and may
/// take the launch (ftSs_Init_80128944).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnerBlast {
    pub owner: u8,
    /// Hitbox 0's x58 and x4C as they stand (a new capsule is not placed
    /// before item link 11, so these can be the cleared slot's).
    pub previous: hsd_types::Vec3,
    pub position: hsd_types::Vec3,
    /// lbColl_80008248's radius: hitbox 0's scale, times the item's unless
    /// the hitbox ignores scale (x43_b1).
    pub contact_radius: f32,
    /// Hitbox 0's stored scale, the launch's horizontal range.
    pub range: f32,
    /// The item's x, the launch's centre.
    pub x: f32,
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
    /// Fighter.x2070 (the motion row's x4_flags), whose x2071 nibble an
    /// article may test (ftLk_SpecialS_Is2071b0_1to13); None for a
    /// character row whose column the port does not carry.
    pub motion_flags: Option<u32>,
    /// x2219_b5: the owner is in hitlag.
    pub in_hitlag: bool,
    /// The point a returning article homes on
    /// (ftLk_SpecialHi_GetPosWithAdjustedY: cur_pos raised by an attribute);
    /// the position for other kinds.
    pub anchor: Vec3,
    /// The stage of the owner's special its articles follow
    /// (ftLk_SpecialN_GetIndex: 0..5 through the bow special's rows),
    /// or None.
    pub article_stage: Option<u8>,
    /// The owner's count of articles it has fired, which its articles
    /// compare with the count at their own launch (Samus's missiles read
    /// u.ss.x2238 through ftSs_SpecialS_8012A068). Zero for other kinds.
    pub articles_fired: u32,
    /// ftSs_SpecialLw_80129100: while the owner holds a charging article
    /// (u.ss.x222C), its charge level and full level (fctiwz of x18).
    pub charge: Option<(i32, i32)>,
    /// Sheik's held needles (ftSeak_FighterVars x4, read through
    /// ftSk_SpecialS_80111F70): the needle item in her hand lives while
    /// she keeps it.
    pub holds_needles: bool,
    /// fp->input.lstick[0] (ftLib_800865D8).
    pub stick: hsd_types::Vec2,
    /// The owner steers the article it keeps (Zelda in Din's Fire's loop
    /// with her fire out, ftZd_SpecialLw_8013B540).
    pub steering_article: bool,
    /// The owner's script detonates the article it keeps (Zelda's
    /// cmd_vars[1] in Din's Fire's end, ftZd_SpecialLw_8013B574).
    pub detonating_article: bool,
}

/// A fighter an article may lock on to (ftLib_80086368's candidates): its
/// player, whether it is out of play (x221F_b3) and its camera bone's
/// position with the zoom offset (ftLib_800866DC).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockOnFighter {
    pub player: u8,
    pub disabled: bool,
    pub position: Vec3,
}

/// An item an article may lock on to (it_8026C258's candidates, already
/// filtered by hold kind, grab and holder): its ECB-centre position
/// (it_8026BB88).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockOnItem {
    pub position: Vec3,
}

/// The candidates of a homing article's physics, in the object lists'
/// order. Only kinds with [`crate::ItemLogic::LOCKS_ON`] receive them.
#[derive(Clone, Debug, Default)]
pub struct LockOnTargets {
    pub fighters: melee_types::fixed::FixedVec<LockOnFighter, 6>,
    pub items: melee_types::fixed::FixedVec<LockOnItem, 64>,
}
