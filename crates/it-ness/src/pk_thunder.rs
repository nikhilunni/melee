//! PK Thunder: the head (It_Kind_Ness_PKThunder, itnesspkthunderball.c
//! 802AB3F0..802AC43C) and the six tail segments that follow it
//! (It_Kind_Ness_PKThunder1..4, itnesspkthundertrail.c 802AC43C..802AC7A0).
//!
//! The head flies at a fixed speed along an angle its creator's stick
//! turns while he holds the control row. It keeps its last sixteen
//! positions; segment `i` sits at every second one (it_802AB3F0) and
//! points at the next. Retail's segments read the head's arrays in their
//! own physics callbacks; here the head sends each segment its two points
//! when its physics callback has shifted them, and the segment places
//! itself in its own callback.
use crate::no_collision;
use gekko_math::{
    fma::{fmadds, fnmsubs},
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEventContext, ItemLogic,
    ItemPhysicsContext, ItemScratch, ItemStateRow, LinkMessage, LinkRequest, LinkTarget,
    PkThunderState, PkThunderTrailState, SpawnItem,
};
use melee_lb::trigf::atan2f;
use melee_types::{mp::collide, ItemKind};

pub struct NessPkThunder;
/// One of the four tail kinds (It_Kind_Ness_PKThunder1..4), `N` 0..=3.
pub struct NessPkThunderTrail<const N: usize>;

/// ftData.x48_items indices (ftNs_Init_OnLoad): the head, then the four
/// tail kinds.
pub const HEAD_ARTICLE_INDEX: u32 = 3;
pub const TRAIL_ARTICLE_INDICES: [u32; 4] = [4, 5, 6, 7];
/// it_803F6BC8's and it_803F6C08's anim_id columns.
pub const HEAD_ARTICLE_STATES: [i32; 1] = [0];
pub const TRAIL_ARTICLE_STATES: [i32; 1] = [0];
/// itPKThunderAttributes x0..x10; the tail kinds have one float.
pub const HEAD_SPECIAL_ATTRIBUTES: u32 = 5;
pub const TRAIL_SPECIAL_ATTRIBUTES: u32 = 1;

/// it_803F6C18: the tail kind of each of the six segments.
const SEGMENT_KINDS: [ItemKind; SEGMENTS] = [
    ItemKind::NessPKThunder1,
    ItemKind::NessPKThunder1,
    ItemKind::NessPKThunder1,
    ItemKind::NessPKThunder2,
    ItemKind::NessPKThunder3,
    ItemKind::NessPKThunder4,
];
const TRAIL_KINDS: [ItemKind; 4] = [
    ItemKind::NessPKThunder1,
    ItemKind::NessPKThunder2,
    ItemKind::NessPKThunder3,
    ItemKind::NessPKThunder4,
];
const SEGMENTS: usize = 6;
/// The positions and angles the head keeps.
const HISTORY: usize = 16;
/// ftNs_MS_SpecialHiHold and ftNs_MS_SpecialAirHiHold
/// (ftNs_SpecialHi_CheckSpecialHiHold): its creator steers.
const CONTROL_MOTIONS: [u16; 2] = [359, 363];
/// 0.017453292f, degrees to radians (MTXDegToRad).
const DEGREES: f32 = 0.017453292;
/// MTXDegToRad(45.0f): a stick further than this from the heading turns
/// the head by the whole turn rate.
const FULL_TURN_ANGLE: f32 = std::f32::consts::FRAC_PI_4;
const FULL_TURN_DEGREES: f32 = 45.0;
const TAU: f64 = std::f64::consts::TAU;

/// The head's special attributes (itPKThunderAttributes).
struct Head<'a>(&'a [f32]);
impl Head<'_> {
    /// x0 PKTHUNDER_LIFETIME.
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// x4 PKTHUNDER_SPEED.
    fn speed(&self) -> f32 {
        self.0[1]
    }
    /// x8 PKTHUNDER_SPAWN_ANGLE, degrees.
    fn spawn_degrees(&self) -> f32 {
        self.0[2]
    }
    /// xC PKTHUNDER_STICK_THRESHOLD.
    fn stick_threshold(&self) -> f32 {
        self.0[3]
    }
    /// x10 PKTHUNDER_TURN_RADIUS: degrees turned per frame.
    fn turn_degrees(&self) -> f32 {
        self.0[4]
    }
}

fn head(item: &mut ItemCore) -> &mut PkThunderState {
    match &mut item.scratch {
        ItemScratch::PkThunder(state) => state,
        _ => unreachable!("a PK Thunder head without its state"),
    }
}

fn trail(item: &mut ItemCore) -> &mut PkThunderTrailState {
    match &mut item.scratch {
        ItemScratch::PkThunderTrail(state) => state,
        _ => unreachable!("a PK Thunder segment without its state"),
    }
}

