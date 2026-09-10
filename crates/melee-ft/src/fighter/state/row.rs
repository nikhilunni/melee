use super::{
    names::COMMON_NAMES, ActionId, AnimFn, AnimationPhase, CameraFn, CameraPhase, CollisionFn,
    CollisionPhase, InputFn, InputPhase, PhysicsFn, PhysicsPhase, COMMON_COUNT,
};
use crate::anim::WaitChoice;
use crate::fighter::assets::Result;
use crate::fighter::Fighter;
use melee_types::CommonMotionState;

/// Retail MotionState (ft/types.h:853): animation ID and five event callbacks.
/// `x4_flags` and the packed move-ID word are not yet modelled by this port;
/// animation playback's MotionFlags are a different word and are not reused.
pub struct MotionRow {
    /// Retail table index, including the character-owned table's 341 offset.
    pub action: ActionId,
    /// Existing common semantic identity used by shared state scratch and hooks.
    /// Character rows may share an identity while retaining distinct action IDs.
    pub id: CommonMotionState,
    /// Retail anim_id, previously selected as an i32 submotion in spawn.rs.
    pub animation: i32,
    pub anim: AnimFn,
    pub iasa: InputFn,
    pub physics: PhysicsFn,
    pub collision: CollisionFn,
    pub camera: CameraFn,
    /// Port coverage, formerly the supported arms of motion-entry dispatch.
    pub implemented: bool,
}
impl Copy for MotionRow {}
impl Clone for MotionRow {
    fn clone(&self) -> Self {
        *self
    }
}
impl std::fmt::Debug for MotionRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionRow")
            .field("action", &self.action)
            .field("id", &self.id)
            .field("animation", &self.animation)
            .field("implemented", &self.implemented)
            .finish_non_exhaustive()
    }
}

/// Scalar live state, shared by physics, animation and raw snapshots.
/// The typed callbacks live separately in Fighter::motion_row.
#[derive(Clone, Copy, Debug)]
pub struct MotionState {
    pub action: ActionId,
    pub id: CommonMotionState,
    pub animation: i32,
    pub implemented: bool,
}
impl MotionState {
    pub const fn new(row: MotionRow) -> Self {
        Self {
            action: row.action,
            id: row.id,
            animation: row.animation,
            implemented: row.implemented,
        }
    }
}

/// Placeholder for an unported ftData_MotionStateList index. All five callbacks
/// identify the live ftCo_MS_* name and retail index instead of silently running.
pub const fn unimplemented_row() -> MotionRow {
    MotionRow {
        action: ActionId(0),
        id: CommonMotionState::None,
        animation: -1,
        anim: unimplemented_anim,
        iasa: unimplemented_iasa,
        physics: unimplemented_physics,
        collision: unimplemented_collision,
        camera: unimplemented_camera,
        implemented: false,
    }
}

pub fn unimplemented_anim(
    fighter: &mut Fighter,
    _phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_iasa(fighter: &mut Fighter, _phase: InputPhase<'_>) {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_physics(fighter: &mut Fighter, _phase: PhysicsPhase<'_>) {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_collision(fighter: &mut Fighter, _phase: CollisionPhase<'_>) -> Result<()> {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_camera(fighter: &mut Fighter, _phase: CameraPhase<'_>) {
    unsupported_action(fighter.core.motion_state.action)
}
pub(crate) fn unsupported_action(action: ActionId) -> ! {
    let index = usize::from(action.0);
    let name = COMMON_NAMES
        .get(index)
        .copied()
        .unwrap_or("character motion state");
    unimplemented!(
        "fighter.c:1190-1194: unsupported motion entry {name} (retail table index {index})"
    );
}

impl Fighter {
    /// Fighter_ChangeMotionState (800693AC), fighter.c:1177-1181. Common entry
    /// identities can resolve through the existing character action-ID hook;
    /// both cases return the actual row, including its action number.
    pub fn row(&self, action: ActionId) -> MotionRow {
        let mut index = usize::from(action.0);
        if index < COMMON_COUNT {
            let common = super::COMMON[index];
            index = usize::try_from(self.character.action_id(common.id))
                .expect("nonnegative character action");
        }
        if index < COMMON_COUNT {
            super::COMMON[index]
        } else {
            self.character
                .table()
                .special_rows
                .get(index - COMMON_COUNT)
                .copied()
                .unwrap_or_else(|| unsupported_action(ActionId(index as u16)))
        }
    }
}
