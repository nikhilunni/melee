//! Menu art decoded from the user's disc: character portraits, grid faces,
//! stock icons, series emblems, stage icons and stage names. Nothing here
//! ships with the program; every image comes from the menu archives of the
//! disc the user opened, decoded on first use and cached.
//!
//! Each image is chosen the way the retail menus choose it: the same model,
//! the same joint (numbered as `lb_80011E24` counts) and the same texture
//! animation frame (`retail.rs` cites the code). Pixels are the GX
//! texture's own, converted to RGBA8 with straight alpha and never
//! resampled.
//!
//! | Image                         | Archive       | Native size        |
//! |-------------------------------|---------------|--------------------|
//! | [`Piece::Portrait`]           | `MnSlChr.usd` | 136 x 188 (CI8)    |
//! | [`Piece::Face`]               | `MnSlChr.usd` | 64 x 56 (CI8)      |
//! | [`Piece::CharacterEmblem`]    | `MnSlChr.usd` | 80 x 64 (I4)       |
//! | [`Piece::Stock`]              | `IfAll.usd`   | 24 x 24 (CI4)      |
//! | [`Piece::StageIcon`]          | `MnSlMap.usd` | 64 x 56 (CI8); 48 x 48 in the Past Stages row (Dream Land) |
//! | [`Piece::StageName`]          | `MnSlMap.usd` | 224 x 56 (I4)      |
//! | [`Piece::StageEmblem`]        | `MnSlMap.usd` | 64 x 64 (I4)       |
//!
//! Intensity (I4) images carry the intensity in every channel, the way GX
//! samples them: use their alpha as a mask and tint it. The character
//! emblems and stage names reach full intensity; the stage emblems are
//! retail's faint watermarks (alpha peaks at 119).
//!
//! Retail has no 2D stage preview: the stage select shows miniature 3D
//! models. [`Piece::StagePreview`] is rendered at runtime instead
//! (`crate::preview`). Sheik has no portrait or
//! face either (she is chosen through Zelda's); her emblem is Zelda's and
//! her stock icons are her own.
mod retail;
mod scene;

use crate::catalog;
use hsd_archive::{visual::read_image, Archive};
use melee_lib::{Character, Stage};
use std::{collections::BTreeMap, sync::Arc};

/// An RGBA8 image, rows top to bottom, straight (unassociated) alpha.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The archives menu art comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtFile {
    /// `MnSlChr.usd`: the character select screen (US English).
    CharacterSelect,
    /// `MnSlMap.usd`: the stage select screen.
    StageSelect,
    /// `IfAll.usd`: the in-match interface.
    Interface,
}
impl ArtFile {
    pub const ALL: [Self; 3] = [Self::CharacterSelect, Self::StageSelect, Self::Interface];
    /// The disc file name. Retail loads the `.usd` variants when the saved
    /// language is English (`mnCharSel_Scene_OnEnter`, mncharsel.c:5340;
    /// `mnStageSel_Scene_OnEnter`, mnstagesel.c:457).
    pub fn name(self) -> &'static str {
        match self {
            Self::CharacterSelect => "MnSlChr.usd",
            Self::StageSelect => "MnSlMap.usd",
            Self::Interface => "IfAll.usd",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }
}

/// One piece of menu art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// The character select portrait (CSP) for a costume. Sheik has none:
    /// retail selects her through Zelda's portrait.
    Portrait(Character, u8),
    /// The character select grid face, with the name plate.
    Face(Character),
    /// The in-match stock icon for a costume.
    Stock(Character, u8),
    /// The series emblem shown behind the portrait.
    CharacterEmblem(Character),
    /// The stage select icon.
    StageIcon(Stage),
    /// The stage select name plate (series above, stage name below).
    StageName(Stage),
    /// The series emblem the stage select shows for the stage.
    StageEmblem(Stage),
    /// A wide picture of the stage, rendered at runtime (`crate::preview`):
    /// the disc has no 2D stage preview to decode.
    StagePreview(Stage),
}
/// The kinds of [`Piece`], numbered for the C and wasm bindings
/// (`melee_art_e` in `melee_platform.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ArtKind {
    Portrait = 0,
    Face = 1,
    Stock = 2,
    CharacterEmblem = 3,
    StageIcon = 4,
    StageName = 5,
    StageEmblem = 6,
    StagePreview = 7,
}
impl ArtKind {
    pub const ALL: [Self; 8] = [
        Self::Portrait,
        Self::Face,
        Self::Stock,
        Self::CharacterEmblem,
        Self::StageIcon,
        Self::StageName,
        Self::StageEmblem,
        Self::StagePreview,
    ];
    pub fn from_u32(value: u32) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }
}