/// The segment the last spawn request created: the scene links the new
/// item as the head's partner, and the head files it under its index.
fn collect_segment(item: &mut ItemCore) {
    if let Some(id) = item.partner.take() {
        let state = head(item);
        if let Some(slot) = state.trails.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(id);
        }
    }
}

/// `angle` wrapped into [0, tau] in double steps (normalizeAngle).
fn normalized(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > TAU {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// The velocity along the newest angle at the head's speed (separate
/// fmuls).
fn set_velocity(item: &mut ItemCore) {
    let state = head(item);
    let (speed, angle) = (state.speed, state.angles[0]);
    item.velocity.x = speed * cosf(angle);
    item.velocity.y = speed * sinf(angle);
    item.velocity.z = 0.0;
}

static HEAD_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: HEAD_ARTICLE_STATES[0],
    animation: head_anim,
    physics: head_physics,
    collision: head_collision,
}];

/// it_3F2F.c's Logic26 row.
impl ItemLogic for NessPkThunder {
    const KIND: ItemKind = ItemKind::NessPKThunder;
    const STATES: &'static [ItemStateRow] = &HEAD_STATES;
    /// it_802AB58C: spawn.x40 = Item_8026AE60(), the hit group its
    /// segments join.
    const NEW_HIT_GROUP: bool = true;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::PkThunder(PkThunderState::default());
    }
    /// it_802AB58C (802AB58C) once Item_80268B18 returns: the command
    /// variables, the lifetime, every kept position at the hand and every
    /// kept angle at x8, no segments, the creator (the parent); then
    /// it_802ABA4C (802ABA4C): not grabbable, shown, state 0, rising at x4.
    /// xDCC b3 clears: the blast zones do not end it.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let a = Head(&assets.special_attributes);
        item.command_variables = [0; 4];
        let creator = item.owner;
        *head(item) = PkThunderState {
            positions: [spawn.previous_position; HISTORY],
            angles: [DEGREES * a.spawn_degrees(); HISTORY],
            trails: [None; SEGMENTS],
            speed: a.speed(),
            frames: 0,
            spawned: 0,
            reflected: false,
            creator,
        };
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(0, HEAD_ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.life_timer = a.lifetime();
        item.half_life = a.lifetime() * common.half_life_scale;
        item.velocity = Vec3::new(0.0, a.speed(), 0.0);
        item.blast_zone_checked = false;
    }
    /// itNessPKThunderball_Logic26_DmgDealt (802AC050).
    fn damage_dealt(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end(item);
        true
    }
    /// it_802AC074.
    fn clanked(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end(item);
        true
    }
    /// it_802AC338.
    fn absorbed(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end(item);
        true
    }
    /// it_802AC3F8.
    fn hit_shield(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        end(item);
        true
    }
    /// it_802AC098 (802AC098): out of its creator's hands for good and
    /// flying back the way it came; its segments go and new ones follow
    /// from the turning point.
    fn reflected(item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        collect_segment(item);
        let state = head(item);
        state.creator = None;
        state.reflected = true;
        let angle = (f64::from(state.angles[0]) + std::f64::consts::PI) as f32;
        state.angles[0] = normalized(angle);
        end_segments(item);
        let state = head(item);
        state.frames = 0;
        state.spawned = 0;
        state.positions = [state.positions[0]; HISTORY];
        state.angles = [state.angles[0]; HISTORY];
        set_velocity(item);
        false
    }
    /// it_802AC35C (802AC35C): the velocity mirrors off the shield (xC58)
    /// and the heading follows it.
    fn shield_bounced(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.velocity = melee_lb::vector::mirror(item.velocity, ctx.shield_normal);
        item.velocity.z = 0.0;
        let angle = normalized(atan2f(item.velocity.y, item.velocity.x));
        head(item).angles[0] = angle;
        false
    }
    /// it_802AB9C0 (802AB9C0), from ftNs_SpecialHi_TakeDamage when Ness is
    /// hit or dies with his thunder out: unless it was reflected, the head
    /// and its segments lose their owner.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Orphan => {
                collect_segment(item);
                if head(item).reflected {
                    return;
                }
                item.owner = None;
                item.held = false;
                for id in head(item).trails.into_iter().flatten() {
                    item.link_requests.push(LinkRequest {
                        target: LinkTarget::Item(id),
                        message: LinkMessage::TrailOrphan,
                    });
                }
            }
            _ => unreachable!("{control:?} sent to PK Thunder"),
        }
    }
    /// it_802AB90C (802AB90C): its creator hears of its end
    /// (ftNs_SpecialHi_ItemPKThunderRemove) while the head, unreflected,
    /// is still his.
    fn notifies_owner(item: &ItemCore) -> bool {
        match &item.scratch {
            ItemScratch::PkThunder(state) => {
                !state.reflected && state.creator.is_some() && state.creator == item.owner
            }
            _ => false,
        }
    }
    /// it_802AB568 / it_802AB3F0: Ness reads whether the head is still his
    /// (the report exists) and its newest kept position.
    fn owner_report(item: &ItemCore, _assets: &ItemAssets) -> Option<melee_it::ArticleReport> {
        match &item.scratch {
            ItemScratch::PkThunder(state) => Some(melee_it::ArticleReport {
                point: state.positions[0],
                struck: false,
            }),
            _ => None,
        }
    }
}

