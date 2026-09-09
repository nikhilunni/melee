//! Character erasure is confined to scene composition.
//!
//! **Adding a character:** add its crate to melee-sim's Cargo.toml and one
//! line to `scene_characters!` below. Nothing else in the simulator names
//! characters: loading, construction, dispatch and the scenario `kind`
//! strings all come from this list and each crate's `CharacterCallbacks`.
use crate::assets::CharacterArchive;
use crate::initial_state::{CollMap, SavedPose};
use hsd_types::Mtx;
use melee_ft::fighter::{
    assets::CharacterDescriptor, assets::FighterAssets, CharacterCallbacks, Fighter,
};
use melee_types::snapshot::{Snapshot, SnapshotSink};

/// A closed enum keeps each character's generic fighter monomorphised, with
/// no object-safe mirror of the gameplay API. Boxes keep the scene small and
/// avoid copying large fighter states. Only composition dispatches variants;
/// shared action-state behavior remains in CharacterCallbacks and archive data.
macro_rules! scene_characters {
    ($( $name:literal => $variant:ident ( $ty:path ) ),* $(,)?) => {
        pub(crate) enum SceneFighter {
            $( $variant(Box<Fighter<$ty>>), )*
        }
        /// Run one generic expression on whichever character is in the slot:
        /// `with_fighter!(scene_fighter, |f| f.snapshot(sink))`.
        macro_rules! with_fighter {
            ($fighter:expr, |$f:ident| $body:expr) => {
                match $fighter {
                    $( $crate::scene_fighter::SceneFighter::$variant($f) => $body, )*
                }
            };
        }
        // Used by the test modules under frame/; the macro is textually in
        // scope here, so the re-export looks unused in a non-test build.
        #[allow(unused_imports)]
        pub(crate) use with_fighter;
        impl SceneFighter {
            /// Scenario `kind` strings accepted by `Scenario::validate`.
            pub(crate) const NAMES: &'static [&'static str] = &[$( $name ),*];
            pub(crate) fn descriptor_for(name: &str) -> Option<&'static CharacterDescriptor> {
                match name {
                    $( $name => Some(<$ty as CharacterCallbacks>::descriptor()), )*
                    _ => None,
                }
            }
            /// Build the slot's fighter from the loaded archives and a retail
            /// Fighter dump, dispatching on the archive's descriptor kind.
            pub(crate) fn from_saved(
                archive: &CharacterArchive,
                resources: &FighterAssets,
                map: &CollMap,
                raw: &[u8],
                saved: &SavedPose,
            ) -> Self {
                $(
                    if archive.descriptor.kind == <$ty as CharacterCallbacks>::descriptor().kind {
                        return Self::$variant(Box::new(construct::<$ty>(archive, resources, map, raw, saved)));
                    }
                )*
                unreachable!("validated character descriptor {:?}", archive.descriptor.kind)
            }
        }
    };
}

scene_characters! {
    "Fox" => Fox(ft_fox::init::Fox),
    "Marth" => Marth(ft_mars::init::Marth),
    "Falco" => Falco(ft_falco::init::Falco),
}

fn construct<C: CharacterCallbacks>(
    archive: &CharacterArchive,
    resources: &FighterAssets,
    map: &CollMap,
    raw: &[u8],
    saved: &SavedPose,
) -> Fighter<C> {
    let mut character = C::from_archive(&archive.data)
        .unwrap_or_else(|e| panic!("{:?} character data: {e}", archive.descriptor.kind));
    character.restore_saved(raw);
    let mut fighter = crate::initial_state::import_fighter(archive, resources, character, map, raw);
    saved.restore(&mut fighter, raw);
    fighter
}

impl Snapshot for SceneFighter {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        with_fighter!(self, |f| f.snapshot(sink))
    }
}
impl SceneFighter {
    pub fn bone_matrix(&mut self, bone: Option<usize>) -> Mtx {
        fn matrix<C: CharacterCallbacks>(f: &mut Fighter<C>, bone: Option<usize>) -> Mtx {
            let joint = bone.map_or(f.animation.root, |i| f.animation.parts[i].joint);
            f.skeleton.setup_matrix(joint);
            f.skeleton.get(joint).mtx
        }
        with_fighter!(self, |f| matrix(f, bone))
    }
}
