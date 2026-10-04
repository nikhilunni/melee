//! PK Flash (It_Kind_Ness_PKFlush, itnesspkflash.c 802AA7E4..802AB3F0)
//! and its explosion (It_Kind_Ness_PKFlush_Explode, itnesspkflashexplode.c
//! 802AF940..802AFD8C). The flash rises from Ness's hand, drifts with his
//! stick while he holds the charge row, grows with every frame of flight
//! and bursts when he lets go, its lifetime ends or it meets a surface; the
//! burst spawns the explosion, sized and powered by the charge.
use gekko_math::{
    fma::fmadds,
    msl::{cosf, fctiwz, sinf},
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, LinkMessage,
    LinkRequest, LinkTarget, PkFlashState, SpawnItem,
};
use melee_types::{mp::collide, ItemKind};

pub struct NessPkFlash;
pub struct NessPkFlashExplode;

/// ftData.x48_items indices (ftNs_Init_OnLoad).
pub const FLASH_ARTICLE_INDEX: u32 = 2;
pub const EXPLODE_ARTICLE_INDEX: u32 = 8;
/// it_803F6B70's anim_id column: flying, bursting, burst on a surface.
pub const FLASH_ARTICLE_STATES: [i32; 3] = [0, 1, 2];
/// it_803F6F40's anim_id column.
pub const EXPLODE_ARTICLE_STATES: [i32; 1] = [0];
/// itFlashAttributes x0..x28 and itFlashExplAttributes x0..x10.
pub const FLASH_SPECIAL_ATTRIBUTES: u32 = 11;
pub const EXPLODE_SPECIAL_ATTRIBUTES: u32 = 5;

/// The flash's motion states.
pub const FLYING: u16 = 0;
const BURSTING: u16 = 1;
/// The burst a surface starts, which Ness reads (it_802AA7F0).
pub const STRUCK: u16 = 2;

/// Item_8026AE84(ip, 0x86, 0x7F, 0x40): the flash meeting a surface.
const SURFACE_SOUND: u32 = 0x86;
/// it_802AFA70: it_80275158(gobj, 1024.0f).
const EXPLOSION_LIFETIME: f32 = 1024.0;
/// it_802AFA70: the charge's share of the full charge above which the
/// explosion shakes the camera (Camera_RequestQuake's small and medium
/// kinds, 2 and 3; the rumble has no simulated observer).
const SMALL_QUAKE_SHARE: f32 = 0.65;
const MEDIUM_QUAKE_SHARE: f32 = 0.849_999_96;
const SMALL_QUAKE: u16 = 2;
const MEDIUM_QUAKE: u16 = 3;
/// itNesspkflash_UnkMotion0_Phys: the horizontal stick needed to steer.
const STEER_THRESHOLD: f32 = 0.2;
/// `M_PI_2` as the float the launch adds (retail @211).
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
/// 0.017453292f, degrees to radians.
const DEGREES: f32 = 0.017453292;

/// The flash's special attributes (itFlashAttributes).
struct Flash<'a>(&'a [f32]);
impl Flash<'_> {
    /// x0 FLASH_LIFETIMER.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4 FLASH_HITBOX_SIZE_MUL: the frames of flight to full charge.
    fn charge_frames(&self) -> f32 {
        self.0[1]
    }
    /// x8 / xC: the model's scale uncharged and fully charged.
    fn scale(&self) -> (f32, f32) {
        (self.0[2], self.0[3])
    }
    /// x10: the launch's lean toward the facing, in degrees.
    fn launch_degrees(&self) -> f32 {
        self.0[4]
    }
    /// x14 FLASH_PEAK_RISE_HEIGHT: the launch speed.
    fn launch_speed(&self) -> f32 {
        self.0[5]
    }
    /// x18 FLASH_CONTROL: horizontal speed per unit of stick, per frame.
    fn control(&self) -> f32 {
        self.0[6]
    }
    /// x1C FLASH_GRAVITY.
    fn gravity(&self) -> f32 {
        self.0[7]
    }
    /// x20: the horizontal speed limit.
    fn drift_limit(&self) -> f32 {
        self.0[8]
    }
    /// x24: the fall's terminal speed.
    fn terminal_speed(&self) -> f32 {
        self.0[9]
    }
    /// x28 FLASH_EXPLOSION_DELAY: the burst's lifetime.
    fn burst_frames(&self) -> f32 {
        self.0[10]
    }
}