/// it_802AC58C for every segment: each loses its hitboxes, its head and
/// its owner, and ends at its next animation callback.
fn end_segments(item: &mut ItemCore) {
    collect_segment(item);
    let trails = std::mem::take(&mut head(item).trails);
    for id in trails.into_iter().flatten() {
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Item(id),
            message: LinkMessage::TrailEnd,
        });
    }
    // A segment this callback asked for is created and ended at once.
    for request in item.link_requests.iter_mut() {
        if matches!(request.target, LinkTarget::Spawn(_)) {
            request.message = LinkMessage::TrailEnd;
        }
    }
}

/// it_802AB90C (802AB90C) from the head's own callbacks: the hitboxes
/// clear and the segments go. The creator's notice
/// (ftNs_SpecialHi_ItemPKThunderRemove) is the scene's, through
/// `notifies_owner`, when the item is removed.
fn end(item: &mut ItemCore) {
    item.hitboxes.fill(None);
    end_segments(item);
    item.held = false;
}

/// itNesspkthunderball_UnkMotion0_Anim (802ABB1C): on every second frame
/// up to the eleventh a segment spawns at the head (for its creator while
/// the head is his, else unowned); an unreflected head ends once its
/// creator leaves the control row or no longer owns it; the lifetime.
fn head_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    collect_segment(item);
    let state = *head(item);
    if state.frames <= 10 && state.frames % 2 == 0 && state.spawned <= 5 {
        let kind = SEGMENT_KINDS[state.spawned as usize];
        let owned = item.owner == state.creator;
        let parent = item.owner.filter(|_| owned);
        let mut spawn = SpawnItem::ray(kind, parent.unwrap_or(0), item.position, item.facing);
        if parent.is_none() {
            spawn.owner = None;
            spawn.secondary_owner = None;
        }
        item.link_requests.push(LinkRequest {
            target: LinkTarget::Spawn(spawn),
            message: segment_message(&state, state.spawned as usize),
        });
        head(item).spawned += 1;
    }
    head(item).frames += 1;
    if !state.reflected && state.creator.is_some() {
        if item.owner == state.creator {
            let steering = ctx
                .owner
                .is_some_and(|owner| CONTROL_MOTIONS.contains(&owner.motion));
            if !steering {
                end_segments(item);
                return true;
            }
        } else {
            // it_802AB90C with an owner that is not the creator: no notice.
            head(item).creator = None;
            end(item);
            return true;
        }
    }
    item.life_timer -= 1.0;
    if item.life_timer <= 0.0 {
        end_segments(item);
        return true;
    }
    false
}

/// What segment `index` reads of the head (it_802AB3F0 at `index` and
/// `index + 1`, it_802AB468).
fn segment_message(state: &PkThunderState, index: usize) -> LinkMessage {
    LinkMessage::Trail {
        index: index as u8,
        point: state.positions[index * 2],
        next: state.positions[(index + 1) * 2],
        length: state.speed,
    }
}

