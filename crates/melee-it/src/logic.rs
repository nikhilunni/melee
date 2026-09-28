use crate::{desc::ItemAssets, ItemControl, ItemCore, ItemOwner};
use melee_types::ItemKind;

pub struct ItemAnimationContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    /// Present while the item is held (xDC8 x13).
    pub holder: Option<crate::ItemHolder<'a>>,
    pub map: &'a mut melee_mp::CollMap,
    pub assets: &'a ItemAssets,
    /// The item's partner as it stands (see [`crate::PartnerView`]).
    pub partner: Option<crate::PartnerView>,
}
pub struct ItemPhysicsContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    /// Lock-on candidates, for kinds with `ItemLogic::LOCKS_ON`.
    pub targets: &'a crate::LockOnTargets,
    pub assets: &'a ItemAssets,
    pub bounds: &'a crate::ItemBounds,
    /// HSD_Randi / HSD_Randf for callbacks that draw.
    pub rng: &'a core::cell::Cell<gekko_math::HsdRng>,
}
pub struct ItemCollisionContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    pub stage_contact: bool,
    pub map: &'a mut melee_mp::CollMap,
    pub assets: &'a ItemAssets,
    pub bounds: &'a crate::ItemBounds,
}
#[derive(Clone, Copy)]
pub struct ItemEventContext<'a> {
    pub reflected_facing: f32,
    /// xC70: the reflector's speed multiplier (ReflectAttr x1A38).
    pub reflected_speed: f32,
    pub shield_normal: hsd_types::Vec3,
    pub assets: &'a ItemAssets,
    /// p_ftCommonData's launch constants (it_8027B798).
    pub launch: crate::hurt::ItemLaunch,
    /// HSD_Randi / HSD_Randf, where the event may draw (OnTakeDamage).
    pub rng: Option<&'a core::cell::Cell<gekko_math::HsdRng>>,
    /// it_804D6D28 +58..+60, for itColl_BounceOffVictim.
    pub victim_bounce: crate::desc::VictimBounce,
}
impl<'a> ItemEventContext<'a> {
    pub fn new(assets: &'a ItemAssets, common: &crate::desc::ItemCommonData) -> Self {
        Self {
            reflected_facing: 0.0,
            reflected_speed: 1.0,
            shield_normal: hsd_types::Vec3::ZERO,
            assets,
            launch: common.launch,
            rng: None,
            victim_bounce: common.victim_bounce,
        }
    }
}
#[derive(Clone, Copy)]
pub struct ItemStateRow {
    pub animation_id: i32,
    pub animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
    pub physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    pub collision: fn(&mut ItemCore, &mut ItemCollisionContext<'_>) -> bool,
}
/// `it/kinds/types.h:26`: one implementation per ItemLogicTable row.
pub trait ItemLogic {
    const KIND: ItemKind;
    const STATES: &'static [ItemStateRow];
    /// Zero uses rigid projectile instancing; positive counts prepare owned article poses.
    const MODEL_COPIES: usize = 0;
    const HELD_SCALE: f32 = 1.0;
    const HELD_PART: Option<melee_types::FtPart> = None;
    /// Read-only pose corrections, after authored animation and before capture.
    fn model_pose(_item: &ItemCore, _tree: &mut hsd_anim::jobj::JObjTree, _copy: usize) -> bool {
        true
    }

