//! Item archive boundary: `it/types.h` Article and ItemCommonData.
use hsd_archive::{Archive, Result};
use melee_cmd::Command;
use melee_types::combat::HitboxDescriptor;

#[derive(Clone, Debug)]
pub struct ItemCommonData {
    pub hold_limits: [Option<usize>; 13],
    pub lifetime: f32,
    /// +2C: frames a new hold-kind 0 or 6 item takes no hits (item.c
    /// foobar's xD40), converted from its integer.
    pub spawn_intangible_frames: f32,
    /// +4C: it_80275158's half-life (xD48) as a fraction of the lifetime.
    pub half_life_scale: f32,
    /// +94 / +98 (x80[5], x80[6]): it_8026B1D4's contact damage per unit of
    /// a thrown item's speed, and its constant term.
    pub speed_damage_scale: f32,
    pub speed_damage_base: f32,
    pub shield_bounce_degrees: f32,
    /// +B4: an item hitbox stops against another item's when its damage,
    /// less this, is below the other's (it_8026FE68).
    pub clank_priority_gap: i32,
    pub maximum_reflected_damage: u32,
    /// +B8/+BC: item hitlag frames from contact damage (it_8026B424).
    pub hitlag_scale: f32,
    pub hitlag_base: f32,
    /// +F8: it_8027518C's lifetime once an item explodes.
    pub explosion_lifetime: f32,
    /// +6C / +68: it_80274658's spin degrees at creation (Item_80267130) and
    /// while falling (Item_ApplyFallingPhysics).
    pub spawn_spin_degrees: f32,
    pub fall_spin_degrees: f32,
    /// x9C..xB0 (it_80270E30).
    pub knockback: crate::hurt::ItemKnockback,
    /// Fighter common data it_8027B798 reads; the scene fills it in from
    /// PlCo after loading ItCo.
    pub launch: crate::hurt::ItemLaunch,
    /// +58 / +5C / +60: itColl_BounceOffVictim's rebound.
    pub victim_bounce: VictimBounce,
    /// +14C..+158: a framed item's camera extents (item.c foobar3):
    /// left, right, top and bottom reach.
    pub camera_extents: [f32; 4],
    /// +7C: the speed a push moves a grounded item (it_80271B60).
    pub push_speed: f32,
    /// +F0: how far above a floor a released enemy may be set down on it
    /// (it_802759DC).
    pub release_floor_depth: f32,
    /// +D4: under this speed a hit's direction (xCB8) is the item's side
    /// of its victim rather than its motion.
    pub direction_still_speed: f32,
    pub slide_rule: SlideRule,
    /// What the random item spawner reads (itspawn.c).
    pub random_items: RandomItemData,
}

/// ItCo common data the random item spawner reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RandomItemData {
    /// +FC: per Item Switch frequency (Very Low .. Very High), the range of
    /// ticks between spawns (fn_8026C88C, it_8026D018).
    pub intervals: [[i32; 2]; 5],
    /// +128: the monster table's weights (it_8026CF04).
    pub monster_weights: [i32; 4],
    /// +54: a released item's sideways launch reach (it_8026F53C).
    pub release_spread: f32,
    /// +70: a released item's spin, in degrees (it_8026F5C8).
    pub release_spin_degrees: f32,
    /// +124: one crate in this many lets out a monster (it_8026F6BC).
    pub monster_odds: i32,
    /// +14: the hold-kind 6 items (Item_804A0C64.x2C) above which no
    /// monster comes out (x30).
    pub monster_limit: i32,
}

/// ItCo common data for an item sliding down a floor (it_80277040,
/// it_8027737C, it_80277544).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideRule {
    /// +C0: the least slope, in radians from level, that slides an item.
    pub minimum_slope: f32,
    /// +C4: a surface turned this far from the one the item rested on
    /// starts the slide whatever its speed.
    pub rearm_turn: f32,
    /// +C8: the slide's top speed.
    pub speed_limit: f32,
    /// +CC: the squared speed under which a slide ends, and at which a
    /// moving item starts one.
    pub stop_speed_squared: f32,
    /// +38 (an unsigned int, retail 0x80277310: lwz, cmplw): frames the
    /// slope's pull may turn against the frame before.
    pub reversal_frames: u32,
}
impl SlideRule {
    /// No floor slides anything: for assets built without ItCo.
    pub const NEVER: Self = Self {
        minimum_slope: f32::INFINITY,
        rearm_turn: f32::INFINITY,
        speed_limit: 0.0,
        stop_speed_squared: 0.0,
        reversal_frames: 0,
    };
}

/// itColl_BounceOffVictim (80272DB0): an item rebounding off what it hit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VictimBounce {
    /// +58: the horizontal speed kept.
    pub horizontal_scale: f32,
    /// +5C / +60: the vertical speed kept, and the pop added to it.
    pub vertical_scale: f32,
    pub vertical_pop: f32,
}
impl ItemCommonData {
    /// Item_80266FCC: maps the common data fields into hold-kind counters.
    /// Categories 4, 6 and 8 have no admission limit in Item_8026784C.
    pub fn read(archive: &Archive, public: u32) -> Result<Self> {
        let r = archive.reader();
        let base = r.u32(public)?;
        let mut hold_limits = [None; 13];
        for (kind, offset) in [
            (0, 0),
            (1, 4),
            (2, 8),
            (9, 12),
            (10, 16),
            (7, 24),
            (5, 28),
            (12, 32),
            (11, 36),
            (3, 40),
        ] {
            hold_limits[kind] = Some(r.u32(base + offset)? as usize);
        }
        let mut random_items = RandomItemData::default();
        for (i, range) in random_items.intervals.iter_mut().enumerate() {
            let at = base + 0xFC + 8 * i as u32;
            *range = [r.s32(at)?, r.s32(at + 4)?];
        }
        for (i, weight) in random_items.monster_weights.iter_mut().enumerate() {
            *weight = r.s32(base + 0x128 + 4 * i as u32)?;
        }
        random_items.release_spread = r.f32(base + 0x54)?;
        random_items.release_spin_degrees = r.f32(base + 0x70)?;
        random_items.monster_odds = r.s32(base + 0x124)?;
        random_items.monster_limit = r.s32(base + 0x14)?;
        Ok(Self {
            hold_limits,
            lifetime: r.u32(base + 0x30)? as f32,
            spawn_intangible_frames: r.u32(base + 0x2C)? as i32 as f32,
            half_life_scale: r.f32(base + 0x4C)?,
            speed_damage_scale: r.f32(base + 0x94)?,
            speed_damage_base: r.f32(base + 0x98)?,
            shield_bounce_degrees: r.f32(base + 0xE0)?,
            maximum_reflected_damage: r.u32(base + 0xD8)?,
            clank_priority_gap: r.u32(base + 0xB4)? as i32,
            hitlag_scale: r.f32(base + 0xB8)?,
            hitlag_base: r.f32(base + 0xBC)?,
            explosion_lifetime: r.f32(base + 0xF8)?,
            spawn_spin_degrees: r.f32(base + 0x6C)?,
            fall_spin_degrees: r.f32(base + 0x68)?,
            knockback: crate::hurt::ItemKnockback {
                cap: r.f32(base + 0x9C)?,
                percent: r.f32(base + 0xA0)?,
                damage_percent: r.f32(base + 0xA4)?,
                weight_set_percent: r.f32(base + 0xA8)?,
                scale: r.f32(base + 0xAC)?,
                base: r.f32(base + 0xB0)?,
                still_speed: r.f32(base + 0x78)?,
            },
            launch: Default::default(),
            victim_bounce: VictimBounce {
                horizontal_scale: r.f32(base + 0x58)?,
                vertical_scale: r.f32(base + 0x5C)?,
                vertical_pop: r.f32(base + 0x60)?,
            },
            camera_extents: [
                r.f32(base + 0x14C)?,
                r.f32(base + 0x150)?,
                r.f32(base + 0x154)?,
                r.f32(base + 0x158)?,
            ],
            push_speed: r.f32(base + 0x7C)?,
            release_floor_depth: r.f32(base + 0xF0)?,
            direction_still_speed: r.f32(base + 0xD4)?,
            slide_rule: SlideRule {
                minimum_slope: r.f32(base + 0xC0)?,
                rearm_turn: r.f32(base + 0xC4)?,
                speed_limit: r.f32(base + 0xC8)?,
                stop_speed_squared: r.f32(base + 0xCC)?,
                reversal_frames: r.u32(base + 0x38)?,
            },
            random_items,
        })
    }
}

