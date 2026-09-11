//! Borrowed gameplay facts. These views neither advance state nor compute poses.
use crate::{initial_state::InitialState, Character, MatchStatus, Port, Stage, Tick};
pub use hsd_types::{Vec2, Vec3};
pub use melee_ft::fighter::ActionId;

pub struct Observation<'a> {
    state: &'a InitialState,
    pub tick: Tick,
    pub status: MatchStatus,
    pub stage: Stage,
}
impl<'a> Observation<'a> {
    pub(crate) fn new(
        state: &'a InitialState,
        tick: Tick,
        status: MatchStatus,
        stage: Stage,
    ) -> Self {
        Self {
            state,
            tick,
            status,
            stage,
        }
    }
    pub fn fighters(&self) -> impl ExactSizeIterator<Item = FighterObservation<'a>> + '_ {
        self.state.fighters.iter().map(|f| FighterObservation(&f.0))
    }
    pub fn fighter(&self, port: Port) -> Option<FighterObservation<'a>> {
        self.fighters().find(|f| f.port() == port)
    }
    pub fn items(&self) -> impl Iterator<Item = ItemObservation<'a>> + '_ {
        self.state.items.iter().map(ItemObservation)
    }
    /// Current world-space stage collision vertices, including stage animation.
    pub fn vertices(&self) -> impl ExactSizeIterator<Item = Vec2> + '_ {
        (0..self.state.map.vertices().len()).map(|i| {
            let (x, y) = self.state.map.vtx_get_pos(i as i32);
            Vec2::new(x, y)
        })
    }
    pub fn blast_zones(&self) -> BlastZones {
        let arena = &self.state.assets.arena;
        BlastZones {
            left: arena.left,
            right: arena.right,
            top: arena.top,
            bottom: arena.bottom,
        }
    }
    /// Active collision segments in scheduler-updated world coordinates.
    pub fn surfaces(&self) -> impl Iterator<Item = StageSurface> + '_ {
        let map = &self.state.map;
        (0..map.coll_lines().len()).filter_map(move |index| {
            let id = index as i32;
            if !map.line_is_active(id) {
                return None;
            }
            let flags = map.coll_lines()[index].flags;
            use melee_types::mp::{line_flag, line_kind};
            Some(StageSurface {
                id: SurfaceId(index as u32),
                start: map.line_get_v0_pos(id),
                end: map.line_get_v1_pos(id),
                floor: flags & line_kind::FLOOR != 0,
                ceiling: flags & line_kind::CEILING != 0,
                left_wall: flags & line_kind::LEFT_WALL != 0,
                right_wall: flags & line_kind::RIGHT_WALL != 0,
                pass_through: flags & line_flag::PLATFORM != 0,
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageSurface {
    pub id: SurfaceId,
    pub start: Vec3,
    pub end: Vec3,
    pub floor: bool,
    pub ceiling: bool,
    pub left_wall: bool,
    pub right_wall: bool,
    pub pass_through: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlastZones {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy)]
pub struct FighterObservation<'a>(&'a melee_ft::fighter::Fighter);
impl FighterObservation<'_> {
    pub fn port(self) -> Port {
        Port::from_index(self.0.player.id)
    }
    pub fn character(self) -> Character {
        Character::from_kind(self.0.kind)
    }
    pub fn position(self) -> Vec3 {
        self.0.physics.position
    }
    pub fn self_velocity(self) -> Vec3 {
        self.0.physics.self_velocity
    }
    pub fn knockback_velocity(self) -> Vec3 {
        self.0.physics.knockback_velocity
    }
    pub fn facing(self) -> f32 {
        self.0.physics.facing
    }
    pub fn grounded(self) -> bool {
        self.0.physics.ground_or_air == melee_types::GroundOrAir::Ground
    }
    pub fn jumps_used(self) -> u8 {
        self.0.physics.jumps_used
    }
    pub fn action(self) -> ActionId {
        self.0.motion_state.action
    }
    pub fn animation_frame(self) -> f32 {
        self.0.animation.frame
    }
    pub fn percent(self) -> f32 {
        self.0.physics.percent
    }
    pub fn stocks(self) -> u8 {
        self.0.player.stocks
    }
    pub fn shield_health(self) -> f32 {
        self.0.status.shield_health
    }
    pub fn shield_active(self) -> bool {
        self.0.shield.active
    }
    pub fn hitlag_remaining(self) -> f32 {
        self.0.combat.hitlag_remaining
    }
    pub fn hitstun_remaining(self) -> f32 {
        match &self.0.state_data {
            melee_ft::fighter::MotionData::Damage(damage) => damage.hitstun,
            _ => 0.0,
        }
    }
    pub fn on_ledge(self) -> bool {
        self.0.status.on_ledge
    }
    pub fn ledge_intangibility_remaining(self) -> i32 {
        self.0.status.ledge_intangibility
    }
    pub fn revival_invincibility_remaining(self) -> i32 {
        self.0.status.revival_invincibility
    }
    pub fn grabbed(self) -> bool {
        matches!(
            self.0.combat.grab,
            Some(melee_ft::fighter::grab::GrabLink::Captured { .. })
        )
    }
    pub fn holding_fighter(self) -> bool {
        matches!(
            self.0.combat.grab,
            Some(melee_ft::fighter::grab::GrabLink::Holding { .. })
        )
    }
}

/// Stable identity within a continuation; two cloned matches may share IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemKind(pub i32);
#[derive(Clone, Copy)]
pub struct ItemObservation<'a>(&'a melee_it::ItemCore);
impl ItemObservation<'_> {
    pub fn id(self) -> ItemId {
        ItemId(self.0.id)
    }
    pub fn kind(self) -> ItemKind {
        ItemKind(self.0.kind.into())
    }
    pub fn owner(self) -> Option<Port> {
        self.0.owner.map(Port::from_index)
    }
    pub fn position(self) -> Vec3 {
        self.0.position
    }
    pub fn velocity(self) -> Vec3 {
        self.0.velocity
    }
    pub fn animation_frame(self) -> f32 {
        self.0.animation_frame
    }
}
