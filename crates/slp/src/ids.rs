//! Melee ID tables referenced by the Slippi spec ("Melee IDs" section, the
//! Dan Salvato community sheet). Names are CamelCase so they can be used as
//! `kind` / `stage` strings in harness scenario TOML.

/// External (character-select-screen) character id, as stored in the Game
/// Start block at `0x60 + 0x24*i`.
pub fn external_character_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0 => "CaptainFalcon",
        1 => "DonkeyKong",
        2 => "Fox",
        3 => "GameAndWatch",
        4 => "Kirby",
        5 => "Bowser",
        6 => "Link",
        7 => "Luigi",
        8 => "Mario",
        9 => "Marth",
        10 => "Mewtwo",
        11 => "Ness",
        12 => "Peach",
        13 => "Pikachu",
        14 => "IceClimbers",
        15 => "Jigglypuff",
        16 => "Samus",
        17 => "Yoshi",
        18 => "Zelda",
        19 => "Sheik",
        20 => "Falco",
        21 => "YoungLink",
        22 => "DrMario",
        23 => "Roy",
        24 => "Pichu",
        25 => "Ganondorf",
        26 => "MasterHand",
        27 => "WireframeMale",
        28 => "WireframeFemale",
        29 => "GigaBowser",
        30 => "CrazyHand",
        31 => "Sandbag",
        32 => "Popo",
        _ => return None,
    })
}

/// Internal character id (`FighterKind`, `Fighter.kind` at fp+0x4), as
/// stored in Post Frame Update at 0x7.
pub fn internal_character_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0 => "Mario",
        1 => "Fox",
        2 => "CaptainFalcon",
        3 => "DonkeyKong",
        4 => "Kirby",
        5 => "Bowser",
        6 => "Link",
        7 => "Sheik",
        8 => "Ness",
        9 => "Peach",
        10 => "Popo",
        11 => "Nana",
        12 => "Pikachu",
        13 => "Samus",
        14 => "Yoshi",
        15 => "Jigglypuff",
        16 => "Mewtwo",
        17 => "Luigi",
        18 => "Marth",
        19 => "Zelda",
        20 => "YoungLink",
        21 => "DrMario",
        22 => "Falco",
        23 => "Pichu",
        24 => "GameAndWatch",
        25 => "Ganondorf",
        26 => "Roy",
        27 => "MasterHand",
        28 => "CrazyHand",
        29 => "WireframeMale",
        30 => "WireframeFemale",
        31 => "GigaBowser",
        32 => "Sandbag",
        _ => return None,
    })
}

/// Stage id as stored in the Game Info Block at 0xE.
pub fn stage_name(id: u16) -> Option<&'static str> {
    Some(match id {
        2 => "FountainOfDreams",
        3 => "PokemonStadium",
        4 => "PrincessPeachsCastle",
        5 => "KongoJungle",
        6 => "Brinstar",
        7 => "Corneria",
        8 => "YoshisStory",
        9 => "Onett",
        10 => "MuteCity",
        11 => "RainbowCruise",
        12 => "JungleJapes",
        13 => "GreatBay",
        14 => "HyruleTemple",
        15 => "BrinstarDepths",
        16 => "YoshisIsland",
        17 => "GreenGreens",
        18 => "Fourside",
        19 => "MushroomKingdom",
        20 => "MushroomKingdom2",
        22 => "Venom",
        23 => "PokeFloats",
        24 => "BigBlue",
        25 => "IcicleMountain",
        26 => "Icetop",
        27 => "FlatZone",
        28 => "DreamLand",
        29 => "YoshisIsland64",
        30 => "KongoJungle64",
        31 => "Battlefield",
        32 => "FinalDestination",
        _ => return None,
    })
}

/// Physical button bits (SPEC.md "Physical Buttons"), lowest bit first.
pub const PHYSICAL_BUTTONS: &[(u16, &str)] = &[
    (0x0001, "DpadLeft"),
    (0x0002, "DpadRight"),
    (0x0004, "DpadDown"),
    (0x0008, "DpadUp"),
    (0x0010, "Z"),
    (0x0020, "R"),
    (0x0040, "L"),
    (0x0100, "A"),
    (0x0200, "B"),
    (0x0400, "X"),
    (0x0800, "Y"),
    (0x1000, "Start"),
];

/// Names of the set physical buttons, in bit order.
pub fn button_names(bits: u16) -> Vec<&'static str> {
    PHYSICAL_BUTTONS
        .iter()
        .filter(|(m, _)| bits & m != 0)
        .map(|(_, n)| *n)
        .collect()
}
