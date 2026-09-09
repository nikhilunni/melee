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
    Catch,
    Damage,
    Jab,
    Pass,
    GuardOn,
    Guard,
    GuardOff,
    GuardSetOff,
    GuardReflect,
    Escape,
    EscapeN,
    EscapeAir,
    CliffCatch,
    CliffWait,
    CliffJump1,
    CliffJump2,

    KneeBend,
    Jump,
    JumpAerial,
    Dash,
    Run,
    RunBrake,
    TurnRun,
    CliffClimb,
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
    Catch,
    Tilt,
    Damage,
    Jab,
    GuardOn,
    Guard,
    GuardOff,
    GuardSetOff,
    GuardReflect,
    Escape,
    EscapeN,
    EscapeAir,
    CliffCatch,
    CliffWait,
    CliffJump1,
    CliffJump2,

    KneeBend,
    Jump,
    JumpAerial,
    Dash,
    Run,
    RunBrake,
    TurnRun,
    CliffClimb,
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
    FallSpecial,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsCallback {
    Catch,
    Damage,
    Jab,
    Pass,
    GuardOn,
    Guard,
    GuardOff,
    GuardSetOff,
    GuardReflect,
    Escape,
    EscapeN,
    EscapeAir,
    CliffCatch,
    CliffWait,
    CliffJump1,
    CliffJump2,

    KneeBend,
    Jump,
    JumpAerial,
    Dash,
    Run,
    RunBrake,
    TurnRun,
    CliffClimb,
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
    FallSpecial,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionCallback {
    Catch,
    Damage,
    Pass,
    GuardOn,
    Guard,
    GuardOff,
    GuardSetOff,
    GuardReflect,
    Escape,
    EscapeN,
    EscapeAir,
    CliffCatch,
    CliffWait,
    CliffJump1,
    CliffJump2,

    KneeBend,
    Jump,
    JumpAerial,
    Dash,
    Run,
    RunBrake,
    TurnRun,
    CliffClimb,
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
    FallSpecial,
    Landing,
    FallUnimplemented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraCallback {
    FollowFighter,
    Cliff,
}

/// `motion_id` (+0x10), distinct from animation's submotion (+0x14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionState {
    /// Shared callback-family identity, even for a character-owned table row.
    pub id: CommonMotionState,
    /// Live action number from the selected common or character state table.
    pub action_id: i32,
    /// input_cb/anim_cb/phys_cb/coll_cb/cam_cb, +219C..+21AC.
    pub callbacks: StateCallbacks,
}
impl MotionState {
    pub const CATCH: Self = Self {
        id: CommonMotionState::Catch,
        action_id: CommonMotionState::Catch as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Catch,
            input: InputCallback::Catch,
            physics: PhysicsCallback::Catch,
            collision: CollisionCallback::Catch,
            camera: CameraCallback::FollowFighter,
        },
    };

    pub const DAMAGE_N2: Self = Self {
        id: CommonMotionState::DamageN2,
        action_id: CommonMotionState::DamageN2 as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Damage,
            input: InputCallback::Damage,
            physics: PhysicsCallback::Damage,
            collision: CollisionCallback::Damage,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const UP_TILT: Self = Self {
        id: CommonMotionState::AttackHi3,
        action_id: CommonMotionState::AttackHi3 as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Jab,
            input: InputCallback::Tilt,
            physics: PhysicsCallback::Squat,
            collision: CollisionCallback::Escape,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const JAB: Self = Self {
        id: CommonMotionState::Attack11,
        action_id: CommonMotionState::Attack11 as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Jab,
            input: InputCallback::Jab,
            physics: PhysicsCallback::Jab,
            collision: CollisionCallback::Escape,
            camera: CameraCallback::FollowFighter,
        },
    };
    /// ftCo_Pass_* (ftCo_Pass.c), action 244, submotion 209.
    pub const PASS: Self = Self {
        id: CommonMotionState::Pass,
        action_id: CommonMotionState::Pass as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Pass,
            input: InputCallback::Fall,
            physics: PhysicsCallback::Pass,
            collision: CollisionCallback::Pass,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const TURN_RUN: Self = Self {
        id: CommonMotionState::TurnRun,
        action_id: CommonMotionState::TurnRun as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::TurnRun,
            input: InputCallback::TurnRun,
            physics: PhysicsCallback::TurnRun,
            collision: CollisionCallback::TurnRun,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const CLIFF_CLIMB: Self = Self {
        id: CommonMotionState::CliffClimbQuick,
        action_id: CommonMotionState::CliffClimbQuick as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffClimb,
            input: InputCallback::CliffClimb,
            physics: PhysicsCallback::CliffClimb,
            collision: CollisionCallback::CliffClimb,
            camera: CameraCallback::Cliff,
        },
    };
    pub const CLIFF_ESCAPE: Self = Self {
        id: CommonMotionState::CliffEscapeQuick,
        action_id: CommonMotionState::CliffEscapeQuick as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffClimb,
            input: InputCallback::CliffClimb,
            physics: PhysicsCallback::CliffClimb,
            collision: CollisionCallback::CliffClimb,
            camera: CameraCallback::Cliff,
        },
    };

    pub const CLIFF_CATCH: Self = Self {
        id: CommonMotionState::CliffCatch,
        action_id: CommonMotionState::CliffCatch as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffCatch,
            input: InputCallback::CliffCatch,
            physics: PhysicsCallback::CliffCatch,
            collision: CollisionCallback::CliffCatch,
            camera: CameraCallback::Cliff,
        },
    };
    pub const CLIFF_WAIT: Self = Self {
        id: CommonMotionState::CliffWait,
        action_id: CommonMotionState::CliffWait as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffWait,
            input: InputCallback::CliffWait,
            physics: PhysicsCallback::CliffWait,
            collision: CollisionCallback::CliffWait,
            camera: CameraCallback::Cliff,
        },
    };
    pub const CLIFF_JUMP_1: Self = Self {
        id: CommonMotionState::CliffJumpQuick1,
        action_id: CommonMotionState::CliffJumpQuick1 as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffJump1,
            input: InputCallback::CliffJump1,
            physics: PhysicsCallback::CliffJump1,
            collision: CollisionCallback::CliffJump1,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const CLIFF_JUMP_2: Self = Self {
        id: CommonMotionState::CliffJumpQuick2,
        action_id: CommonMotionState::CliffJumpQuick2 as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::CliffJump2,
            input: InputCallback::CliffJump2,
            physics: PhysicsCallback::CliffJump2,
            collision: CollisionCallback::CliffJump2,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const CLIFF_JUMP_SLOW_1: Self = Self {
        id: CommonMotionState::CliffJumpSlow1,
        action_id: CommonMotionState::CliffJumpSlow1 as i32,
        ..Self::CLIFF_JUMP_1
    };
    pub const CLIFF_JUMP_SLOW_2: Self = Self {
        id: CommonMotionState::CliffJumpSlow2,
        action_id: CommonMotionState::CliffJumpSlow2 as i32,
        ..Self::CLIFF_JUMP_2
    };

    /// ftmotionstates.c:2731-2739, EscapeAir (236).
    pub const ESCAPE_AIR: Self = Self {
        id: CommonMotionState::EscapeAir,
        action_id: CommonMotionState::EscapeAir as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::EscapeAir,
            input: InputCallback::EscapeAir,
            physics: PhysicsCallback::EscapeAir,
            collision: CollisionCallback::EscapeAir,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const JUMP_BACK: Self = Self {
        id: CommonMotionState::JumpB,
        action_id: CommonMotionState::JumpB as i32,
        ..Self::JUMP
    };
    pub const JUMP_AERIAL_BACK: Self = Self {
        id: CommonMotionState::JumpAerialB,
        action_id: CommonMotionState::JumpAerialB as i32,
        ..Self::JUMP_AERIAL
    };
    /// ftmotionstates.c:608-616: shares all four Landing callbacks.
    pub const LANDING_FALL_SPECIAL: Self = Self {
        id: CommonMotionState::LandingFallSpecial,
        action_id: CommonMotionState::LandingFallSpecial as i32,
        ..Self::LANDING
    };

    pub const GUARD_ON: Self = Self {
        id: CommonMotionState::GuardOn,
        action_id: CommonMotionState::GuardOn as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::GuardOn,
            input: InputCallback::GuardOn,
            physics: PhysicsCallback::GuardOn,
            collision: CollisionCallback::GuardOn,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const GUARD: Self = Self {
        id: CommonMotionState::Guard,
        action_id: CommonMotionState::Guard as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Guard,
            input: InputCallback::Guard,
            physics: PhysicsCallback::Guard,
            collision: CollisionCallback::Guard,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const GUARD_OFF: Self = Self {
        id: CommonMotionState::GuardOff,
        action_id: CommonMotionState::GuardOff as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::GuardOff,
            input: InputCallback::GuardOff,
            physics: PhysicsCallback::GuardOff,
            collision: CollisionCallback::GuardOff,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const GUARD_SET_OFF: Self = Self {
        id: CommonMotionState::GuardSetOff,
        action_id: CommonMotionState::GuardSetOff as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::GuardSetOff,
            input: InputCallback::GuardSetOff,
            physics: PhysicsCallback::GuardSetOff,
            collision: CollisionCallback::GuardSetOff,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const GUARD_REFLECT: Self = Self {
        id: CommonMotionState::GuardReflect,
        action_id: CommonMotionState::GuardReflect as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::GuardReflect,
            input: InputCallback::GuardReflect,
            physics: PhysicsCallback::GuardReflect,
            collision: CollisionCallback::GuardReflect,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ESCAPE_F: Self = Self {
        id: CommonMotionState::EscapeF,
        action_id: CommonMotionState::EscapeF as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Escape,
            input: InputCallback::Escape,
            physics: PhysicsCallback::Escape,
            collision: CollisionCallback::Escape,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ESCAPE_B: Self = Self {
        id: CommonMotionState::EscapeB,
        action_id: CommonMotionState::EscapeB as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Escape,
            input: InputCallback::Escape,
            physics: PhysicsCallback::Escape,
            collision: CollisionCallback::Escape,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const ESCAPE_N: Self = Self {
        id: CommonMotionState::EscapeN,
        action_id: CommonMotionState::EscapeN as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::EscapeN,
            input: InputCallback::EscapeN,
            physics: PhysicsCallback::EscapeN,
            collision: CollisionCallback::EscapeN,
            camera: CameraCallback::FollowFighter,
        },
    };

    pub const KNEE_BEND: Self = Self {
        id: CommonMotionState::KneeBend,
        action_id: CommonMotionState::KneeBend as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::KneeBend,
            input: InputCallback::KneeBend,
            physics: PhysicsCallback::KneeBend,
            collision: CollisionCallback::KneeBend,
            camera: CameraCallback::FollowFighter,
        },
    };

    pub const JUMP: Self = Self {
        id: CommonMotionState::JumpF,
        action_id: CommonMotionState::JumpF as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Jump,
            input: InputCallback::Jump,
            physics: PhysicsCallback::Jump,
            collision: CollisionCallback::Jump,
            camera: CameraCallback::FollowFighter,
        },
    };

    pub const JUMP_AERIAL: Self = Self {
        id: CommonMotionState::JumpAerialF,
        action_id: CommonMotionState::JumpAerialF as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::JumpAerial,
            input: InputCallback::JumpAerial,
            physics: PhysicsCallback::JumpAerial,
            collision: CollisionCallback::JumpAerial,
            camera: CameraCallback::FollowFighter,
        },
    };

    pub const RUN_BRAKE: Self = Self {
        id: CommonMotionState::RunBrake,
        action_id: CommonMotionState::RunBrake as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::RunBrake,
            input: InputCallback::RunBrake,
            physics: PhysicsCallback::RunBrake,
            collision: CollisionCallback::RunBrake,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const RUN: Self = Self {
        id: CommonMotionState::Run,
        action_id: CommonMotionState::Run as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Run,
            input: InputCallback::Run,
            physics: PhysicsCallback::Run,
            collision: CollisionCallback::Run,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const DASH: Self = Self {
        id: CommonMotionState::Dash,
        action_id: CommonMotionState::Dash as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Dash,
            input: InputCallback::Dash,
            physics: PhysicsCallback::Dash,
            collision: CollisionCallback::Dash,
            camera: CameraCallback::FollowFighter,
        },
    };
    pub const SQUAT: Self = Self {
        id: CommonMotionState::Squat,
        action_id: CommonMotionState::Squat as i32,
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
        action_id: CommonMotionState::SquatWait as i32,
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
        action_id: CommonMotionState::SquatRv as i32,
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
        action_id: CommonMotionState::Turn as i32,
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
        action_id: CommonMotionState::WalkSlow as i32,
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
        action_id: CommonMotionState::WalkMiddle as i32,
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
        action_id: CommonMotionState::WalkFast as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Walk,
            input: InputCallback::Walk,
            physics: PhysicsCallback::Walk,
            collision: CollisionCallback::Walk,
            camera: CameraCallback::FollowFighter,
        },
    };

    /// ftCo_FallAerial_* (800CCDA8..800CCE94): shared fall blend and collision.
    pub const FALL_AERIAL: Self = Self {
        id: CommonMotionState::FallAerial,
        action_id: CommonMotionState::FallAerial as i32,
        callbacks: Self::FALL.callbacks,
    };
    /// ftCo_FallSpecial_* (80096AA0..80096C98).
    pub const FALL_SPECIAL: Self = Self {
        id: CommonMotionState::FallSpecial,
        action_id: CommonMotionState::FallSpecial as i32,
        callbacks: StateCallbacks {
            animation: AnimationCallback::Fall,
            input: InputCallback::FallSpecial,
            physics: PhysicsCallback::FallSpecial,
            collision: CollisionCallback::FallSpecial,
            camera: CameraCallback::FollowFighter,
        },
    };
    /// ftCo_Fall_* (800CCA00..800CCD80).
    pub const FALL: Self = Self {
        id: CommonMotionState::Fall,
        action_id: CommonMotionState::Fall as i32,
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
        action_id: CommonMotionState::Entry as i32,
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
        action_id: CommonMotionState::EntryStart as i32,
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
        action_id: CommonMotionState::EntryEnd as i32,
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
        action_id: CommonMotionState::Landing as i32,
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
        action_id: CommonMotionState::Wait as i32,
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
