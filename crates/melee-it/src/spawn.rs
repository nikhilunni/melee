use hsd_types::Vec3;
use melee_types::{GroundOrAir, ItemKind};

/// Owned counterpart of `it/types.h:689`, retail `SpawnItem` (0x4C bytes).
/// Parent GObjs become player slots; memory offsets stay out of the tick path.
#[derive(Clone, Copy, Debug)]
pub struct SpawnItem {
    pub owner: Option<u8>,
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
}
#[derive(Clone, Copy, Debug)]
pub enum ItemRequest {
    Spawn(SpawnItem),
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
}

/// Fighter-owned inputs sampled for the item callback; no fighter dependency.
#[derive(Clone, Copy, Debug)]
pub struct IllusionOwner {
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
}