/// itLeadeadAttributes (itCommonItems.h:1002), with the pair its x0 points at.
#[derive(Clone, Copy, Debug)]
pub struct ReDeadAttributes {
    /// x0->x0: the damage (xC9C) beyond which a hit ends it.
    pub health: i32,
    /// x0->x4: the walking speed.
    pub walk_speed: f32,
    /// x4: frames a light hit keeps it reeling before it may recover.
    pub reel_frames: i32,
    /// x8: the share of `health` beyond which a hit staggers it.
    pub stagger_share: f32,
    /// xC: the horizontal speed kept by a fall (it_802E9308).
    pub fall_speed_scale: f32,
    /// x10: how far ahead a fighter may stand for a lunge.
    pub lunge_reach: f32,
    /// x14: the lunge's speed.
    pub lunge_speed: f32,
    /// x18 (s16): frames between turns toward the nearest fighter.
    pub walk_frames: i32,
    /// x1A (s16): stun frames per point of damage (it_802E98E0).
    pub stun_per_damage: i32,
    /// x1C (s16): frames after a release or hit before it may lunge again.
    pub lunge_cooldown: i32,
    /// x1E (s8): one in this many frames it looks for a lunge.
    pub lunge_odds: i32,
}

/// itOldkuriAttributes (itoldkuri.c), with the pair its x0 points at.
#[derive(Clone, Copy, Debug)]
pub struct KuribohAttributes {
    /// x0->x0: the damage (xC9C) beyond which a hit ends it.
    pub health: i32,
    /// x0->x4: the walking speed.
    pub walk_speed: f32,
    /// x8: the X speed a stomp in the air keeps (it_802D81FC).
    pub stomp_air_speed_scale: f32,
    /// xC: the frames it stands after a landing (fctiwz).
    pub stand_frames: f32,
}

/// it_804D6D40 (itPublicData +10): what every stage enemy shares.
#[derive(Clone, Copy, Debug, Default)]
pub struct MonsterCommonData {
    /// x0: wall and floor bounces before a launched enemy is gone.
    pub bounce_limit: i32,
    /// x4: a launch's speed (it_8027B964).
    pub launch_speed: f32,
    /// x8: the chance a killing hit sends it toward the camera
    /// (it_802EA334) rather than tumbling (it_802EA2A0).
    pub screen_chance: f32,
    /// xC / x10: the damage a Nap or Sleep hit, and a Ground hit, adds
    /// (it_8027CBFC).
    pub sleep_damage: f32,
    pub ground_damage: f32,
    /// x14 / x18: it_8027CC88's 1P-mode drop odds and cooldown.
    pub drop_odds: f32,
    pub drop_cooldown: f32,
}

