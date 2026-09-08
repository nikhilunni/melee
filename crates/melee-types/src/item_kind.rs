//! Generated from `src/melee/it/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Item kind (`ItemKind`, the `It_Kind_*` and `It_PKind_*` ids).
    ///
    /// Poke Ball Pokemon occupy `[POKEMON_START, POKEMON_TERMINATE)`.
    ///
    /// Source: `src/melee/it/forward.h`.
    pub enum ItemKind: i32 {
        /// `It_Kind_Capsule` — Capsule
        Capsule = 0x00,
        /// `It_Kind_Box` — Crate
        Box = 0x01,
        /// `It_Kind_Taru` — Barrel
        Taru = 0x02,
        /// `It_Kind_Egg` — Egg
        Egg = 0x03,
        /// `It_Kind_Kusudama` — Party Ball (Kusudama)
        Kusudama = 0x04,
        /// `It_Kind_TaruCann` — Barrel Cannon (TaruCann)
        TaruCann = 0x05,
        /// `It_Kind_BombHei` — Bob-omb (BombHei)
        BombHei = 0x06,
        /// `It_Kind_Dosei` — Mr. Saturn (Dosei)
        Dosei = 0x07,
        /// `It_Kind_Heart` — Heart Container
        Heart = 0x08,
        /// `It_Kind_Tomato` — Maxim Tomato
        Tomato = 0x09,
        /// `It_Kind_Star` — Starman (Super Star)
        Star = 0x0A,
        /// `It_Kind_Bat` — Home-Run Bat
        Bat = 0x0B,
        /// `It_Kind_Sword` — Beam Sword
        Sword = 0x0C,
        /// `It_Kind_Parasol` — Parasol
        Parasol = 0x0D,
        /// `It_Kind_G_Shell` — Green Shell (G Shell)
        GShell = 0x0E,
        /// `It_Kind_R_Shell` — Red Shell (R Shell)
        RShell = 0x0F,
        /// `It_Kind_L_Gun` — Ray Gun (L Gun)
        LGun = 0x10,
        /// `It_Kind_Freeze` — Freezie (Freeze)
        Freeze = 0x11,
        /// `It_Kind_Foods` — Food
        Foods = 0x12,
        /// `It_Kind_MSBomb` — Proximity Mine (MSBomb)
        MSBomb = 0x13,
        /// `It_Kind_Flipper` — Flipper
        Flipper = 0x14,
        /// `It_Kind_S_Scope` — Super Scope (S Scope)
        SScope = 0x15,
        /// `It_Kind_StarRod` — Star Rod
        StarRod = 0x16,
        /// `It_Kind_LipStick` — Lip's Stick
        LipStick = 0x17,
        /// `It_Kind_Harisen` — Fan (Harisen)
        Harisen = 0x18,
        /// `It_Kind_F_Flower` — Fire Flower (F Flower)
        FFlower = 0x19,
        /// `It_Kind_Kinoko` — Super Mushroom (Kinoko)
        Kinoko = 0x1A,
        /// `It_Kind_DKinoko` — Poison Mushroom (DKinoko)
        DKinoko = 0x1B,
        /// `It_Kind_Hammer` — Hammer
        Hammer = 0x1C,
        /// `It_Kind_WStar` — Warp Star (WStar)
        WStar = 0x1D,
        /// `It_Kind_ScBall` — Screw Attack (ScBall)
        ScBall = 0x1E,
        /// `It_Kind_RabbitC` — Bunny Hood (RabbitC)
        RabbitC = 0x1F,
        /// `It_Kind_MetalB` — Metal Box
        MetalB = 0x20,
        /// `It_Kind_Spycloak` — Cloaking Device (Spycloak)
        Spycloak = 0x21,
        /// `It_Kind_M_Ball` — Poke Ball (M Ball)
        MBall = 0x22,
        /// `It_Kind_L_Gun_Ray` — Ray Gun recoil effect (?)
        LGunRay = 0x23,
        /// `It_Kind_StarRod_Star` — Star Rod Star
        StarRodStar = 0x24,
        /// `It_Kind_LipStick_Spore` — Lips Stick Dust
        LipStickSpore = 0x25,
        /// `It_Kind_S_Scope_Beam` — Super Scope Beam
        SScopeBeam = 0x26,
        /// `It_Kind_L_Gun_Beam` — Ray Gun Beam
        LGunBeam = 0x27,
        /// `It_Kind_Hammer_Head` — Hammer Head
        HammerHead = 0x28,
        /// `It_Kind_F_Flower_Flame` — Flower
        FFlowerFlame = 0x29,
        /// `It_Kind_EvYoshiEgg` — Yoshi's Egg (Event)
        EvYoshiEgg = 0x2A,
        /// `It_Kind_Kuriboh` — Goomba (Kuriboh)
        Kuriboh = 0x2B,
        /// `It_Kind_Leadead` — Redead (Leadead)
        Leadead = 0x2C,
        /// `It_Kind_Octarock` — Octarok (Octarock)
        Octarock = 0x2D,
        /// `It_Kind_Ottosea` — Ottosea
        Ottosea = 0x2E,
        /// `It_Kind_Octarock_Stone` — Stone (Octarok Projectile)
        OctarockStone = 0x2F,
        /// `It_Kind_Mario_Fire` — Mario's fireball
        MarioFire = 0x30,
        /// `It_Kind_DrMario_Vitamin` — Dr. Mario's pill
        DrMarioVitamin = 0x31,
        /// `It_Kind_Kirby_CBeam` — Kirby's Cutter beam
        KirbyCBeam = 0x32,
        /// `It_Kind_Kirby_Hammer` — Kirby's Hammer
        KirbyHammer = 0x33,
        /// `It_Kind_Unk1` — Maybe Kirby copy star?
        Unk1 = 0x34,
        /// `It_Kind_Unk2`
        Unk2 = 0x35,
        /// `It_Kind_Fox_Laser` — Fox's Laser
        FoxLaser = 0x36,
        /// `It_Kind_Falco_Laser` — Falco's Laser
        FalcoLaser = 0x37,
        /// `It_Kind_Fox_Illusion` — Fox's Illusion
        FoxIllusion = 0x38,
        /// `It_Kind_Falco_Phantasm` — Falco's Phantasm
        FalcoPhantasm = 0x39,
        /// `It_Kind_Link_Bomb` — Link's bomb
        LinkBomb = 0x3A,
        /// `It_Kind_CLink_Bomb` — Young Link's bomb
        CLinkBomb = 0x3B,
        /// `It_Kind_Link_Boomerang` — Link's boomerang
        LinkBoomerang = 0x3C,
        /// `It_Kind_CLink_Boomerang` — Young Link's boomerang
        CLinkBoomerang = 0x3D,
        /// `It_Kind_Link_HShot` — Link's Hookshot
        LinkHShot = 0x3E,
        /// `It_Kind_CLink_HShot` — Young Link's Hookshot
        CLinkHShot = 0x3F,
        /// `It_Kind_Link_Arrow` — Link's Arrow
        LinkArrow = 0x40,
        /// `It_Kind_CLink_Arrow` — Young Link's Fire Arrow
        CLinkArrow = 0x41,
        /// `It_Kind_Ness_PKFire` — PK Fire
        NessPKFire = 0x42,
        /// `It_Kind_Ness_PKFire_Flame` — PK Fire Pillar
        NessPKFireFlame = 0x43,
        /// `It_Kind_Ness_PKFlush` — PK Flash (charging state)
        NessPKFlush = 0x44,
        /// `It_Kind_Ness_PKThunder` — PK Thunder (Ball)
        NessPKThunder = 0x45,
        /// `It_Kind_Ness_PKThunder1` — PK Thunder (Trail 1)
        NessPKThunder1 = 0x46,
        /// `It_Kind_Ness_PKThunder2` — PK Thunder (Trail 2)
        NessPKThunder2 = 0x47,
        /// `It_Kind_Ness_PKThunder3` — PK Thunder (Trail 3)
        NessPKThunder3 = 0x48,
        /// `It_Kind_Ness_PKThunder4` — PK Thunder (Trail 4)
        NessPKThunder4 = 0x49,
        /// `It_Kind_Fox_Blaster` — Fox's Blaster
        FoxBlaster = 0x4A,
        /// `It_Kind_Falco_Blaster` — Falco's Blaster
        FalcoBlaster = 0x4B,
        /// `It_Kind_Link_Bow` — Link's Bow
        LinkBow = 0x4C,
        /// `It_Kind_CLink_Bow` — Young Link's Bow
        CLinkBow = 0x4D,
        /// `It_Kind_Ness_PKFlush_Explode` — PK Flash (explosion)
        NessPKFlushExplode = 0x4E,
        /// `It_Kind_Seak_NeedleThrow` — Needle (thrown)
        SeakNeedleThrow = 0x4F,
        /// `It_Kind_Seak_NeedleHeld` — Needle (held)
        SeakNeedleHeld = 0x50,
        /// `It_Kind_Pikachu_Thunder` — Pikachu's Thunder
        PikachuThunder = 0x51,
        /// `It_Kind_Pichu_Thunder` — Pichu's Thunder
        PichuThunder = 0x52,
        /// `It_Kind_Mario_Cape` — Mario's cape
        MarioCape = 0x53,
        /// `It_Kind_DrMario_Sheet` — Dr. Mario's cape
        DrMarioSheet = 0x54,
        /// `It_Kind_Seak_Vanish` — Smoke (Sheik)
        SeakVanish = 0x55,
        /// `It_Kind_Yoshi_EggThrow` — Yoshi's Egg (thrown)
        YoshiEggThrow = 0x56,
        /// `It_Kind_Yoshi_EggLay` — Yoshi's Egg Lay???
        YoshiEggLay = 0x57,
        /// `It_Kind_Yoshi_Star` — Yoshi's Star
        YoshiStar = 0x58,
        /// `It_Kind_Pikachu_TJolt_Ground` — Pikachu's thunder (B)
        PikachuTJoltGround = 0x59,
        /// `It_Kind_Pikachu_TJolt_Air` — Pikachu's thunder (B)
        PikachuTJoltAir = 0x5A,
        /// `It_Kind_Pichu_TJolt_Ground` — Pichu's thunder (B)
        PichuTJoltGround = 0x5B,
        /// `It_Kind_Pichu_TJolt_Air` — Pichu's thunder (B)
        PichuTJoltAir = 0x5C,
        /// `It_Kind_Samus_Bomb` — Samus's bomb
        SamusBomb = 0x5D,
        /// `It_Kind_Samus_Charge` — Samus's chargeshot
        SamusCharge = 0x5E,
        /// `It_Kind_Samus_Missile` — Missile
        SamusMissile = 0x5F,
        /// `It_Kind_Samus_GBeam` — Grapple beam
        SamusGBeam = 0x60,
        /// `It_Kind_Seak_Chain` — Sheik's chain
        SeakChain = 0x61,
        /// `It_Kind_Peach_Explode` — Peach Bomber explosion?
        PeachExplode = 0x62,
        /// `It_Kind_Peach_Turnip` — Peach's turnip
        PeachTurnip = 0x63,
        /// `It_Kind_Koopa_Flame` — Bowser's flame
        KoopaFlame = 0x64,
        /// `It_Kind_Ness_Bat` — Ness's baseball bat
        NessBat = 0x65,
        /// `It_Kind_Ness_Yoyo` — Ness's Yo-Yo
        NessYoyo = 0x66,
        /// `It_Kind_Peach_Parasol` — Peach's parasol
        PeachParasol = 0x67,
        /// `It_Kind_Peach_Toad` — Peach's Toad special
        PeachToad = 0x68,
        /// `It_Kind_Luigi_Fire` — Luigi's fireball
        LuigiFire = 0x69,
        /// `It_Kind_IceClimber_Ice` — Ice (Ice Climbers)
        IceClimberIce = 0x6A,
        /// `It_Kind_IceClimber_Blizzard` — Blizzard
        IceClimberBlizzard = 0x6B,
        /// `It_Kind_Zelda_DinFire` — Din's Fire (charging state)
        ZeldaDinFire = 0x6C,
        /// `It_Kind_Zelda_DinFire_Explode` — Din's Fire (explosion)
        ZeldaDinFireExplode = 0x6D,
        /// `It_Kind_Mewtwo_Disable` — Mewtwo's Disable Projectile
        MewtwoDisable = 0x6E,
        /// `It_Kind_Peach_ToadSpore` — Peach Toad's spore effect
        PeachToadSpore = 0x6F,
        /// `It_Kind_Mewtwo_ShadowBall` — Mewtwo's Shadowball
        MewtwoShadowBall = 0x70,
        /// `It_Kind_IceClimber_GumStrings` — Ice Climbers Belay (Up B)
        IceClimberGumStrings = 0x71,
        /// `It_Kind_GameWatch_Greenhouse` — Mr. Game & Watch's Insecticide Spray
        GameWatchGreenhouse = 0x72,
        /// `It_Kind_GameWatch_Manhole` — Mr. Game & Watch's Manhole
        GameWatchManhole = 0x73,
        /// `It_Kind_GameWatch_Fire` — Mr. Game & Watch's Fire (?)
        GameWatchFire = 0x74,
        /// `It_Kind_GameWatch_Parachute` — Mr. Game & Watch's Parachute
        GameWatchParachute = 0x75,
        /// `It_Kind_GameWatch_Turtle` — Mr. Game & Watch's Turtle
        GameWatchTurtle = 0x76,
        /// `It_Kind_GameWatch_Breath` — Mr. Game & Watch's Sparky
        GameWatchBreath = 0x77,
        /// `It_Kind_GameWatch_Judge` — Mr. Game & Watch's Judge
        GameWatchJudge = 0x78,
        /// `It_Kind_GameWatch_Panic` — Mr. Game & Watch's Oil Panic (?)
        GameWatchPanic = 0x79,
        /// `It_Kind_GameWatch_Chef` — Sausage
        GameWatchChef = 0x7A,
        /// `It_Kind_CLink_Milk` — Milk (Young Link)
        CLinkMilk = 0x7B,
        /// `It_Kind_GameWatch_Rescue` — Mr. Game & Watch's Firefighter
        GameWatchRescue = 0x7C,
        /// `It_Kind_MasterHand_Laser` — Master Hand's Laser
        MasterHandLaser = 0x7D,
        /// `It_Kind_MasterHand_Bullet` — Master Hand's Bullet
        MasterHandBullet = 0x7E,
        /// `It_Kind_CrazyHand_Laser` — Crazy Hand's Laser
        CrazyHandLaser = 0x7F,
        /// `It_Kind_CrazyHand_Bullet` — Crazy Hand's Bullet
        CrazyHandBullet = 0x80,
        /// `It_Kind_CrazyHand_Bomb` — Crazy Hand's Bomb
        CrazyHandBomb = 0x81,
        /// `It_Kind_Kirby_MarioFire` — Kirby copy Mario's Fire (B)
        KirbyMarioFire = 0x82,
        /// `It_Kind_Kirby_DrMarioVitamin` — Kirby copy Dr. Mario's Capsule (B)
        KirbyDrMarioVitamin = 0x83,
        /// `It_Kind_Kirby_LuigiFire` — Kirby copy Luigi's Fire (B)
        KirbyLuigiFire = 0x84,
        /// `It_Kind_Kirby_IceClimberIce` — Kirby copy Ice Climbers' Ice Shot (B)
        KirbyIceClimberIce = 0x85,
        /// `It_Kind_Kirby_PeachToad` — Kirby copy Peach's Toad (B)
        KirbyPeachToad = 0x86,
        /// `It_Kind_Kirby_PeachToadSpore` — Kirby copy Toad's Spore (B)
        KirbyPeachToadSpore = 0x87,
        /// `It_Kind_Kirby_FoxLaser` — Kirby copy Fox's Laser (B)
        KirbyFoxLaser = 0x88,
        /// `It_Kind_Kirby_FalcoLaser` — Kirby copy Falco's Laser (B)
        KirbyFalcoLaser = 0x89,
        /// `It_Kind_Kirby_FoxBlaster` — Kirby copy Fox's Blaster (B)
        KirbyFoxBlaster = 0x8A,
        /// `It_Kind_Kirby_FalcoBlaster` — Kirby copy Falco's Blaster (B)
        KirbyFalcoBlaster = 0x8B,
        /// `It_Kind_Kirby_LinkArrow` — Kirby copy Link's Arrow (B)
        KirbyLinkArrow = 0x8C,
        /// `It_Kind_Kirby_CLinkArrow` — Kirby copy Young Link's Arrow (B)
        KirbyCLinkArrow = 0x8D,
        /// `It_Kind_Kirby_LinkBow` — Kirby copy Link's Arrow (B)
        KirbyLinkBow = 0x8E,
        /// `It_Kind_Kirby_CLinkBow` — Kirby copy Young Link's Arrow (B)
        KirbyCLinkBow = 0x8F,
        /// `It_Kind_Kirby_MewtwoShadowBall` — Kirby copy Mewtwo's Shadowball (B)
        KirbyMewtwoShadowBall = 0x90,
        /// `It_Kind_Kirby_NessPKFlush` — Kirby copy PK Flash (B)
        KirbyNessPKFlush = 0x91,
        /// `It_Kind_Kirby_NessPKFlush_Explode` — Kirby copy PK Flash Explosion (B)
        KirbyNessPKFlushExplode = 0x92,
        /// `It_Kind_Kirby_PikachuTJolt_Ground` — Kirby copy Pikachu's Thunder (B)
        KirbyPikachuTJoltGround = 0x93,
        /// `It_Kind_Kirby_PikachuTJolt_Air` — Kirby copy Pikachu's Thunder (B)
        KirbyPikachuTJoltAir = 0x94,
        /// `It_Kind_Kirby_PichuTJolt_Ground` — Kirby copy Pichu's Thunder (B)
        KirbyPichuTJoltGround = 0x95,
        /// `It_Kind_Kirby_PichuTJolt_Air` — Kirby copy Pichu's Thunder (B)
        KirbyPichuTJoltAir = 0x96,
        /// `It_Kind_Kirby_SamusCharge` — Kirby copy Samus' Chargeshot (B)
        KirbySamusCharge = 0x97,
        /// `It_Kind_Kirby_SeakNeedleThrow` — Kirby copy Sheik's Needle (thrown) (B)
        KirbySeakNeedleThrow = 0x98,
        /// `It_Kind_Kirby_SeakNeedleHeld` — Kirby copy Sheik's Needle (ground) (B)
        KirbySeakNeedleHeld = 0x99,
        /// `It_Kind_Kirby_KoopaFlame` — Kirby copy Bowser's Flame (B)
        KirbyKoopaFlame = 0x9A,
        /// `It_Kind_Kirby_GameWatchChef` — Kirby copy Mr. Game & Watch's Sausage (B)
        KirbyGameWatchChef = 0x9B,
        /// `It_Kind_Kirby_GameWatchChefPan` — Kirby copy Mr. Game & Watch's Chef Pan
        KirbyGameWatchChefPan = 0x9C,
        /// `It_Kind_Kirby_YoshiEggLay` — Kirby's Yoshi Egg Lay??? (B)
        KirbyYoshiEggLay = 0x9D,
        /// `It_Kind_Unk4` — (unique)
        Unk4 = 0x9E,
        /// `It_Kind_Coin` — Coin (?)
        Coin = 0x9F,
        /// `It_PKind_Random` — Used for Random Pokemon value
        PokemonRandom = 0xA0,
        /// `It_PKind_Tosakinto` — Goldeen (Tosakinto)
        Tosakinto = 0xA1,
        /// `It_PKind_Chicorita` — Chikorita (Chicorita)
        Chicorita = 0xA2,
        /// `It_PKind_Kabigon` — Snorlax (Kabigon)
        Kabigon = 0xA3,
        /// `It_PKind_Kamex` — Blastoise (Kamex)
        Kamex = 0xA4,
        /// `It_PKind_Matadogas` — Weezing (Matadogas)
        Matadogas = 0xA5,
        /// `It_PKind_Lizardon` — Charizard (Lizardon)
        Lizardon = 0xA6,
        /// `It_PKind_Fire` — Moltres (Fire)
        Fire = 0xA7,
        /// `It_PKind_Thunder` — Zapdos (Thunder)
        Thunder = 0xA8,
        /// `It_PKind_Freezer` — Articuno (Freezer)
        Freezer = 0xA9,
        /// `It_PKind_Sonans` — Wobbuffet (Sonans)
        Sonans = 0xAA,
        /// `It_PKind_Hassam` — Scizor (Hassam)
        Hassam = 0xAB,
        /// `It_PKind_Unknown` — Unown (Unknown)
        Unknown = 0xAC,
        /// `It_PKind_Entei` — Entei
        Entei = 0xAD,
        /// `It_PKind_Raikou` — Raikou
        Raikou = 0xAE,
        /// `It_PKind_Suikun` — Suicune (Suikun)
        Suikun = 0xAF,
        /// `It_PKind_Kireihana` — Bellossom (Kireihana)
        Kireihana = 0xB0,
        /// `It_PKind_Marumine` — Electrode (Marumine)
        Marumine = 0xB1,
        /// `It_PKind_Lugia` — Lugia
        Lugia = 0xB2,
        /// `It_PKind_Houou` — Ho-oh (Houou)
        Houou = 0xB3,
        /// `It_PKind_Metamon` — Ditto (Metamon)
        Metamon = 0xB4,
        /// `It_PKind_Pippi` — Clefairy (Pippi)
        Pippi = 0xB5,
        /// `It_PKind_Togepy` — Togepi (Togepy)
        Togepy = 0xB6,
        /// `It_PKind_Mew` — Mew
        Mew = 0xB7,
        /// `It_PKind_Cerebi` — Celebi (Cerebi)
        Cerebi = 0xB8,
        /// `It_PKind_Hitodeman` — Staryu (Hitodeman)
        Hitodeman = 0xB9,
        /// `It_PKind_Lucky` — Chansey (Lucky)
        Lucky = 0xBA,
        /// `It_PKind_Porygon2` — Porygon2
        Porygon2 = 0xBB,
        /// `It_PKind_Hinoarashi` — Cyndaquil (Hinoarashi)
        Hinoarashi = 0xBC,
        /// `It_PKind_Maril` — Marill (Maril)
        Maril = 0xBD,
        /// `It_PKind_Fushigibana` — Venusaur (Fushigibana)
        Fushigibana = 0xBE,
        /// `It_Kind_Chicorita_Leaf` — Chikorita's Leaf
        ChicoritaLeaf = 0xBF,
        /// `It_Kind_Kamex_HydroPump` — Blastoise's Water
        KamexHydroPump = 0xC0,
        /// `It_Kind_Matadogas_Gas1` — Weezing's Gas
        MatadogasGas1 = 0xC1,
        /// `It_Kind_Matadogas_Gas2` — Weezing's Gas
        MatadogasGas2 = 0xC2,
        /// `It_Kind_Lizardon_Flame1` — Charizard's Breath
        LizardonFlame1 = 0xC3,
        /// `It_Kind_Lizardon_Flame2` — Charizard's Breath
        LizardonFlame2 = 0xC4,
        /// `It_Kind_Lizardon_Flame3` — Charizard's Breath
        LizardonFlame3 = 0xC5,
        /// `It_Kind_Lizardon_Flame4` — Charizard's Breath
        LizardonFlame4 = 0xC6,
        /// `It_Kind_Unknown_Swarm` — Mini-Unowns
        UnknownSwarm = 0xC7,
        /// `It_Kind_Lugia_Aeroblast` — Lugia's Aeroblast
        LugiaAeroblast = 0xC8,
        /// `It_Kind_Lugia_Aeroblast2` — Lugia's Aeroblast
        LugiaAeroblast2 = 0xC9,
        /// `It_Kind_Lugia_Aeroblast3` — Lugia's Aeroblast
        LugiaAeroblast3 = 0xCA,
        /// `It_Kind_Houou_SacredFire` — Ho-Oh's Flame
        HououSacredFire = 0xCB,
        /// `It_Kind_Hitodeman_Star` — Staryu's Star
        HitodemanStar = 0xCC,
        /// `It_Kind_Lucky_Egg` — Chansey's Healing Egg
        LuckyEgg = 0xCD,
        /// `It_Kind_Hinoarashi_Flame` — Cyndaquil's Fire
        HinoarashiFlame = 0xCE,
        /// `It_Kind_Pokemon_Unk` — ???
        PokemonUnk = 0xCF,
        /// `It_Kind_Old_Kuri` — Old Goomba (old-Kuri)
        OldKuri = 0xD0,
        /// `It_Kind_Mato` — Target (Mato)
        Mato = 0xD1,
        /// `It_Kind_Heiho` — Yoshi's Story Shy Guy (Heiho)
        Heiho = 0xD2,
        /// `It_Kind_Nokonoko` — Koopa Troopa (Green) (Nokonoko)
        Nokonoko = 0xD3,
        /// `It_Kind_Patapata` — Koopa Troopa (Red) (Patapata)
        Patapata = 0xD4,
        /// `It_Kind_Likelike` — Like-Like (likelike)
        Likelike = 0xD5,
        /// `It_Kind_Old_Lead` — Old Redead (old-lead) [invalid]
        OldLead = 0xD6,
        /// `It_Kind_Old_Octa` — Old Octorok (old-octa) [invalid]
        OldOcta = 0xD7,
        /// `It_Kind_Old_Otto` — Old Ottosea (old-otto)
        OldOtto = 0xD8,
        /// `It_Kind_Whitebea` — Polar Bear (whitebea)
        Whitebea = 0xD9,
        /// `It_Kind_Klap` — Klaptrap (klap)
        Klap = 0xDA,
        /// `It_Kind_ZGShell` — Green Shell (zgshell)
        ZGShell = 0xDB,
        /// `It_Kind_ZRShell` — Red Shell (green act) (zrshell)
        ZRShell = 0xDC,
        /// `It_Kind_Tincle` — Tingle (Tincle) (on balloon)
        Tincle = 0xDD,
        /// `It_Kind_Invalid1` — [Invalid]
        Invalid1 = 0xDE,
        /// `It_Kind_Invalid2` — [Invalid]
        Invalid2 = 0xDF,
        /// `It_Kind_Invalid3` — [Invalid]
        Invalid3 = 0xE0,
        /// `It_Kind_WhispyApple` — Whispy Apple
        WhispyApple = 0xE1,
        /// `It_Kind_WhispyHealApple` — Whispy's Healing Apple
        WhispyHealApple = 0xE2,
        /// `It_Kind_Invalid4` — [Invalid]
        Invalid4 = 0xE3,
        /// `It_Kind_Invalid5` — [Invalid]
        Invalid5 = 0xE4,
        /// `It_Kind_Invalid6` — [Invalid]
        Invalid6 = 0xE5,
        /// `It_Kind_Tools` — Tool (Flatzone)
        Tools = 0xE6,
        /// `It_Kind_Invalid7` — [Invalid]
        Invalid7 = 0xE7,
        /// `It_Kind_Invalid8` — [Invalid]
        Invalid8 = 0xE8,
        /// `It_Kind_Kyasarin` — Birdo (Kyasarin)
        Kyasarin = 0xE9,
        /// `It_Kind_Arwing_Laser` — Arwing Laser
        ArwingLaser = 0xEA,
        /// `It_Kind_GreatFox_Laser` — Great Fox's Laser
        GreatFoxLaser = 0xEB,
        /// `It_Kind_Kyasarin_Egg` — Birdo's Egg
        KyasarinEgg = 0xEC,
        /// `It_Kind_None`
        None = -999,
    }
}

impl ItemKind {
    /// `It_PKind_Start` — first Poke Ball Pokemon; alias of `It_PKind_Tosakinto`
    pub const POKEMON_START: i32 = 0xA1;

    /// `It_PKind_Terminate` — one past the last Poke Ball Pokemon; alias of `It_Kind_Chicorita_Leaf`
    pub const POKEMON_TERMINATE: i32 = 0xBF;
}
