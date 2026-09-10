use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Deliberately contains an aligned scalar and a resource owner, both of which
/// must survive moves of the inline store and exactly one eventual destruction.
struct Probe {
    value: u128,
    drops: Arc<AtomicUsize>,
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl CharacterCallbacks for Probe {
    fn descriptor() -> &'static assets::CharacterDescriptor {
        panic!("no assets in ownership test")
    }
    fn from_archive(_: &hsd_archive::Archive) -> Result<Self, crate::desc::FighterDescError> {
        panic!("no archive in ownership test")
    }
    fn kind(&self) -> FighterKind {
        FighterKind::Fox
    }
    fn on_load(&mut self, _: &mut Capabilities) {}
    fn on_reset(&mut self) {
        self.value += 1;
    }
}

struct Other;
struct Oversized([u8; PAYLOAD_BYTES + 1]);
#[repr(align(32))]
struct Overaligned;
struct WrongTable;
macro_rules! inert_character {
    ($ty:ty) => {
        fn descriptor() -> &'static assets::CharacterDescriptor {
            Probe::descriptor()
        }
        fn from_archive(_: &hsd_archive::Archive) -> Result<Self, crate::desc::FighterDescError> {
            panic!("no archive in ownership test")
        }
        fn kind(&self) -> FighterKind {
            FighterKind::Fox
        }
        fn on_load(&mut self, _: &mut Capabilities) {}
        fn on_reset(&mut self) {}
    };
}
impl CharacterCallbacks for Other {
    inert_character!(Other);
}
impl CharacterCallbacks for Oversized {
    inert_character!(Oversized);
}
impl CharacterCallbacks for Overaligned {
    inert_character!(Overaligned);
}
impl CharacterCallbacks for WrongTable {
    inert_character!(WrongTable);
    fn table() -> &'static CharacterTable {
        Other::table()
    }
}

#[test]
fn aligned_payload_moves_and_dispatches_then_drops_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let state = Probe {
        value: u128::MAX - 1,
        drops: Arc::clone(&drops),
    }
    .into_state();
    let mut moved = [state];
    moved[0].on_reset();
    assert_eq!(moved[0].get::<Probe>().value, u128::MAX);
    moved[0].get_mut::<Probe>().value = 9;
    assert_eq!(moved[0].get::<Probe>().value, 9);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(moved);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn wrong_typed_access_panics_before_borrowing_payload() {
    let mut state = Other.into_state();
    assert!(std::panic::catch_unwind(|| state.get::<Probe>()).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = state.get_mut::<Probe>();
    }))
    .is_err());
    let _ = state.get::<Other>();
}

#[test]
fn construction_rejects_mismatched_table() {
    assert!(std::panic::catch_unwind(|| WrongTable.into_state()).is_err());
}

#[test]
fn construction_rejects_unsupported_size_and_alignment() {
    let large = Oversized([0; PAYLOAD_BYTES + 1]);
    assert_eq!(large.0.len(), PAYLOAD_BYTES + 1);
    assert!(std::panic::catch_unwind(|| large.into_state()).is_err());
    assert!(std::panic::catch_unwind(|| Overaligned.into_state()).is_err());
}
