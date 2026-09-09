use melee_types::CommonMotionState;

/// Retail Fighter.motion_id: an index into ftData_MotionStateList below 341,
/// or into the character's ftData_CharacterStateTables entry after subtracting 341.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u16);

/// ftCo_MS_Count: the character-owned table begins at retail action 341.
pub const COMMON_COUNT: usize = 341;

impl From<CommonMotionState> for ActionId {
    fn from(state: CommonMotionState) -> Self {
        Self(u16::try_from(i32::from(state)).expect("motion entry requires a nonnegative action"))
    }
}

impl From<ActionId> for i32 {
    fn from(action: ActionId) -> Self {
        Self::from(action.0)
    }
}

/// retail: ftData_SpecialN, ftData_SpecialS, ftData_SpecialHi, ftData_SpecialLw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialSlot {
    Neutral,
    Side,
    Up,
    Down,
}
