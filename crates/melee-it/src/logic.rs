use crate::{desc::ItemAssets, ItemControl, ItemCore, ItemOwner};
use melee_types::ItemKind;

pub struct ItemAnimationContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    /// Present while the item is held (xDC8 x13).
    pub holder: Option<crate::ItemHolder<'a>>,
    pub map: &'a mut melee_mp::CollMap,
    pub assets: &'a ItemAssets,
}
pub struct ItemPhysicsContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    pub assets: &'a ItemAssets,
}
pub struct ItemCollisionContext<'a> {
    pub stage_contact: bool,
    pub map: &'a mut melee_mp::CollMap,
    pub assets: &'a ItemAssets,
}
#[derive(Clone, Copy)]
pub struct ItemEventContext<'a> {
    pub reflected_facing: f32,
    pub shield_normal: hsd_types::Vec3,
    pub assets: &'a ItemAssets,
}
impl<'a> ItemEventContext<'a> {
    pub fn new(assets: &'a ItemAssets) -> Self {
        Self {
            reflected_facing: 0.0,
            shield_normal: hsd_types::Vec3::ZERO,
            assets,
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
    fn spawned(_item: &mut ItemCore, _assets: &ItemAssets) {}
    fn destroyed(_item: &mut ItemCore) {}
    fn picked_up(_item: &mut ItemCore, _context: &mut ItemAnimationContext<'_>) {}
    fn dropped(_item: &mut ItemCore) {}
    fn thrown(_item: &mut ItemCore) {}
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
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
    fn control(_item: &mut ItemCore, _control: ItemControl) {
        unimplemented!("item control for this kind")
    }
    const LOGIC: ItemLogicRow = ItemLogicRow {
        states: Self::STATES,
        held_part: Self::HELD_PART,
        held_scale: Self::HELD_SCALE,
        model_pose: Self::model_pose,
        model_copies: Self::MODEL_COPIES,
        spawned: Self::spawned,
        pickup_possible: Self::pickup_possible,
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
        owner_removed: Self::owner_removed,
        control: Self::control,
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
    pub pickup_possible: fn(&ItemCore) -> bool,
    pub destroyed: fn(&mut ItemCore),
    pub picked_up: fn(&mut ItemCore, &mut ItemAnimationContext<'_>),
    pub dropped: fn(&mut ItemCore),
    pub thrown: fn(&mut ItemCore),
    pub entered_air: fn(&mut ItemCore),
    pub damage_dealt: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub damage_received: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub reflected: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub clanked: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub absorbed: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub shield_bounced: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub hit_shield: fn(&mut ItemCore, &ItemEventContext<'_>) -> bool,
    pub owner_removed: fn(&mut ItemCore, u8),
    pub control: fn(&mut ItemCore, ItemControl),
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