    /// Conservative predicate: false proves Item_IsGrabbable cannot succeed.
    /// Unreviewed kinds remain possible; they require the full pickup search.
    fn pickup_possible(_item: &ItemCore) -> bool {
        true
    }
    /// it_802750F8 (802750F8): the spawner runs the new item's physics
    /// (Item_802697D4) and collision (Item_80269978) procs once at once,
    /// with the blast-zone check (xDCC b3) off (it_8029B6F8).
    const PROCS_AT_SPAWN: bool = false;
    /// The physics callback looks for a target (Samus's missile,
    /// it_802B64FC): the scene supplies `ItemPhysicsContext::targets`.
    const LOCKS_ON: bool = false;
    fn spawned(_item: &mut ItemCore, _assets: &ItemAssets) {}
    /// The spawning code's own set-up once Item_80268B18 returns (e.g.
    /// it_802BE2E8 for Toad's spores), which may draw from the RNG.
    fn launched(
        _item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &crate::desc::ItemCommonData,
        _spawn: &crate::SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
    }
    /// The spawner's set-up that queries the map once Item_80268B18
    /// returns (itPikachuThunderJolt_Spawn's it_8026E9A4), after
    /// [`Self::launched`].
    fn spawned_with_map(
        _item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &crate::desc::ItemCommonData,
        _spawn: &crate::SpawnItem,
        _map: &mut melee_mp::CollMap,
    ) {
    }
    /// The holder sends its held article out (ItemRequest::Launch): the
    /// Egg Throw's it_802B28C8, the charge shot's it_802B56E4.
    fn launch(
        _item: &mut ItemCore,
        _launch: &crate::Launch,
        _common: &crate::desc::ItemCommonData,
        _map: &mut melee_mp::CollMap,
        _assets: &ItemAssets,
    ) {
        unimplemented!("held article launch for this kind")
    }
    fn destroyed(_item: &mut ItemCore) {}
    fn picked_up(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {}
    fn dropped(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("dropped callback for this kind")
    }
    fn thrown(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {
        unimplemented!("thrown callback for this kind")
    }
    fn entered_air(_item: &mut ItemCore) {}
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn damage_received(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// Item_80269A9C (item link 9): the item's on_accessory callback, which
    /// hitlag skips.
    /// A blast the item offers its owner reaches it at once.
    fn accessory(
        _item: &mut ItemCore,
        _owner: Option<&ItemOwner>,
        _assets: &ItemAssets,
    ) -> Option<crate::OwnerBlast> {
        None
    }
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
    fn control(_item: &mut ItemCore, _control: ItemControl, _assets: &ItemAssets) {
        unimplemented!("item control for this kind")
    }
    /// The partner's joint the animation callback reads (its dynamic bone
    /// index), if any.
    const PARTNER_BONE: Option<usize> = None;
    /// The Destroyed callback clears the partner's pointer back (as
    /// it_2725_Logic106_Destroyed's it_802B43B0 does).
    const UNLINKS_PARTNER_ON_DESTROY: bool = false;
    /// The world matrix of the joint the kind's own generators follow
    /// (efSync_Spawn on one of its JObjs), given the root's. The root by
    /// default; Samus's missile's trail rides its model's grandchild.
    fn effect_joint_matrix(
        _item: &ItemCore,
        _assets: &ItemAssets,
        root: hsd_types::Mtx,
    ) -> hsd_types::Mtx {
        root
    }
    /// What the owner reads of this article, if it is the one tracked.
    fn owner_report(_item: &ItemCore, _assets: &ItemAssets) -> Option<crate::ArticleReport> {
        None
    }
    /// Whether the owner hears of this article's end (its ARTICLE_DESTROYED
    /// hook); a chain notifies only through its lead.
    fn notifies_owner(_item: &ItemCore) -> bool {
        true
    }
    /// A linked item's request arrived; true destroys the receiver.
    fn link_received(
        _item: &mut ItemCore,
        message: crate::LinkMessage,
        _assets: &ItemAssets,
    ) -> bool {
        unimplemented!("linked item message {message:?} for this kind")
    }
    const LOGIC: ItemLogicRow = ItemLogicRow {
        states: Self::STATES,
        held_part: Self::HELD_PART,
        held_scale: Self::HELD_SCALE,
        model_pose: Self::model_pose,
        model_copies: Self::MODEL_COPIES,
        spawned: Self::spawned,
        launched: Self::launched,
        spawned_with_map: Self::spawned_with_map,
        pickup_possible: Self::pickup_possible,
        procs_at_spawn: Self::PROCS_AT_SPAWN,
        locks_on: Self::LOCKS_ON,
        launch: Self::launch,
        destroyed: Self::destroyed,
        picked_up: Self::picked_up,
        dropped: Self::dropped,
        thrown: Self::thrown,
        entered_air: Self::entered_air,
        damage_dealt: Self::damage_dealt,
        damage_received: Self::damage_received,
        reflected: Self::reflected,
        clanked: Self::clanked,
        absorbed: Self::absorbed,
        shield_bounced: Self::shield_bounced,
        hit_shield: Self::hit_shield,
        accessory: Self::accessory,
        owner_removed: Self::owner_removed,
        control: Self::control,
        partner_bone: Self::PARTNER_BONE,
        unlinks_partner_on_destroy: Self::UNLINKS_PARTNER_ON_DESTROY,
        owner_report: Self::owner_report,
        effect_joint_matrix: Self::effect_joint_matrix,
        notifies_owner: Self::notifies_owner,
        link_received: Self::link_received,
    };
}
#[derive(Clone, Copy)]
pub struct ItemLogicRow {
    pub model_copies: usize,
    pub held_scale: f32,
    pub held_part: Option<melee_types::FtPart>,
    pub model_pose: fn(&ItemCore, &mut hsd_anim::jobj::JObjTree, usize) -> bool,
    pub states: &'static [ItemStateRow],
    pub spawned: fn(&mut ItemCore, &ItemAssets),
    pub launched: fn(
        &mut ItemCore,
        &ItemAssets,
        &crate::desc::ItemCommonData,
        &crate::SpawnItem,
        &mut gekko_math::HsdRng,
    ),
    pub spawned_with_map: fn(
        &mut ItemCore,
        &ItemAssets,
        &crate::desc::ItemCommonData,
        &crate::SpawnItem,
        &mut melee_mp::CollMap,
    ),
    pub pickup_possible: fn(&ItemCore) -> bool,
    pub procs_at_spawn: bool,
    pub locks_on: bool,
    pub launch: fn(
        &mut ItemCore,
        &crate::Launch,
        &crate::desc::ItemCommonData,
        &mut melee_mp::CollMap,
        &ItemAssets,
    ),
    pub destroyed: fn(&mut ItemCore),
    pub picked_up: fn(&mut ItemCore, &mut ItemAnimationContext<'_>),
    pub dropped: fn(&mut ItemCore, &mut ItemAnimationContext<'_>),
    pub thrown: fn(&mut ItemCore, &mut ItemAnimationContext<'_>),
    pub entered_air: fn(&mut ItemCore),
    pub damage_dealt: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub damage_received: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub reflected: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub clanked: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub absorbed: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub shield_bounced: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub hit_shield: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub accessory: fn(&mut ItemCore, Option<&ItemOwner>, &ItemAssets) -> Option<crate::OwnerBlast>,
    pub owner_removed: fn(&mut ItemCore, u8),
    pub control: fn(&mut ItemCore, ItemControl, &ItemAssets),
    pub partner_bone: Option<usize>,
    pub unlinks_partner_on_destroy: bool,
    pub owner_report: fn(&ItemCore, &ItemAssets) -> Option<crate::ArticleReport>,
    pub effect_joint_matrix: fn(&ItemCore, &ItemAssets, hsd_types::Mtx) -> hsd_types::Mtx,
    pub notifies_owner: fn(&ItemCore) -> bool,
    pub link_received: fn(&mut ItemCore, crate::LinkMessage, &ItemAssets) -> bool,
}
pub trait ItemDispatch {
    fn logic(kind: ItemKind) -> &'static ItemLogicRow;
}
/// Consumer-owned kind inventory; melee-it never depends on per-kind crates.
#[macro_export]
macro_rules! item_kinds {
    ($vis:vis enum $name:ident { $($variant:ident : $logic:ty),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug)]
        $vis enum $name { $($variant),+ }
        impl $crate::ItemDispatch for $name {
            fn logic(kind: melee_types::ItemKind) -> &'static $crate::ItemLogicRow {
                $(if kind == <$logic as $crate::ItemLogic>::KIND { return &<$logic as $crate::ItemLogic>::LOGIC; })+
                unimplemented!("item kind {:?}", kind)
            }
        }
    };
}
