//! Fox Illusion / Falco Phantasm articles, itfoxillusion.c (8029CEB4..8029D948).
use melee_it::{desc::ItemAssets, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct FoxIllusion;
pub struct FalcoPhantasm;

/// it_803F6818: two travelling poses and the final trailing pose.
pub static STATES: [ItemStateRow; 3] = [
    ItemStateRow {
        animation_id: 0,
        animation: travel_animation,
        physics: travel_physics,
        collision,
    },
    ItemStateRow {
        animation_id: 1,
        animation: travel_animation,
        physics: travel_physics,
        collision,
    },
    ItemStateRow {
        animation_id: 2,
        animation: end_animation,
        physics: end_physics,
        collision,
    },
];

macro_rules! ghost {
    ($ty:ty, $kind:ident) => {
        impl ItemLogic for $ty {
            const KIND: ItemKind = ItemKind::$kind;
            const MODEL_COPIES: usize = 2;
            fn model_pose(
                item: &ItemCore,
                tree: &mut hsd_anim::jobj::JObjTree,
                copy: usize,
            ) -> bool {
                model_pose(item, tree, copy)
            }
            const STATES: &'static [ItemStateRow] = &STATES;
            // it_3F2F.c: both ghost kinds have a NULL picked_up callback.
            fn pickup_possible(_item: &ItemCore) -> bool {
                false
            }
            /// itFoxIllusion_Logic14_DmgDealt (8029CF8C): hitting a fighter
            /// never puts the ghost into hitlag (xCA8 = 0).
            fn damage_dealt(item: &mut ItemCore, _context: &ItemEventContext) -> bool {
                item.hitlag_damage = 0;
                false
            }
            fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
                // it_8029CFF0: owner air state chooses the hitbox script.
                item.scratch = ItemScratch::Afterimage(AfterimageState {
                    secondary_rotation: item.rotation,
                    ..Default::default()
                });
                item.life_timer = assets.special_attributes[0];
                item.change_motion(u16::from(item.ground_or_air == GroundOrAir::Air), assets);
            }
        }
    };
}
ghost!(FoxIllusion, FoxIllusion);
ghost!(FalcoPhantasm, FalcoPhantasm);

/// itFoxillusion_UnkMotion0_Anim (8029D094), shared by motion 1.
fn travel_animation(item: &mut ItemCore, ctx: &ItemAnimationContext<'_>) -> bool {
    let Some(owner) = ctx.owner.and_then(|owner| owner.illusion) else {
        return true;
    };
    if let ItemScratch::Afterimage(state) = &mut item.scratch {
        state.secondary_visible |= owner.create_secondary;
    }
    // Retail fsubs and direct store; no fused operations.
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        // it_8029D798: restart lifetime from the second attribute.
        item.life_timer = ctx.assets.special_attributes[1];
        item.change_motion(2, ctx.assets);
    }
    false
}
/// itFoxillusion_Phys: the primary article follows history entry one.
fn travel_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if let Some(owner) = ctx.owner.and_then(|owner| owner.illusion) {
        item.position = owner.positions[1];
        item.rotation.x = owner.rotations[1];
        if let ItemScratch::Afterimage(state) = &mut item.scratch {
            if state.secondary_visible {
                state.secondary_position = owner.positions[3];
                state.secondary_rotation.x = owner.rotations[3];
            }
        }
    }
}
/// itFoxillusion_UnkMotion2_Anim (8029D7EC): expire after the trailing lifetime.
fn end_animation(item: &mut ItemCore, ctx: &ItemAnimationContext<'_>) -> bool {
    if ctx.owner.and_then(|owner| owner.illusion).is_none() {
        return true;
    }
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        item.life_timer = 0.0;
        return true;
    }
    false
}
/// Motion 2 moves only the secondary display joint; Item.pos stays unchanged.
fn end_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if let (ItemScratch::Afterimage(state), Some(owner)) =
        (&mut item.scratch, ctx.owner.and_then(|o| o.illusion))
    {
        if state.secondary_visible {
            state.secondary_position = owner.positions[3];
        }
    }
}
fn collision(_item: &mut ItemCore, _ctx: &ItemCollisionContext) -> bool {
    false
}

/// it_8029CD78 / itFoxillusion_Phys: a second static model follows history entry 3.
fn model_pose(item: &ItemCore, tree: &mut hsd_anim::jobj::JObjTree, copy: usize) -> bool {
    let ItemScratch::Afterimage(state) = &item.scratch else {
        return false;
    };
    let (position, rotation, visible) = if copy == 0 {
        (item.position, item.rotation, item.motion != 2)
    } else {
        (
            state.secondary_position,
            state.secondary_rotation,
            state.secondary_visible,
        )
    };
    let root = hsd_anim::jobj::JObjId(0);
    tree.set_translate(root, &position);
    tree.set_rotation_x(root, rotation.x);
    tree.set_rotation_y(root, rotation.y);
    tree.set_rotation_z(root, rotation.z);
    tree.set_scale(root, &item.model_scale);
    visible
}
