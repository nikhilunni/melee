//! Character erasure is confined to scene composition.
//!
//! **Adding a character:** add its crate to melee-lib's Cargo.toml and one
//! line to `scene_characters!` below, then extend the public Character enum.
//! Composition uses this roster for loading, construction, dispatch and the scenario `kind`
//! strings all come from this list and each crate's `CharacterCallbacks`.
use crate::assets::CharacterArchive;
use crate::initial_state::{CollMap, SavedPose};
use hsd_types::Mtx;
use melee_ft::fighter::{
    assets::CharacterDescriptor, assets::FighterAssets, CharacterCallbacks, Fighter,
};
use melee_types::snapshot::{Snapshot, SnapshotSink};
use std::mem::ManuallyDrop;

/// The roster selects archives and typed payloads only during construction.
/// Every scene slot then owns one concrete fighter. Its box is allocated once
/// at setup; common gameplay dispatches through the installed static table.
macro_rules! scene_characters {
    ($( $name:literal => $variant:ident ( $ty:path ) ),* $(,)?) => {
        #[derive(Clone)]
        pub(crate) struct SceneFighter(pub(crate) ManuallyDrop<Box<Fighter>>);
        macro_rules! with_fighter {
            ($fighter:expr, |$f:ident| $body:expr) => {{
                match $fighter { $crate::scene_fighter::SceneFighter($f) => $body }
            }};
        }
        // Used by the test modules under frame/; the macro is textually in
        // scope here, so the re-export looks unused in a non-test build.
        #[allow(unused_imports)]
        pub(crate) use with_fighter;
        impl SceneFighter {
            fn new(fighter: Fighter) -> Self { Self(ManuallyDrop::new(Box::new(fighter))) }
            pub(crate) fn from_parameters(
                archive: &CharacterArchive,
                resources: &FighterAssets,
                player: melee_ft::fighter::PlayerSlot,
                delay: i32,
                context: melee_ft::fighter::SpawnContext<'_>,
            ) -> anyhow::Result<Self> {
                let character = Self::character_for_costume(archive, player.costume)?;
                let (skeleton, root) = archive.model(player.costume);
                Ok(Self::new(Fighter::spawn_for_match(
                    player, character, resources, skeleton, root, context, delay,
                ).map_err(|e| anyhow::anyhow!("{e}"))?))
            }
            // Keep the concrete Fighter result outside the roster expansion.
            // At opt-level 0 each expanded result otherwise occupies stack space.
            fn character_for_costume(
                archive: &CharacterArchive,
                costume: u8,
            ) -> anyhow::Result<melee_ft::fighter::CharacterState> {
                $(
                    if archive.descriptor.kind == <$ty as CharacterCallbacks>::descriptor().kind {
                        let mut character = <$ty as CharacterCallbacks>::from_archive(&archive.data)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                        character.on_costume_loaded(archive.costume(costume), costume).map_err(|e| anyhow::anyhow!("{e}"))?;
                        return Ok(character.into_state());
                    }
                )*
                unreachable!("validated character descriptor")
            }
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
                        return Self::new(construct::<$ty>(archive, resources, map, raw, saved));
                    }
                )*
                unreachable!("validated character descriptor {:?}", archive.descriptor.kind)
            }
        }
    };
}

scene_characters! {
    "Fox" => Fox(ft_fox::init::Fox),
    "Peach" => Peach(ft_peach::init::Peach),
    "Yoshi" => Yoshi(ft_yoshi::init::Yoshi),
    "Jigglypuff" => Jigglypuff(ft_purin::init::Jigglypuff),
    "Pikachu" => Pikachu(ft_pikachu::init::Pikachu),
    "Marth" => Marth(ft_mars::init::Marth),
    "Roy" => Roy(ft_emblem::init::Roy),
    "Falco" => Falco(ft_falco::init::Falco),
    "CaptainFalcon" => CaptainFalcon(ft_captain::init::CaptainFalcon),
    "Mario" => Mario(ft_mario::init::Mario),
    "DrMario" => DrMario(ft_drmario::init::DrMario),
    "Luigi" => Luigi(ft_luigi::init::Luigi),
    "Pichu" => Pichu(ft_pichu::init::Pichu),
}

fn construct<C: CharacterCallbacks>(
    archive: &CharacterArchive,
    resources: &FighterAssets,
    map: &CollMap,
    raw: &[u8],
    saved: &SavedPose,
) -> Fighter {
    let mut character = C::from_archive(&archive.data)
        .unwrap_or_else(|e| panic!("{:?} character data: {e}", archive.descriptor.kind));
    character
        .on_costume_loaded(archive.costume(raw[0x619]), raw[0x619])
        .expect("character costume data");
    character.restore_saved(raw);
    let mut fighter =
        crate::initial_state::import_fighter(archive, resources, character.into_state(), map, raw);
    saved.restore(&mut fighter, raw);
    fighter
}

impl Snapshot for SceneFighter {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        self.0.snapshot(sink)
    }
}
impl SceneFighter {
    pub fn bone_matrix(&mut self, bone: Option<usize>) -> Mtx {
        fn matrix(f: &mut Fighter, bone: Option<usize>) -> Mtx {
            let joint = bone.map_or(f.animation.root, |i| f.animation.parts[i].joint);
            f.skeleton.setup_matrix(joint);
            f.skeleton.get(joint).mtx
        }
        with_fighter!(self, |f| matrix(f, bone))
    }
    /// Part `part` as an orientation constraint's target (resolveCnsOrientation).
    pub fn orientation_target(&mut self, part: usize) -> hsd_anim::orientation::OrientationTarget {
        with_fighter!(self, |f| {
            let joint = f.animation.parts[part].joint;
            f.skeleton.orientation_target(joint)
        })
    }
}

impl std::ops::Deref for SceneFighter {
    type Target = Fighter;
    fn deref(&self) -> &Fighter {
        &self.0
    }
}
impl std::ops::DerefMut for SceneFighter {
    fn deref_mut(&mut self) -> &mut Fighter {
        &mut self.0
    }
}

// Keep destruction of the opaque match's fighter graph in its owning crate.
// Otherwise downstream drop glue repeats this entire graph in each consumer.
impl Drop for SceneFighter {
    #[inline(never)]
    fn drop(&mut self) {
        // SAFETY: new/Clone initialize the sole owner; it is never taken out,
        // and ManuallyDrop suppresses the automatic second drop of this field.
        unsafe {
            ManuallyDrop::drop(&mut self.0);
        }
    }
}