#[derive(Clone, Debug)]
pub struct ItemAssets {
    pub visual: hsd_archive::desc::item_visual::ItemVisual,
    pub scripts: Vec<Vec<Command>>,
    pub hit_flags: Vec<Vec<Option<ItemHitFlags>>>,
    pub special_attributes: Vec<f32>,
    pub redead: Option<ReDeadAttributes>,
    pub kuriboh: Option<KuribohAttributes>,
    /// it_804D6D40, for the monster bank's kinds.
    pub monster_common: Option<MonsterCommonData>,
    pub scale: f32,
    pub model: u32,
    pub rotate_to_facing: bool,
    pub collision_box: melee_types::mp::ItEcb,
    /// ItemAttr x20: the push box (Item xBEC).
    pub push_box: melee_types::mp::ItEcb,
    /// ItemAttr x1_4 (Item.xDC8 x1A at creation): grounded, it can be pushed.
    pub pushable: bool,
    pub collision_damage_multiplier: f32,
    /// ItemAttr x1_5 (Item.xDC8 xC): contacts put this kind into hitlag.
    pub hitlag: bool,
    /// ItemAttr x1_67_cam_kind (Item.xDCD): 0 none, 1 an Active camera
    /// subject, 2 an Auto one (item.c foobar3).
    pub camera_kind: u8,
    /// ItemAttr x10 / x14: Item_ApplyFallingPhysics' gravity and the speed
    /// it stops accelerating at (it_80272860).
    pub fall_acceleration: f32,
    pub fall_speed_limit: f32,
    /// ItemAttr x18: a spawned article's vertical launch speed (it_802B322C).
    pub launch_vertical_velocity: f32,
    /// ItemAttr x1_1 (Item.xDC8 x17 at creation): the ECB rotation axis.
    pub rotation_axis: u8,
    /// ItCo common data +F8 (it_8027518C): an explosion's lifetime. Zero
    /// for character articles, which do not explode.
    pub explosion_lifetime: f32,
    /// ItCo common data +30 (it_804D6D28->x30_lifetime, an int): the
    /// common item lifetime a lit Bob-omb restarts (it_8027F8E0). Zero
    /// until `read_common_release`.
    pub common_lifetime: f32,
    /// ItCo +4C: it_80275158's half-life as a share of a lifetime.
    pub half_life_scale: f32,
    /// ItemAttr xC (spin_spd).
    pub spin_rate: f32,
    /// ItCo common data +68: Item_ApplyFallingPhysics' spin degrees.
    pub fall_spin_degrees: f32,
    /// Per article state: the frame its joint animation stops, if it does.
    pub animation_ends: Vec<Option<f32>>,
    /// Per article state: whether its joint animation gives some joint an
    /// AObj (lbFindJObjWithAObj finds one). Without one lbGetJObjCurrFrame
    /// reads zero, so the item's x5CC stays zero through the state.
    pub joint_animated: Vec<bool>,
    /// Article x8 (ItHurtBoneList, it_8027163C): at most two hurt capsules.
    pub hurtboxes: Vec<ItemHurtbox>,
    /// it_804D6D28->xE8 (it_80275BC8): the ECB scale for a released item's
    /// first sweep. Zero for fighter articles, which are never released.
    pub release_box_scale: f32,
    /// ItCo common data +48 (it_8026DDFC): a thrown item's first landing
    /// breaks it after `throw_break >> 4` throws, or when
    /// HSD_Randi(`throw_break & 0xF`) draws zero; zero until
    /// `read_common_release`.
    pub throw_break: u8,
    /// ItemAttr x0 bit 0x80 (itIsHeavy): picked up with HeavyGet.
    pub heavy: bool,
    /// ItemAttr x0 bits 0x78 (it_8026B30C): how a holder uses the item;
    /// 5 is consumed on pickup (ftpickupitem_8009447C).
    pub use_kind: u8,
    /// ItemAttr x0 bits 0x07 (itGetHoldKind): the holder's hand pose.
    pub hand_hold_kind: u8,
    /// ItemAttr x4 (it_80273B50): the release velocity multiplier.
    pub throw_speed_multiplier: f32,
    /// ItemAttr x58 (it_80276FC4): the speed and hitbox damage kept by a
    /// wall or ceiling bounce.
    pub bounce_scale: f32,
    /// ItemAttr x80 (Item.xD84, it_8027321C): the bounce sound.
    pub bounce_sound: u32,
    /// ItemAttr x5C (it_8026DC24): a landing at or below this speed on both
    /// axes comes to rest.
    pub rest_speed: f32,
    /// ItemAttr x50 (it_80277040): the slope slide's speed; zero never slides.
    pub slide_speed: f32,
    /// ItemAttr x54 (it_8027737C): what a slide keeps of its speed a frame.
    pub slide_friction: f32,
    /// ItCo common data the slide reads; the scene copies it from
    /// [`ItemCommonData::slide_rule`] into every kind.
    pub slide_rule: SlideRule,
    /// ItCo common data +74 (it_8026DC24): the spin degrees a first landing
    /// sets.
    pub landing_spin_degrees: f32,
    /// ItemAttr x64 (destroy_gfx): the effect when an animation callback
    /// ends the item (ItemSwitch, destroy_type 0).
    pub destroy_effect: Option<u16>,
    /// ItemAttr x68: the effect when an event callback ends it (types 1/2).
    pub event_destroy_effect: Option<u16>,
    /// ItemAttr x30 / x38: the pickup box offset and half extents.
    pub grab_offset: hsd_types::Vec2,
    pub grab_range: hsd_types::Vec2,
    /// it_80272C90: the authored translation of the model's attachment
    /// bone (ItemModelDesc x8), which it_80273B50 undoes when a character
    /// article (hold kind 8) leaves the hand. None when the authored model
    /// has no such joint: Food's placeholder model, which it_80273318
    /// replaces with its dish's before anything reads it.
    pub attachment_translation: Option<hsd_types::Vec3>,
    /// The joint whose animation moves the kind (itUpdateVelocityFromBone),
    /// sampled by [`Self::read_bone_motion`].
    pub bone_motion: Option<crate::bone_motion::BoneMotion>,
    /// Every joint's locals per animation step, sampled by
    /// [`Self::read_pose`] for kinds that read a joint below the root.
    pub pose: Option<crate::pose::ItemPose>,
    /// The DPtcl keys of each article state's joint animation, sampled by
    /// [`Self::read_particle_tracks`] for kinds whose model carries them.
    pub particle_tracks: Option<crate::particle_track::ParticleTracks>,
    /// Special attribute words that point at an integer (itHeiho x0), read
    /// through by [`Self::from_stage_item`]: the integer at each.
    pub special_pointees: Vec<i32>,
}
/// it_802E6AEC: `it_804D6D38[It_PKind_Random - It_Kind_Kuriboh]`.
const YAKU_ARTICLE_INDEX: u32 = 0xA0 - 0x2B;
/// it_803F8C8C: the stage material item's twenty motion states, each
/// playing the article state of its own number.
pub const YAKU_STATES: usize = 20;
/// it_803F8C08 (ityaku.c), the ItemAttr it_802E6AEC installs, as it lies
/// in memory: x0 0x31 (use kind 6, hold kind 1), x1 0x08 (hitlag), the
/// collision box 2.0 on each side (x40..x4C), scale 1.0 (x60), no destroy
/// effects (x64, x68 = -1) and 0x83D60 for every sound (x6C..x80).
const YAKU_ATTRIBUTES: [u8; 0x84] = {
    let mut bytes = [0u8; 0x84];
    bytes[0] = 0x31;
    bytes[1] = 0x08;
    let two = 2.0f32.to_bits().to_be_bytes();
    let mut at = 0x40;
    while at < 0x50 {
        bytes[at] = two[0];
        bytes[at + 1] = two[1];
        bytes[at + 2] = two[2];
        bytes[at + 3] = two[3];
        at += 4;
    }
    let one = 1.0f32.to_bits().to_be_bytes();
    bytes[0x60] = one[0];
    bytes[0x61] = one[1];
    bytes[0x62] = one[2];
    bytes[0x63] = one[3];
    let mut at = 0x64;
    while at < 0x6C {
        bytes[at] = 0xFF;
        at += 1;
    }
    let sound = 0x0008_3D60u32.to_be_bytes();
    let mut at = 0x6C;
    while at < 0x84 {
        bytes[at] = sound[0];
        bytes[at + 1] = sound[1];
        bytes[at + 2] = sound[2];
        bytes[at + 3] = sound[3];
        at += 4;
    }
    bytes
};
/// The `target`-th joint in depth-first order (a joint, its children, then
/// its next siblings), counting from `index`; no heap stack, as it runs in
/// the tick.
fn depth_first_joint<'a>(
    mut joint: &'a hsd_archive::desc::jobj::JObjDesc,
    target: usize,
    index: &mut usize,
) -> Option<&'a hsd_archive::desc::jobj::JObjDesc> {
    loop {
        if *index == target {
            return Some(joint);
        }
        *index += 1;
        if let Some(child) = joint.child.as_deref() {
            if let Some(found) = depth_first_joint(child, target, index) {
                return Some(found);
            }
        }
        joint = joint.next.as_deref()?;
    }
}

