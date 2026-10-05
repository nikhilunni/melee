//! Where retail finds each piece of menu art: which model in which archive,
//! which joint, and which texture animation frame.
use super::{
    scene::{Model, Texture},
    Piece,
};
use hsd_archive::Archive;
use melee_lib::{Character, Stage};

/// Texture animation frames per costume in the character select portrait,
/// emblem and stock icon tables: `hud_idx += color * 0x1E`
/// (`mnCharSel_8025DB34`, mncharsel.c:1568) and `mulli r0, r5, 0x1e` in
/// `gm_80168B34` (0x80168BCC).
const FRAMES_PER_COSTUME: u32 = 30;

/// `MnSelectChrDataTable` (MnSlChr): `ANIM = table + 0x10`, an array of
/// `{joint, anim, matanim, shapeanim}` (`mnCharSel_Scene_OnEnter`,
/// mncharsel.c:5343-5345).
const CSS_MODELS: u32 = 0x10;
/// `ANIM[3]`: the versus-mode panel model (mncharsel.c:4336).
const CSS_VERSUS_PANEL: u32 = 3;
/// Port 1's `costume_joint` and `emblem_joint` in `mnCharSel_803F0DFC`
/// (mncharsel.c:260): the portrait and series emblem `mnCharSel_8025D5AC`
/// (0x8025D5AC) animates to the costume's frame.
const CSS_PORTRAIT_JOINT: usize = 0x33;
const CSS_EMBLEM_JOINT: usize = 0x2E;
/// The icon joint's second texture is the face with its name plate; the
/// first is a transparent overlay layer.
const FACE_TEXTURE: usize = 1;

/// `MnSelectStageDataTable` (MnSlMap): camera, two lights and fog, then the
/// `StaticModelDesc`s `mnStageSel_804D6C98` points at (table + 0x10,
/// `mnStageSel_Scene_OnEnter`, mnstagesel.c:462).
const SSS_MODELS: u32 = 0x10;

/// `HSD_JObjLoadJoint`'s model `n` of a `{joint, anim, matanim, shapeanim}`
/// array at `table + base`.
fn static_model(archive: &Archive, base: u32, n: u32) -> Result<Model, String> {
    let table = archive
        .publics()
        .first()
        .map(|p| p.offset)
        .ok_or("the archive has no public symbol")?;
    Model::static_at(archive, table + base + 16 * n)
}

/// The texture `piece` shows in retail.
pub(super) fn locate(archive: &Archive, piece: Piece) -> Result<Texture, String> {
    match piece {
        Piece::Portrait(character, costume) => {
            costume_in_range(character, costume)?;
            let frame = portrait_frame(character, costume)?;
            animated(archive, CSS_MODELS, CSS_VERSUS_PANEL, CSS_PORTRAIT_JOINT, frame)
        }
        Piece::CharacterEmblem(character) => {
            // Sheik shares Zelda's series and her cell.
            let series = match character {
                Character::Sheik => Character::Zelda,
                other => other,
            };
            let frame = portrait_frame(series, 0)?;
            animated(archive, CSS_MODELS, CSS_VERSUS_PANEL, CSS_EMBLEM_JOINT, frame)
        }
        Piece::Face(character) => {
            let joint = face_joint(character).ok_or_else(|| no_art(character, "grid face"))?;
            let model = static_model(archive, CSS_MODELS, CSS_VERSUS_PANEL)?;
            model
                .textures(archive, joint, 0.0)?
                .get(FACE_TEXTURE)
                .copied()
                .ok_or_else(|| format!("joint {joint} has no face texture"))
        }
        Piece::Stock(character, costume) => {
            costume_in_range(character, costume)?;
            stock(archive, character, costume)
        }
        Piece::StageIcon(stage) => {
            let (model, joint, frame) = stage_icon(stage_row(stage));
            animated(archive, SSS_MODELS, model, joint, frame)
        }
        Piece::StageName(stage) => {
            // mnStageSel_80259ED8 (0x80259ED8): `do_anim(jobj, 20 * x9)` on
            // model x30, whose joint 2 is the name plate.
            let frame = 20.0 * f32::from(stage_row(stage).name_frame);
            animated(archive, SSS_MODELS, 3, 2, frame)
        }
        Piece::StageEmblem(stage) => {
            // fn_8025A090 (0x8025A090): `HSD_JObjReqAnimAll(jobj, 50 * x9)`
            // on model x60, whose joint 108 is the series emblem.
            let frame = 50.0 * f32::from(stage_row(stage).name_frame);
            animated(archive, SSS_MODELS, 6, 108, frame)
        }
    }
}

/// The first texture of `joint` that a texture animation drives at `frame`.
fn animated(archive: &Archive, base: u32, model: u32, joint: usize, frame: f32) -> Result<Texture, String> {
    static_model(archive, base, model)?
        .textures(archive, joint, frame)?
        .into_iter()
        .find(|t| t.animated)
        .ok_or_else(|| format!("joint {joint} of model {model} shows no animated texture at frame {frame}"))
}

fn no_art(character: Character, what: &str) -> String {
    format!(
        "{} has no {what} in the retail menus (Sheik is chosen through Zelda's)",
        character.display_name()
    )
}

fn costume_in_range(character: Character, costume: u8) -> Result<(), String> {
    if costume < character.costume_count() {
        Ok(())
    } else {
        Err(format!(
            "{} has {} costumes",
            character.display_name(),
            character.costume_count()
        ))
    }
}

