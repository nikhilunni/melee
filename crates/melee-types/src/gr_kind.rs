//! Generated from `src/melee/gr/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Internal stage kind (`GrKind`, the `Gr_Kind_*` ids).
    ///
    /// This is the internal ground numbering, not the stage-select `StKind`.
    ///
    /// Source: `src/melee/gr/forward.h`.
    pub enum GrKind: i32 {
        /// `Gr_Kind_Unk00`
        Unk00 = 0x00,
        /// `Gr_Kind_Test`
        Test = 0x01,
        /// `Gr_Kind_Castle` — Princess Peach's Castle
        Castle = 0x02,
        /// `Gr_Kind_RCruise` — Rainbow Cruise
        RCruise = 0x03,
        /// `Gr_Kind_Kongo` — Kongo Jungle
        Kongo = 0x04,
        /// `Gr_Kind_Garden` — Jungle Japes
        Garden = 0x05,
        /// `Gr_Kind_GreatBay` — Great Bay
        GreatBay = 0x06,
        /// `Gr_Kind_Shrine` — Hyrule Temple
        Shrine = 0x07,
        /// `Gr_Kind_Zebes` — Brinstar
        Zebes = 0x08,
        /// `Gr_Kind_Kraid` — Brinstar Depths
        Kraid = 0x09,
        /// `Gr_Kind_Story` — Yoshi's Story
        Story = 0x0A,
        /// `Gr_Kind_Yorster` — Yoshi's Island
        Yorster = 0x0B,
        /// `Gr_Kind_Izumi` — Fountain of Dreams
        Izumi = 0x0C,
        /// `Gr_Kind_Greens` — Green Greens
        Greens = 0x0D,
        /// `Gr_Kind_Corneria` — Corneria
        Corneria = 0x0E,
        /// `Gr_Kind_Venom` — Venom
        Venom = 0x0F,
        /// `Gr_Kind_PStadium` — Pokemon Stadium
        PStadium = 0x10,
        /// `Gr_Kind_Pura` — Poke Floats
        Pura = 0x11,
        /// `Gr_Kind_MuteCity` — Mute City
        MuteCity = 0x12,
        /// `Gr_Kind_BigBlue` — Big Blue
        BigBlue = 0x13,
        /// `Gr_Kind_Onett` — Onett
        Onett = 0x14,
        /// `Gr_Kind_Fourside` — Fourside
        Fourside = 0x15,
        /// `Gr_Kind_Icemt` — Icicle Mountain
        Icemt = 0x16,
        /// `Gr_Kind_Unk23`
        Unk23 = 0x17,
        /// `Gr_Kind_Inishie1` — Mushroom Kingdom
        Inishie1 = 0x18,
        /// `Gr_Kind_Inishie2` — Mushroom Kingdom II
        Inishie2 = 0x19,
        /// `Gr_Kind_Unk26`
        Unk26 = 0x1A,
        /// `Gr_Kind_Flatzone` — Flat Zone
        Flatzone = 0x1B,
        /// `Gr_Kind_OldPupupu` — Dream Land
        OldPupupu = 0x1C,
        /// `Gr_Kind_OldYoshi` — Yoshi's Island (64)
        OldYoshi = 0x1D,
        /// `Gr_Kind_OldKongo` — Kongo Jungle (64)
        OldKongo = 0x1E,
        /// `Gr_Kind_KinokoRoute`
        KinokoRoute = 0x1F,
        /// `Gr_Kind_ShrineRoute`
        ShrineRoute = 0x20,
        /// `Gr_Kind_ZebesRoute`
        ZebesRoute = 0x21,
        /// `Gr_Kind_BigBlueRoute`
        BigBlueRoute = 0x22,
        /// `Gr_Kind_Unk35`
        Unk35 = 0x23,
        /// `Gr_Kind_Battle` — Battlefield
        Battle = 0x24,
        /// `Gr_Kind_Last` — Final Destination
        Last = 0x25,
        /// `Gr_Kind_FigureGet`
        FigureGet = 0x26,
        /// `Gr_Kind_Pushon`
        Pushon = 0x27,
        /// `Gr_Kind_TMario`
        TMario = 0x28,
        /// `Gr_Kind_TCaptain`
        TCaptain = 0x29,
        /// `Gr_Kind_TClink`
        TClink = 0x2A,
        /// `Gr_Kind_TDonkey`
        TDonkey = 0x2B,
        /// `Gr_Kind_TDrmario`
        TDrmario = 0x2C,
        /// `Gr_Kind_TFalco`
        TFalco = 0x2D,
        /// `Gr_Kind_TFox`
        TFox = 0x2E,
        /// `Gr_Kind_TIceclimber`
        TIceclimber = 0x2F,
        /// `Gr_Kind_TKirby`
        TKirby = 0x30,
        /// `Gr_Kind_TKoopa`
        TKoopa = 0x31,
        /// `Gr_Kind_TLink`
        TLink = 0x32,
        /// `Gr_Kind_TLuigi`
        TLuigi = 0x33,
        /// `Gr_Kind_TMars`
        TMars = 0x34,
        /// `Gr_Kind_TMewtwo`
        TMewtwo = 0x35,
        /// `Gr_Kind_TNess`
        TNess = 0x36,
        /// `Gr_Kind_TPeach`
        TPeach = 0x37,
        /// `Gr_Kind_TPichu`
        TPichu = 0x38,
        /// `Gr_Kind_TPikachu`
        TPikachu = 0x39,
        /// `Gr_Kind_TPurin`
        TPurin = 0x3A,
        /// `Gr_Kind_TSamus`
        TSamus = 0x3B,
        /// `Gr_Kind_TSeak`
        TSeak = 0x3C,
        /// `Gr_Kind_TYoshi`
        TYoshi = 0x3D,
        /// `Gr_Kind_TZelda`
        TZelda = 0x3E,
        /// `Gr_Kind_TGamewatch`
        TGamewatch = 0x3F,
        /// `Gr_Kind_TEmblem`
        TEmblem = 0x40,
        /// `Gr_Kind_TGanon`
        TGanon = 0x41,
        /// `Gr_Kind_Heal`
        Heal = 0x42,
        /// `Gr_Kind_Homerun` — Home run contest
        Homerun = 0x43,
        /// `Gr_Kind_Figure1`
        Figure1 = 0x44,
        /// `Gr_Kind_Figure2`
        Figure2 = 0x45,
        /// `Gr_Kind_Figure3`
        Figure3 = 0x46,
    }
}

impl GrKind {
    /// `Gr_Kind_Count` — explicit sentinel in the header (221), not one past `Gr_Kind_Figure3`
    pub const COUNT: i32 = 0xDD;
}
