use crate::{
    desc::{ItemAssets, ItemCommonData},
    *,
};
use hsd_types::Vec3;
use melee_cmd::{Command, ScriptState};
use melee_coll::hitbox::HitCapsule;
use melee_types::{GroundOrAir, ItemKind};

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
    pub opening_pose_frame: usize,
    pub recoil_pose_frame: usize,
    pub visibility: i32,
    pub recoil_frame: usize,
    pub opening_frame: i32,
    pub opening_direction: i32,
    pub shot_pending: bool,
    pub opening_sound_played: bool,
}
#[derive(Clone, Debug, Default)]
pub struct AfterimageState {
    pub secondary_visible: bool,
    pub secondary_position: Vec3,
    pub secondary_rotation: Vec3,
}
/// Requests an item makes of the scene during a proc, drained after it.
#[derive(Clone, Debug)]
pub enum ItemEvent {
    /// efSync_Spawn(id, gobj, &pos): a world-space effect.
    Effect { id: u16, position: Vec3 },
    /// efSync_Spawn(0x3E8, gobj, &pos, &damage): the spark of a hit landing
    /// on the item (it_80270E30).
    HitSpark { position: Vec3, damage: f32 },
    /// Script opcode 10 (it_80278F2C): an effect at a joint whose offset gets
    /// a random spread (it_80278800) when the scene resolves it.
    ScriptEffect(melee_types::combat::GraphicsCommand),
    /// EF_SPAWN_CAMERA_SHAKE: Camera_RequestQuake(kind) at a joint offset.
    Quake {
        kind: u16,
        joint: usize,
        offset: Vec3,
    },
    /// lb_800119DC: a radial gust.
    Gust {
        center: Vec3,
        frames: i32,
        strength: f32,
        decay: f32,
        phase_step: f32,
    },
}

/// Item_StateChangeFlags (it/forward.h) that Item_80268E5C consults.
pub mod state_change {
    pub const ANIM_UPDATE: u32 = 1 << 1;
    pub const HIT_PRESERVE: u32 = 1 << 4;
    pub const CMD_UPDATE: u32 = 1 << 8;
}

