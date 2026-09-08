//! Generated from `src/melee/ft/kinds/ftCommon/forward.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// Motion state ids shared by every fighter (`ftCommon_MotionState`, the `ftCo_MS_*` ids).
    ///
    /// Character-specific states start at [`CommonMotionState::COUNT`].
    ///
    /// Source: `src/melee/ft/kinds/ftCommon/forward.h`.
    pub enum CommonMotionState: i32 {
        /// `ftCo_MS_None`
        None = -1,
        /// `ftCo_MS_DeadDown`
        DeadDown = 0,
        /// `ftCo_MS_DeadLeft`
        DeadLeft = 1,
        /// `ftCo_MS_DeadRight`
        DeadRight = 2,
        /// `ftCo_MS_DeadUp`
        DeadUp = 3,
        /// `ftCo_MS_DeadUpStar`
        DeadUpStar = 4,
        /// `ftCo_MS_DeadUpStarIce`
        DeadUpStarIce = 5,
        /// `ftCo_MS_DeadUpFall`
        DeadUpFall = 6,
        /// `ftCo_MS_DeadUpFallHitCamera`
        DeadUpFallHitCamera = 7,
        /// `ftCo_MS_DeadUpFallHitCameraFlat`
        DeadUpFallHitCameraFlat = 8,
        /// `ftCo_MS_DeadUpFallIce`
        DeadUpFallIce = 9,
        /// `ftCo_MS_DeadUpFallHitCameraIce`
        DeadUpFallHitCameraIce = 10,
        /// `ftCo_MS_Sleep`
        Sleep = 11,
        /// `ftCo_MS_Rebirth`
        Rebirth = 12,
        /// `ftCo_MS_RebirthWait`
        RebirthWait = 13,
        /// `ftCo_MS_Wait`
        Wait = 14,
        /// `ftCo_MS_WalkSlow`
        WalkSlow = 15,
        /// `ftCo_MS_WalkMiddle`
        WalkMiddle = 16,
        /// `ftCo_MS_WalkFast`
        WalkFast = 17,
        /// `ftCo_MS_Turn`
        Turn = 18,
        /// `ftCo_MS_TurnRun`
        TurnRun = 19,
        /// `ftCo_MS_Dash`
        Dash = 20,
        /// `ftCo_MS_Run`
        Run = 21,
        /// `ftCo_MS_RunDirect`
        RunDirect = 22,
        /// `ftCo_MS_RunBrake`
        RunBrake = 23,
        /// `ftCo_MS_KneeBend`
        KneeBend = 24,
        /// `ftCo_MS_JumpF`
        JumpF = 25,
        /// `ftCo_MS_JumpB`
        JumpB = 26,
        /// `ftCo_MS_JumpAerialF`
        JumpAerialF = 27,
        /// `ftCo_MS_JumpAerialB`
        JumpAerialB = 28,
        /// `ftCo_MS_Fall`
        Fall = 29,
        /// `ftCo_MS_FallF`
        FallF = 30,
        /// `ftCo_MS_FallB`
        FallB = 31,
        /// `ftCo_MS_FallAerial`
        FallAerial = 32,
        /// `ftCo_MS_FallAerialF`
        FallAerialF = 33,
        /// `ftCo_MS_FallAerialB`
        FallAerialB = 34,
        /// `ftCo_MS_FallSpecial`
        FallSpecial = 35,
        /// `ftCo_MS_FallSpecialF`
        FallSpecialF = 36,
        /// `ftCo_MS_FallSpecialB`
        FallSpecialB = 37,
        /// `ftCo_MS_DamageFall`
        DamageFall = 38,
        /// `ftCo_MS_Squat`
        Squat = 39,
        /// `ftCo_MS_SquatWait`
        SquatWait = 40,
        /// `ftCo_MS_SquatRv`
        SquatRv = 41,
        /// `ftCo_MS_Landing`
        Landing = 42,
        /// `ftCo_MS_LandingFallSpecial`
        LandingFallSpecial = 43,
        /// `ftCo_MS_Attack11`
        Attack11 = 44,
        /// `ftCo_MS_Attack12`
        Attack12 = 45,
        /// `ftCo_MS_Attack13`
        Attack13 = 46,
        /// `ftCo_MS_Attack100Start`
        Attack100Start = 47,
        /// `ftCo_MS_Attack100Loop`
        Attack100Loop = 48,
        /// `ftCo_MS_Attack100End`
        Attack100End = 49,
        /// `ftCo_MS_AttackDash`
        AttackDash = 50,
        /// `ftCo_MS_AttackS3Hi`
        AttackS3Hi = 51,
        /// `ftCo_MS_AttackS3HiS`
        AttackS3HiS = 52,
        /// `ftCo_MS_AttackS3S`
        AttackS3S = 53,
        /// `ftCo_MS_AttackS3LwS`
        AttackS3LwS = 54,
        /// `ftCo_MS_AttackS3Lw`
        AttackS3Lw = 55,
        /// `ftCo_MS_AttackHi3`
        AttackHi3 = 56,
        /// `ftCo_MS_AttackLw3`
        AttackLw3 = 57,
        /// `ftCo_MS_AttackS4Hi`
        AttackS4Hi = 58,
        /// `ftCo_MS_AttackS4HiS`
        AttackS4HiS = 59,
        /// `ftCo_MS_AttackS4S`
        AttackS4S = 60,
        /// `ftCo_MS_AttackS4LwS`
        AttackS4LwS = 61,
        /// `ftCo_MS_AttackS4Lw`
        AttackS4Lw = 62,
        /// `ftCo_MS_AttackHi4`
        AttackHi4 = 63,
        /// `ftCo_MS_AttackLw4`
        AttackLw4 = 64,
        /// `ftCo_MS_AttackAirN`
        AttackAirN = 65,
        /// `ftCo_MS_AttackAirF`
        AttackAirF = 66,
        /// `ftCo_MS_AttackAirB`
        AttackAirB = 67,
        /// `ftCo_MS_AttackAirHi`
        AttackAirHi = 68,
        /// `ftCo_MS_AttackAirLw`
        AttackAirLw = 69,
        /// `ftCo_MS_LandingAirN`
        LandingAirN = 70,
        /// `ftCo_MS_LandingAirF`
        LandingAirF = 71,
        /// `ftCo_MS_LandingAirB`
        LandingAirB = 72,
        /// `ftCo_MS_LandingAirHi`
        LandingAirHi = 73,
        /// `ftCo_MS_LandingAirLw`
        LandingAirLw = 74,
        /// `ftCo_MS_DamageHi1`
        DamageHi1 = 75,
        /// `ftCo_MS_DamageHi2`
        DamageHi2 = 76,
        /// `ftCo_MS_DamageHi3`
        DamageHi3 = 77,
        /// `ftCo_MS_DamageN1`
        DamageN1 = 78,
        /// `ftCo_MS_DamageN2`
        DamageN2 = 79,
        /// `ftCo_MS_DamageN3`
        DamageN3 = 80,
        /// `ftCo_MS_DamageLw1`
        DamageLw1 = 81,
        /// `ftCo_MS_DamageLw2`
        DamageLw2 = 82,
        /// `ftCo_MS_DamageLw3`
        DamageLw3 = 83,
        /// `ftCo_MS_DamageAir1`
        DamageAir1 = 84,
        /// `ftCo_MS_DamageAir2`
        DamageAir2 = 85,
        /// `ftCo_MS_DamageAir3`
        DamageAir3 = 86,
        /// `ftCo_MS_DamageFlyHi`
        DamageFlyHi = 87,
        /// `ftCo_MS_DamageFlyN`
        DamageFlyN = 88,
        /// `ftCo_MS_DamageFlyLw`
        DamageFlyLw = 89,
        /// `ftCo_MS_DamageFlyTop`
        DamageFlyTop = 90,
        /// `ftCo_MS_DamageFlyRoll`
        DamageFlyRoll = 91,
        /// `ftCo_MS_LightGet`
        LightGet = 92,
        /// `ftCo_MS_HeavyGet`
        HeavyGet = 93,
        /// `ftCo_MS_LightThrowF`
        LightThrowF = 94,
        /// `ftCo_MS_LightThrowB`
        LightThrowB = 95,
        /// `ftCo_MS_LightThrowHi`
        LightThrowHi = 96,
        /// `ftCo_MS_LightThrowLw`
        LightThrowLw = 97,
        /// `ftCo_MS_LightThrowDash`
        LightThrowDash = 98,
        /// `ftCo_MS_LightThrowDrop`
        LightThrowDrop = 99,
        /// `ftCo_MS_LightThrowAirF`
        LightThrowAirF = 100,
        /// `ftCo_MS_LightThrowAirB`
        LightThrowAirB = 101,
        /// `ftCo_MS_LightThrowAirHi`
        LightThrowAirHi = 102,
        /// `ftCo_MS_LightThrowAirLw`
        LightThrowAirLw = 103,
        /// `ftCo_MS_HeavyThrowF`
        HeavyThrowF = 104,
        /// `ftCo_MS_HeavyThrowB`
        HeavyThrowB = 105,
        /// `ftCo_MS_HeavyThrowHi`
        HeavyThrowHi = 106,
        /// `ftCo_MS_HeavyThrowLw`
        HeavyThrowLw = 107,
        /// `ftCo_MS_LightThrowF4`
        LightThrowF4 = 108,
        /// `ftCo_MS_LightThrowB4`
        LightThrowB4 = 109,
        /// `ftCo_MS_LightThrowHi4`
        LightThrowHi4 = 110,
        /// `ftCo_MS_LightThrowLw4`
        LightThrowLw4 = 111,
        /// `ftCo_MS_LightThrowAirF4`
        LightThrowAirF4 = 112,
        /// `ftCo_MS_LightThrowAirB4`
        LightThrowAirB4 = 113,
        /// `ftCo_MS_LightThrowAirHi4`
        LightThrowAirHi4 = 114,
        /// `ftCo_MS_LightThrowAirLw4`
        LightThrowAirLw4 = 115,
        /// `ftCo_MS_HeavyThrowF4`
        HeavyThrowF4 = 116,
        /// `ftCo_MS_HeavyThrowB4`
        HeavyThrowB4 = 117,
        /// `ftCo_MS_HeavyThrowHi4`
        HeavyThrowHi4 = 118,
        /// `ftCo_MS_HeavyThrowLw4`
        HeavyThrowLw4 = 119,
        /// `ftCo_MS_SwordSwing1`
        SwordSwing1 = 120,
        /// `ftCo_MS_SwordSwing3`
        SwordSwing3 = 121,
        /// `ftCo_MS_SwordSwing4`
        SwordSwing4 = 122,
        /// `ftCo_MS_SwordSwingDash`
        SwordSwingDash = 123,
        /// `ftCo_MS_BatSwing1`
        BatSwing1 = 124,
        /// `ftCo_MS_BatSwing3`
        BatSwing3 = 125,
        /// `ftCo_MS_BatSwing4`
        BatSwing4 = 126,
        /// `ftCo_MS_BatSwingDash`
        BatSwingDash = 127,
        /// `ftCo_MS_ParasolSwing1`
        ParasolSwing1 = 128,
        /// `ftCo_MS_ParasolSwing3`
        ParasolSwing3 = 129,
        /// `ftCo_MS_ParasolSwing4`
        ParasolSwing4 = 130,
        /// `ftCo_MS_ParasolSwingDash`
        ParasolSwingDash = 131,
        /// `ftCo_MS_HarisenSwing1`
        HarisenSwing1 = 132,
        /// `ftCo_MS_HarisenSwing3`
        HarisenSwing3 = 133,
        /// `ftCo_MS_HarisenSwing4`
        HarisenSwing4 = 134,
        /// `ftCo_MS_HarisenSwingDash`
        HarisenSwingDash = 135,
        /// `ftCo_MS_StarRodSwing1`
        StarRodSwing1 = 136,
        /// `ftCo_MS_StarRodSwing3`
        StarRodSwing3 = 137,
        /// `ftCo_MS_StarRodSwing4`
        StarRodSwing4 = 138,
        /// `ftCo_MS_StarRodSwingDash`
        StarRodSwingDash = 139,
        /// `ftCo_MS_LipstickSwing1`
        LipstickSwing1 = 140,
        /// `ftCo_MS_LipstickSwing3`
        LipstickSwing3 = 141,
        /// `ftCo_MS_LipstickSwing4`
        LipstickSwing4 = 142,
        /// `ftCo_MS_LipstickSwingDash`
        LipstickSwingDash = 143,
        /// `ftCo_MS_ItemParasolOpen`
        ItemParasolOpen = 144,
        /// `ftCo_MS_ItemParasolFall`
        ItemParasolFall = 145,
        /// `ftCo_MS_ItemParasolFallSpecial`
        ItemParasolFallSpecial = 146,
        /// `ftCo_MS_ItemParasolDamageFall`
        ItemParasolDamageFall = 147,
        /// `ftCo_MS_LGunShoot`
        LGunShoot = 148,
        /// `ftCo_MS_LGunShootAir`
        LGunShootAir = 149,
        /// `ftCo_MS_LGunShootEmpty`
        LGunShootEmpty = 150,
        /// `ftCo_MS_LGunShootAirEmpty`
        LGunShootAirEmpty = 151,
        /// `ftCo_MS_FireFlowerShoot`
        FireFlowerShoot = 152,
        /// `ftCo_MS_FireFlowerShootAir`
        FireFlowerShootAir = 153,
        /// `ftCo_MS_ItemScrew`
        ItemScrew = 154,
        /// `ftCo_MS_ItemScrewAir`
        ItemScrewAir = 155,
        /// `ftCo_MS_DamageScrew`
        DamageScrew = 156,
        /// `ftCo_MS_DamageScrewAir`
        DamageScrewAir = 157,
        /// `ftCo_MS_ItemScopeStart`
        ItemScopeStart = 158,
        /// `ftCo_MS_ItemScopeRapid`
        ItemScopeRapid = 159,
        /// `ftCo_MS_ItemScopeFire`
        ItemScopeFire = 160,
        /// `ftCo_MS_ItemScopeEnd`
        ItemScopeEnd = 161,
        /// `ftCo_MS_ItemScopeAirStart`
        ItemScopeAirStart = 162,
        /// `ftCo_MS_ItemScopeAirRapid`
        ItemScopeAirRapid = 163,
        /// `ftCo_MS_ItemScopeAirFire`
        ItemScopeAirFire = 164,
        /// `ftCo_MS_ItemScopeAirEnd`
        ItemScopeAirEnd = 165,
        /// `ftCo_MS_ItemScopeStartEmpty`
        ItemScopeStartEmpty = 166,
        /// `ftCo_MS_ItemScopeRapidEmpty`
        ItemScopeRapidEmpty = 167,
        /// `ftCo_MS_ItemScopeFireEmpty`
        ItemScopeFireEmpty = 168,
        /// `ftCo_MS_ItemScopeEndEmpty`
        ItemScopeEndEmpty = 169,
        /// `ftCo_MS_ItemScopeAirStartEmpty`
        ItemScopeAirStartEmpty = 170,
        /// `ftCo_MS_ItemScopeAirRapidEmpty`
        ItemScopeAirRapidEmpty = 171,
        /// `ftCo_MS_ItemScopeAirFireEmpty`
        ItemScopeAirFireEmpty = 172,
        /// `ftCo_MS_ItemScopeAirEndEmpty`
        ItemScopeAirEndEmpty = 173,
        /// `ftCo_MS_LiftWait`
        LiftWait = 174,
        /// `ftCo_MS_LiftWalk1`
        LiftWalk1 = 175,
        /// `ftCo_MS_LiftWalk2`
        LiftWalk2 = 176,
        /// `ftCo_MS_LiftTurn`
        LiftTurn = 177,
        /// `ftCo_MS_GuardOn`
        GuardOn = 178,
        /// `ftCo_MS_Guard`
        Guard = 179,
        /// `ftCo_MS_GuardOff`
        GuardOff = 180,
        /// `ftCo_MS_GuardSetOff`
        GuardSetOff = 181,
        /// `ftCo_MS_GuardReflect`
        GuardReflect = 182,
        /// `ftCo_MS_DownBoundU`
        DownBoundU = 183,
        /// `ftCo_MS_DownWaitU`
        DownWaitU = 184,
        /// `ftCo_MS_DownDamageU`
        DownDamageU = 185,
        /// `ftCo_MS_DownStandU`
        DownStandU = 186,
        /// `ftCo_MS_DownAttackU`
        DownAttackU = 187,
        /// `ftCo_MS_DownFowardU`
        DownFowardU = 188,
        /// `ftCo_MS_DownBackU`
        DownBackU = 189,
        /// `ftCo_MS_DownSpotU`
        DownSpotU = 190,
        /// `ftCo_MS_DownBoundD`
        DownBoundD = 191,
        /// `ftCo_MS_DownWaitD`
        DownWaitD = 192,
        /// `ftCo_MS_DownDamageD`
        DownDamageD = 193,
        /// `ftCo_MS_DownStandD`
        DownStandD = 194,
        /// `ftCo_MS_DownAttackD`
        DownAttackD = 195,
        /// `ftCo_MS_DownFowardD`
        DownFowardD = 196,
        /// `ftCo_MS_DownBackD`
        DownBackD = 197,
        /// `ftCo_MS_DownSpotD`
        DownSpotD = 198,
        /// `ftCo_MS_Passive`
        Passive = 199,
        /// `ftCo_MS_PassiveStandF`
        PassiveStandF = 200,
        /// `ftCo_MS_PassiveStandB`
        PassiveStandB = 201,
        /// `ftCo_MS_PassiveWall`
        PassiveWall = 202,
        /// `ftCo_MS_PassiveWallJump`
        PassiveWallJump = 203,
        /// `ftCo_MS_PassiveCeil`
        PassiveCeil = 204,
        /// `ftCo_MS_ShieldBreakFly`
        ShieldBreakFly = 205,
        /// `ftCo_MS_ShieldBreakFall`
        ShieldBreakFall = 206,
        /// `ftCo_MS_ShieldBreakDownU`
        ShieldBreakDownU = 207,
        /// `ftCo_MS_ShieldBreakDownD`
        ShieldBreakDownD = 208,
        /// `ftCo_MS_ShieldBreakStandU`
        ShieldBreakStandU = 209,
        /// `ftCo_MS_ShieldBreakStandD`
        ShieldBreakStandD = 210,
        /// `ftCo_MS_Furafura` — Dazed
        Furafura = 211,
        /// `ftCo_MS_Catch`
        Catch = 212,
        /// `ftCo_MS_CatchPull`
        CatchPull = 213,
        /// `ftCo_MS_CatchDash`
        CatchDash = 214,
        /// `ftCo_MS_CatchDashPull`
        CatchDashPull = 215,
        /// `ftCo_MS_CatchWait`
        CatchWait = 216,
        /// `ftCo_MS_CatchAttack`
        CatchAttack = 217,
        /// `ftCo_MS_CatchCut`
        CatchCut = 218,
        /// `ftCo_MS_ThrowF`
        ThrowF = 219,
        /// `ftCo_MS_ThrowB`
        ThrowB = 220,
        /// `ftCo_MS_ThrowHi`
        ThrowHi = 221,
        /// `ftCo_MS_ThrowLw`
        ThrowLw = 222,
        /// `ftCo_MS_CapturePulledHi`
        CapturePulledHi = 223,
        /// `ftCo_MS_CaptureWaitHi`
        CaptureWaitHi = 224,
        /// `ftCo_MS_CaptureDamageHi`
        CaptureDamageHi = 225,
        /// `ftCo_MS_CapturePulledLw`
        CapturePulledLw = 226,
        /// `ftCo_MS_CaptureWaitLw`
        CaptureWaitLw = 227,
        /// `ftCo_MS_CaptureDamageLw`
        CaptureDamageLw = 228,
        /// `ftCo_MS_CaptureCut`
        CaptureCut = 229,
        /// `ftCo_MS_CaptureJump`
        CaptureJump = 230,
        /// `ftCo_MS_CaptureNeck`
        CaptureNeck = 231,
        /// `ftCo_MS_CaptureFoot`
        CaptureFoot = 232,
        /// `ftCo_MS_EscapeF`
        EscapeF = 233,
        /// `ftCo_MS_EscapeB`
        EscapeB = 234,
        /// `ftCo_MS_EscapeN`
        EscapeN = 235,
        /// `ftCo_MS_EscapeAir`
        EscapeAir = 236,
        /// `ftCo_MS_ReboundStop`
        ReboundStop = 237,
        /// `ftCo_MS_Rebound`
        Rebound = 238,
        /// `ftCo_MS_ThrownF`
        ThrownF = 239,
        /// `ftCo_MS_ThrownB`
        ThrownB = 240,
        /// `ftCo_MS_ThrownHi`
        ThrownHi = 241,
        /// `ftCo_MS_ThrownLw`
        ThrownLw = 242,
        /// `ftCo_MS_ThrownlwWomen`
        ThrownlwWomen = 243,
        /// `ftCo_MS_Pass`
        Pass = 244,
        /// `ftCo_MS_Ottotto` — teeter
        Ottotto = 245,
        /// `ftCo_MS_OttottoWait` — teeter wait
        OttottoWait = 246,
        /// `ftCo_MS_FlyReflectWall`
        FlyReflectWall = 247,
        /// `ftCo_MS_FlyReflectCeil`
        FlyReflectCeil = 248,
        /// `ftCo_MS_StopWall`
        StopWall = 249,
        /// `ftCo_MS_StopCeil`
        StopCeil = 250,
        /// `ftCo_MS_MissFoot`
        MissFoot = 251,
        /// `ftCo_MS_CliffCatch`
        CliffCatch = 252,
        /// `ftCo_MS_CliffWait`
        CliffWait = 253,
        /// `ftCo_MS_CliffClimbSlow`
        CliffClimbSlow = 254,
        /// `ftCo_MS_CliffClimbQuick`
        CliffClimbQuick = 255,
        /// `ftCo_MS_CliffAttackSlow`
        CliffAttackSlow = 256,
        /// `ftCo_MS_CliffAttackQuick`
        CliffAttackQuick = 257,
        /// `ftCo_MS_CliffEscapeSlow`
        CliffEscapeSlow = 258,
        /// `ftCo_MS_CliffEscapeQuick`
        CliffEscapeQuick = 259,
        /// `ftCo_MS_CliffJumpSlow1`
        CliffJumpSlow1 = 260,
        /// `ftCo_MS_CliffJumpSlow2`
        CliffJumpSlow2 = 261,
        /// `ftCo_MS_CliffJumpQuick1`
        CliffJumpQuick1 = 262,
        /// `ftCo_MS_CliffJumpQuick2`
        CliffJumpQuick2 = 263,
        /// `ftCo_MS_AppealSR`
        AppealSR = 264,
        /// `ftCo_MS_AppealSL`
        AppealSL = 265,
        /// `ftCo_MS_ShoulderedWait`
        ShoulderedWait = 266,
        /// `ftCo_MS_ShoulderedWalkSlow`
        ShoulderedWalkSlow = 267,
        /// `ftCo_MS_ShoulderedWalkMiddle`
        ShoulderedWalkMiddle = 268,
        /// `ftCo_MS_ShoulderedWalkFast`
        ShoulderedWalkFast = 269,
        /// `ftCo_MS_ShoulderedTurn`
        ShoulderedTurn = 270,
        /// `ftCo_MS_ThrownFF`
        ThrownFF = 271,
        /// `ftCo_MS_ThrownFB`
        ThrownFB = 272,
        /// `ftCo_MS_ThrownFHi`
        ThrownFHi = 273,
        /// `ftCo_MS_ThrownFLw`
        ThrownFLw = 274,
        /// `ftCo_MS_CaptureCaptain`
        CaptureCaptain = 275,
        /// `ftCo_MS_CaptureYoshi`
        CaptureYoshi = 276,
        /// `ftCo_MS_YoshiEgg`
        YoshiEgg = 277,
        /// `ftCo_MS_CaptureKoopa`
        CaptureKoopa = 278,
        /// `ftCo_MS_CaptureDamageKoopa`
        CaptureDamageKoopa = 279,
        /// `ftCo_MS_CaptureWaitKoopa`
        CaptureWaitKoopa = 280,
        /// `ftCo_MS_ThrownKoopaF`
        ThrownKoopaF = 281,
        /// `ftCo_MS_ThrownKoopaB`
        ThrownKoopaB = 282,
        /// `ftCo_MS_CaptureKoopaAir`
        CaptureKoopaAir = 283,
        /// `ftCo_MS_CaptureDamageKoopaAir`
        CaptureDamageKoopaAir = 284,
        /// `ftCo_MS_CaptureWaitKoopaAir`
        CaptureWaitKoopaAir = 285,
        /// `ftCo_MS_ThrownKoopaAirF`
        ThrownKoopaAirF = 286,
        /// `ftCo_MS_ThrownKoopaAirB`
        ThrownKoopaAirB = 287,
        /// `ftCo_MS_CaptureKirby`
        CaptureKirby = 288,
        /// `ftCo_MS_CaptureWaitKirby`
        CaptureWaitKirby = 289,
        /// `ftCo_MS_ThrownKirbyStar`
        ThrownKirbyStar = 290,
        /// `ftCo_MS_ThrownCopyStar`
        ThrownCopyStar = 291,
        /// `ftCo_MS_ThrownKirby`
        ThrownKirby = 292,
        /// `ftCo_MS_BarrelWait`
        BarrelWait = 293,
        /// `ftCo_MS_Bury`
        Bury = 294,
        /// `ftCo_MS_BuryWait`
        BuryWait = 295,
        /// `ftCo_MS_BuryJump`
        BuryJump = 296,
        /// `ftCo_MS_DamageSong`
        DamageSong = 297,
        /// `ftCo_MS_DamageSongWait`
        DamageSongWait = 298,
        /// `ftCo_MS_DamageSongRv`
        DamageSongRv = 299,
        /// `ftCo_MS_DamageBind`
        DamageBind = 300,
        /// `ftCo_MS_CaptureMewtwo`
        CaptureMewtwo = 301,
        /// `ftCo_MS_CaptureMewtwoAir`
        CaptureMewtwoAir = 302,
        /// `ftCo_MS_ThrownMewtwo`
        ThrownMewtwo = 303,
        /// `ftCo_MS_ThrownMewtwoAir`
        ThrownMewtwoAir = 304,
        /// `ftCo_MS_WarpStarJump`
        WarpStarJump = 305,
        /// `ftCo_MS_WarpStarFall`
        WarpStarFall = 306,
        /// `ftCo_MS_HammerWait`
        HammerWait = 307,
        /// `ftCo_MS_HammerWalk`
        HammerWalk = 308,
        /// `ftCo_MS_HammerTurn`
        HammerTurn = 309,
        /// `ftCo_MS_HammerKneeBend`
        HammerKneeBend = 310,
        /// `ftCo_MS_HammerFall`
        HammerFall = 311,
        /// `ftCo_MS_HammerJump`
        HammerJump = 312,
        /// `ftCo_MS_HammerLanding`
        HammerLanding = 313,
        /// `ftCo_MS_KinokoGiantStart`
        KinokoGiantStart = 314,
        /// `ftCo_MS_KinokoGiantStartAir`
        KinokoGiantStartAir = 315,
        /// `ftCo_MS_KinokoGiantEnd`
        KinokoGiantEnd = 316,
        /// `ftCo_MS_KinokoGiantEndAir`
        KinokoGiantEndAir = 317,
        /// `ftCo_MS_KinokoSmallStart`
        KinokoSmallStart = 318,
        /// `ftCo_MS_KinokoSmallStartAir`
        KinokoSmallStartAir = 319,
        /// `ftCo_MS_KinokoSmallEnd`
        KinokoSmallEnd = 320,
        /// `ftCo_MS_KinokoSmallEndAir`
        KinokoSmallEndAir = 321,
        /// `ftCo_MS_Entry`
        Entry = 322,
        /// `ftCo_MS_EntryStart`
        EntryStart = 323,
        /// `ftCo_MS_EntryEnd`
        EntryEnd = 324,
        /// `ftCo_MS_DamageIce`
        DamageIce = 325,
        /// `ftCo_MS_DamageIceJump`
        DamageIceJump = 326,
        /// `ftCo_MS_CaptureMasterHand`
        CaptureMasterHand = 327,
        /// `ftCo_MS_CaptureDamageMasterHand`
        CaptureDamageMasterHand = 328,
        /// `ftCo_MS_CaptureWaitMasterHand`
        CaptureWaitMasterHand = 329,
        /// `ftCo_MS_ThrownMasterHand`
        ThrownMasterHand = 330,
        /// `ftCo_MS_CaptureKirbyYoshi`
        CaptureKirbyYoshi = 331,
        /// `ftCo_MS_KirbyYoshiEgg`
        KirbyYoshiEgg = 332,
        /// `ftCo_MS_CaptureLeadead`
        CaptureLeadead = 333,
        /// `ftCo_MS_CaptureLikelike`
        CaptureLikelike = 334,
        /// `ftCo_MS_DownReflect`
        DownReflect = 335,
        /// `ftCo_MS_CaptureCrazyHand`
        CaptureCrazyHand = 336,
        /// `ftCo_MS_CaptureDamageCrazyHand`
        CaptureDamageCrazyHand = 337,
        /// `ftCo_MS_CaptureWaitCrazyHand`
        CaptureWaitCrazyHand = 338,
        /// `ftCo_MS_ThrownCrazyHand`
        ThrownCrazyHand = 339,
        /// `ftCo_MS_Barrel`
        Barrel = 340,
    }
}

impl CommonMotionState {
    /// `ftCo_MS_Count` — number of common motion states; one past `ftCo_MS_Barrel`
    pub const COUNT: i32 = 341;
}
