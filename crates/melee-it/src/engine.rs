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
    /// Stage_GetBlastZoneTopOffset: read by kinds that watch the blast zone
    /// themselves (it_802D9714); Item_802696CC's ceiling is fixed.
    pub top: f32,
    /// Stage_UnkSetVec3TCam_Offset: the camera's centre (cam_x_offset,
    /// cam_y_offset).
    pub camera_offset: hsd_types::Vec2,
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
    /// efSync_Spawn(id, gobj, jobj): generators that follow the item's own
    /// root JObj (Toad's spores, 0x4D3).
    OwnEffect { id: u16 },
    /// efLib_DestroyAll(gobj) on a living item: the generators on its JObj
    /// go (the Thunder Jolt ball's trail).
    DestroyEffects,
    /// ItemSwitch -> it_8027327C -> it_802787B4 (802787B4): the kind's
    /// destroy effect at the item's root, through it_80278800's zero-range
    /// offset spread (three HSD_Randf draws). `root` is the JObj translation
    /// when it trails the item's position (a collision destroy); otherwise
    /// the item's position when the event is handled.
    DestroyEffect { id: u16, root: Option<Vec3> },
    /// efSync_Spawn(0x3E8, gobj, &pos, &damage): the spark of a hit landing
    /// on the item (it_80270E30).
    HitSpark { position: Vec3, damage: f32 },
    /// efSync_Spawn(0x3EC, item, &pos, item): a slashing hit's spark on a
    /// stage enemy or Pokemon (it_80270E30).
    SlashSpark { position: Vec3 },
    /// Script opcode 10 (it_80278F2C): an effect at a joint whose offset gets
    /// a random spread (it_80278800) when the scene resolves it.
    ScriptEffect(melee_types::combat::GraphicsCommand),
    /// EF_SPAWN_CAMERA_SHAKE: Camera_RequestQuake(kind) at a joint offset.
    Quake {
        kind: u16,
        joint: usize,
        offset: Vec3,
    },
    /// efAsync kind 2: an effect at a joint offset, dispatched from the
    /// item's queue (it_80278800's default path).
    JointEffect { id: u16, joint: usize, offset: Vec3 },
    /// efAsync kinds 1 (EF_SPAWN_POS) and 4 (EF_SPAWN_POS_PARAM) on the
    /// model root: effect `id` at the root's world translation when the
    /// request is processed, with kind 4's float parameter.
    RootEffect { id: u16, parameter: Option<f32> },
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
    /// ITEM_DROP_UPDATE: hitboxes take the throw speed as a damage scale.
    pub const DROP_UPDATE: u32 = 1 << 2;
    /// ITEM_MODEL_UPDATE (it_80274740): the spin stops and its angle resets.
    pub const MODEL_UPDATE: u32 = 1 << 3;
    pub const HIT_PRESERVE: u32 = 1 << 4;
    pub const CMD_UPDATE: u32 = 1 << 8;
}

