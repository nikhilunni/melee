use crate::{
    desc::{ItemAssets, ItemCommonData},
    *,
};
use hsd_types::Vec3;
use melee_cmd::{Command, ScriptState};
use melee_coll::hitbox::HitCapsule;
use melee_types::ItemKind;

/// Checked port storage budget. Retail Item_8026784C does NOT cap hold-kind 8;
/// unlike the admission limits this is deliberately not called a retail bound.
pub const ITEM_CAPACITY: usize = 128;
pub const ITEM_GOBJ_LINK: u8 = 9;
pub const ITEM_GOBJ_PRIORITY: u8 = 0;
pub const ITEM_PROCESS_LINKS: [u8; 10] = [0, 1, 4, 5, 9, 11, 12, 13, 14, 16];
/// Stage offsets consumed by Item_802696CC. Items use a fixed 10,000-unit ceiling.
#[derive(Clone, Copy, Debug)]
pub struct ItemBounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}
#[derive(Clone, Debug, Default)]
pub struct RayState {
    pub angle: f32,
    pub speed: f32,
    pub scale: f32,
    pub previous_position: Vec3,
}
#[derive(Clone, Debug, Default)]
pub struct HeldState {
    pub visibility: i32,
    pub recoil_frame: usize,
    pub opening_frame: i32,
    pub opening_direction: i32,
    pub shot_pending: bool,
    pub opening_sound_played: bool,
}
#[derive(Clone, Debug)]
pub enum ItemScratch {
    Ray(RayState),
    Held(HeldState),
    None,
}
#[derive(Clone, Debug)]
pub struct ItemCore {
    pub ground_or_air: melee_types::GroundOrAir,
    pub id: u32,
    pub kind: ItemKind,
    pub owner: Option<u8>,
    pub stale_source: Option<melee_types::combat::AttackInstance>,
    pub hold_kind: u8,
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub nudge: Vec3,
    pub environmental_velocity: Vec3,
    pub platform_velocity: Vec3,
    pub facing: f32,
    pub rotation: Vec3,
    pub model_scale: Vec3,
    pub motion: u16,
    pub life_timer: f32,
    pub scale: f32,
    pub attached: bool,
    pub frozen: bool,
    pub destroyed: bool,
    pub pending_damage_dealt: i32,
    pub sound_requests: melee_types::fixed::FixedVec<u32, 8>,
    pub animation_frame: f32,
    pub script: ScriptState,
    pub command_variables: [u32; 4],
    pub hitboxes: [Option<HitCapsule>; 4],
    pub scratch: ItemScratch,
    pub hit_flags: [desc::ItemHitFlags; 4],
}
impl ItemCore {
    /// ftColl_80077C60 keeps the largest dealt damage until item link 14.
    pub fn record_damage_dealt(&mut self, damage: f32) {
        if damage > self.pending_damage_dealt as f32 {
            self.pending_damage_dealt = gekko_math::msl::fctiwz(damage);
        }
    }
    /// Item_80267130 -> it_80275E98 -> it_80276100, before kind initialization.
    /// Even a clear spawn path passes through mpColl's six-unit subdivisions;
    /// assigning the muzzle directly misses their float rounding boundary.
    pub fn initialize_collision(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        map: &mut melee_mp::CollMap,
    ) {
        let mut collision = melee_types::mp::CollData {
            cur_pos: spawn.previous_position,
            ..Default::default()
        };
        map.coll_data_init(&mut collision);
        // Item_802674AC/it_80275E98: implemented character articles use category5.
        assert_eq!(
            spawn.hold_kind, 8,
            "initial collision for other hold kinds is not ported"
        );
        collision.x34_flags.b1234 = 5;
        let b = assets.collision_box;
        melee_mp::set_ecb_source_fixed(
            &mut collision,
            b.top * self.scale,
            b.bottom * self.scale,
            b.right * self.scale,
            b.left * self.scale,
        );
        melee_mp::set_facing_dir(&mut collision, if self.facing == -1.0 { -1 } else { 1 });
        collision.x50 = assets.collision_damage_multiplier;
        collision.last_pos = spawn.position;
        melee_mp::mark_ecb_clear(&mut collision);
        if spawn.initial_collision {
            map.air_collide_pass(&mut collision, None);
        }
        self.position = collision.cur_pos;
    }
    pub fn change_motion(&mut self, motion: u16, assets: &ItemAssets) {
        self.motion = motion;
        self.animation_frame = 0.0;
        self.script = ScriptState::default();
        if assets
            .scripts
            .get(motion as usize)
            .is_some_and(|s| !s.is_empty())
        {
            self.script.restart(0);
            self.advance_script(assets);
        }
    }
    /// it_8027137C / it_8027129C: refresh world capsules at item s-link 11.
    pub fn update_hitboxes(&mut self) {
        let mut matrix = hsd_types::Mtx::default();
        hsd_anim::mtx::hsd_mtx_srt(
            &mut matrix,
            &self.model_scale,
            &self.rotation,
            &self.position,
            None,
        );
        for hit in self.hitboxes.iter_mut().flatten() {
            assert_eq!(
                hit.descriptor.bone, 0,
                "item non-root hitbox bone is not ported"
            );
            let mut position = Vec3::ZERO;
            hsd_anim::mtx::mtx_mult_vec(&matrix, &hit.descriptor.offset, &mut position);
            hit.update_position(position);
        }
    }
    /// Item_802799E4: shared command timing, item-owned application.
    fn advance_script(&mut self, assets: &ItemAssets) {
        let Some(script) = assets.scripts.get(self.motion as usize) else {
            return;
        };
        self.script.begin_frame(self.animation_frame, 1.0);
        while let Some(command) = self.script.next(script, 1.0) {
            match command {
                Command::SpawnHitbox { id, descriptor } => {
                    melee_coll::hitbox::spawn(&mut self.hitboxes, *id, descriptor);
                    let index = self
                        .script
                        .instruction
                        .expect("emitted command has continuation")
                        - 1;
                    self.hit_flags[*id] =
                        assets.hit_flags[self.motion as usize][index].expect("item hit flags");
                }
                Command::SetHitboxDamage { id, damage } => {
                    if let Some(hit) = &mut self.hitboxes[*id] {
                        hit.descriptor.damage = *damage;
                    }
                }
                Command::ClearHitbox(id) => {
                    self.hitboxes[*id] = None;
                    // it_80272560 updates every surviving capsule immediately.
                    self.update_hitboxes();
                }
                Command::ClearHitboxes => self.hitboxes.fill(None),
                Command::SetVariable { index, value } => self.command_variables[*index] = *value,
                _ => unimplemented!("item command {command:?}"),
            }
        }
    }
}
#[derive(Debug)]
pub struct ItemPool {
    // Reserve the complete pool on the heap at initialization. Inline storage
    // would copy hundreds of KiB through constructors on small test stacks.
    // spawn checks ITEM_CAPACITY before push, so this buffer never grows.
    items: Vec<ItemCore>,
    common: ItemCommonData,
    next_id: u32,
}
impl ItemPool {
    pub fn new(common: ItemCommonData) -> Self {
        Self {
            items: Vec::with_capacity(ITEM_CAPACITY),
            common,
            next_id: 0,
        }
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &ItemCore> {
        self.items.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut ItemCore> {
        self.items.iter_mut()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Item_80268B18 -> Item_8026862C. Equal p-link priority appends in spawn order.
    pub fn spawn<D: ItemDispatch>(&mut self, spawn: SpawnItem, assets: &ItemAssets) -> Option<u32> {
        if let Some(limit) = self.common.hold_limits[usize::from(spawn.hold_kind)] {
            if self
                .items
                .iter()
                .filter(|i| i.hold_kind == spawn.hold_kind)
                .count()
                >= limit
            {
                return None;
            }
        }
        assert!(
            self.len() < ITEM_CAPACITY,
            "explicit item storage budget exhausted"
        );
        let id = self.next_id;
        self.next_id += 1;
        let mut item = ItemCore {
            ground_or_air: spawn.ground_or_air,
            id,
            kind: spawn.kind,
            owner: spawn.owner,
            stale_source: spawn.stale_source,
            hold_kind: spawn.hold_kind,
            position: if spawn.initial_collision {
                spawn.previous_position
            } else {
                spawn.position
            },
            previous_position: spawn.previous_position,
            velocity: spawn.velocity,
            nudge: Vec3::ZERO,
            environmental_velocity: Vec3::ZERO,
            platform_velocity: Vec3::ZERO,
            facing: spawn.facing,
            rotation: Vec3::new(
                0.0,
                if assets.rotate_to_facing {
                    (std::f64::consts::FRAC_PI_2 * f64::from(spawn.facing)) as f32
                } else {
                    0.0
                },
                0.0,
            ),
            model_scale: [assets.scale; 3].into(),
            motion: 0,
            life_timer: self.common.lifetime,
            scale: assets.scale,
            attached: false,
            frozen: false,
            destroyed: false,
            pending_damage_dealt: 0,
            sound_requests: Default::default(),
            animation_frame: 0.0,
            script: ScriptState::default(),
            command_variables: [0; 4],
            hitboxes: std::array::from_fn(|_| None),
            scratch: ItemScratch::None,
            hit_flags: [desc::ItemHitFlags::default(); 4],
        };
        (D::logic(item.kind).spawned)(&mut item, assets);
        self.items.push(item);
        Some(id)
    }
    /// ftColl_80077C60 records xC34_damageDealt; item callbacks run at s-link14.
    pub fn record_damage_dealt(&mut self, id: u32, damage: f32) {
        if let Some(item) = self.get_mut(id) {
            item.record_damage_dealt(damage);
        }
    }
    /// Item_8026A294 -> OnGiveDamageThink, after all fighter/item detection.
    /// Multiple hits accumulate a maximum; one callback runs in this slot.
    pub fn process_events<D: ItemDispatch>(&mut self, id: u32) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if item.pending_damage_dealt != 0 {
            item.destroyed |=
                (D::logic(item.kind).damage_dealt)(item, &ItemEventContext::default());
        }
        // Item_80269CC4 resets per-frame contact accumulators.
        item.pending_damage_dealt = 0;
    }
    pub fn retire(&mut self, id: u32) {
        if let Some(item) = self.get_mut(id) {
            item.destroyed = true;
        }
    }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut ItemCore> {
        self.items.iter_mut().find(|i| i.id == id)
    }
    pub fn control<D: ItemDispatch>(&mut self, owner: u8, kind: ItemKind, control: ItemControl) {
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.owner == Some(owner) && i.kind == kind)
        {
            (D::logic(kind).control)(item, control);
        }
    }
    pub fn animate<D: ItemDispatch>(
        &mut self,
        id: u32,
        assets: &ItemAssets,
        owner: Option<&ItemOwner>,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if item.frozen {
            return;
        }
        item.animation_frame += 1.0;
        item.advance_script(assets);
        let row = D::logic(item.kind).states[item.motion as usize];
        item.destroyed |= (row.animation)(item, &ItemAnimationContext { owner, assets });
    }
    pub fn physics<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        bounds: &ItemBounds,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if !item.frozen {
            (D::logic(item.kind).states[item.motion as usize].physics)(
                item,
                &ItemPhysicsContext { owner },
            );
        }
        integrate(item, bounds);
    }
    /// it_8026E9A4 -> mpCheckAllRemap, used by ray state collision callbacks.
    pub fn stage_contact(&self, id: u32, map: &mut melee_mp::CollMap) -> bool {
        let Some(item) = self.items.iter().find(|i| i.id == id) else {
            return false;
        };
        let ItemScratch::Ray(ray) = &item.scratch else {
            return false;
        };
        map.check_all_remap(
            -1,
            -1,
            ray.previous_position.x,
            ray.previous_position.y,
            item.position.x,
            item.position.y,
        )
        .is_some()
    }
    pub fn collide<D: ItemDispatch>(&mut self, id: u32, stage_contact: bool) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        item.destroyed |= (D::logic(item.kind).states[item.motion as usize].collision)(
            item,
            &ItemCollisionContext { stage_contact },
        );
    }
    pub fn remove_destroyed<D: ItemDispatch>(&mut self) {
        let mut index = 0;
        while index < self.items.len() {
            if self.items[index].destroyed {
                let mut item = self.items.remove(index);
                (D::logic(item.kind).destroyed)(&mut item);
            } else {
                index += 1;
            }
        }
    }
}
/// Item_802697D4: audited retail uses four PSVECAdd calls; never reassociate
/// velocity+nudge, position+delta, environmental movement, platform movement.
fn integrate(item: &mut ItemCore, bounds: &ItemBounds) {
    if !item.attached && !item.frozen {
        let delta = add(item.velocity, item.nudge);
        item.position = add(item.position, delta);
    }
    // Item_802697D4 -> Item_802696CC, before environmental/platform movement.
    // Item_802680CC enables all four bounds; attachment skips this whole block.
    if !item.attached
        && (item.position.x > bounds.right
            || item.position.x < bounds.left
            || item.position.y > 10000.0
            || item.position.y < bounds.bottom)
    {
        item.destroyed = true;
        return;
    }
    item.position = add(item.position, item.environmental_velocity);
    item.position = add(item.position, item.platform_velocity);
}
fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