/// The explosion's special attributes (itFlashExplAttributes).
struct Explosion<'a>(&'a [f32]);
impl Explosion<'_> {
    /// x0: the full charge.
    fn full_charge(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the model's scale uncharged and fully charged.
    fn scale(&self) -> (f32, f32) {
        (self.0[1], self.0[2])
    }
    /// xC / x10: the hitbox's damage uncharged and per unit of charge.
    fn damage(&self) -> (f32, f32) {
        (self.0[3], self.0[4])
    }
}

fn state(item: &mut ItemCore) -> &mut PkFlashState {
    match &mut item.scratch {
        ItemScratch::PkFlash(state) => state,
        _ => unreachable!("a PK Flash article without its state"),
    }
}

/// Whether the flash is still in its creator's hands (xDE0 and the owner
/// agree).
fn creator_holds(item: &mut ItemCore) -> bool {
    let owner = item.owner;
    let creator = state(item).creator;
    creator.is_some() && owner == creator
}

static FLASH_STATES: [ItemStateRow; 3] = [
    ItemStateRow {
        animation_id: FLASH_ARTICLE_STATES[0],
        animation: flying_anim,
        physics: flying_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: FLASH_ARTICLE_STATES[1],
        animation: burst_anim,
        physics: stop,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: FLASH_ARTICLE_STATES[2],
        animation: struck_anim,
        physics: stop,
        collision: no_collision,
    },
];

/// it_3F2F.c's Logic102 row.
impl ItemLogic for NessPkFlash {
    const KIND: ItemKind = ItemKind::NessPKFlush;
    const STATES: &'static [ItemStateRow] = &FLASH_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::PkFlash(PkFlashState::default());
    }
    /// it_802AA8C0 (802AA8C0) once Item_80268B18 returns: the command
    /// variables, the lifetime, the creator (the parent), then it_802AAA80
    /// (802AAA80): not grabbable, shown, flying; the launch leans x10
    /// degrees toward the facing from straight up (retail 0x802AAB08:
    /// fmadds) at speed x14.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Flash(&assets.special_attributes);
        item.command_variables = [0; 4];
        let creator = item.owner;
        *state(item) = PkFlashState {
            creator,
            ..Default::default()
        };
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(FLYING, FLASH_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.life_timer = a.lifetime();
        item.half_life = a.lifetime() * common.half_life_scale;
        let angle = fmadds(DEGREES * a.launch_degrees(), item.facing, HALF_PI);
        item.velocity.x = -a.launch_speed() * cosf(angle);
        item.velocity.y = a.launch_speed() * sinf(angle);
        item.velocity.z = 0.0;
    }
    /// itNesspkflash_Logic102_Reflected (802AB2AC): out of its creator's
    /// hands, turned round (the JObj's Y rotation in double) and flying
    /// back.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        state(item).reflected = true;
        item.facing = -item.facing;
        item.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        item.velocity.z = 0.0;
        false
    }
    /// itNesspkflash_Logic102_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNesspkflash_Logic102_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_802AAA50 (802AAA50), from ftNs_SpecialN_ItemPKFlushSetNULL when
    /// Ness is hit or dies with his flash out: it flies on without an owner.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Orphan => {
                item.owner = None;
                state(item).creator = None;
                item.held = false;
            }
            _ => unreachable!("{control:?} sent to PK Flash"),
        }
    }
    /// it_2725_Logic102_Destroyed (802AA9CC): its creator hears of it
    /// (ftNs_SpecialN_SetNULL) while the flash is still his.
    fn notifies_owner(item: &ItemCore) -> bool {
        match &item.scratch {
            ItemScratch::PkFlash(flash) => flash.creator.is_some() && flash.creator == item.owner,
            _ => false,
        }
    }
    /// it_802AA7E4 / it_802AA7F0: Ness reads whether the flash is still
    /// his (the report exists) and whether a surface burst it (motion 2).
    fn owner_report(item: &ItemCore, _assets: &ItemAssets) -> Option<melee_it::ArticleReport> {
        Some(melee_it::ArticleReport {
            point: item.position,
            struck: item.motion == STRUCK,
        })
    }
}