#[derive(Clone, Debug)]
pub enum ItemScratch {
    Afterimage(AfterimageState),
    Ray(RayState),
    Held(HeldState),
    Bomb(BombState),
    Heiho(HeihoState),
    Turnip(TurnipState),
    Jolt(JoltState),
    Thunder(ThunderState),
    None,
}
/// Item.xDD4_itemVar.pikachuthunder (itpikachuthunder.c): one bolt of
/// Pikachu's Thunder chain; the partner is the next bolt (x34).
#[derive(Clone, Copy, Debug, Default)]
pub struct ThunderState {
    /// x0: the bolt's place in the chain; the lead is 0.
    pub index: i32,
    /// x4: the bolt struck (its lead reached it, or it met the floor).
    pub struck: bool,
    /// x8: frames before it starts falling.
    pub delay: i32,
    /// xC / x10: the bolt's length before and after this frame's shrink.
    pub length: f32,
    pub next_length: f32,
    /// x14: the struck bolt's lifetime.
    pub strike_frames: f32,
    /// x18: its model's y scale.
    pub scale: f32,
    /// x1C: the fall's velocity.
    pub velocity: Vec3,
    /// x28: where the bolt before it stopped.
    pub reached: Vec3,
}
/// Item.xDD4_itemVar.pikachujoltground and .pikachujoltair
/// (itpikachutjoltground.c / itpikachutjoltair.c): the Thunder Jolt ball
/// and the crawler it rides along a surface.
#[derive(Clone, Copy, Debug, Default)]
pub struct JoltState {
    /// Ball xDD4: the flight's angle.
    pub angle: f32,
    /// Ball xDE8: the position before this frame's move, the ray's start.
    pub previous_position: Vec3,
    /// Ball xDF4, crawler xDE8: the surface's normal (the crawler's
    /// rotation follows it).
    pub normal: Vec3,
    /// Ball xE00: the normal before the last contact.
    pub previous_normal: Vec3,
    /// Ball xDE4: frames since the last contact; crawler xDF4: frames on
    /// the current surface.
    pub frames: i32,
    /// Ball xDE0: its trail generator (efSync 0x4BD) still plays.
    pub trail: bool,
}
/// Item.xDD4_itemVar.heiho (itheiho.c): a Yoshi's Story Shy Guy.
#[derive(Clone, Debug)]
pub struct HeihoState {
    /// x20: its place in the spawned group.
    pub group_index: i8,
    /// x21: which walking speed (special attributes x4..xC) it uses.
    pub speed_variant: i8,
    /// x24: frames left of the spawn delay, the stun, or the turn.
    pub countdown: i32,
    /// x22: it has been inside the blast zones once (it_802D9714).
    pub entered_screen: bool,
    /// x3C: the gait joint's last reading (itUpdateVelocityFromBone).
    pub bone_previous: Vec3,
    /// Where the gait joint's animation stands: its rate schedule and the
    /// steps taken since the state began.
    pub bone_rate: crate::bone_motion::BoneRate,
    pub bone_step: usize,
}
impl Default for HeihoState {
    fn default() -> Self {
        Self {
            group_index: 0,
            speed_variant: 0,
            countdown: 0,
            entered_screen: false,
            bone_previous: Vec3::ZERO,
            bone_rate: crate::bone_motion::BoneRate::Steady,
            bone_step: 0,
        }
    }
}
/// Item.xDD4_itemVar.peachturnip (itpeachturnip.c).
#[derive(Clone, Copy, Debug, Default)]
pub struct TurnipState {
    /// xDD8: the face (article state 0's animation frame).
    pub face: i32,
    /// xDDC: the face's damage, which the thrown hitbox takes.
    pub damage: i32,
    /// xDD4 b0: it has been in a hand before (a later pickup plays state 4).
    pub picked_up: bool,
    /// xDE0: the scale when pulled, which each throw restores.
    pub scale: f32,
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
    /// xD48: the half-life it_80275158 sets beside the lifetime; a
    /// reflection (it_80273030) restarts the lifetime from it.
    pub half_life: f32,
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
    /// x5D0_animFrameSpeed: the joint animation's per-frame advance.
    pub animation_rate: f32,
    /// xDCF b2 (it_8027518C): the item ends without its destroy effect.
    pub destroy_effect_suppressed: bool,
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
    /// The model root JObj's translation, which trails `position`: the
    /// collision proc (Item_80269978) and a release from the hand
    /// (it_80273748 / it_80273B50) copy `position` into it.
    pub root_translation: Vec3,
    /// Item.x3C: the size of the last hitbox a script command made, before
    /// it_80275594 divides the capsule by the scale.
    pub hitbox_size: f32,
    /// xDC8 x13 (it_802742F4): held by its owner. A held item neither moves
    /// nor leaves the blast zones.
    pub held: bool,
    /// xDC4: the holder's part it hangs from, a fp->parts index (the
    /// character's own joint numbering, as ftData x8 stores it).
    pub holder_part: u8,
    /// xDCD b5 (it_80275444 / it_80275474): hitboxes may hit the owner.
    pub hits_owner: bool,
    /// xC44: the speed the item was thrown or dropped at (Item_8026AD20).
    pub throw_speed: f32,
    /// xC40: the damage scale new hitboxes take, the throw speed after a
    /// DROP_UPDATE state change and 1 otherwise (Item_80268E5C).
    pub hitbox_damage_scale: f32,
    /// HSD_GObj_804D7838->s_link > 11: the running proc comes after item
    /// link 11's capsule refresh (it_802790C0).
    pub past_hitbox_refresh: bool,
    /// xD0C == 2 (it_802756D0 / it_802756E0): hurtboxes take no hits.
    pub hurt_intangible: bool,
    /// xD40 with xDD0 b6 (item.c foobar): creation's intangible frames
    /// still to run, counted down in Item_80269528.
    pub spawn_intangible_frames: Option<f32>,
    /// xDD0 b7: the item was already intangible at creation, so the
    /// countdown's end leaves it so.
    pub keeps_intangible: bool,
    /// xDC8 x14: set when a throw ends the hold (it_80273F34), cleared by
    /// the next state change or a pickup; it_8026B1D4 then adds speed to
    /// the item's contact damage.
    pub speed_damage: bool,
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
    /// xDCC b3 (Item_80268B18 sets it): Item_802696CC removes the item past
    /// the blast zones. A Shy Guy clears it until it has been on screen.
    pub blast_zone_checked: bool,
    /// The other item this one points at (see [`crate::LinkRequest`]).
    pub partner: Option<u32>,
    /// Requests for linked items, delivered once the proc returns.
    pub link_requests: melee_types::fixed::FixedVec<crate::LinkRequest, 4>,
    /// HSD_JObjAnimAll steps since the article state's animation began
    /// (Item_80268D34's HSD_JObjReqAnimAll), for [`crate::pose::ItemPose`].
    pub pose_steps: u32,
    /// The article state the motion plays (its ItemStateTable anim_id).
    pub article_state: usize,
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
    /// efAsync_Spawn on the item's queue (xBC0): below s_link 9 the request
    /// waits for the item's link 9 flush (Item_80269A9C); the event
    /// callbacks run past it (`past_hitbox_refresh`) and dispatch at once.
    /// efAsync_Spawn links a queued request at the head, so the flush
    /// (efAsync_QueueFlush) processes the latest request first.
    pub fn spawn_async(&mut self, event: ItemEvent) {
        if self.past_hitbox_refresh {
            self.events.push(event);
        } else {
            self.queued_events.insert(0, event);
        }
    }
    /// it_80272860 (80272860): accelerate while below `limit` or while the
    /// velocity still points against gravity. There is no clamp at the limit.
    pub fn fall(&mut self, acceleration: f32, limit: f32) {
        let gravity_sign = if acceleration < 0.0 { -1 } else { 1 };
        let speed = self.velocity.y;
        let velocity_sign = if speed < 0.0 { -1 } else { 1 };
        if velocity_sign == gravity_sign || gekko_math::msl::fabsf(speed) < limit {
            self.velocity.y -= acceleration;
        }
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
    pub fn attach_to_holder(
        &mut self,
        owner: u8,
        part: u8,
        assets: &ItemAssets,
        lifetime: f32,
        half_life_scale: f32,
    ) {
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
        // The pickup (it_26B1.c:597) clears xDC8 x14.
        self.speed_damage = false;
        if (self.kind as u32) < ItemKind::LGunRay as u32 {
            // it_80275158: both timers.
            self.life_timer = lifetime;
            self.half_life = lifetime * half_life_scale;
        }
    }
    pub fn change_motion(&mut self, motion: u16, assets: &ItemAssets) {
        self.motion = motion;
        self.article_state = usize::from(motion);
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
    pub fn spin(&mut self) {
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
        self.speed_damage = false;
        if flags & state_change::MODEL_UPDATE != 0 {
            // it_80274740 (80274740): the spin joint's angle about xDC8 x17's
            // axis and the spin speed return to zero.
            self.spin_speed = 0.0;
            match self.rotation_axis {
                0 => self.rotation.z = 0.0,
                1 => self.rotation.x = 0.0,
                _ => self.rotation.y = 0.0,
            }
        }
        self.hitbox_damage_scale = if flags & state_change::DROP_UPDATE != 0 {
            self.throw_speed
        } else {
            1.0
        };
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
        self.article_state = article_state as usize;
        // Item_80268D34's HSD_JObjReqAnimAll(0) with ANIM_UPDATE, then the
        // HSD_JObjAnimAll every state change takes.
        self.pose_steps = if flags & state_change::ANIM_UPDATE != 0 {
            1
        } else {
            self.pose_steps + 1
        };
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
    /// item.c foobar (Item_80268B18): common and hold-kind 6 items start
    /// intangible for ItCo +2C frames. The it_80279B88 colour flash is visual.
    fn begin_spawn_intangibility(&mut self, frames: f32) {
        if matches!(self.hold_kind, 0 | 6) {
            self.keeps_intangible = self.hurt_intangible;
            self.spawn_intangible_frames = Some(frames);
            self.hurt_intangible = true;
        }
    }
    /// Item_80269528: the creation countdown runs even while frozen; at its
    /// end it_802756E0 restores hits unless the item began intangible.
    fn count_down_spawn_intangibility(&mut self) {
        let Some(frames) = &mut self.spawn_intangible_frames else {
            return;
        };
        *frames -= 1.0;
        if *frames <= 0.0 {
            self.spawn_intangible_frames = None;
            if !self.keeps_intangible {
                self.hurt_intangible = false;
            }
        }
    }
    /// it_8027137C / it_8027129C: refresh world capsules at item s-link 11.
    pub fn update_hitboxes(&mut self, assets: &ItemAssets) {
        for id in 0..self.hitboxes.len() {
            self.update_hitbox(id, assets);
        }
    }
    /// The model root's transform as the kind code set it.
    pub fn root_srt(&self) -> crate::pose::RootSrt {
        crate::pose::RootSrt {
            translate: self.position,
            rotate: self.rotation,
            scale: self.model_scale,
        }
    }
    /// it_8027129C (8027129C): one capsule's position from its bone.
    fn update_hitbox(&mut self, id: usize, assets: &ItemAssets) {
        let root = self.root_srt();
        let (state, steps) = (self.article_state, self.pose_steps);
        let Some(hit) = &mut self.hitboxes[id] else {
            return;
        };
        let matrix = if hit.descriptor.bone == 0 {
            let mut matrix = hsd_types::Mtx::default();
            hsd_anim::mtx::hsd_mtx_srt(
                &mut matrix,
                &self.model_scale,
                &self.rotation,
                &self.position,
                None,
            );
            matrix
        } else {
            let pose = assets
                .pose
                .as_ref()
                .expect("item non-root hitbox bone without a sampled pose");
            pose.bone_matrix(state, steps, hit.descriptor.bone, root)
        };
        let mut position = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(&matrix, &hit.descriptor.offset, &mut position);
        hit.update_position(position);
    }
    /// Item_8026A8EC's ItemSwitch (item.c:1993-1995): the kind's destroy
    /// effect, unless suppressed or the item is still in its owner's hand.
    fn queue_destroy_effect(&mut self, effect: Option<u16>, root: Option<Vec3>) {
        if self.destroy_effect_suppressed || (self.held && self.owner.is_some()) {
            return;
        }
        if let Some(id) = effect {
            self.events.push(ItemEvent::DestroyEffect { id, root });
        }
    }
    /// it_80273670(item, 0, frame) (80273670): the model posed at article
    /// state 0's `frame`; its joint animation and script are removed, so no
    /// further script command runs until the next state change.
    pub fn pose_article_frame(&mut self, frame: f32) {
        self.animation_frame = frame;
        self.script = ScriptState::default();
    }
    /// it_80274484 (80274484): the model at `scale`; every live hitbox
    /// takes x3C, the command size, as its radius (it_80275534), and the
    /// grab range grows by the scale (it_80274DFC). it_80274E44's ECB boxes
    /// (xBDC / xBEC) have no port consumer.
    pub fn rescale(&mut self, scale: f32) {
        self.scale = scale;
        self.model_scale = Vec3::new(scale, scale, scale);
        let size = self.hitbox_size;
        for hit in self.hitboxes.iter_mut().flatten() {
            hit.descriptor.radius = size;
        }
        self.grab_range.x *= scale;
        self.grab_range.y *= scale;
    }
    /// it_80272460 (80272460) for an existing hitbox `id`: its count and
    /// the damage restaled for the item's attack (ft_80089228).
    pub fn set_hitbox_damage(&mut self, id: usize, damage: u32) {
        if let Some(hit) = &mut self.hitboxes[id] {
            hit.knockback_damage = damage;
            hit.descriptor.damage = damage as f32 * self.stale_multiplier;
        }
    }
    /// Item_802694CC (802694CC): the joint animation advances by the item's
    /// rate (HSD_JObjAnimAll, x5CC), then its script runs (it_802799E4).
    pub fn advance_animation(&mut self, assets: &ItemAssets) {
        // lbGetJObjCurrFrame: an article without a model (Item_80267978's
        // bare JObj) has no AObj, so x5CC stays zero.
        if assets.model != 0 {
            self.animation_frame += self.animation_rate;
        }
        self.pose_steps += 1;
        self.advance_script(assets);
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
                    // it_802790C0 -> it_80272460: the command damage scaled by
                    // xC40 (xC3C is 1) and truncated to the hit's count.
                    let mut descriptor = descriptor.clone();
                    if self.hitbox_damage_scale != 1.0 {
                        descriptor.damage =
                            gekko_math::msl::fctiwz(descriptor.damage * self.hitbox_damage_scale)
                                as f32;
                    }
                    melee_coll::hitbox::spawn(&mut self.hitboxes, *id, &descriptor);
                    let hit = self.hitboxes[*id].as_mut().unwrap();
                    hit.descriptor.damage *= self.stale_multiplier;
                    // it_802790C0: x3C keeps the command's size, then
                    // it_80275594(1 / scl): the capsule radius is stored
                    // unscaled; contacts multiply the item scale back in.
                    self.hitbox_size = hit.descriptor.radius;
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
                        self.update_hitbox(*id, assets);
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
                    self.update_hitboxes(assets);
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
    /// The common data beside mutable items, for passes that read both.
    pub fn common_and_iter_mut(
        &mut self,
    ) -> (
        &ItemCommonData,
        impl DoubleEndedIterator<Item = &mut ItemCore>,
    ) {
        (&self.common, self.items.iter_mut())
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
            // Item_80268B18's it_80275158: both timers.
            life_timer: self.common.lifetime,
            half_life: self.common.lifetime * self.common.half_life_scale,
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
            animation_rate: 1.0,
            destroy_effect_suppressed: false,
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
            hitbox_size: 0.0,
            root_translation: if spawn.initial_collision {
                spawn.previous_position
            } else {
                spawn.position
            },
            held: false,
            holder_part: 0,
            hits_owner: false,
            throw_speed: 1.0,
            hitbox_damage_scale: 1.0,
            past_hitbox_refresh: false,
            hurt_intangible: false,
            spawn_intangible_frames: None,
            keeps_intangible: false,
            speed_damage: false,
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
            blast_zone_checked: true,
            partner: None,
            link_requests: Default::default(),
            pose_steps: 0,
            article_state: 0,
        };
        // Item_80267130 -> it_80274658(x6C) before the kind's spawn callback.
        item.update_spin(self.common.spawn_spin_degrees);
        (D::logic(item.kind).spawned)(&mut item, assets);
        item.begin_spawn_intangibility(self.common.spawn_intangible_frames);
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
        let rng = core::cell::Cell::new(gekko_math::HsdRng::default());
        self.process_events_with_stale::<D>(id, 1.0, assets, &rng);
    }
    pub fn process_events_with_stale<D: ItemDispatch>(
        &mut self,
        id: u32,
        reflected_stale: f32,
        assets: &ItemAssets,
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let cap = self.common.maximum_reflected_damage;
        // retail 80269E18/20: add then multiply, no FMA or double promotion.
        let bounce_limit =
            (std::f32::consts::PI / 180.0) * (90.0 + self.common.shield_bounce_degrees);
        let common = self.common.clone();
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let alive = !item.destroyed;
        item.past_hitbox_refresh = true;
        if item.pending_knockback != 0.0 || item.pending_damage_taken != 0 {
            // OnTakeDamageThink: the percent grows (capped at 999) and the
            // frame's damage becomes the hitlag damage (xCA8), unconditionally.
            item.damage_percent = (item.damage_percent + item.pending_damage_taken).min(999);
            item.hitlag_damage = item.pending_damage_taken;
            let context = ItemEventContext {
                rng: Some(rng),
                ..ItemEventContext::new(assets, &common)
            };
            item.destroyed |= (D::logic(item.kind).damage_received)(item, &context);
        } else if item.pending_shield_damage != 0 {
            if let Some(deflection) = item.pending_shield_deflection.filter(|d| {
                item.ground_or_air == melee_types::GroundOrAir::Air && d.angle < bounce_limit
            }) {
                let context = ItemEventContext {
                    shield_normal: deflection.normal,
                    ..ItemEventContext::new(assets, &common)
                };
                item.destroyed |= (D::logic(item.kind).shield_bounced)(item, &context);
            } else {
                if item.hitlag_enabled {
                    item.hitlag_damage = item.pending_shield_damage;
                }
                item.destroyed |=
                    (D::logic(item.kind).hit_shield)(item, &ItemEventContext::new(assets, &common));
            }
        } else if item.pending_clank_damage != 0 {
            // OnClankThink: the clank damage becomes the hitlag damage.
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_clank_damage;
            }
            item.destroyed |= (D::logic(item.kind).clanked)(item, &ItemEventContext::new(assets, &common));
        } else if item.pending_damage_dealt != 0 || item.pending_damage_without_hitlag != 0 {
            if item.hitlag_enabled {
                item.hitlag_damage = item.pending_damage_dealt;
            }
            item.destroyed |=
                (D::logic(item.kind).damage_dealt)(item, &ItemEventContext::new(assets, &common));
        } else if let Some(reflection) = item.pending_reflection {
            item.reflect::<D>(reflection, reflected_stale, cap, assets, &common);
        }
        // processCallback (item.c:1739): destroy_type 2.
        if alive && item.destroyed {
            item.queue_destroy_effect(assets.event_destroy_effect, None);
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
    pub fn control<D: ItemDispatch>(
        &mut self,
        owner: u8,
        kind: ItemKind,
        control: ItemControl,
        assets: &ItemAssets,
    ) {
        for item in self
            .items
            .iter_mut()
            .filter(|i| i.owner == Some(owner) && i.kind == kind)
        {
            (D::logic(kind).control)(item, control, assets);
        }
    }
    pub fn animate<D: ItemDispatch>(
        &mut self,
        id: u32,
        assets: &ItemAssets,
        owner: Option<&ItemOwner>,
        holder: Option<crate::ItemHolder<'_>>,
        map: &mut melee_mp::CollMap,
        partner: Option<crate::PartnerView>,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        // Item_80269528: hitlag pauses the animation and its callback.
        if !(item.frozen || item.in_hitlag) {
            item.advance_animation(assets);
            let row = D::logic(item.kind).states[item.motion as usize];
            let ended = (row.animation)(
                item,
                &mut ItemAnimationContext {
                    owner,
                    holder,
                    map,
                    assets,
                    partner,
                },
            );
            if ended && !item.destroyed {
                // Item_80269528: destroy_type 0.
                item.queue_destroy_effect(assets.destroy_effect, None);
            }
            item.destroyed |= ended;
            if item.destroyed {
                return;
            }
        }
        item.count_down_spawn_intangibility();
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
        rng: &core::cell::Cell<gekko_math::HsdRng>,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        if !item.frozen && !item.in_hitlag {
            (D::logic(item.kind).states[item.motion as usize].physics)(
                item,
                &ItemPhysicsContext {
                    owner,
                    assets,
                    bounds,
                    rng,
                },
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
    #[allow(clippy::too_many_arguments)] // Owner, map and bounds stay separate.
    pub fn collide<D: ItemDispatch>(
        &mut self,
        id: u32,
        owner: Option<&ItemOwner>,
        stage_contact: bool,
        map: &mut melee_mp::CollMap,
        bounds: &ItemBounds,
        assets: &ItemAssets,
    ) {
        let Some(item) = self.get_mut(id) else {
            return;
        };
        let alive = !item.destroyed;
        item.destroyed |= (D::logic(item.kind).states[item.motion as usize].collision)(
            item,
            &mut ItemCollisionContext {
                owner,
                stage_contact,
                map,
                assets,
                bounds,
            },
        );
        if alive && item.destroyed {
            // Item_80269978: destroy_type 1, ItemSwitch's x68 effect at the
            // root JObj, which this proc has not yet moved to `position`.
            let root = item.root_translation;
            item.queue_destroy_effect(assets.event_destroy_effect, Some(root));
        } else {
            // Item_80269978: HSD_JObjSetTranslate(jobj, &pos).
            item.root_translation = item.position;
        }
    }
    /// Item_8026A8EC (8026A8EC) outside the item procs: remove `id` now and
    /// run its kind's destroyed callback.
    pub fn destroy<D: ItemDispatch>(&mut self, id: u32) {
        let index = self
            .items
            .iter()
            .position(|item| item.id == id)
            .expect("Item_8026A8EC: Not Found Item_Struct");
        let mut item = self.items.remove(index);
        (D::logic(item.kind).destroyed)(&mut item);
    }
    pub fn remove_destroyed<D: ItemDispatch>(&mut self) {
        let mut index = 0;
        while index < self.items.len() {
            if self.items[index].destroyed {
                let mut item = self.items.remove(index);
                (D::logic(item.kind).destroyed)(&mut item);
                if D::logic(item.kind).unlinks_partner_on_destroy {
                    if let Some(partner) = item.partner.and_then(|id| self.get_mut(id)) {
                        partner.partner = None;
                    }
                }
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
        && item.blast_zone_checked
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
            launch_vertical_velocity: 0.0,
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
            bounce_scale: 1.0,
            bounce_sound: 0,
            destroy_effect: None,
            event_destroy_effect: None,
            grab_offset: hsd_types::Vec2::ZERO,
            grab_range: hsd_types::Vec2::ZERO,
            attachment_translation: Vec3::ZERO,
            bone_motion: None,
            pose: None,
            special_pointees: Vec::new(),
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
            spawn_intangible_frames: 0.0,
            half_life_scale: 0.5,
            speed_damage_scale: 0.0,
            speed_damage_base: 0.0,
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
            launch: Default::default(),
            victim_bounce: Default::default(),
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
            spawn_intangible_frames: 0.0,
            half_life_scale: 0.5,
            speed_damage_scale: 0.0,
            speed_damage_base: 0.0,
            shield_bounce_degrees: 0.0,
            maximum_reflected_damage: 999,
            hitlag_scale: 0.0,
            hitlag_base: 0.0,
            explosion_lifetime: 0.0,
            spawn_spin_degrees: 0.0,
            fall_spin_degrees: 0.0,
            knockback: Default::default(),
            launch: Default::default(),
            victim_bounce: Default::default(),
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
                top: f32::INFINITY,
                camera_offset: hsd_types::Vec2::ZERO,
            },
        );
        assert_eq!(item.position.x.to_bits(), 0.0f32.to_bits());
    }
}
