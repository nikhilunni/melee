//! Din's Fire (It_Kind_Zelda_DinFire), itzeldadinfire.c
//! (802C3AFC..802C4580): the fireball Zelda sends out from her hand. It
//! grows with every frame of flight, turns up or down with its creator's
//! stick while she keeps it (ftZd_SpecialLw_8013B540), and bursts when it
//! meets a surface, its lifetime ends or her script detonates it
//! (ftZd_SpecialLw_8013B574); the burst spawns the explosion.
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    DinFireState, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, LinkMessage,
    LinkRequest, LinkTarget, SpawnItem,
};
use melee_types::{mp::collide, ItemKind};

pub struct DinFire;

/// ItemStateTable_ZeldaDinFire's anim_id column: flying, then bursting.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
const FLYING: u16 = 0;
const BURSTING: u16 = 1;
/// ftData.x48_items index (ftZd_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;
/// ItZeldaDinFire_ItemVars x0..x2C.
pub const SPECIAL_ATTRIBUTES: u32 = 12;

/// efSync_Spawn(1272 / 1273, gobj, jobj): the flying and bursting flames
/// (efLib_CreateGenerator_Attach_Scale 0x6E / 0x1C8 on the fire's JObj).
const FLYING_FLAME: u16 = 0x4F8;
const BURST_FLAME: u16 = 0x4F9;
/// `M_PI` and `M_PI_2` as the C's doubles.
const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;

/// The fire's special attributes (ItZeldaDinFire_ItemVars).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the flight's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: the frames of flight to full charge.
    fn charge_frames(&self) -> f32 {
        self.0[1]
    }
    /// x8 / xC: the model's scale uncharged and fully charged.
    fn scale(&self) -> (f32, f32) {
        (self.0[2], self.0[3])
    }
    /// x10: the launch angle offset, facing right.
    fn launch_angle(&self) -> f32 {
        self.0[4]
    }
    /// x14: the launch speed.
    fn launch_speed(&self) -> f32 {
        self.0[5]
    }
    /// x18 / x1C: the speed gained per steered frame and its limit.
    fn acceleration(&self) -> (f32, f32) {
        (self.0[6], self.0[7])
    }
    /// x20: the vertical stick needed to steer.
    fn steer_threshold(&self) -> f32 {
        self.0[8]
    }
    /// x24: the angle per unit of vertical stick.
    fn steer_rate(&self) -> f32 {
        self.0[9]
    }
    /// x28: the steering's angle limit.
    fn steer_limit(&self) -> f32 {
        self.0[10]
    }
    /// x2C: the burst's lifetime.
    fn burst_lifetime(&self) -> f32 {
        self.0[11]
    }
}

pub(crate) fn state(item: &mut ItemCore) -> &mut DinFireState {
    match &mut item.scratch {
        ItemScratch::DinFire(state) => state,
        _ => unreachable!("a Din's Fire article without its state"),
    }
}

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: flying_anim,
        physics: flying_physics,
        collision: flying_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: burst_anim,
        physics: stop,
        collision: no_collision,
    },
];

/// it_3F2F.c's Logic65 row.
impl ItemLogic for DinFire {
    const KIND: ItemKind = ItemKind::ZeldaDinFire;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::DinFire(DinFireState::default());
    }
    /// it_802C3BAC (802C3BAC) once Item_80268B18 returns: the command
    /// variables, the lifetime, the creator (the parent), then it_802C3D74.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        set_lifetime(item, assets, common);
        let creator = item.owner;
        let fire = state(item);
        fire.charge = 0.0;
        fire.reflected = false;
        fire.creator = creator;
        launch(item, assets, common);
    }
    /// itZeldaDinFire_Logic65_Reflected (802C44A8): out of its creator's
    /// hands, turned round (the JObj's Y rotation in double) and flying
    /// back.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        state(item).reflected = true;
        item.facing = -item.facing;
        item.rotation.y = (HALF_PI * f64::from(item.facing)) as f32;
        item.velocity.x = -item.velocity.x;
        item.velocity.y = -item.velocity.y;
        item.velocity.z = 0.0;
        false
    }
    /// itZeldaDinFire_Logic65_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itZeldaDinFire_Logic65_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// it_802C3D44 (802C3D44), from ftZd_SpecialLw_8013B5EC when Zelda is
    /// hit or dies with her fire out: it flies on without an owner.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Orphan => {
                item.owner = None;
                state(item).creator = None;
                item.held = false;
            }
            _ => unreachable!("{control:?} sent to Din's Fire"),
        }
    }
    /// itZeldaDinFire_Logic65_Destroyed: its creator hears of it
    /// (ftZd_SpecialLw_8013B5C4) while the fire is still hers.
    fn notifies_owner(item: &ItemCore) -> bool {
        match &item.scratch {
            ItemScratch::DinFire(fire) => fire.creator.is_some() && fire.creator == item.owner,
            _ => false,
        }
    }
    /// ftZd's itZeldaDinFire_GetOwner: Zelda reads whether the fire is
    /// still hers through the article her owner report finds.
    fn owner_report(item: &ItemCore, _assets: &ItemAssets) -> Option<melee_it::ArticleReport> {
        Some(melee_it::ArticleReport {
            point: item.position,
            struck: false,
        })
    }
}

/// it_80275158: both timers.
fn set_lifetime(item: &mut ItemCore, assets: &ItemAssets, common: &ItemCommonData) {
    let lifetime = Attributes(&assets.special_attributes).lifetime();
    item.life_timer = lifetime;
    item.half_life = lifetime * common.half_life_scale;
}