impl ItemAssets {
    /// [`Self::joint_animated`] for `article_state`; a kind without
    /// visual states (a test's bare assets) counts as animated.
    pub fn has_joint_animation(&self, article_state: usize) -> bool {
        self.joint_animated.get(article_state).copied().unwrap_or(true)
    }
    /// it_80272C90 / it_2725_JObjGetTranslation: the local translation of
    /// the model's attach joint (ItemModelDesc x8), in depth-first order.
    pub fn attach_translation(&self) -> hsd_types::Vec3 {
        let mut index = 0;
        let joint = depth_first_joint(&self.visual.model, self.visual.attachment_bone, &mut index)
            .unwrap_or_else(|| panic!("item attach joint {} missing", self.visual.attachment_bone));
        let p = joint.position;
        hsd_types::Vec3::new(p.x, p.y, p.z)
    }
    /// ftData.x48_items -> Article, loaded once before any item exists. A
    /// character article's state rows are its motion states in order.
    pub fn from_fighter(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        states: usize,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        let rows: Vec<i32> = (0..states as i32).collect();
        Self::from_article(archive, article, &rows, 10)
    }

    /// The Article a pointer at `slot` names, its `states` motion states in
    /// order, as [`Self::from_fighter`] reads them: Kirby's copy archives
    /// keep their articles in KirbyHatStruct.hat_dynamics (+C), which
    /// ftKb_SpecialN_800F16D0 registers (it_8026B3F8).
    pub fn from_article_slot(
        archive: &Archive,
        slot: u32,
        states: usize,
    ) -> hsd_archive::desc::Result<Self> {
        let article = archive.reader().u32(slot)?;
        let rows: Vec<i32> = (0..states as i32).collect();
        Self::from_article(archive, article, &rows, 10)
    }

    /// [`Self::from_article_slot`] for an article whose motion states do not
    /// map one to one onto article states, or with more special attribute
    /// words, as for [`Self::from_fighter_states`].
    pub fn from_article_slot_states(
        archive: &Archive,
        slot: u32,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let article = archive.reader().u32(slot)?;
        Self::from_article(archive, article, article_states, special_attributes)
    }

    /// ftData.x48_items[item_index] for an article whose motion states do
    /// not map one to one onto article states: `article_states` is the
    /// kind's ItemStateTable anim_id column, as for [`Self::from_common`].
    pub fn from_fighter_states(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        Self::from_article(archive, article, article_states, special_attributes)
    }

    /// [`Self::from_fighter_states`] for an article whose code plays an
    /// article state no motion state names (Item_80268D34 on a fixed desc,
    /// the Link bomb's lit fuse): its visual reads `article_state_count`
    /// states at least.
    pub fn from_fighter_states_with_count(
        archive: &Archive,
        fighter_data: u32,
        item_index: u32,
        article_states: &[i32],
        special_attributes: u32,
        article_state_count: usize,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let items = r.u32(fighter_data + 0x48)?;
        let article = r.u32(items + item_index * 4)?;
        Self::from_article_with_count(
            archive,
            article,
            article_states,
            special_attributes,
            article_state_count,
        )
    }

    /// The ItCo common data (it_804D6D28) an article reads once it leaves
    /// the hand or explodes: +F8 (it_8027518C), +68 and +E8 (it_80275BC8),
    /// and +74 (it_8026DC24).
    pub fn read_common_release(
        &mut self,
        archive: &Archive,
        public: u32,
    ) -> hsd_archive::desc::Result<()> {
        let r = archive.reader();
        let common = r.u32(public)?;
        self.explosion_lifetime = r.f32(common + 0xF8)?;
        self.common_lifetime = r.u32(common + 0x30)? as f32;
        self.half_life_scale = r.f32(common + 0x4C)?;
        self.fall_spin_degrees = r.f32(common + 0x68)?;
        self.release_box_scale = r.f32(common + 0xE8)?;
        self.landing_spin_degrees = r.f32(common + 0x74)?;
        self.throw_break = r.u8(common + 0x48)?;
        Ok(())
    }

    /// it_804D6D40 (itPublicData +0x10): what every stage enemy shares,
    /// for a stage's own enemy (Birdo) as for the monster bank's.
    pub fn read_monster_common(
        &mut self,
        archive: &Archive,
        public: u32,
    ) -> hsd_archive::desc::Result<()> {
        let reader = archive.reader();
        let common = reader.u32(public + 0x10)?;
        self.monster_common = Some(MonsterCommonData {
            bounce_limit: reader.s32(common)?,
            launch_speed: reader.f32(common + 4)?,
            screen_chance: reader.f32(common + 8)?,
            sleep_damage: reader.f32(common + 0xC)?,
            ground_damage: reader.f32(common + 0x10)?,
            drop_odds: reader.f32(common + 0x14)?,
            drop_cooldown: reader.f32(common + 0x18)?,
        });
        Ok(())
    }

    /// it_804D6D38 (itPublicData +8): the separate monster article bank.
    /// itPublicData x8[kind - Kuriboh] (it_804D6D38, Item_80267978's
    /// character bank) for a kind ItCo itself carries: the Coin match's
    /// coin (It_Kind_Unk4, slot 0x73).
    pub fn from_bank(
        archive: &Archive,
        public: u32,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let index = (kind as u32)
            .checked_sub(melee_types::ItemKind::Kuriboh as u32)
            .expect("a character bank kind");
        let articles = archive.reader().u32(public + 8)?;
        let mut assets = Self::from_article_slot_states(
            archive,
            articles + index * 4,
            article_states,
            special_attributes,
        )?;
        assets.read_common_release(archive, public)?;
        Ok(assets)
    }