fn portrait_frame(character: Character, costume: u8) -> Result<f32, String> {
    let hud = hud_index(character).ok_or_else(|| no_art(character, "portrait"))?;
    Ok((u32::from(hud) + FRAMES_PER_COSTUME * u32::from(costume)) as f32)
}

/// `CSSIcon.ft_hudindex` (`ICONHUD_*`, mn/forward.h): the character's
/// frame in the portrait and emblem tables. The order is the external
/// character id with Sheik left out.
fn hud_index(character: Character) -> Option<u8> {
    use Character::*;
    Some(match character {
        CaptainFalcon => 0x00,
        DonkeyKong => 0x01,
        Fox => 0x02,
        GameAndWatch => 0x03,
        // Kirby is 0x04.
        Bowser => 0x05,
        Link => 0x06,
        Luigi => 0x07,
        Mario => 0x08,
        Marth => 0x09,
        Mewtwo => 0x0A,
        Ness => 0x0B,
        Peach => 0x0C,
        Pikachu => 0x0D,
        IceClimbers => 0x0E,
        Jigglypuff => 0x0F,
        Samus => 0x10,
        Yoshi => 0x11,
        Zelda => 0x12,
        Falco => 0x13,
        YoungLink => 0x14,
        DrMario => 0x15,
        Roy => 0x16,
        Pichu => 0x17,
        Ganondorf => 0x18,
        Sheik => return None,
    })
}

/// `CSSIcon.joint_id_vs` (`ICONJOINT_*`, mn/types.h): the grid icon's joint
/// in the versus panel.
fn face_joint(character: Character) -> Option<usize> {
    use Character::*;
    Some(match character {
        DrMario => 0x04,
        Ganondorf => 0x06,
        Falco => 0x08,
        YoungLink => 0x0A,
        Pichu => 0x0C,
        Roy => 0x0E,
        Mario => 0x10,
        Luigi => 0x11,
        Bowser => 0x12,
        Peach => 0x13,
        Yoshi => 0x14,
        DonkeyKong => 0x15,
        CaptainFalcon => 0x16,
        Fox => 0x17,
        Ness => 0x18,
        IceClimbers => 0x19,
        // Kirby is 0x1A.
        Samus => 0x1B,
        Zelda => 0x1C,
        Link => 0x1D,
        Pikachu => 0x1E,
        Jigglypuff => 0x1F,
        Mewtwo => 0x20,
        GameAndWatch => 0x21,
        Marth => 0x22,
        Sheik => return None,
    })
}

/// The stock icon: `IfAll`'s `Stc_scemdls` model 0, material animation 0,
/// joint 1 (`ifStock_804A1378.player[p].x4[1]`, ifstock.c:1040), at the
/// frame `gm_80168B34` (0x80168B34) computes: the external character id,
/// one less past Sheik, Sheik herself 0x19, plus 30 per costume.
fn stock(archive: &Archive, character: Character, costume: u8) -> Result<Texture, String> {
    let base: u32 = match character {
        Character::Sheik => 0x19,
        // Zelda is 0x12 like her hud index; everyone else matches it too.
        other => hud_index(other).map(u32::from).expect("only Sheik lacks a hud index"),
    };
    let frame = (base + FRAMES_PER_COSTUME * u32::from(costume)) as f32;
    let list = archive
        .public("Stc_scemdls")
        .ok_or("IfAll.usd has no Stc_scemdls")?;
    let model = archive
        .link(list)
        .map_err(|e| e.to_string())?
        .ok_or("Stc_scemdls is empty")?;
    Model::dynamic_at(archive, model, 0)?
        .textures(archive, 1, frame)?
        .into_iter()
        .find(|t| t.animated)
        .ok_or_else(|| format!("no stock icon at frame {frame}"))
}

/// A row of `mnStageSel_803F06D0` (mnstagesel.static.h): the stage's place
/// on the stage select screen and its `x9` frame (the name table index).
#[derive(Clone, Copy)]
struct StageRow {
    row: u8,
    name_frame: u8,
}

fn stage_row(stage: Stage) -> StageRow {
    let (row, name_frame) = match stage {
        Stage::YoshisStory => (6, 0x08),
        Stage::FountainOfDreams => (8, 0x0A),
        Stage::PokemonStadium => (18, 0x0E),
        Stage::Battlefield => (24, 0x18),
        Stage::FinalDestination => (25, 0x19),
        Stage::DreamLand => (26, 0x1A),
    };
    StageRow { row, name_frame }
}

/// Model, joint and frame of a stage's icon, as `mnStageSel_Scene_OnEnter`
/// (0x8025A998, mnstagesel.c:540-668) places them.
fn stage_icon(row: StageRow) -> (u32, usize, f32) {
    let x9 = u32::from(row.name_frame);
    match row.row {
        // Icon pairs on model x40: the even row on `child->next` (joint 2),
        // the odd row on `child` (joint 1), at frame x9 / 2 + 2.
        0..=21 => (4, if row.row.is_multiple_of(2) { 2 } else { 1 }, (x9 / 2 + 2) as f32),
        // Model x0, joint 1, frame x9 - 0x14.
        22..=23 => (0, 1, (x9 - 0x14) as f32),
        // Model x20, joint 1, frame x9 - 0x16.
        _ => (2, 1, (x9 - 0x16) as f32),
    }
}
