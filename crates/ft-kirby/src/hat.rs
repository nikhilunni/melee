//! The hats of the copied abilities, ftKb_Init_803C9CC8 (ftkirby.c:2280):
//! a loader and an unloader per copied kind.
//!
//! Most hats are a model of their own (ftKb_LoadHat: `hat.jobj`), drawn at
//! Kirby's part 6 matrix (ftKb_UnkMtxFunc0) while x2225_b2 is set. Nothing
//! the simulation reads hangs on that model: loading and unloading it
//! creates no GObj and draws no random number. Some of them carry spring
//! chains (ftCo_8009D074..8009DA38), which move the hat model's own joints
//! and are skipped for a Kirby by every blend entry of ftCo_8009E7B4.
//!
//! Five kinds (Donkey Kong, Jigglypuff, Mewtwo, Falco, Mr. Game & Watch:
//! the LOAD_HAT loaders 800F0FC0, 800F10D4, 800F11F0, 800F130C, 800F14B4,
//! ftkirby.c:3578-3592) graft a model into Kirby's own parts instead. Each
//! adds the hat's spring chains (ftKb_SpecialN_800EF040, ftParts_800753D4;
//! Jigglypuff's and Mewtwo's also ftCo_8009D81C / 8009DB50), allocates two
//! DObj tables, hangs the hat's DObjs on Kirby's thirteen empty parts
//! (800EF0E4, 800EF438: parts flags b6 / b7; no joint is moved or added,
//! and no bone table of PlKb.dat names one of those parts),
//! adds their material animations (800EF35C) and builds the part
//! visibility tables (ftParts_8007487C, ftAnim_80070200). Mr. Game &
//! Watch's loader also selects his flat model group (ftParts_80074D7C,
//! 800F1420: material colors), sets x610's second color and sets the root
//! scale again from the unchanged model scale (Fighter_UpdateModelScale).
//! The unloaders (ftKb_SpecialN_800EF69C, ftCo_UnloadDynamicBones) undo
//! the same. None of it creates a GObj, draws a random number or moves a
//! bone the simulation reads, so they are drawn only. What a copied ability does
//! that the game sees lives in `crate::ability` and
//! `crate::damage_callbacks`.
use melee_types::FighterKind;

/// fn_800F1CA0 (800F1CA0): the hats that cover Kirby's face, under which
/// his midair jumps play the helmeted animations.
pub fn is_helmet(kind: FighterKind) -> bool {
    matches!(
        kind,
        FighterKind::Captain
            | FighterKind::Seak
            | FighterKind::Popo
            | FighterKind::Nana
            | FighterKind::Samus
    )
}
