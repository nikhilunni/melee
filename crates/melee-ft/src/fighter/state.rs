//! State-table identities, independent of host function-pointer representation.
use melee_types::CommonMotionState;

/// Callbacks installed by Fighter_ChangeMotionState (0x800693AC),
/// fighter.c:1367-1371. Wait's row is ftmotionstates.c:288-299.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateCallbacks {
    pub animation: AnimationCallback,
    pub input: InputCallback,
    pub physics: PhysicsCallback,
    pub collision: CollisionCallback,
    pub camera: CameraCallback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationCallback {
    Squat,
    SquatWait,
    SquatRv,
    Turn,
    Walk,
    Wait,
    Entry,
    EntryStart,
    EntryEnd,
    Fall,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCallback {
    Squat,
    SquatWait,
    SquatRv,
    Turn,
    Walk,
    Wait,
    Entry,
    EntryStart,
    EntryEnd,
    Fall,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsCallback {
    Squat,
    SquatWait,
    SquatRv,
    Turn,
    Walk,
    Wait,
    Entry,
    EntryStart,
    EntryEnd,
    Fall,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionCallback {
    Squat,
    SquatWait,
    SquatRv,
    Turn,
    Walk,
    Wait,
    Entry,
    EntryStart,
    EntryEnd,
    Fall,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraCallback {
    FollowFighter,
}

/// `motion_id` (+0x10), distinct from animation's submotion (+0x14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionState {
    pub id: CommonMotionState,
    /// input_cb/anim_cb/phys_cb/coll_cb/cam_cb, +219C..+21AC.
    pub callbacks: StateCallbacks,
}
impl MotionState {
    pub const SQUAT: Self = Self {
        id: CommonMotionState::Squat,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Squat,
            input: InputCallback::Squat,
            physics: PhysicsCallback::Squat,
            collision: CollisionCallback::Squat,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const SQUAT_WAIT: Self = Self {
        id: CommonMotionState::SquatWait,
        callbacks: StateCallbacks {
            animation: AnimationCallback::SquatWait,
            input: InputCallback::SquatWait,
            physics: PhysicsCallback::SquatWait,
            collision: CollisionCallback::SquatWait,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const SQUAT_RV: Self = Self {
        id: CommonMotionState::SquatRv,
        callbacks: StateCallbacks {
            animation: AnimationCallback::SquatRv,
            input: InputCallback::SquatRv,
            physics: PhysicsCallback::SquatRv,
            collision: CollisionCallback::SquatRv,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const TURN: Self = Self {
        id: CommonMotionState::Turn,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Turn,
            input: InputCallback::Turn,
            physics: PhysicsCallback::Turn,
            collision: CollisionCallback::Turn,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const WALK_SLOW: Self = Self {
        id: CommonMotionState::WalkSlow,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Walk,
            input: InputCallback::Walk,
            physics: PhysicsCallback::Walk,
            collision: CollisionCallback::Walk,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const WALK_MIDDLE: Self = Self {
        id: CommonMotionState::WalkMiddle,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Walk,
            input: InputCallback::Walk,
            physics: PhysicsCallback::Walk,
            collision: CollisionCallback::Walk,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const WALK_FAST: Self = Self {
        id: CommonMotionState::WalkFast,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Walk,
            input: InputCallback::Walk,
            physics: PhysicsCallback::Walk,
            collision: CollisionCallback::Walk,
            camera: CameraCallback::FollowFighter,
        },
    };

    /// Fall callbacks; neutral Anim/IASA/Phys/Coll are implemented.
    pub const FALL: Self = Self {
        id: CommonMotionState::Fall,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Fall,
            input: InputCallback::Fall,
            physics: PhysicsCallback::Fall,
            collision: CollisionCallback::Fall,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ENTRY: Self = Self {
        id: CommonMotionState::Entry,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Entry,
            input: InputCallback::Entry,
            physics: PhysicsCallback::Entry,
            collision: CollisionCallback::Entry,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ENTRY_START: Self = Self {
        id: CommonMotionState::EntryStart,
        callbacks: StateCallbacks {
            animation: AnimationCallback::EntryStart,
            input: InputCallback::EntryStart,
            physics: PhysicsCallback::EntryStart,
            collision: CollisionCallback::EntryStart,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ENTRY_END: Self = Self {
        id: CommonMotionState::EntryEnd,
        callbacks: StateCallbacks {
            animation: AnimationCallback::EntryEnd,
            input: InputCallback::EntryEnd,
            physics: PhysicsCallback::EntryEnd,
            collision: CollisionCallback::EntryEnd,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const LANDING: Self = Self {
        id: CommonMotionState::Landing,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Landing,
            input: InputCallback::Landing,
            physics: PhysicsCallback::Landing,
            collision: CollisionCallback::Landing,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const WAIT: Self = Self {
        id: CommonMotionState::Wait,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Wait,
            input: InputCallback::Wait,
            physics: PhysicsCallback::Wait,
            collision: CollisionCallback::Wait,
            camera: CameraCallback::FollowFighter,
        },
    };
}

/// Fighter_Create (0x80068E98), fighter.c:897-911, in scheduler order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FighterProc {
    Status = 0,
    Animation = 1,
    CpuGate = 2,
    Input = 3,
    Update = 4,
    Map = 6,
    Pose = 7,
    Accessories = 8,
    HitboxPositions = 9,
    Grab = 12,
    HitDetection = 13,
    ProcessHit = 14,
    Dynamics = 16,
    Camera = 18,
    PlayerMirror = 22,
}
impl FighterProc {
    pub const ALL: [Self; 15] = [
        Self::Status,
        Self::Animation,
        Self::CpuGate,
        Self::Input,
        Self::Update,
        Self::Map,
        Self::Pose,
        Self::Accessories,
        Self::HitboxPositions,
        Self::Grab,
        Self::HitDetection,
        Self::ProcessHit,
        Self::Dynamics,
        Self::Camera,
        Self::PlayerMirror,
    ];
    pub const fn s_link(self) -> u8 {
        self as u8
    }
}

/// Fighter-only visitation; the scene inserts stage/particle procs between
/// these phases. Equal-priority fighters retain their creation/list order.
/// HSD_GObj_80390CFC (0x80390CFC), baselib/gobj.c:88-142.
pub fn interleaved_order(fighters: usize) -> impl Iterator<Item = (FighterProc, usize)> {
    FighterProc::ALL
        .into_iter()
        .flat_map(move |proc| (0..fighters).map(move |player| (proc, player)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wait_callback_table() {
        let state = MotionState::WAIT;
        assert_eq!(i32::from(state.id), 14);
        assert_eq!(
            state.callbacks,
            StateCallbacks {
                animation: AnimationCallback::Wait,
                input: InputCallback::Wait,
                physics: PhysicsCallback::Wait,
                collision: CollisionCallback::Wait,
                camera: CameraCallback::FollowFighter,
            }
        );
    }
    #[test]
    fn fighters_interleave_within_each_scheduler_link() {
        let expected = [0, 1, 2, 3, 4, 6, 7, 8, 9, 12, 13, 14, 16, 18, 22]
            .into_iter()
            .flat_map(|link| [(link, 0), (link, 1)])
            .collect::<Vec<_>>();
        let actual = interleaved_order(2)
            .map(|(proc, player)| (proc.s_link(), player))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(interleaved_order(0).count(), 0);
    }
}
