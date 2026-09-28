//! Thunder's bolts (It_Kind_Pikachu_Thunder, itpikachuthunder.c,
//! 802B1DEC..802B25B8): a chain spawned high above Pikachu. Each waits its
//! delay, falls, and once struck (the lead meeting the floor or Pikachu,
//! the others reaching where the one before stopped) shrinks away.
use hsd_types::Vec3;
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, state_change::HIT_PRESERVE, ArticleReport,
    ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent, ItemEventContext,
    ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, LinkMessage, LinkRequest,
    LinkTarget, ThunderState,
};
use melee_types::ItemKind;

pub struct ThunderBolt;

/// it_803F70C8's anim_id column: waiting (no animation), falling, struck.
pub const ARTICLE_STATES: [i32; 3] = [-1, 0, 0];
const WAITING: u16 = 0;
const FALLING: u16 = 1;
const STRUCK: u16 = 2;

/// it_80272AC4's spark where the lead meets the floor.
const FLOOR_SPARK: u16 = 0x40C;
/// Camera_RequestQuake(QuakeKind_Small) (ftPk_SpecialLw_80127608), as the
/// scene's quake kinds count it.
const SMALL_QUAKE: u16 = 2;
/// ftPk_MF_Special's x2071 nibble (Ft_MF_UnkUpdatePhys | FreezeState),
/// shared by every Pikachu special row.
const SPECIAL_PROPERTY: u8 = 3;

/// itPikachuthunderAttributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the falling bolt's lifetime.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4: a bolt's full length.
    fn length(&self) -> f32 {
        self.0[1]
    }
    /// x8: the tip's distance below the bolt's position.
    fn tip(&self) -> f32 {
        self.0[2]
    }
}

fn thunder(item: &mut ItemCore) -> &mut ThunderState {
    match &mut item.scratch {
        ItemScratch::Thunder(state) => state,
        _ => unreachable!("a Thunder bolt without its state"),
    }
}
fn thunder_ref(item: &ItemCore) -> &ThunderState {
    match &item.scratch {
        ItemScratch::Thunder(state) => state,
        _ => unreachable!("a Thunder bolt without its state"),
    }
}

/// ftPk_SpecialLw_CheckProperty (8012757C) on the owner's motion: its
/// x2071 nibble is one of 1, 2 or 4..=13.
pub fn owner_busy(motion: u16) -> bool {
    let property = melee_types::motion_property::COMMON_MOTION_PROPERTIES
        .get(usize::from(motion))
        .copied()
        .unwrap_or(SPECIAL_PROPERTY);
    matches!(property, 1 | 2 | 4..=13)
}

static STATES: [ItemStateRow; 3] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: waiting_anim,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: falling_anim,
        physics: no_physics,
        collision: falling_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[2],
        animation: struck_anim,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for ThunderBolt {
    const KIND: ItemKind = ItemKind::PikachuThunder;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::Thunder(ThunderState::default());
    }
    /// it_80275158's half-life for the lifetime every state here restarts
    /// (it_802B2080, it_802B211C); the struck lifetime leaves it.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &melee_it::SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.half_life = Attributes(&assets.special_attributes).lifetime() * common.half_life_scale;
    }
    /// it_802B1DF8's loop body (802B1ED8..F90) for one bolt: it takes no
    /// hits (it_802756D0), waits hidden (it_802B2080: motion 0 with no
    /// animation, the lifetime), leaves the blast zones unchecked until it
    /// falls, and learns its place, delay and velocity; the struck
    /// lifetime is |length / fall speed| (fdivs). xDC8 xD's clearing only
    /// concerns a counter's own hitlag, which never meets a bolt here.
    fn link_received(item: &mut ItemCore, message: LinkMessage, assets: &ItemAssets) -> bool {
        match message {
            LinkMessage::Chain {
                index,
                delay,
                velocity,
            } => {
                let a = Attributes(&assets.special_attributes);
                item.hurt_intangible = true;
                item.change_motion_with(WAITING, ARTICLE_STATES[0], ANIM_UPDATE, assets);
                item.life_timer = a.lifetime();
                item.hidden = true;
                item.blast_zone_checked = false;
                let length = a.length();
                *thunder(item) = ThunderState {
                    index,
                    struck: false,
                    delay,
                    length,
                    next_length: 0.0,
                    strike_frames: (length / velocity.y).abs(),
                    scale: 1.0,
                    velocity,
                    reached: Vec3::ZERO,
                };
            }
            // it_802B22B8's inline update of the next bolt.
            LinkMessage::Reached { position } => {
                let state = thunder(item);
                state.reached = position;
                state.struck = true;
            }
            _ => unimplemented!("Thunder bolt message {message:?}"),
        }
        false
    }
    /// it_802B1FE8 / it_802B1DEC for Pikachu's lead-bolt check: the tip
    /// below the bolt (fmuls of the tip by the scale, fadds) and whether it
    /// struck.
    fn owner_report(item: &ItemCore, assets: &ItemAssets) -> Option<ArticleReport> {
        let state = thunder_ref(item);
        if state.index != 0 {
            return None;
        }
        let offset = -Attributes(&assets.special_attributes).tip() * state.scale;
        let mut point = item.position;
        point.y += offset;
        Some(ArticleReport {
            point,
            struck: state.struck,
        })
    }
    /// it_2725_Logic39_Destroyed: only the lead tells Pikachu.
    fn notifies_owner(item: &ItemCore) -> bool {
        thunder_ref(item).index == 0
    }
    /// it_802B1FC8: Pikachu struck by its lead bolt.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        match control {
            ItemControl::Strike if thunder_ref(item).index == 0 => strike(item, assets),
            ItemControl::Strike => {}
            _ => unimplemented!("Thunder bolt control {control:?}"),
        }
    }
    /// itPikachuThunder_Logic39_DmgDealt: the bolt keeps going.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        false
    }
}