/// itNesspkthunderball_UnkMotion0_Phys (802ABCC8): the kept angles shift;
/// while its creator steers, a stick past xC turns the heading toward it by
/// x10 degrees a frame (retail 0x802ABE08: fmadds, 0x802ABE24: fnmsubs), or
/// by the angle's share of 45 degrees when closer than that (fdivs, fdivs,
/// then fadds or fsubs); the kept positions shift and take the head's.
fn head_physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    collect_segment(item);
    let a = Head(&ctx.assets.special_attributes);
    let state = *head(item);
    let steering = !state.reflected
        && state.creator.is_some()
        && item.owner == state.creator
        && ctx
            .owner
            .is_some_and(|owner| CONTROL_MOTIONS.contains(&owner.motion));
    head(item).angles.copy_within(0..HISTORY - 1, 1);
    if steering {
        let owner = ctx.owner.expect("the steering creator");
        let stick = Vec3::new(owner.stick.x, owner.stick.y, 0.0);
        let threshold = a.stick_threshold();
        if stick.x.abs() > threshold || stick.y.abs() > threshold {
            let apart = melee_lb::vector::angle(item.velocity, stick);
            let cross = melee_lb::vector::cross(item.velocity, stick);
            let turn = a.turn_degrees();
            let heading = &mut head(item).angles[0];
            if apart >= FULL_TURN_ANGLE {
                if cross.z > 0.0 {
                    *heading = fmadds(DEGREES, turn, *heading);
                } else if cross.z < 0.0 {
                    *heading = fnmsubs(DEGREES, turn, *heading);
                }
            }
            if apart < FULL_TURN_ANGLE {
                if cross.z > 0.0 {
                    *heading += apart / (FULL_TURN_DEGREES / turn);
                } else if cross.z < 0.0 {
                    *heading -= apart / (FULL_TURN_DEGREES / turn);
                }
            }
            while f64::from(*heading) < -TAU {
                *heading = (f64::from(*heading) + TAU) as f32;
            }
            while f64::from(*heading) > TAU {
                *heading = (f64::from(*heading) - TAU) as f32;
            }
            set_velocity(item);
        }
    }
    let position = item.position;
    let state = head(item);
    state.positions.copy_within(0..HISTORY - 1, 1);
    state.positions[0] = position;
    let state = *state;
    for (index, id) in state.trails.iter().enumerate() {
        if let Some(id) = id {
            item.link_requests.push(LinkRequest {
                target: LinkTarget::Item(*id),
                message: segment_message(&state, index),
            });
        }
    }
}

/// itNesspkthunderball_UnkMotion0_Coll (802AC000) -> it_802AB4B8
/// (802AB4B8): an airborne pass (it_8026DA08); a ceiling while rising or a
/// floor otherwise, a left wall moving right or a right wall otherwise,
/// ends it.
fn head_collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    collect_segment(item);
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
        item.hitboxes.fill(None);
        end_segments(item);
        return true;
    }
    false
}

static TRAIL_STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: TRAIL_ARTICLE_STATES[0],
    animation: trail_anim,
    physics: trail_physics,
    collision: no_collision,
}];

/// it_3F2F.c's rows for the four tail kinds: no callbacks but the state
/// table.
impl<const N: usize> ItemLogic for NessPkThunderTrail<N> {
    const KIND: ItemKind = TRAIL_KINDS[N];
    const STATES: &'static [ItemStateRow] = &TRAIL_STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::PkThunderTrail(PkThunderTrailState::default());
    }
    /// it_802AC43C (802AC43C) once Item_80268B18 returns: the command
    /// variables, not grabbable, shown, at rest, the blast zones off, then
    /// it_802AC604's state 0. The head, the index and what it reads of
    /// the head arrive with the spawn's message.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        _common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.velocity = Vec3::ZERO;
        item.blast_zone_checked = false;
        item.change_motion_with(0, TRAIL_ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// The head's messages: its points for this segment, the end
    /// (it_802AC58C: hitboxes, head and owner go) and the loss of the
    /// owner alone (it_802AC5D8).
    fn link_received(item: &mut ItemCore, message: LinkMessage, _assets: &ItemAssets) -> bool {
        match message {
            LinkMessage::Trail {
                index,
                point,
                next,
                length,
            } => {
                let state = trail(item);
                state.index = index;
                state.point = point;
                state.next = next;
                state.length = length;
            }
            LinkMessage::TrailEnd => {
                item.hitboxes.fill(None);
                item.partner = None;
                item.owner = None;
                item.held = false;
            }
            LinkMessage::TrailOrphan => {
                item.owner = None;
                item.held = false;
            }
            _ => unreachable!("{message:?} sent to a PK Thunder segment"),
        }
        false
    }
}

/// itNesspkthundertrail_UnkMotion0_Anim (802AC624): the model shows on
/// every second frame; the segment ends once its head is gone.
fn trail_anim(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    let state = trail(item);
    let hidden = state.blink & 1 != 0;
    state.blink = state.blink.wrapping_add(1);
    item.hidden = hidden;
    item.partner.is_none()
}

/// itNesspkthundertrail_UnkMotion0_Phys (802AC6B0): the segment sits at
/// its kept position of the head, stretched to half the head's speed along
/// Z and turned about X toward the next kept position.
fn trail_physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    if item.partner.is_none() {
        return;
    }
    let state = *trail(item);
    item.position = state.point;
    item.model_scale.z = 2.0 * state.length / 4.0;
    let lean = atan2f(state.point.y - state.next.y, state.point.x - state.next.x);
    let angle = if item.facing == 1.0 {
        lean
    } else {
        (std::f64::consts::PI + f64::from(lean)) as f32
    };
    item.rotation.x = angle * -item.facing;
}
