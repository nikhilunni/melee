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
//! the LOAD_HAT loaders 800F0FC0, 800F10D4, 800F11F0, 800F130C, 800F14B4)
//! graft joints into Kirby's own empty parts and hang a replacement body
//! model on his joints. They are not ported.
use melee_types::FighterKind;

/// The kinds whose hat is grafted into Kirby's own parts.
fn replaces_parts(kind: FighterKind) -> bool {
    matches!(
        kind,
        FighterKind::Donkey
            | FighterKind::Purin
            | FighterKind::Mewtwo
            | FighterKind::Falco
            | FighterKind::GameWatch
    )
}

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

/// ftKb_Init_803C9CC8[kind * 2]: refuse a hat the port cannot wear.
pub fn require_supported(kind: FighterKind) {
    if replaces_parts(kind) {
        unimplemented!(
            "ftKb_Init_803C9CC8[{kind:?}] (ftkirby.c:3578-3592, LOAD_HAT): a hat grafted into \
             Kirby's parts with a replacement model"
        );
    }
}