impl Piece {
    /// The piece a binding names by kind, catalog id (a character id for
    /// character kinds, a stage id for stage kinds) and costume (ignored
    /// where it does not apply).
    pub fn new(kind: ArtKind, id: u32, costume: u8) -> Result<Self, String> {
        let character = || catalog::character(id).ok_or("no character with that id");
        let stage = || catalog::stage(id).ok_or("no stage with that id");
        Ok(match kind {
            ArtKind::Portrait => Self::Portrait(character()?, costume),
            ArtKind::Face => Self::Face(character()?),
            ArtKind::Stock => Self::Stock(character()?, costume),
            ArtKind::CharacterEmblem => Self::CharacterEmblem(character()?),
            ArtKind::StageIcon => Self::StageIcon(stage()?),
            ArtKind::StageName => Self::StageName(stage()?),
            ArtKind::StageEmblem => Self::StageEmblem(stage()?),
            ArtKind::StagePreview => Self::StagePreview(stage()?),
        })
    }
    /// The menu archive the piece is decoded from; `None` for the stage
    /// previews, which are rendered.
    pub fn file(self) -> Option<ArtFile> {
        Some(match self {
            Self::Portrait(..) | Self::Face(_) | Self::CharacterEmblem(_) => {
                ArtFile::CharacterSelect
            }
            Self::Stock(..) => ArtFile::Interface,
            Self::StageIcon(_) | Self::StageName(_) | Self::StageEmblem(_) => ArtFile::StageSelect,
            Self::StagePreview(_) => return None,
        })
    }
    /// A stable, ordered cache key.
    fn key(self) -> (u8, u32, u8) {
        let c = catalog::character_id;
        let s = catalog::stage_id;
        match self {
            Self::Portrait(ch, costume) => (0, c(ch), costume),
            Self::Face(ch) => (1, c(ch), 0),
            Self::Stock(ch, costume) => (2, c(ch), costume),
            Self::CharacterEmblem(ch) => (3, c(ch), 0),
            Self::StageIcon(st) => (4, s(st), 0),
            Self::StageName(st) => (5, s(st), 0),
            Self::StageEmblem(st) => (6, s(st), 0),
            Self::StagePreview(st) => (7, s(st), 0),
        }
    }
}

/// The parsed menu archives and the images decoded from them so far.
#[derive(Default)]
pub struct Art {
    archives: BTreeMap<ArtFile, Archive>,
    cache: BTreeMap<(u8, u32, u8), Result<Arc<Image>, String>>,
}

impl Art {
    pub fn new() -> Self {
        Self::default()
    }
    /// Archives not provided yet, in [`ArtFile::ALL`] order.
    pub fn missing(&self) -> Vec<ArtFile> {
        ArtFile::ALL
            .into_iter()
            .filter(|f| !self.archives.contains_key(f))
            .collect()
    }
    pub fn has(&self, file: ArtFile) -> bool {
        self.archives.contains_key(&file)
    }
    /// Whether every archive is in; until then the missing pieces fail
    /// with a message naming the archive.
    pub fn is_ready(&self) -> bool {
        self.archives.len() == ArtFile::ALL.len()
    }
    /// Parse an archive the host read from the disc.
    pub fn insert(&mut self, file: ArtFile, bytes: &[u8]) -> Result<(), String> {
        let archive = Archive::parse(bytes).map_err(|e| format!("{}: {e}", file.name()))?;
        self.archives.insert(file, archive);
        // Earlier "not loaded yet" answers for this archive are stale.
        self.cache.retain(|_, entry| entry.is_ok());
        Ok(())
    }

    /// The image for `piece`, decoded once and then shared.
    pub fn image(&mut self, piece: Piece) -> Result<Arc<Image>, String> {
        let key = piece.key();
        if let Some(entry) = self.cache.get(&key) {
            return entry.clone();
        }
        let decoded = self.decode(piece).map(Arc::new);
        // Remember failures too, except a missing archive (it may arrive).
        if decoded.is_ok() || piece.file().is_some_and(|file| self.has(file)) {
            self.cache.insert(key, decoded.clone());
        }
        decoded
    }

    pub fn character_portrait(
        &mut self,
        character: Character,
        costume: u8,
    ) -> Result<Arc<Image>, String> {
        self.image(Piece::Portrait(character, costume))
    }
    pub fn character_face(&mut self, character: Character) -> Result<Arc<Image>, String> {
        self.image(Piece::Face(character))
    }
    pub fn stock_icon(&mut self, character: Character, costume: u8) -> Result<Arc<Image>, String> {
        self.image(Piece::Stock(character, costume))
    }
    pub fn character_emblem(&mut self, character: Character) -> Result<Arc<Image>, String> {
        self.image(Piece::CharacterEmblem(character))
    }
    pub fn stage_icon(&mut self, stage: Stage) -> Result<Arc<Image>, String> {
        self.image(Piece::StageIcon(stage))
    }
    pub fn stage_name(&mut self, stage: Stage) -> Result<Arc<Image>, String> {
        self.image(Piece::StageName(stage))
    }
    pub fn stage_emblem(&mut self, stage: Stage) -> Result<Arc<Image>, String> {
        self.image(Piece::StageEmblem(stage))
    }

    fn decode(&self, piece: Piece) -> Result<Image, String> {
        let file = piece
            .file()
            .ok_or("stage previews are rendered, not decoded (App::art_image)")?;
        let archive = self
            .archives
            .get(&file)
            .ok_or_else(|| format!("{} is not loaded yet", file.name()))?;
        let texture = retail::locate(archive, piece)?;
        let decoded = read_image(archive, texture.image, texture.palette)
            .map_err(|e| format!("{}: {e}", file.name()))?;
        Ok(Image {
            width: decoded.width.into(),
            height: decoded.height.into(),
            rgba: decoded.rgba,
        })
    }
}