    pub fn from_monster(
        archive: &Archive,
        public: u32,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let index = (kind as u32)
            .checked_sub(melee_types::ItemKind::Kuriboh as u32)
            .filter(|index| *index < 4)
            .expect("a monster bank kind");
        let articles = archive.reader().u32(public + 8)?;
        let mut assets = Self::from_article_slot_states(
            archive,
            articles + index * 4,
            article_states,
            special_attributes,
        )?;
        assets.read_common_release(archive, public)?;
        let reader = archive.reader();
        let common = reader.u32(public + 0x10)?;
        assets.monster_common = Some(MonsterCommonData {
            bounce_limit: reader.s32(common)?,
            launch_speed: reader.f32(common + 4)?,
            screen_chance: reader.f32(common + 8)?,
            sleep_damage: reader.f32(common + 0xC)?,
            ground_damage: reader.f32(common + 0x10)?,
            drop_odds: reader.f32(common + 0x14)?,
            drop_cooldown: reader.f32(common + 0x18)?,
        });
        if kind == melee_types::ItemKind::Kuriboh {
            let article = reader.u32(articles + index * 4)?;
            let special = reader.u32(article + 4)?;
            let pair = reader.u32(special)?;
            assets.kuriboh = Some(KuribohAttributes {
                health: reader.s32(pair)?,
                walk_speed: reader.f32(pair + 4)?,
                stomp_air_speed_scale: reader.f32(special + 8)?,
                stand_frames: reader.f32(special + 0xC)?,
            });
        }
        if kind == melee_types::ItemKind::Leadead {
            let article = reader.u32(articles + index * 4)?;
            let special = reader.u32(article + 4)?;
            let pair = reader.u32(special)?;
            // itCommonItems.h:1002: x18..x1C are s16, x1E an s8.
            assets.redead = Some(ReDeadAttributes {
                health: reader.s32(pair)?,
                walk_speed: reader.f32(pair + 4)?,
                reel_frames: reader.s32(special + 4)?,
                stagger_share: reader.f32(special + 8)?,
                fall_speed_scale: reader.f32(special + 0xC)?,
                lunge_reach: reader.f32(special + 0x10)?,
                lunge_speed: reader.f32(special + 0x14)?,
                walk_frames: i32::from(reader.s16(special + 0x18)?),
                stun_per_damage: i32::from(reader.s16(special + 0x1A)?),
                lunge_cooldown: i32::from(reader.s16(special + 0x1C)?),
                lunge_odds: i32::from(reader.s8(special + 0x1E)?),
            });
        }
        Ok(assets)
    }

    /// it_804D6D24 (itPublicData +4)[kind]: a common item's Article in ItCo.
    /// `article_states` is the kind's ItemStateTable anim_id column: the
    /// article state each motion state plays, or -1 for none.
    pub fn from_common(
        archive: &Archive,
        public: u32,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let articles = r.u32(public + 4)?;
        let article = r.u32(articles + kind as u32 * 4)?;
        let mut assets = Self::from_article(archive, article, article_states, special_attributes)?;
        assets.read_common_release(archive, public)?;
        Ok(assets)
    }

    fn from_article(
        archive: &Archive,
        article: u32,
        article_states: &[i32],
        special_count: u32,
    ) -> hsd_archive::desc::Result<Self> {
        Self::from_article_with_count(archive, article, article_states, special_count, 0)
    }

    fn from_article_with_count(
        archive: &Archive,
        article: u32,
        article_states: &[i32],
        special_count: u32,
        minimum_article_states: usize,
    ) -> hsd_archive::desc::Result<Self> {
        Self::from_article_with_attributes(
            archive,
            article,
            article_states,
            special_count,
            minimum_article_states,
            None,
        )
    }