#[derive(Clone, Debug)]
pub enum ItemScratch {
    Afterimage(AfterimageState),
    Ray(RayState),
    Held(HeldState),
    Bomb(BombState),
    None,
}
/// Item.xDD4_itemVar.bombhei (itbombhei.c).
#[derive(Clone, Debug, Default)]
pub struct BombState {
    /// xDD4: the current phase's frame countdown.
    pub countdown: i32,
    /// xDD8: the blink's scale direction, +1 or -1.
    pub blink_direction: i32,
    /// xDDC: it_80280B60 has run.
    pub exploded: bool,
    /// xDE0: the fuse is lit.
    pub lit: bool,
    /// xDE4: walked before (it_8027E978).
    pub walked: bool,
    /// xDE8: the thrown scale factor.
    pub throw_scale: f32,
    /// xDEC: frames left in the whole life, counting every phase.
    pub life_frames: f32,
    /// xDF0: frames left on the lit fuse.
    pub fuse: f32,
    /// xDF4: the facing to restore after turning.
    pub turn_facing: f32,
    /// xDF8 / xDFC: per-frame squash and tilt while waking.
    pub squash_step: f32,
    pub tilt_step: f32,
    /// xE04: the per-frame turn.
    pub turn_step: f32,
    /// xE0C: the velocity before this frame's landing.
    pub landing_velocity: Vec3,
}
#[derive(Clone, Debug)]
pub struct ItemCore {
    pub ground_or_air: melee_types::GroundOrAir,
    pub id: u32,
    pub kind: ItemKind,
    pub owner: Option<u8>,
    pub stale_source: Option<melee_types::combat::AttackInstance>,
    /// Current owner factor used only when authoring/re-authoring a hitbox.
    pub stale_multiplier: f32,
    pub pending_reflection: Option<PendingReflection>,
    pub reflection_direction: f32,
    pub reflection_history: [melee_types::fixed::FixedVec<RehitVictim, 12>; 4],
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
    pub frozen: bool,
    pub destroyed: bool,
    pub pending_damage_dealt: i32,
    pub pending_damage_without_hitlag: i32,
    /// ftColl_80077688's xC50; shield contact has priority at item link 14.
    pub pending_shield_damage: i32,
    pub pending_shield_deflection: Option<melee_lb::shield::ShieldDeflection>,
    /// xCC0: a counter-style shield's own hitlag for this item (the
    /// defender's shield_unk1), which overrides the damage formula.
    pub pending_shield_hitlag: f32,
    /// xC48: the strongest clank against a fighter hitbox this frame
    /// (ftColl_80077970).
    pub pending_clank_damage: i32,
    /// ItemAttr x1_5 (xDC8 xC): contacts put this kind into hitlag.
    pub hitlag_enabled: bool,
    /// xCA8: this frame's contact damage that sets hitlag. Event callbacks
    /// may clear it (itFoxIllusion_Logic14_DmgDealt).
    pub hitlag_damage: i32,
    /// xCBC: hitlag frames left.
    pub hitlag_frames: f32,
    /// xDC8 x9: animation, physics, movement and accessory are paused.
    pub in_hitlag: bool,
    pub sound_requests: melee_types::fixed::FixedVec<u32, 8>,
    pub animation_frame: f32,
    pub script: ScriptState,
    pub command_variables: [u32; 4],
    pub hitboxes: [Option<HitCapsule>; 4],
    pub scratch: ItemScratch,
    pub hit_flags: [desc::ItemHitFlags; 4],
    /// x378_itemColl, created by it_80275E98 at spawn.
    pub collision: Option<melee_types::mp::CollData>,
    /// xDC8 x17: the model rotation axis the fixed ECB follows (0 Z, 1 X, else Y).
    pub rotation_axis: u8,
    /// xC30: the floor (or wall/ceiling) line of the last map contact.
    pub floor_line: i32,
    /// xD5C: it_80277544's fall-through-platform state.
    pub platform_drop: u32,
    /// JOBJ_HIDDEN on the model (it_80280B60).
    pub hidden: bool,
    /// spin_spd: ItemAttr xC, degrees-per-frame scale of the spin.
    pub spin_rate: f32,
    /// xD3C_spinSpeed: radians added to the model rotation per airborne frame.
    pub spin_speed: f32,
    /// xDC8 x19: the spin keeps its sign regardless of facing.
    pub spin_ignores_facing: bool,
    /// xDC8 x15 (it_8026B390 / it_8026B3A8): Item_IsGrabbable requires it,
    /// and it gates the common lifetime countdown (Item_80269528).
    pub grabbable: bool,
    /// xBCC / xBD4 (ItemAttr x30 / x38): the pickup box's offset from the
    /// item's position and its half extents (ftpickupitem_800942A0).
    pub grab_offset: hsd_types::Vec2,
    pub grab_range: hsd_types::Vec2,
    /// xDC8 x13 (it_802742F4): held by its owner. A held item neither moves
    /// nor leaves the blast zones.
    pub held: bool,
    /// xDC4: the holder's part it hangs from, a fp->parts index (the
    /// character's own joint numbering, as ftData x8 stores it).
    pub holder_part: u8,
    /// xDCD b5 (it_80275444 / it_80275474): hitboxes may hit the owner.
    pub hits_owner: bool,
    /// HSD_GObj_804D7838->s_link > 11: the running proc comes after item
    /// link 11's capsule refresh (it_802790C0).
    pub past_hitbox_refresh: bool,
    /// xD0C == 2 (it_802756D0 / it_802756E0): hurtboxes take no hits.
    pub hurt_intangible: bool,
    /// xDCE b0 (it_802754D4): the owner's hits land too, once dropped or thrown.
    pub hurt_by_owner: bool,
    /// xDCD b7 (it_80275444 / it_80275474): hitboxes reach items whose owner
    /// matches (both unowned counts), as a blast does.
    pub strikes_kindred_items: bool,
    /// Damage taken: xC9C (its percent, capped at 999), and this frame's
    /// total xCA0 and largest hit xCA4.
    pub damage_percent: i32,
    pub pending_damage_taken: i32,
    pub largest_damage_taken: i32,
    /// it_80270E30's result for this frame: xCC8, xCAC, xCCC and xCB0.
    pub pending_knockback: f32,
    pub knockback_angle: u16,
    pub hit_direction: f32,
    pub hit_by: Option<u8>,
    /// efAsync requests queued below s_link 9, flushed at link 9 (Item_80269A9C).
    pub queued_events: melee_types::fixed::FixedVec<ItemEvent, 4>,
    pub events: melee_types::fixed::FixedVec<ItemEvent, 8>,
}
/// lb_8000B804: the model root's authored rotation.
pub(crate) fn rest_rotation(assets: &ItemAssets) -> Vec3 {
    let r = assets.visual.model.rotation;
    Vec3::new(r.x, r.y, r.z)
}
/// The tag that keeps item victim ids apart from fighter spawn numbers.
const ITEM_VICTIM: u32 = 1 << 31;

