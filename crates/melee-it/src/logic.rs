use crate::{desc::ItemAssets, ItemControl, ItemCore, ItemOwner};
use melee_types::ItemKind;

pub struct ItemAnimationContext<'a> {
    pub owner: Option<&'a ItemOwner>,
    pub assets: &'a ItemAssets,
}
pub struct ItemPhysicsContext<'a> {
    pub owner: Option<&'a ItemOwner>,
}
pub struct ItemCollisionContext {
    pub stage_contact: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemEventContext {
    pub reflected_facing: f32,
    pub shield_normal: hsd_types::Vec3,
}
#[derive(Clone, Copy)]
pub struct ItemStateRow {
    pub animation_id: i32,
    pub animation: fn(&mut ItemCore, &ItemAnimationContext<'_>) -> bool,
    pub physics: fn(&mut ItemCore, &ItemPhysicsContext<'_>),
    pub collision: fn(&mut ItemCore, &ItemCollisionContext) -> bool,
}
/// `it/kinds/types.h:26`: one implementation per ItemLogicTable row.
pub trait ItemLogic {
    const KIND: ItemKind;
    const STATES: &'static [ItemStateRow];
    fn spawned(_item: &mut ItemCore, _assets: &ItemAssets) {}
    fn destroyed(_item: &mut ItemCore) {}
    fn picked_up(_item: &mut ItemCore) {}
    fn dropped(_item: &mut ItemCore) {}
    fn thrown(_item: &mut ItemCore) {}
    fn entered_air(_item: &mut ItemCore) {}
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn damage_received(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn shield_bounced(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
        false
    }
    fn hit_shield(_item: &mut ItemCore, _context: &ItemEventContext) -> bool {
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
        spawned: Self::spawned,
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
    pub states: &'static [ItemStateRow],
    pub spawned: fn(&mut ItemCore, &ItemAssets),
    pub destroyed: fn(&mut ItemCore),
    pub picked_up: fn(&mut ItemCore),
    pub dropped: fn(&mut ItemCore),
    pub thrown: fn(&mut ItemCore),
    pub entered_air: fn(&mut ItemCore),
    pub damage_dealt: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub damage_received: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub reflected: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub clanked: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub absorbed: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub shield_bounced: fn(&mut ItemCore, &ItemEventContext) -> bool,
    pub hit_shield: fn(&mut ItemCore, &ItemEventContext) -> bool,
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