/// The states' NULL physics: the velocity stays.
fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// itPikachuthunder_UnkMotion0_Anim (802B20D8): the delay counts down,
/// then the bolt falls (it_802B211C: motion 1, the lifetime, its
/// velocity, the blast zones checked and the model shown).
fn waiting_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = thunder(item);
    if state.delay <= 0 {
        let velocity = state.velocity;
        item.change_motion_with(FALLING, ARTICLE_STATES[1], ANIM_UPDATE, ctx.assets);
        let lifetime = Attributes(&ctx.assets.special_attributes).lifetime();
        item.life_timer = lifetime;
        item.velocity = velocity;
        item.blast_zone_checked = true;
        item.hidden = false;
    } else {
        state.delay -= 1;
    }
    false
}

/// itPikachuthunder_UnkMotion1_Anim (802B2194): the lifetime.
fn falling_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itPikachuthunder_UnkMotion1_Coll (802B21D4): the lead strikes where it
/// meets the floor (it_8026DA70), with the spark below its tip
/// (it_80272AC4: three fadds, no more hitlag) and, unless Pikachu is
/// busy, a small camera quake; the others strike on reaching where the
/// bolt before them stopped.
fn falling_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    let state = *thunder(item);
    if state.index == 0 {
        if item.sense_air_collision(ctx.map) {
            let tip = Attributes(&ctx.assets.special_attributes).tip();
            let offset = Vec3::new(0.0, -tip, 0.0);
            let position = Vec3::new(
                offset.x + item.position.x,
                offset.y + item.position.y,
                offset.z + item.position.z,
            );
            item.events.push(ItemEvent::Effect {
                id: FLOOR_SPARK,
                position,
            });
            item.sound_requests.push(0x74);
            item.hitlag_enabled = false;
            strike(item, ctx.assets);
            if ctx.owner.is_some_and(|owner| !owner_busy(owner.motion)) {
                item.events.push(ItemEvent::Quake {
                    kind: SMALL_QUAKE,
                    joint: 0,
                    offset: Vec3::ZERO,
                });
            }
        }
    } else if state.struck && item.position.y <= state.reached.y {
        strike(item, ctx.assets);
    }
    false
}

/// it_802B22B8 (802B22B8): motion 2 keeping the hitboxes and the running
/// animation, the struck lifetime, and the next bolt told where this one
/// stopped.
fn strike(item: &mut ItemCore, assets: &ItemAssets) {
    item.change_motion_with(STRUCK, ARTICLE_STATES[2], HIT_PRESERVE, assets);
    let frames = thunder(item).strike_frames;
    item.life_timer = frames;
    thunder(item).struck = true;
    if let Some(next) = item.partner {
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Item(next),
            message: LinkMessage::Reached {
                position: item.position,
            },
        });
    }
}

/// pika_scale: 10000 * `a` / `b` (fmuls, fdivs), plus one, truncated
/// (fctiwz), back to single and / 10000 (fdivs).
fn pika_scale(a: f32, b: f32) -> f32 {
    let ratio = (10000.0 * a) / b;
    gekko_math::msl::fctiwz(1.0 + ratio) as f32 / 10000.0
}

/// itPikachuthunder_UnkMotion2_Anim (802B2340): the lifetime; the bolt
/// shortens by its fall (fadds) while any length is left, its model's y
/// scale to the share left (at least 0.01) and hitbox 0's size by this
/// frame's shrink (it_80275594).
fn struck_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        return true;
    }
    let full = Attributes(&ctx.assets.special_attributes).length();
    let state = thunder(item);
    state.next_length = state.length + state.velocity.y;
    if state.next_length > 0.0 {
        state.scale = pika_scale(state.next_length, full);
        if state.scale <= 0.0 {
            state.scale = 0.01;
        }
        let ratio = pika_scale(state.next_length, state.length);
        state.length = state.next_length;
        if let Some(hit) = item.hitboxes[0].as_mut() {
            hit.descriptor.radius *= ratio;
        }
    }
    let scale = thunder(item).scale;
    item.model_scale.y = scale;
    false
}