    /// [`Self::from_article_with_count`], with the ItemAttr block taken
    /// from `attributes` (big-endian, as in memory) rather than the
    /// article's own pointer when given.
    fn from_article_with_attributes(
        archive: &Archive,
        article: u32,
        article_states: &[i32],
        special_count: u32,
        minimum_article_states: usize,
        attributes: Option<&[u8]>,
    ) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let (c, common) = match attributes {
            Some(bytes) => (hsd_archive::Reader::new(bytes), 0),
            None => (archive.reader(), r.u32(article)?),
        };
        let special = r.u32(article + 4)?;
        let state_array = r.u32(article + 12)?;
        let model_desc = r.u32(article + 16)?;
        let mut scripts = Vec::with_capacity(article_states.len());
        let mut hit_flags = Vec::with_capacity(article_states.len());
        for &row in article_states {
            let (commands, flags) = if row < 0 {
                (Vec::new(), Vec::new())
            } else {
                read_script(archive, r.u32(state_array + row as u32 * 16 + 12)?)?
            };
            scripts.push(commands);
            hit_flags.push(flags);
        }
        let article_state_count = article_states
            .iter()
            .map(|&s| s + 1)
            .max()
            .unwrap_or(0)
            .max(0)
            .max(minimum_article_states as i32);
        let special_attributes = (0..special_count)
            .map(|i| r.f32(special + i * 4))
            .collect::<Result<_>>()?;
        let visual = hsd_archive::desc::item_visual::ItemVisual::read(
            archive,
            model_desc,
            state_array,
            article_state_count as usize,
        )?;
        let attachment_translation = attachment_translation(
            &visual.model,
            r.u32(model_desc + 4)?,
            visual.attachment_bone,
        );
        let hurtbones = r.u32(article + 8)?;
        let hurtboxes = if hurtbones == 0 {
            Vec::new()
        } else {
            let descs = r.u32(hurtbones + 4)?;
            let vec = |at: u32| -> hsd_archive::desc::Result<hsd_types::Vec3> {
                Ok(hsd_types::Vec3::new(
                    r.f32(at)?,
                    r.f32(at + 4)?,
                    r.f32(at + 8)?,
                ))
            };
            (0..r.u32(hurtbones)?)
                .map(|i| {
                    let at = descs + i * 0x20;
                    Ok(ItemHurtbox {
                        bone: r.u32(at)?,
                        offsets: [vec(at + 4)?, vec(at + 0x10)?],
                        radius: r.f32(at + 0x1C)?,
                    })
                })
                .collect::<hsd_archive::desc::Result<_>>()?
        };
        assert!(hurtboxes.len() <= 2, "it_8027163C: item hit num over!");
        Ok(Self {
            hurtboxes,
            animation_ends: visual.states.iter().map(animation_end).collect(),
            joint_animated: visual.states.iter().map(joint_animated).collect(),
            visual,
            scripts,
            hit_flags,
            special_attributes,
            scale: c.f32(common + 0x60)?,
            model: r.u32(model_desc)?,
            rotate_to_facing: c.u8(common + 1)? & 0x20 != 0,
            push_box: melee_types::mp::ItEcb {
                top: c.f32(common + 0x20)?,
                bottom: c.f32(common + 0x24)?,
                right: c.f32(common + 0x28)?,
                left: c.f32(common + 0x2C)?,
            },
            pushable: c.u8(common + 1)? & 0x10 != 0,
            collision_box: melee_types::mp::ItEcb {
                top: c.f32(common + 0x40)?,
                bottom: c.f32(common + 0x44)?,
                right: c.f32(common + 0x48)?,
                left: c.f32(common + 0x4C)?,
            },
            collision_damage_multiplier: c.f32(common + 0x1C)?,
            hitlag: c.u8(common + 1)? & 0x08 != 0,
            camera_kind: (c.u8(common + 1)? >> 1) & 3,
            fall_acceleration: c.f32(common + 0x10)?,
            fall_speed_limit: c.f32(common + 0x14)?,
            launch_vertical_velocity: c.f32(common + 0x18)?,
            rotation_axis: c.u8(common + 1)? >> 6,
            explosion_lifetime: 0.0,
            common_lifetime: 0.0,
            half_life_scale: 0.0,
            spin_rate: c.f32(common + 0xC)?,
            fall_spin_degrees: 0.0,
            release_box_scale: 0.0,
            throw_break: 0,
            heavy: c.u8(common)? & 0x80 != 0,
            use_kind: (c.u8(common)? >> 3) & 0xF,
            hand_hold_kind: c.u8(common)? & 7,
            throw_speed_multiplier: c.f32(common + 4)?,
            bounce_scale: c.f32(common + 0x58)?,
            bounce_sound: c.u32(common + 0x80)?,
            rest_speed: c.f32(common + 0x5C)?,
            slide_speed: c.f32(common + 0x50)?,
            slide_friction: c.f32(common + 0x54)?,
            slide_rule: SlideRule::NEVER,
            landing_spin_degrees: 0.0,
            destroy_effect: u16::try_from(c.s32(common + 0x64)?).ok(),
            event_destroy_effect: u16::try_from(c.s32(common + 0x68)?).ok(),
            grab_offset: hsd_types::Vec2::new(c.f32(common + 0x30)?, c.f32(common + 0x34)?),
            grab_range: hsd_types::Vec2::new(c.f32(common + 0x38)?, c.f32(common + 0x3C)?),
            attachment_translation,
            bone_motion: None,
            pose: None,
            particle_tracks: None,
            special_pointees: Vec::new(),
            redead: None,
            kuriboh: None,
            monster_common: None,
        })
    }

    /// it_802E6AEC (ityaku.c) with Ground_801C0800 (ground.c:493-498): the
    /// stage material item, kind It_PKind_Random (0xA0). Its Article is
    /// ItCo's it_804D6D38[0xA0 - It_Kind_Kuriboh] (itPublicData +8), with
    /// it_803F8C08 for its ItemAttr; the stage's `ALDYakuAll[i]`, from
    /// i = 1 to the first null, replaces article state i's script. `None`
    /// when the stage has no ALDYakuAll.
    pub fn from_yaku(
        itco: &Archive,
        public: u32,
        stage: &Archive,
    ) -> hsd_archive::desc::Result<Option<Self>> {
        let Some(table) = stage.public("ALDYakuAll") else {
            return Ok(None);
        };
        let r = itco.reader();
        let articles = r.u32(public + 8)?;
        let article = r.u32(articles + YAKU_ARTICLE_INDEX * 4)?;
        let states: Vec<i32> = (0..YAKU_STATES as i32).collect();
        let mut assets = Self::from_article_with_attributes(
            itco,
            article,
            &states,
            0,
            0,
            Some(&YAKU_ATTRIBUTES),
        )?;
        for state in 1..YAKU_STATES {
            let Some(script) = stage.link(table + state as u32 * 4)? else {
                break;
            };
            let (commands, flags) = read_script(stage, script)?;
            assets.scripts[state] = commands;
            assets.hit_flags[state] = flags;
        }
        Ok(Some(assets))
    }

    /// Ground_801C0800 -> it_8026B40C: a stage's `itemdata` table pairs an
    /// item kind with its Article (it_804A0F60). `pointer_attributes` lists
    /// the special attribute words that hold a pointer to an integer; their
    /// integers become [`Self::special_pointees`] in that order. `None`
    /// when the stage carries no Article for `kind`.
    pub fn from_stage_item(
        archive: &Archive,
        kind: melee_types::ItemKind,
        article_states: &[i32],
        special_attributes: u32,
        pointer_attributes: &[u32],
    ) -> hsd_archive::desc::Result<Option<Self>> {
        let Some(table) = archive.public("itemdata") else {
            return Ok(None);
        };
        let r = archive.reader();
        for index in 0.. {
            let entry = r.u32(table + index * 4)?;
            if entry == 0 {
                return Ok(None);
            }
            if r.u32(entry)? != kind as u32 {
                continue;
            }
            let article = r.u32(entry + 4)?;
            let mut assets =
                Self::from_article(archive, article, article_states, special_attributes)?;
            let special = r.u32(article + 4)?;
            for &word in pointer_attributes {
                let pointee = r.u32(special + word * 4)?;
                assets.special_pointees.push(r.u32(pointee)? as i32);
            }
            return Ok(Some(assets));
        }
        unreachable!()
    }

    /// Samples every article state's joint animation for its DPtcl keys.
    pub fn read_particle_tracks(
        &mut self,
        archive: &Archive,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        let tracks = crate::particle_track::ParticleTracks::read(archive, &self.visual)?;
        // A key on a joint below the root attaches its generator to that
        // joint, whose matrix the pose gives.
        if tracks.below_root() && self.pose.is_none() {
            self.read_pose(archive)?;
        }
        self.particle_tracks = Some(tracks);
        Ok(())
    }

    /// Samples dynamic bone `bone`'s animation in every article state for
    /// itUpdateVelocityFromBone.
    /// Sample [`Self::pose`] from the article's model and joint animations.
    pub fn read_pose(
        &mut self,
        archive: &Archive,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        self.pose = Some(crate::pose::ItemPose::read(archive, &self.visual)?);
        Ok(())
    }
    pub fn read_bone_motion(
        &mut self,
        archive: &Archive,
        bone: usize,
    ) -> std::result::Result<(), hsd_anim::load::LoadError> {
        self.bone_motion = Some(crate::bone_motion::BoneMotion::read(
            archive,
            &self.visual,
            bone,
        )?);
        Ok(())
    }
}