/// itNesspkflash_SetScale: the model's scale for the charge (fsubs, fdivs,
/// then fmadds at 0x802AACA0 and its copies).
fn charged_scale(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Flash(&assets.special_attributes);
    let (empty, full) = a.scale();
    let charge = state(item).charge;
    let scale = fmadds(charge, (full - empty) / a.charge_frames(), empty);
    item.model_scale = Vec3::new(scale, scale, scale);
}

/// it_80273130: the lifetime counts down; true when it is spent.
fn lifetime_spent(item: &mut ItemCore) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// The burst both of the flying row's exits start: motion 1 with the burst
/// lifetime, the first command variable counting its frames.
fn burst(item: &mut ItemCore, assets: &ItemAssets) {
    item.change_motion_with(BURSTING, FLASH_ARTICLE_STATES[1], ANIM_UPDATE, assets);
    item.life_timer = Flash(&assets.special_attributes).burst_frames();
    item.command_variables[0] = 0;
}

/// itNesspkflash_UnkMotion0_Anim (802AAB70): the looping animation
/// restarts at its end; unreflected, the charge grows to its limit and the
/// flash bursts once its creator leaves the charge row; a spent lifetime
/// bursts it too.
fn flying_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        item.change_motion_with(FLYING, FLASH_ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    }
    if !state(item).reflected {
        let frames = Flash(&ctx.assets.special_attributes).charge_frames();
        let flash = state(item);
        flash.charge += 1.0;
        if flash.charge >= frames {
            flash.charge = frames;
        }
        if creator_holds(item) && !ctx.owner.is_some_and(|owner| owner.steering_article) {
            burst(item, ctx.assets);
        }
    }
    if lifetime_spent(item) {
        burst(item, ctx.assets);
    }
    charged_scale(item, ctx.assets);
    false
}

/// itNesspkflash_UnkMotion1_Anim (802AAD30): two frames before the burst's
/// lifetime ends a flash still in its creator's hands leaves the explosion
/// (it_802AF940) and goes; otherwise it lasts the lifetime out.
fn burst_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.command_variables[0] = item.command_variables[0].wrapping_add(1);
    if !state(item).reflected && creator_holds(item) {
        let delay = fctiwz(Flash(&ctx.assets.special_attributes).burst_frames());
        if item.command_variables[0] as i32 >= delay - 2 {
            let owner = item.owner.expect("the creator");
            let mut spawn = SpawnItem::ray(
                ItemKind::NessPKFlushExplode,
                owner,
                item.position,
                item.facing,
            );
            spawn.spawn_argument = state(item).charge.to_bits() as i32;
            item.link_requests.push(LinkRequest {
                target: LinkTarget::Spawn(spawn),
                message: LinkMessage::Spawned,
            });
            return true;
        }
    }
    charged_scale(item, ctx.assets);
    lifetime_spent(item)
}

/// itNesspkflash_UnkMotion2_Anim (802AAEE0): the scale and the lifetime.
fn struck_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    charged_scale(item, ctx.assets);
    lifetime_spent(item)
}

/// itNesspkflash_UnkMotion0_Phys (802AAF70): while its creator holds the
/// charge row a horizontal stick past 0.2 drifts it (retail 0x802AB098:
/// fmadds) up to x20; it falls at x1C to x24.
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    let a = Flash(&ctx.assets.special_attributes);
    if !state(item).reflected && creator_holds(item) {
        if let Some(owner) = ctx.owner.filter(|owner| owner.steering_article) {
            let stick_x = owner.stick.x;
            if stick_x.abs() > STEER_THRESHOLD {
                item.velocity.x = fmadds(stick_x, a.control(), item.velocity.x);
                if item.velocity.x.abs() > a.drift_limit() {
                    item.velocity.x = if item.velocity.x > 0.0 {
                        a.drift_limit()
                    } else {
                        -a.drift_limit()
                    };
                }
            }
        }
    }
    item.velocity.y -= a.gravity();
    if item.velocity.y < -a.terminal_speed() {
        item.velocity.y = -a.terminal_speed();
    }
    item.velocity.z = 0.0;
}

