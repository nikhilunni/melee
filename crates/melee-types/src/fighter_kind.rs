//! Generated from `src/melee/ft/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Internal fighter kind (`FighterKind`, the `FTKIND_*` ids).
    ///
    /// This is the *internal* numbering used by `Fighter::kind`, not the
    /// character-select `CharacterKind`. It includes ids that are never
    /// selectable: Nana, Master Hand, Crazy Hand, the Wireframes, Giga
    /// Bowser and Sandbag.
    ///
    /// Source: `src/melee/ft/forward.h`.
    pub enum FighterKind: i32 {
        /// `FTKIND_MARIO` — Mario
        Mario = 0x00,
        /// `FTKIND_FOX` — Fox
        Fox = 0x01,
        /// `FTKIND_CAPTAIN` — Captain Falcon
        Captain = 0x02,
        /// `FTKIND_DONKEY` — Donkey Kong
        Donkey = 0x03,
        /// `FTKIND_KIRBY` — Kirby
        Kirby = 0x04,
        /// `FTKIND_KOOPA` — Bowser
        Koopa = 0x05,
        /// `FTKIND_LINK` — Link
        Link = 0x06,
        /// `FTKIND_SEAK` — Sheik
        Seak = 0x07,
        /// `FTKIND_NESS` — Ness
        Ness = 0x08,
        /// `FTKIND_PEACH` — Peach
        Peach = 0x09,
        /// `FTKIND_POPO` — Ice Climbers leader (Popo)
        Popo = 0x0A,
        /// `FTKIND_NANA` — Ice Climbers partner (Nana); internal id, not selectable
        Nana = 0x0B,
        /// `FTKIND_PIKACHU` — Pikachu
        Pikachu = 0x0C,
        /// `FTKIND_SAMUS` — Samus
        Samus = 0x0D,
        /// `FTKIND_YOSHI` — Yoshi
        Yoshi = 0x0E,
        /// `FTKIND_PURIN` — Jigglypuff
        Purin = 0x0F,
        /// `FTKIND_MEWTWO` — Mewtwo
        Mewtwo = 0x10,
        /// `FTKIND_LUIGI` — Luigi
        Luigi = 0x11,
        /// `FTKIND_MARS` — Marth
        Mars = 0x12,
        /// `FTKIND_ZELDA` — Zelda
        Zelda = 0x13,
        /// `FTKIND_CLINK` — Young Link
        CLink = 0x14,
        /// `FTKIND_DRMARIO` — Dr. Mario
        DrMario = 0x15,
        /// `FTKIND_FALCO` — Falco
        Falco = 0x16,
        /// `FTKIND_PICHU` — Pichu
        Pichu = 0x17,
        /// `FTKIND_GAMEWATCH` — Mr. Game & Watch
        GameWatch = 0x18,
        /// `FTKIND_GANON` — Ganondorf
        Ganon = 0x19,
        /// `FTKIND_EMBLEM` — Roy
        Emblem = 0x1A,
        /// `FTKIND_MASTERH` — Master Hand; internal id
        MasterH = 0x1B,
        /// `FTKIND_CREZYH` — Crazy Hand; internal id (header spelling)
        CrezyH = 0x1C,
        /// `FTKIND_BOY` — Male Wireframe; internal id
        Boy = 0x1D,
        /// `FTKIND_GIRL` — Female Wireframe; internal id
        Girl = 0x1E,
        /// `FTKIND_GKOOPS` — Giga Bowser; internal id
        GKoops = 0x1F,
        /// `FTKIND_SANDBAG` — Sandbag; internal id
        Sandbag = 0x20,
        /// `FTKIND_NONE` — No fighter
        None = 0x21,
    }
}

impl FighterKind {
    /// `FTKIND_MAX` — alias of `FTKIND_NONE`; one past the last real fighter kind
    pub const MAX: i32 = 0x21;
}