impl ItemCore {
    /// checkHitLag / EnterHitlagThink (Item_8026A294): hitlag frames from
    /// it_8026B424 (retail 8026B450: fmadds, then fctiwz), never lowered.
    fn enter_hitlag(&mut self, common: &desc::ItemCommonData, damage: i32) {
        let frames = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(
            damage as f32,
            common.hitlag_scale,
            common.hitlag_base,
        )) as f32;
        self.enter_hitlag_frames(frames);
    }
    /// checkHitLag (Item_8026A294): raise the hitlag to `frames`.
    fn enter_hitlag_frames(&mut self, frames: f32) {
        if self.hitlag_frames < frames {
            self.hitlag_frames = frames;
        }
        // No supported kind has an entered_hitlag callback.
        self.in_hitlag = true;
    }
    /// How fighter hitboxes list this item among their victims. Retail
    /// stores gobj pointers; fighters are keyed by spawn number, items by
    /// their id with the top bit set.
    pub fn hitbox_victim(&self) -> u32 {
        ITEM_VICTIM | self.id
    }
    /// ftColl_80077C60 keeps the largest dealt damage until item link 14.
    pub fn record_damage_dealt(&mut self, damage: f32) {
        if damage > self.pending_damage_dealt as f32 {
            self.pending_damage_dealt = gekko_math::msl::fctiwz(damage);
        }
    }
    /// it_802742F4 (802742F4), Item_8026AB54's attachment before the kind's
    /// pickup callback: the model returns to its rest pose at the item's
    /// scale, `owner` holds it at `part`, pickup and the lifetime countdown
    /// stop, and a common item's lifetime restarts at `lifetime`.
    /// it_80273168's pickup sound and the hand constraint are presentation.
    pub fn attach_to_holder(&mut self, owner: u8, part: u8, assets: &ItemAssets, lifetime: f32) {
        // lb_8000B804, then Item_8026849C.
        self.rotation = rest_rotation(assets);
        self.model_scale = Vec3::new(self.scale, self.scale, self.scale);
        // ftLib_80086960: fighters only; xDC8 x0 (a secondary owner) is
        // never set on a pickup-capable kind here.
        self.owner = Some(owner);
        self.held = true;
        self.holder_part = part;
        // it_802756D0: a held item takes no hits.
        self.hurt_intangible = true;
        // it_8026B3A8.
        self.grabbable = false;
        if (self.kind as u32) < ItemKind::LGunRay as u32 {
            // it_80275158: xD48's half-life copy only drives the warning blink.
            self.life_timer = lifetime;
        }
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
    /// it_80274658 (80274658): the spin speed for `degrees` per frame at
    /// spin_spd, reversed when the item moves against its facing and, unless
    /// xDC8 x19 is set, signed by the facing. retail 80274674..7C: fmuls
    /// 0.01*spin_spd and (pi/180)*degrees separately, then their product.
    pub fn update_spin(&mut self, degrees: f32) {
        let mut speed = 0.0;
        if self.spin_rate != 0.0 {
            speed = (0.01 * self.spin_rate) * (0.017_453_292 * degrees);
        }
        self.spin_speed = speed;
        let facing = if self.facing < 0.0 { -1 } else { 1 };
        let moving = if self.velocity.x < 0.0 { -1 } else { 1 };
        if moving != facing {
            self.spin_speed = -self.spin_speed;
        }
        if !self.spin_ignores_facing {
            self.spin_speed *= -self.facing;
        }
    }

    /// it_80274A64 (80274A64): add the spin about xDC8 x17's axis (retail
    /// 80274AE4 and siblings: fadds into the JObj rotation).
    fn spin(&mut self) {
        match self.rotation_axis {
            0 => self.rotation.z += self.spin_speed,
            1 => self.rotation.x += self.spin_speed,
            _ => self.rotation.y += self.spin_speed,
        }
    }

    /// Item_80268E5C (80268E5C) with its flags: the frame restarts; without
    /// HIT_PRESERVE any live hitbox is cleared (it_802725D4); ANIM_UPDATE or
    /// CMD_UPDATE restart the script, which then runs its first step; a state
    /// without an article row has no script. Other flags only touch the model.
    pub fn change_motion_with(
        &mut self,
        motion: u16,
        article_state: i32,
        flags: u32,
        assets: &ItemAssets,
    ) {
        self.motion = motion;
        self.animation_frame = 0.0;
        if flags & state_change::HIT_PRESERVE == 0 && self.hitboxes.iter().any(Option::is_some) {
            self.hitboxes.fill(None);
            for history in &mut self.reflection_history {
                history.clear();
            }
        }
        if article_state < 0 {
            self.script = ScriptState::default();
            return;
        }
        if flags & (state_change::ANIM_UPDATE | state_change::CMD_UPDATE) != 0 {
            self.script = ScriptState::default();
            if assets
                .scripts
                .get(motion as usize)
                .is_some_and(|s| !s.is_empty())
            {
                self.script.restart(0);
            }
        }
        self.advance_script(assets);
    }
    /// it_8027137C / it_8027129C: refresh world capsules at item s-link 11.
    pub fn update_hitboxes(&mut self) {
        for id in 0..self.hitboxes.len() {
            self.update_hitbox(id);
        }
    }
    /// it_8027129C (8027129C): one capsule's position from its bone.
    fn update_hitbox(&mut self, id: usize) {
        let Some(hit) = &mut self.hitboxes[id] else {
            return;
        };
        assert_eq!(
            hit.descriptor.bone, 0,
            "item non-root hitbox bone is not ported"
        );
        let mut matrix = hsd_types::Mtx::default();
        hsd_anim::mtx::hsd_mtx_srt(
            &mut matrix,
            &self.model_scale,
            &self.rotation,
            &self.position,
            None,
        );
        let mut position = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(&matrix, &hit.descriptor.offset, &mut position);
        hit.update_position(position);
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
                    if self.hitboxes[*id]
                        .as_ref()
                        .is_none_or(|h| h.descriptor.group != descriptor.group)
                    {
                        self.reflection_history[*id] = self
                            .hitboxes
                            .iter()
                            .position(|h| {
                                h.as_ref()
                                    .is_some_and(|h| h.descriptor.group == descriptor.group)
                            })
                            .map(|i| self.reflection_history[i].clone())
                            .unwrap_or_default();
                    }
                    melee_coll::hitbox::spawn(&mut self.hitboxes, *id, descriptor);
                    let hit = self.hitboxes[*id].as_mut().unwrap();
                    hit.descriptor.damage *= self.stale_multiplier;
                    // it_802790C0 -> it_80275594(1 / scl): the capsule radius is
                    // stored unscaled; contacts multiply the item scale back in.
                    hit.descriptor.radius *= 1.0 / self.scale;
                    let index = self
                        .script
                        .instruction
                        .expect("emitted command has continuation")
                        - 1;
                    self.hit_flags[*id] =
                        assets.hit_flags[self.motion as usize][index].expect("item hit flags");
                    // it_802790C0: created after link 11's refresh, the
                    // capsule is placed at once.
                    if self.past_hitbox_refresh {
                        self.update_hitbox(*id);
                    }
                }
                Command::SetHitboxDamage { id, damage } => {
                    if let Some(hit) = &mut self.hitboxes[*id] {
                        hit.knockback_damage = gekko_math::msl::fctiwz(*damage) as u32;
                        hit.descriptor.damage = hit.knockback_damage as f32 * self.stale_multiplier;
                    }
                }
                Command::ClearHitbox(id) => {
                    self.hitboxes[*id] = None;
                    self.reflection_history[*id].clear();
                    // it_80272560 updates every surviving capsule immediately.
                    self.update_hitboxes();
                }
                Command::ClearHitboxes => {
                    self.hitboxes.fill(None);
                    for history in &mut self.reflection_history {
                        history.clear();
                    }
                }
                Command::SetVariable { index, value } => self.command_variables[*index] = *value,
                Command::Graphics(graphics) => {
                    self.events.push(ItemEvent::ScriptEffect(graphics.clone()))
                }
                // it_8027990C: controller rumble only.
                Command::Rumble { .. } => {}
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
    /// it_804D6D28: ItCo's common item data.
    pub fn common(&self) -> &ItemCommonData {
        &self.common
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
        self.spawn_with_stale::<D>(spawn, assets, 1.0)
    }
    pub fn spawn_with_stale<D: ItemDispatch>(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        stale_multiplier: f32,
    ) -> Option<u32> {
        assert_eq!(
            assets.camera_kind, 0,
            "item.c foobar3: items that the camera frames are not ported"
        );
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
            stale_multiplier,
            pending_reflection: None,
            reflection_direction: 0.0,
            reflection_history: Default::default(),
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
            frozen: false,
            destroyed: false,
            pending_damage_dealt: 0,
            pending_damage_without_hitlag: 0,
            pending_shield_damage: 0,
            pending_shield_deflection: None,
            pending_shield_hitlag: 0.0,
            pending_clank_damage: 0,
            hitlag_enabled: assets.hitlag,
            hitlag_damage: 0,
            hitlag_frames: 0.0,
            in_hitlag: false,
            sound_requests: Default::default(),
            animation_frame: 0.0,
            script: ScriptState::default(),
            command_variables: [0; 4],
            hitboxes: std::array::from_fn(|_| None),
            scratch: ItemScratch::None,
            hit_flags: [desc::ItemHitFlags::default(); 4],
            collision: None,
            rotation_axis: assets.rotation_axis,
            floor_line: -1,
            platform_drop: 0,
            hidden: false,
            spin_rate: assets.spin_rate,
            spin_speed: 0.0,
            spin_ignores_facing: assets.rotate_to_facing,
            grabbable: false,
            grab_offset: assets.grab_offset,
            grab_range: assets.grab_range,
            held: false,
            holder_part: 0,
            hits_owner: false,
            past_hitbox_refresh: false,
            hurt_intangible: false,
            hurt_by_owner: false,
            strikes_kindred_items: false,
            damage_percent: 0,
            pending_damage_taken: 0,
            largest_damage_taken: 0,
            pending_knockback: 0.0,
            knockback_angle: 0,
            hit_direction: 0.0,
            hit_by: None,
            queued_events: Default::default(),
            events: Default::default(),
        };
        // Item_80267130 -> it_80274658(x6C) before the kind's spawn callback.
        item.update_spin(self.common.spawn_spin_degrees);
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
    pub fn process_events<D: ItemDispatch>(&mut self, id: u32, assets: &ItemAssets) {
        self.process_events_with_stale::<D>(id, 1.0, assets);
    }
    pub fn process_events_with_stale<D: ItemDispatch>(
        &mut self,
        id: u32,
        reflected_stale: f32,
        assets: &ItemAssets,
    ) {
        let cap = self.common.maximum_reflected_damage;
        // retail 80269E18/20: add then multiply, no FMA or double promotion.
        let bounce_limit =
            (std::f32::consts::PI / 180.0) * (90.0 + self.common.shield_bounce_degrees);
        let common = self.common.clone();
        let Some(item) = self.get_mut(id) else {
            return;
        };
        item.past_hitbox_refresh = true;
        if item.pending_knockback != 0.0 || item.pending_damage_taken != 0 {
            // OnTakeDamageThink: the percent grows (capped at 999) and the
            // frame's damage becomes the hitlag damage (xCA8), unconditionally.
            item.damage_percent = (item.damage_percent + item.pending_damage_taken).min(999);
            item.hitlag_damage = item.pending_damage_taken;
            item.destroyed |=
                (D::logic(item.kind).damage_received)(item, &ItemEventContext::new(assets));
        } else if item.pending_shield_damage != 0 {
            if let Some(deflection) = item.pending_shield_deflection.filter(|d| {
                item.ground_or_air == melee_types::GroundOrAir::Air && d.angle < bounce_limit
            }) {
                let context = ItemEventContext {
                    shield_normal: deflection.normal,
                    ..ItemEventContext::new(assets)
                };
                item.destroyed |= (D::logic(item.kind).shield_bounced)(item, &context);
            } else {
                if item.hitlag_enabled {
                    item.hitlag_damage = item.pending_shield_damage;
                }
                item.destroyed |=
                    (D::logic(item.kind).hit_shield)(item, &ItemEventContext::new(assets));
            }
        } else if item.pending_clank_damage != 0 {
            // OnClankThink: the clank damage becomes the hitlag damage.
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_clank_damage;
            }
            item.destroyed |= (D::logic(item.kind).clanked)(item, &ItemEventContext::new(assets));
        } else if item.pending_damage_dealt != 0 || item.pending_damage_without_hitlag != 0 {
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_damage_dealt;
            }
            item.destroyed |=
                (D::logic(item.kind).damage_dealt)(item, &ItemEventContext::new(assets));
        } else if let Some(reflection) = item.pending_reflection {
            item.reflect::<D>(reflection, reflected_stale, cap, assets);
        }
        // Item_8026A294: a surviving item enters hitlag from xCA8. (The xCC0
        // path is a counter-style shield's own hitlag; no supported fighter
        // sets Fighter.shield_unk1 while an item can reach it.)
        if !item.destroyed {
            // xDC8 xD is set for every item, so a counter's own hitlag wins.
            if item.pending_shield_hitlag > 0.0 {
                let frames = item.pending_shield_hitlag;
                item.enter_hitlag_frames(frames);
            } else if item.hitlag_damage != 0 {
                let damage = item.hitlag_damage;
                item.enter_hitlag(&common, damage);
            }
        }
        item.past_hitbox_refresh = false;
        // Item_80269CC4 resets per-frame contact accumulators.
        item.pending_damage_taken = 0;
        item.largest_damage_taken = 0;
        item.pending_knockback = 0.0;
        item.hit_direction = 0.0;
        item.pending_reflection = None;
        item.reflection_direction = 0.0;
        item.pending_damage_dealt = 0;
        item.pending_damage_without_hitlag = 0;
        item.pending_shield_damage = 0;
        item.pending_shield_deflection = None;
        item.pending_shield_hitlag = 0.0;
        item.pending_clank_damage = 0;
        item.hitlag_damage = 0;
    }
    /// Item_802693E4 (802693E4), item link 0: count hitlag down and resume.
    pub fn advance_hitlag(&mut self, id: u32) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if item.hitlag_frames > 0.0 {
            item.hitlag_frames -= 1.0;
            if item.hitlag_frames <= 0.0 {
                item.hitlag_frames = 0.0;
                // Item_8026A1E8: no supported kind has an exited_hitlag callback.
                item.in_hitlag = false;
            }
        }
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
        holder: Option<crate::ItemHolder<'_>>,
        map: &mut melee_mp::CollMap,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        // Item_80269528: hitlag pauses the animation and its callback.
        if !(item.frozen || item.in_hitlag) {
            item.animation_frame += 1.0;
            item.advance_script(assets);
            let row = D::logic(item.kind).states[item.motion as usize];
            item.destroyed |= (row.animation)(
                item,
                &mut ItemAnimationContext {
                    owner,
                    holder,
                    map,
                    assets,
                },
            );
            if item.destroyed {
                return;
            }
        }
        // Item_80269528's common-item lifetime: xDC8 x15 on a kind below
        // It_Kind_L_Gun_Ray (xDD0 b3, set on explosion, clears x15 here).
        // The x34 warning blink (it_802728C8) is visual.
        if item.grabbable && (item.kind as u32) < ItemKind::LGunRay as u32 {
            item.life_timer -= 1.0;
            if item.life_timer <= 0.0 {
                item.destroyed = true;
            }
        }
    }
    pub fn physics<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        bounds: &ItemBounds,
        assets: &ItemAssets,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if !item.frozen && !item.in_hitlag {
            (D::logic(item.kind).states[item.motion as usize].physics)(
                item,
                &ItemPhysicsContext { owner, assets },
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
    pub fn collide<D: ItemDispatch>(
        &mut self,
        id: u32,
        stage_contact: bool,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        item.destroyed |= (D::logic(item.kind).states[item.motion as usize].collision)(
            item,
            &mut ItemCollisionContext {
                stage_contact,
                map,
                assets,
            },
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
    if !item.held && !item.frozen && !item.in_hitlag {
        let delta = add(item.velocity, item.nudge);
        item.position = add(item.position, delta);
    }
    // Item_802697D4 -> Item_802696CC, before environmental/platform movement.
    // Item_802680CC enables all four bounds; attachment skips this whole block.
    if !item.held
        && (item.position.x > bounds.right
            || item.position.x < bounds.left
            || item.position.y > 10000.0
            || item.position.y < bounds.bottom)
    {
        item.destroyed = true;
        return;
    }
    // Item_802697D4 -> it_80274A64: an airborne item spins about its axis.
    if !item.held && item.spin_speed != 0.0 && item.ground_or_air == GroundOrAir::Air {
        item.spin();
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
            visual: Default::default(),
            scripts: Vec::new(),
            hit_flags: Vec::new(),
            special_attributes: Vec::new(),
            scale: 1.0,
            model: 0,
            rotate_to_facing: false,
            collision_box: Default::default(),
            collision_damage_multiplier: 1.0,
            hitlag: false,
            camera_kind: 0,
            fall_acceleration: 0.0,
            fall_speed_limit: 0.0,
            rotation_axis: 0,
            explosion_lifetime: 0.0,
            spin_rate: 0.0,
            fall_spin_degrees: 0.0,
            animation_ends: Vec::new(),
            hurtboxes: Vec::new(),
            release_box_scale: 0.0,
            heavy: false,
            use_kind: 0,
            hand_hold_kind: 0,
            throw_speed_multiplier: 1.0,
            grab_offset: hsd_types::Vec2::ZERO,
            grab_range: hsd_types::Vec2::ZERO,
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
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
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
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
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