/// itResetVelocity.
fn stop(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

/// itNesspkflash_UnkMotion0_Coll (802AB154) -> it_802AA810 (802AA810): an
/// airborne pass (it_8026DA08); a ceiling while rising or a floor
/// otherwise, a left wall moving right or a right wall otherwise, bursts
/// it in motion 2 with its sounds stopped and the surface sound.
fn flying_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let env = item.airborne_contacts(ctx.map);
    let vertical = if item.velocity.y > 0.0 {
        collide::CEILING_MASK
    } else {
        collide::FLOOR_MASK
    };
    let horizontal = if item.velocity.x > 0.0 {
        collide::LEFT_WALL_MASK
    } else {
        collide::RIGHT_WALL_MASK
    };
    if env & (vertical | horizontal) != 0 {
        item.change_motion_with(STRUCK, FLASH_ARTICLE_STATES[2], ANIM_UPDATE, ctx.assets);
        item.life_timer = Flash(&ctx.assets.special_attributes).burst_frames();
        item.sound_requests.push(SURFACE_SOUND);
    }
    charged_scale(item, ctx.assets);
    false
}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

static EXPLODE_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: EXPLODE_ARTICLE_STATES[0],
    animation: blast_anim,
    physics: hold_still,
    collision: no_collision,
}];

/// it_3F2F.c's Logic103 row.
impl ItemLogic for NessPkFlashExplode {
    const KIND: ItemKind = ItemKind::NessPKFlushExplode;
    const STATES: &'static [ItemStateRow] = &EXPLODE_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::PkFlash(PkFlashState::default());
    }
    /// it_802AF940 (802AF940) once Item_80268B18 returns, then it_802AFA70
    /// (802AFA70): the charge; not grabbable, shown, state 0, a 1024-frame
    /// life; one animation step (Item_802694CC) and the animation callback
    /// (its result dropped); a quake for a strong charge.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        state(item).charge = f32::from_bits(spawn.spawn_argument as u32);
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(0, EXPLODE_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.life_timer = EXPLOSION_LIFETIME;
        item.half_life = EXPLOSION_LIFETIME * common.half_life_scale;
        state(item).hitbox_size = 0.0;
        item.advance_animation(assets);
        blast(item, assets);
        let share = state(item).charge / Explosion(&assets.special_attributes).full_charge();
        let quake = if share > MEDIUM_QUAKE_SHARE {
            Some(MEDIUM_QUAKE)
        } else if share > SMALL_QUAKE_SHARE {
            Some(SMALL_QUAKE)
        } else {
            None
        };
        if let Some(kind) = quake {
            item.events.push(ItemEvent::Quake {
                kind,
                joint: 0,
                offset: Vec3::ZERO,
            });
        }
    }
    /// itNessPKFlashExplode_Logic103_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNessPKFlashExplode_Logic103_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNessPKFlashExplode_Logic103_ShieldBounced.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itNessPKFlashExplode_Logic103_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// itNessPKFlashExplode_UnkMotion0_Anim's body: the model's scale for the
/// charge (fmadds at 0x802AFBF4); once, while the script's third command
/// variable is clear, a live hitbox takes the charge's damage (fmadds at
/// 0x802AFCA8, then __cvt_fp2unsigned); its authored size, read once,
/// times the scale.
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Explosion(&assets.special_attributes);
    let (empty, full) = a.scale();
    let charge = state(item).charge;
    let scale = fmadds(charge, (full - empty) / a.full_charge(), empty);
    item.model_scale = Vec3::new(scale, scale, scale);
    if item.command_variables[2] == 0 && item.hitboxes[0].is_some() {
        let (base, per_charge) = a.damage();
        let damage = fctiwz(fmadds(charge, per_charge, base)) as u32;
        item.set_hitbox_damage(0, damage);
        item.command_variables[2] = 1;
    }
    // Retail reads and writes the capsule's scale whether or not it is
    // live; a dead capsule keeps the last one made (Item.x3C).
    let radius = match &item.hitboxes[0] {
        Some(hit) => hit.descriptor.radius,
        None => return,
    };
    let size = &mut state(item).hitbox_size;
    if *size == 0.0 {
        *size = radius;
    }
    let size = *size;
    item.hitboxes[0].as_mut().unwrap().descriptor.radius = size * scale;
}

/// itNessPKFlashExplode_UnkMotion0_Anim (802AFB80): the blast; the
/// explosion ends with its animation (it_80272C6C).
fn blast_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    blast(item, ctx.assets);
    item.article_animation_ended(ctx.assets)
}

/// itNessPKFlashExplode_UnkMotion0_Phys: the vertical and depth velocity
/// clear (retail clears z twice and leaves x).
fn hold_still(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity.z = 0.0;
    item.velocity.y = 0.0;
}