/// it_80272CC0 (80272CC0) on a freshly loaded model: with a bone table
/// (ItemModelDesc x4 != 0) the index counts joints depth first, as the
/// table was filled; without one it walks first children from the root.
fn attachment_translation(
    model: &hsd_archive::desc::JObjDesc,
    bone_count: u32,
    index: usize,
) -> Option<hsd_types::Vec3> {
    fn depth_first<'a>(
        joint: &'a hsd_archive::desc::JObjDesc,
        out: &mut Vec<&'a hsd_archive::desc::JObjDesc>,
    ) {
        out.push(joint);
        if let Some(child) = &joint.child {
            depth_first(child, out);
        }
        if let Some(next) = &joint.next {
            depth_first(next, out);
        }
    }
    let joint = if bone_count != 0 {
        let mut order = Vec::new();
        depth_first(model, &mut order);
        order[index]
    } else {
        // it_80272CC0: the attach id's depth along the first children.
        let mut joint = model;
        for _ in 0..index {
            joint = joint.child.as_deref()?;
        }
        joint
    };
    Some(hsd_types::Vec3::new(joint.position.x, joint.position.y, joint.position.z))
}

/// ItHurtBoneDesc (it/types.h:177): a capsule on bone `bone` (0 is the
/// model root) between two offsets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemHurtbox {
    pub bone: u32,
    pub offsets: [hsd_types::Vec3; 2],
    pub radius: f32,
}

/// The frame at which an article state's joint animation stops (lb_8000B09C
/// finds no running AObj), or None when it has none or loops.
fn animation_end(state: &hsd_archive::desc::item_visual::ItemVisualState) -> Option<f32> {
    let mut end: Option<f32> = None;
    let mut stack: Vec<&hsd_archive::desc::AnimJoint> = state.joint.iter().collect();
    while let Some(joint) = stack.pop() {
        if let Some(aobj) = &joint.aobjdesc {
            if aobj.flags & hsd_anim::aobj::AOBJ_LOOP != 0 {
                return None;
            }
            // The latest end frame; a load-time summary of lb_8000B09C's scan.
            end = Some(end.map_or(aobj.end_frame, |e| gekko_math::cmp::max(e, aobj.end_frame)));
        }
        stack.extend(joint.child.as_deref());
        stack.extend(joint.next.as_deref());
    }
    end
}

/// Whether an article state's joint animation has an AObj on any joint
/// (lbFindJObjWithAObj's depth-first search).
fn joint_animated(state: &hsd_archive::desc::item_visual::ItemVisualState) -> bool {
    let mut stack: Vec<&hsd_archive::desc::AnimJoint> = state.joint.iter().collect();
    while let Some(joint) = stack.pop() {
        if joint.aobjdesc.is_some() {
            return true;
        }
        stack.extend(joint.child.as_deref());
        stack.extend(joint.next.as_deref());
    }
    false
}

/// itanimlist.c uses the shared ten control commands and its own payload table.
type DecodedScript = (Vec<Command>, Vec<Option<ItemHitFlags>>);
/// Decode a state's script and every subroutine it calls or jumps to into
/// one list; Call and Goto targets and continuations are indices into it
/// (Command_05/06/07). A target inside an already decoded block resolves to
/// that command.
fn read_script(archive: &Archive, offset: u32) -> Result<DecodedScript> {
    if offset == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut script = (Vec::new(), Vec::new());
    // Each decoded command's archive offset, in list order.
    let mut offsets: Vec<u32> = Vec::new();
    let mut jumps: Vec<(usize, u32)> = Vec::new();
    let mut pending = vec![offset];
    while let Some(block) = pending.pop() {
        if offsets.contains(&block) {
            continue;
        }
        read_block(archive, block, &mut script, &mut offsets, &mut jumps)?;
        pending.extend(jumps.iter().map(|&(_, target)| target));
    }
    for (index, target) in jumps {
        let start = offsets.iter().position(|&o| o == target).unwrap();
        match &mut script.0[index] {
            Command::Call { target, .. } | Command::Goto(target) => *target = start,
            _ => unreachable!("jump fix-up on a non-jump command"),
        }
    }
    Ok(script)
}

/// One block up to its End (Command_00), Return (Command_06) or Goto
/// (Command_07).
fn read_block(
    archive: &Archive,
    mut offset: u32,
    (commands, flags): &mut DecodedScript,
    offsets: &mut Vec<u32>,
    jumps: &mut Vec<(usize, u32)>,
) -> Result<()> {
    let r = archive.reader();
    loop {
        let word = r.u32(offset)?;
        let op = word >> 26;
        offsets.push(offset);
        let mut hit_flags = None;
        let command = match op {
            0..=4 | 6 | 8 => {
                melee_cmd::decode::decode(&[word], None, 0).expect("shared item command")
            }
            // Command_05: the target word follows; resolved once decoded.
            5 => {
                jumps.push((commands.len(), r.u32(offset + 4)?));
                offset += 4;
                Command::Call {
                    target: usize::MAX,
                    continuation: commands.len() + 1,
                }
            }
            // Command_07: a jump to the address in the next word.
            7 => {
                jumps.push((commands.len(), r.u32(offset + 4)?));
                offset += 4;
                Command::Goto(usize::MAX)
            }
            // it_80278F2C: five words, an effect at a joint with a random spread.
            10 => {
                let w = [
                    word,
                    r.u32(offset + 4)?,
                    r.u32(offset + 8)?,
                    r.u32(offset + 12)?,
                    r.u32(offset + 16)?,
                ];
                offset += 16;
                Command::Graphics(item_graphics(w))
            }
            11 => {
                let w = [
                    word,
                    r.u32(offset + 4)?,
                    r.u32(offset + 8)?,
                    r.u32(offset + 12)?,
                    r.u32(offset + 16)?,
                    r.u32(offset + 20)?,
                ];
                hit_flags = Some(ItemHitFlags::decode(w[4], w[5]));
                offset += 20;
                Command::SpawnHitbox {
                    id: ((word >> 23) & 7) as usize,
                    descriptor: decode_hitbox(w),
                }
            }
            12 => Command::SetHitboxDamage {
                id: ((word >> 23) & 7) as usize,
                damage: (word & 8191) as f32,
            },
            // it_802795EC: three-bit id, 23-bit size times 0.003906f.
            13 => Command::SetHitboxRadius {
                id: ((word >> 23) & 7) as usize,
                radius: 0.003906 * (word & 0x007f_ffff) as f32,
            },
            14 => Command::ClearHitbox((word & 0x03ff_ffff) as usize),
            15 => Command::ClearHitboxes,
            17..=19 => Command::SetVariable {
                index: (op - 17) as usize,
                value: word & 0x03ff_ffff,
            },
            // it_8027978C (8027978C): an item sound; its sub-opcode (bits
            // 18..25) 0..2 take the id and pan/volume words, 10 and 11 one
            // unused word, every other one none.
            16 => {
                let sub = (word >> 18) & 0xFF;
                let id = if sub <= 2 { r.u32(offset + 4)? } else { 0 };
                if sub <= 2 || sub == 10 || sub == 11 {
                    offset += 4;
                }
                offset += 4;
                Command::ItemSound { sub: sub as u8, id }
            }
            // it_80279888 -> it_80273598: rumble on the owner's controller.
            21 => Command::Rumble {
                all_players: false,
                id: ((word >> 16) & 0x3FF) as u16,
                duration: word as u16,
            },
            // it_8027990C -> ftLib_80086DC4: rumble on every fighter's controller.
            23 => Command::Rumble {
                all_players: true,
                id: ((word >> 16) & 0x3FF) as u16,
                duration: word as u16,
            },
            // it_80279958: one word, the overlay's index and argument
            // (CommandInfo unk13); it_802799A8: one word.
            24 => Command::ItemColorOverlay(Some(word & 0x03ff_ffff)),
            25 => Command::ItemColorOverlay(None),
            _ => unimplemented!("itanimlist.c item script opcode {op}"),
        };
        commands.push(command);
        flags.push(hit_flags);
        offset += 4;
        if op == 0 || op == 6 || op == 7 {
            return Ok(());
        }
    }
}