/// it_802C3D74 (802C3D74): not grabbable, shown, flying, the lifetime; the
/// flight's angle (the attribute offset toward the facing, from a base of
/// 0 or the double M_PI) and speed; the flame; the velocity along them
/// (separate fmuls of the fadds'd angle's cosine and sine).
fn launch(item: &mut ItemCore, assets: &ItemAssets, common: &ItemCommonData) {
    let a = Attributes(&assets.special_attributes);
    item.grabbable = false;
    item.held = false;
    item.hidden = false;
    item.change_motion_with(FLYING, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    set_lifetime(item, assets, common);
    let facing = item.facing;
    let fire = state(item);
    fire.charge = 0.0;
    fire.reflected = false;
    fire.angle_offset = a.launch_angle() * facing;
    fire.base_angle = if facing == 1.0 { 0.0 } else { PI as f32 };
    fire.speed = a.launch_speed();
    item.events.push(ItemEvent::OwnEffect { id: FLYING_FLAME });
    state(item).effects = true;
    set_velocity(item);
}

/// The velocity along the flight's angle at its speed.
fn set_velocity(item: &mut ItemCore) {
    let fire = *state(item);
    let angle = fire.base_angle + fire.angle_offset;
    item.velocity.x = fire.speed * cosf(angle);
    item.velocity.y = fire.speed * sinf(angle);
    item.velocity.z = 0.0;
}

/// Whether the fire is still in its creator's hands.
fn creator_holds(item: &mut ItemCore) -> bool {
    let owner = item.owner;
    let creator = state(item).creator;
    creator.is_some() && owner == creator
}

/// itZeldadinfire_UnkMotion0_Anim's inline (802C3F10): the flame goes, the
/// fire bursts with the burst's flame and lifetime.
fn burst(item: &mut ItemCore, assets: &ItemAssets) {
    if std::mem::take(&mut state(item).effects) {
        item.events.push(ItemEvent::DestroyEffects);
    }
    item.change_motion_with(BURSTING, ARTICLE_STATES[1], ANIM_UPDATE, assets);
    item.events.push(ItemEvent::OwnEffect { id: BURST_FLAME });
    state(item).effects = true;
    item.life_timer = Attributes(&assets.special_attributes).burst_lifetime();
}

/// The model's scale for the charge: charge * ((full - empty) / frames) +
/// empty (fsubs, fdivs, then fmadds at 802C3F8C / 802C40F0).
fn charged_scale(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
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

/// itZeldadinfire_UnkMotion0_Anim (802C3E94): unreflected, the charge
/// grows to its limit and the creator's script may detonate it; the
/// model's scale; a spent lifetime bursts it too.
fn flying_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if !state(item).reflected {
        let frames = Attributes(&ctx.assets.special_attributes).charge_frames();
        let fire = state(item);
        fire.charge += 1.0;
        if fire.charge >= frames {
            fire.charge = frames;
        }
        if creator_holds(item) && ctx.owner.is_some_and(|owner| owner.detonating_article) {
            burst(item, ctx.assets);
        }
    }
    charged_scale(item, ctx.assets);
    if lifetime_spent(item) {
        burst(item, ctx.assets);
    }
    false
}

/// itZeldadinfire_UnkMotion1_Anim (802C408C): the model's scale; once the
/// burst's lifetime is spent the flame goes and, still in its creator's
/// hands, the fire leaves the explosion (it_802C4580) and lets her go.
fn burst_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    charged_scale(item, ctx.assets);
    if !lifetime_spent(item) {
        return false;
    }
    if std::mem::take(&mut state(item).effects) {
        item.events.push(ItemEvent::DestroyEffects);
    }
    if creator_holds(item) {
        let owner = item.owner.expect("the creator");
        let mut position = item.position;
        position.z = 0.0;
        let mut spawn =
            SpawnItem::attached(ItemKind::ZeldaDinFireExplode, owner, position, item.facing);
        spawn.spawn_argument = crate::explode::charge_argument(state(item).charge);
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Spawn(spawn),
            message: LinkMessage::Spawned,
        });
    }
    true
}

/// itZeldadinfire_UnkMotion0_Phys (802C4198): while its creator steers,
/// a vertical stick past the threshold turns it (fmadds at 802C42A0) up to
/// the limit, and it speeds up to its limit.
fn flying_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    if state(item).reflected || !creator_holds(item) {
        return;
    }
    let Some(owner) = ctx.owner.filter(|owner| owner.steering_article) else {
        return;
    };
    let a = Attributes(&ctx.assets.special_attributes);
    let stick_y = owner.stick.y;
    let facing = item.facing;
    let fire = state(item);
    if stick_y.abs() > a.steer_threshold() {
        fire.angle_offset = fmadds(facing, a.steer_rate() * stick_y, fire.angle_offset);
        if fire.angle_offset.abs() > a.steer_limit() {
            fire.angle_offset = if fire.angle_offset > 0.0 {
                a.steer_limit()
            } else {
                -a.steer_limit()
            };
        }
    }
    let (acceleration, limit) = a.acceleration();
    fire.speed += acceleration;
    if fire.speed > limit {
        fire.speed = limit;
    }
    set_velocity(item);
}

/// itResetVelocity.
fn stop(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity = Vec3::ZERO;
}

/// itZeldadinfire_UnkMotion0_Coll -> it_802C3AFC (802C3AFC): an airborne
/// pass (it_8026DA08); a ceiling while rising or a floor otherwise, a left
/// wall moving right or a right wall otherwise, bursts it.
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
        burst(item, ctx.assets);
    }
    false
}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
