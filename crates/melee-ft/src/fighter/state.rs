//! Retail common and character-owned motion tables and scheduler links.
mod action;
pub mod callbacks;
mod common_table;
mod phase;
pub use phase::{
    AnimFn, AnimationPhase, CameraFn, CameraPhase, CollisionFn, CollisionPhase, InputFn,
    InputPhase, PhysicsFn, PhysicsPhase,
};
mod names;
mod row;
mod special;
pub use action::{ActionId, SpecialSlot, COMMON_COUNT};
pub use common_table::common_table;
pub(crate) use row::unsupported_action;
pub use row::{unimplemented_anim, unimplemented_row, MotionRow, MotionState};

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
        use crate::fighter::CharacterCallbacks;
        struct TestCharacter;
        impl CharacterCallbacks for TestCharacter {
            fn descriptor() -> &'static crate::fighter::assets::CharacterDescriptor {
                panic!("table-only character has no assets")
            }
            fn from_archive(
                _: &hsd_archive::Archive,
            ) -> Result<Self, crate::desc::FighterDescError> {
                panic!("table-only character has no assets")
            }
            fn kind(&self) -> melee_types::FighterKind {
                melee_types::FighterKind::Fox
            }
            fn on_load(&mut self, _: &mut crate::fighter::Capabilities) {}
            fn on_reset(&mut self) {}
        }
        let row = TestCharacter::COMMON[melee_types::CommonMotionState::Wait as usize];
        assert_eq!(i32::from(row.action), 14);
        assert_eq!(i32::from(row.id), 14);
        assert_eq!(row.animation, 2);
        assert!(std::ptr::fn_addr_eq(
            row.anim,
            callbacks::animation::wait::<TestCharacter> as AnimFn<TestCharacter>
        ));
        assert!(std::ptr::fn_addr_eq(
            row.iasa,
            callbacks::input::wait::<TestCharacter> as InputFn<TestCharacter>
        ));
        assert!(std::ptr::fn_addr_eq(
            row.physics,
            callbacks::physics::wait::<TestCharacter> as PhysicsFn<TestCharacter>
        ));
        assert!(std::ptr::fn_addr_eq(
            row.collision,
            callbacks::collision::ground_wait::<TestCharacter> as CollisionFn<TestCharacter>
        ));
        assert!(std::ptr::fn_addr_eq(
            row.camera,
            callbacks::camera::follow_fighter::<TestCharacter> as CameraFn<TestCharacter>
        ));
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
