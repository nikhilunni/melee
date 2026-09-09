//! Character erasure is confined to scene composition.
use hsd_types::Mtx;
use melee_ft::fighter::{CharacterCallbacks, Fighter};
use melee_types::snapshot::{Snapshot, SnapshotSink};

/// A closed enum keeps each character's generic fighter monomorphised, with
/// no object-safe mirror of the gameplay API. Boxes keep the scene small and
/// avoid copying large fighter states. Only composition dispatches variants;
/// shared action-state behavior remains in CharacterCallbacks and archive data.
pub(crate) enum SceneFighter {
    Fox(Box<Fighter<ft_fox::init::Fox>>),
    Marth(Box<Fighter<ft_mars::init::Marth>>),
}
impl Snapshot for SceneFighter {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        match self {
            Self::Fox(f) => f.snapshot(sink),
            Self::Marth(f) => f.snapshot(sink),
        }
    }
}
impl SceneFighter {
    pub fn bone_matrix(&mut self, bone: Option<usize>) -> Mtx {
        fn matrix<C: CharacterCallbacks>(f: &mut Fighter<C>, bone: Option<usize>) -> Mtx {
            let joint = bone.map_or(f.animation.root, |i| f.animation.parts[i].joint);
            f.skeleton.setup_matrix(joint);
            f.skeleton.get(joint).mtx
        }
        match self {
            Self::Fox(f) => matrix(f, bone),
            Self::Marth(f) => matrix(f, bone),
        }
    }
}