/// it_80278F2C (80278F2C): joint (low ten bits), effect id and parameter,
/// then offset and random range in 1/256 units.
fn item_graphics(w: [u32; 5]) -> melee_types::combat::GraphicsCommand {
    const SCALE: f32 = 0.003906;
    let half = |word: u32, high: bool| -> f32 {
        SCALE
            * (if high {
                (word >> 16) as i16
            } else {
                word as i16
            }) as f32
    };
    melee_types::combat::GraphicsCommand {
        bone: (w[0] & 0x3FF) as usize,
        common_bone: false,
        item_bone: false,
        destroy_on_state_change: false,
        id: (w[1] >> 16) as u16,
        parameter: (w[1] & 0xFFFF) as f32,
        offset: hsd_types::Vec3::new(half(w[2], true), half(w[2], false), half(w[3], true)),
        range: hsd_types::Vec3::new(half(w[3], false), half(w[4], true), half(w[4], false)),
        issued_facing: None,
        issued_grounded: None,
        issued_joint: None,
    }
}

/// it_802790C0: audited --fused has no fused instructions. Item hitbox
/// opcode 11 is SIX words; its bone/damage/flags differ from fighter opcode 11.
fn decode_hitbox(w: [u32; 6]) -> HitboxDescriptor {
    const SCALE: f32 = 0.003906;
    HitboxDescriptor {
        group: ((w[0] >> 20) & 7) as u8,
        bone: ((w[0] >> 13) & 127) as usize,
        common_bone: false,
        requires_throw_owner: false,
        // itanimlist.c:149: the sixth word's third byte, bit 3.
        item_hit_interaction: w[5] & (1 << 11) != 0,
        damage: (w[0] & 8191) as f32,
        shield_damage: (w[4] >> 9) as u8 as i8,
        sound_severity: ((w[4] >> 6) & 7) as u8,
        radius: SCALE * (w[1] >> 16) as f32,
        offset: [
            SCALE * (w[1] as i16) as f32,
            SCALE * ((w[2] >> 16) as i16) as f32,
            SCALE * (w[2] as i16) as f32,
        ]
        .into(),
        angle: (w[3] >> 23) as u16,
        growth: ((w[3] >> 14) & 511) as u16,
        weight_knockback: ((w[3] >> 5) & 511) as u16,
        base_knockback: (w[4] >> 23) as u16,
        element: melee_types::HitElement::try_from(((w[4] >> 18) & 31) as i32)
            .expect("item element"),
        // it_create_hitbox_4: x40_b3 (bit 1) hits grounded fighters, x40_b2
        // (bit 0) airborne ones (ftColl_8007925C).
        hit_ground: w[4] & (1 << 1) != 0,
        hit_air: w[4] & 1 != 0,
        ignore_scale: false,
        clank: w[4] & (1 << 17) != 0,
        rebound: false,
    }
}

/// Sixth item hitbox command word, it_802790C0: masks become named meanings
/// where ftcoll.c identifies them; remaining unknown flags stay explicit.
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemHitFlags {
    pub rehit_rate: u8,
    /// x41_b4: a landing on another item's hurt capsules records it with
    /// the rehit timer (it_802706D0_sub3's mode 8), so the hitbox may land
    /// on it again.
    pub item_rehit: bool,
    pub reflectable: bool,
    pub defense_interaction: bool,
    pub damage_without_hitlag: bool,
    /// x41_b6: a shield contact records the victim with the rehit timer
    /// (ftColl_80077688's mode 2), so the hitbox may strike the shield again.
    pub shield_rehit: bool,
    /// x42_b0: an absorbing bubble takes the hitbox (ftcoll.c:2137-2181).
    pub absorbable: bool,
    /// x42_b2: the hitbox misses a fighter facing the way its item does
    /// (ftColl_8007925C, ftcoll.c:2099-2103).
    pub misses_same_facing: bool,
    pub shieldable: bool,
    pub shield_bounce: bool,
    pub hits_hurtboxes: bool,
    pub grabbable_hurtboxes_only: bool,
    /// x42_b7: the hitbox can land on other items (it_802706D0).
    pub hits_items: bool,
    pub sound_kind: u8,
    pub auxiliary: u16,
}
impl ItemHitFlags {
    fn decode(last: u32, extra: u32) -> Self {
        Self {
            rehit_rate: (extra >> 24) as u8,
            item_rehit: extra & (1 << 23) != 0,
            reflectable: extra & (1 << 20) != 0,
            defense_interaction: extra & (1 << 15) != 0,
            damage_without_hitlag: extra & (1 << 22) != 0,
            shield_rehit: extra & (1 << 21) != 0,
            // it_802790C0 (itanimlist.c:140-148): byte 1's bits 4..0 are
            // x41_b7, x42_b0, x42_b1, x42_b2 and x42_b3.
            absorbable: extra & (1 << 19) != 0,
            misses_same_facing: extra & (1 << 17) != 0,
            shieldable: extra & (1 << 18) != 0,
            shield_bounce: extra & (1 << 16) != 0,
            hits_hurtboxes: extra & (1 << 14) != 0,
            grabbable_hurtboxes_only: extra & (1 << 13) != 0,
            hits_items: extra & (1 << 12) != 0,
            sound_kind: ((last >> 2) & 15) as u8,
            auxiliary: (extra >> 8) as u16,
        }
    }
}