impl Clone for ItemPool {
    fn clone(&self) -> Self {
        Self {
            items: hsd_types::storage::clone_vec(&self.items),
            common: self.common.clone(),
            next_id: self.next_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct InertTestItem;
    impl ItemLogic for InertTestItem {
        const KIND: ItemKind = ItemKind::FoxBlaster;
        const STATES: &'static [ItemStateRow] = &[];
    }
    item_kinds! { enum TestKinds { Inert:InertTestItem } }
    fn assets() -> ItemAssets {
        ItemAssets {
            scripts: Vec::new(),
            hit_flags: Vec::new(),
            special_attributes: Vec::new(),
            scale: 1.0,
            model: 0,
            rotate_to_facing: false,
            collision_box: Default::default(),
            collision_damage_multiplier: 1.0,
        }
    }
    #[test]
    fn admission_limit_releases_capacity_without_reordering_survivors() {
        let _kind = TestKinds::Inert;
        let mut hold_limits = [None; 13];
        hold_limits[8] = Some(2);
        let mut pool = ItemPool::new(ItemCommonData {
            hold_limits,
            lifetime: 1.0,
        });
        let spawn = SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0);
        let first = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        let survivor = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        assert!(pool.spawn::<TestKinds>(spawn, &assets()).is_none());
        pool.retire(first);
        pool.remove_destroyed::<TestKinds>();
        let last = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        assert_eq!(
            pool.iter().map(|item| item.id).collect::<Vec<_>>(),
            [survivor, last]
        );
    }
    #[test]
    fn physics_keeps_retail_rounding_boundary_between_nudge_and_position() {
        let mut pool = ItemPool::new(ItemCommonData {
            hold_limits: [None; 13],
            lifetime: 1.0,
        });
        let spawn = SpawnItem::held(
            ItemKind::FoxBlaster,
            0,
            Vec3::new(16777216.0, 0.0, 0.0),
            1.0,
        );
        let id = pool.spawn::<TestKinds>(spawn, &assets()).unwrap();
        let item = pool.get_mut(id).unwrap();
        item.velocity.x = -16777216.0;
        item.nudge.x = -1.0;
        integrate(
            item,
            &ItemBounds {
                left: f32::NEG_INFINITY,
                right: f32::INFINITY,
                bottom: f32::NEG_INFINITY,
            },
        );
        assert_eq!(item.position.x.to_bits(), 0.0f32.to_bits());
    }
}
