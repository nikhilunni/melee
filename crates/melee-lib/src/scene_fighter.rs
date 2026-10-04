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
    assets::CharacterDescriptor, assets::FighterAssets, CharacterCallbacks, CharacterForm, Fighter,
};
use melee_types::snapshot::{Snapshot, SnapshotSink};
use std::mem::ManuallyDrop;

/// The roster selects archives and typed payloads only during construction.
/// Every scene slot then owns one concrete fighter. Its box is allocated once
/// at setup; common gameplay dispatches through the installed static table.
macro_rules! scene_characters {
    ($( $name:literal => $variant:ident ( $ty:path ) $( partner ( $partner:path ) )? ),* $(,)?) => {
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
            /// Player_80031AD0's second Fighter_Create: a transforming
            /// character's other form, asleep beside the one in play.
            pub(crate) fn partner_from_parameters(
                archive: &CharacterArchive,
                resources: &FighterAssets,
                player: melee_ft::fighter::PlayerSlot,
                context: melee_ft::fighter::SpawnContext<'_>,
            ) -> anyhow::Result<Self> {
                let character = Self::character_for_costume(archive, player.costume)?;
                let (skeleton, root) = archive.model(player.costume);
                Ok(Self::new(Fighter::spawn_asleep(
                    player, character, resources, skeleton, root, context,
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
                    $(
                        if archive.descriptor.kind == <$partner as CharacterForm>::form_descriptor().kind {
                            let mut character = <$partner as CharacterForm>::read_form(&archive.data)
                                .map_err(|e| anyhow::anyhow!("{e}"))?;
                            character.on_costume_loaded(archive.costume(costume), costume).map_err(|e| anyhow::anyhow!("{e}"))?;
                            return Ok(character.into_state());
                        }
                    )?
                )*
                unreachable!("validated character descriptor")
            }
            /// Scenario `kind` strings accepted by `Scenario::validate`.
            pub(crate) const NAMES: &'static [&'static str] = &[$( $name ),*];
            /// The kind's `CharacterCallbacks::ONLOAD_ITEM_JOINT`.
            pub(crate) fn onload_item_joint_for(kind: melee_types::FighterKind) -> Option<u32> {
                $(
                    if kind == <$ty as CharacterCallbacks>::descriptor().kind {
                        return <$ty as CharacterCallbacks>::ONLOAD_ITEM_JOINT;
                    }
                )*
                None
            }
            pub(crate) fn descriptor_for(name: &str) -> Option<&'static CharacterDescriptor> {
                match name {
                    $( $name => Some(<$ty as CharacterCallbacks>::descriptor()), )*
                    _ => None,
                }
            }
            /// ftMapping_list's `extra_internal_id` without a transformation:
            /// the second fighter a player of this character creates.
            pub(crate) fn partner_for(
                descriptor: &CharacterDescriptor,
            ) -> Option<&'static CharacterDescriptor> {
                $(
                    $(
                        if descriptor.kind == <$ty as CharacterCallbacks>::descriptor().kind {
                            return Some(<$partner as CharacterForm>::form_descriptor());
                        }
                    )?
                )*
                let _ = descriptor;
                None
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
                        return Self::new(construct(<$ty as CharacterCallbacks>::from_archive(&archive.data), archive, resources, map, raw, saved));
                    }
                    $(
                        if archive.descriptor.kind == <$partner as CharacterForm>::form_descriptor().kind {
                            return Self::new(construct(<$partner as CharacterForm>::read_form(&archive.data), archive, resources, map, raw, saved));
                        }
                    )?
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
    "Ganondorf" => Ganondorf(ft_ganon::init::Ganondorf),
    "IceClimbers" => IceClimbers(ft_iceclimbers::init::IceClimber) partner(ft_iceclimbers::init::Nana),
    "Link" => Link(ft_link::init::Link),
    "YoungLink" => YoungLink(ft_younglink::init::YoungLink),
    "GameAndWatch" => GameAndWatch(ft_gamewatch::init::GameWatch),
    "Samus" => Samus(ft_samus::init::Samus),
    "Sheik" => Sheik(ft_seak::init::Sheik) partner(ft_zelda::init::Zelda),
    "Zelda" => Zelda(ft_zelda::init::Zelda) partner(ft_seak::init::Sheik),
    "DonkeyKong" => DonkeyKong(ft_donkey::init::DonkeyKong),
    "Bowser" => Bowser(ft_koopa::init::Koopa),
}

/// ftMapping_list's `has_transformation` (pl/player.c:62-63): a partner of
/// these kinds is the player's other form, created asleep beside the one in
/// play. Only Zelda and Sheik transform.
pub(crate) fn transforms(kind: melee_types::FighterKind) -> bool {
    matches!(
        kind,
        melee_types::FighterKind::Zelda | melee_types::FighterKind::Seak
    )
}

/// `character`: the form's `from_archive`, read where the form is concrete.
fn construct<C: CharacterCallbacks>(
    character: Result<C, melee_ft::desc::FighterDescError>,
    archive: &CharacterArchive,
    resources: &FighterAssets,
    map: &CollMap,
    raw: &[u8],
    saved: &SavedPose,
) -> Fighter {
    let mut character =
        character.unwrap_or_else(|e| panic!("{:?} character data: {e}", archive.descriptor.kind));
    character
        .on_costume_loaded(archive.costume(raw[0x619]), raw[0x619])
        .expect("character costume data");
    character.restore_saved(raw);
    let mut fighter =
        crate::initial_state::import_fighter(archive, resources, character.into_state(), map, raw);
    saved.restore(&mut fighter, raw);
    fighter
}

/// The fighter-list index of player `player`'s own fighter (x221F_b4
/// clear), `player` counting players in port order.
pub(crate) fn player_fighter_index(fighters: &[SceneFighter], player: usize) -> usize {
    fighters
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.0.player.secondary)
        .nth(player)
        .map(|(index, _)| index)
        .expect("one fighter per player")
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

// Keep destruction of the fighter graph in melee-ft (`Fighter::destroy`).
// Otherwise each crate that drops a fighter repeats this entire graph's glue.
impl Drop for SceneFighter {
    #[inline(never)]
    fn drop(&mut self) {
        // SAFETY: new/Clone initialize the sole owner; it is taken out only
        // here, and ManuallyDrop suppresses the automatic second drop.
        unsafe { ManuallyDrop::take(&mut self.0) }.destroy();
    }
}
