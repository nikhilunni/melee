use super::{
    names::COMMON_NAMES, ActionId, AnimFn, AnimationPhase, CameraFn, CameraPhase, CollisionFn,
    CollisionPhase, InputFn, InputPhase, PhysicsFn, PhysicsPhase, COMMON_COUNT,
};
use crate::anim::WaitChoice;
use crate::fighter::assets::Result;
use crate::fighter::{CharacterCallbacks, Fighter};
use melee_types::CommonMotionState;

/// Retail MotionState (ft/types.h:853): animation ID and five event callbacks.
/// `x4_flags` and the packed move-ID word are not yet modelled by this port;
/// animation playback's MotionFlags are a different word and are not reused.
pub struct MotionRow<C: CharacterCallbacks> {
    /// Retail table index, including the character-owned table's 341 offset.
    pub action: ActionId,
    /// Existing common semantic identity used by shared state scratch and hooks.
    /// Character rows may share an identity while retaining distinct action IDs.
    pub id: CommonMotionState,
    /// Retail anim_id, previously selected as an i32 submotion in spawn.rs.
    pub animation: i32,
    pub anim: AnimFn<C>,
    pub iasa: InputFn<C>,
    pub physics: PhysicsFn<C>,
    pub collision: CollisionFn<C>,
    pub camera: CameraFn<C>,
    /// Port coverage, formerly the supported arms of motion-entry dispatch.
    pub implemented: bool,
}
impl<C: CharacterCallbacks> Copy for MotionRow<C> {}
impl<C: CharacterCallbacks> Clone for MotionRow<C> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<C: CharacterCallbacks> std::fmt::Debug for MotionRow<C> {
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
    pub const fn new<C: CharacterCallbacks>(row: MotionRow<C>) -> Self {
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
pub const fn unimplemented_row<C: CharacterCallbacks>() -> MotionRow<C> {
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

pub fn unimplemented_anim<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    _phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_iasa<C: CharacterCallbacks>(fighter: &mut Fighter<C>, _phase: InputPhase<'_>) {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_physics<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    _phase: PhysicsPhase<'_>,
) {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_collision<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    _phase: CollisionPhase<'_>,
) -> Result<()> {
    unsupported_action(fighter.core.motion_state.action)
}

pub fn unimplemented_camera<C: CharacterCallbacks>(
    fighter: &mut Fighter<C>,
    _phase: CameraPhase<'_>,
) {
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

impl<C: CharacterCallbacks> Fighter<C> {
    /// Fighter_ChangeMotionState (800693AC), fighter.c:1177-1181. Common entry
    /// identities can resolve through the existing character action-ID hook;
    /// both cases return the actual row, including its action number.
    pub fn row(&self, action: ActionId) -> MotionRow<C> {
        let mut index = usize::from(action.0);
        if index < COMMON_COUNT {
            let common = C::COMMON[index];
            index = usize::try_from(self.character.action_id(common.id))
                .expect("nonnegative character action");
        }
        if index < COMMON_COUNT {
            C::COMMON[index]
        } else {
            C::special_rows()
                .get(index - COMMON_COUNT)
                .copied()
                .unwrap_or_else(|| unsupported_action(ActionId(index as u16)))
        }
    }
}
