//! Generated from `src/melee/ft/ftcmdscript.h` by
//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff
//! rather than editing by hand.

c_enum! {
    /// CPU command-script opcodes (`CPUCommand`, the `CpuCmd_*` ids).
    ///
    /// Opcodes `<= ZERO_ARG_END` take no argument, `(ZERO_ARG_END, ONE_ARG_END]`
    /// take one, and the rest take more. Scripts store opcodes as bytes;
    /// see the `u8` conversions below.
    ///
    /// Source: `src/melee/ft/ftcmdscript.h`.
    pub enum CpuCmd: i32 {
        /// `CpuCmd_PressA`
        PressA = 0x01,
        /// `CpuCmd_ReleaseA`
        ReleaseA = 0x02,
        /// `CpuCmd_PressB`
        PressB = 0x03,
        /// `CpuCmd_ReleaseB`
        ReleaseB = 0x04,
        /// `CpuCmd_PressX`
        PressX = 0x05,
        /// `CpuCmd_ReleaseX`
        ReleaseX = 0x06,
        /// `CpuCmd_PressY`
        PressY = 0x07,
        /// `CpuCmd_ReleaseY`
        ReleaseY = 0x08,
        /// `CpuCmd_PressR`
        PressR = 0x09,
        /// `CpuCmd_ReleaseR`
        ReleaseR = 0x0A,
        /// `CpuCmd_PressL`
        PressL = 0x0B,
        /// `CpuCmd_ReleaseL`
        ReleaseL = 0x0C,
        /// `CpuCmd_PressZ`
        PressZ = 0x0D,
        /// `CpuCmd_ReleaseZ`
        ReleaseZ = 0x0E,
        /// `CpuCmd_PressUp`
        PressUp = 0x0F,
        /// `CpuCmd_ReleaseUp`
        ReleaseUp = 0x10,
        /// `CpuCmd_PressDown`
        PressDown = 0x11,
        /// `CpuCmd_ReleaseDown`
        ReleaseDown = 0x12,
        /// `CpuCmd_PressRight`
        PressRight = 0x13,
        /// `CpuCmd_ReleaseRight`
        ReleaseRight = 0x14,
        /// `CpuCmd_PressLeft`
        PressLeft = 0x15,
        /// `CpuCmd_ReleaseLeft`
        ReleaseLeft = 0x16,
        /// `CpuCmd_PressStart`
        PressStart = 0x17,
        /// `CpuCmd_ReleaseStart`
        ReleaseStart = 0x18,
        /// `CpuCmd_ReleaseAll`
        ReleaseAll = 0x19,
        /// `CpuCmd_Done`
        Done = 0x7F,
        /// `CpuCmd_SetLstickX`
        SetLstickX = 0x80,
        /// `CpuCmd_SetLstickY`
        SetLstickY = 0x81,
        /// `CpuCmd_SetCstickX`
        SetCstickX = 0x82,
        /// `CpuCmd_SetCstickY`
        SetCstickY = 0x83,
        /// `CpuCmd_SetRtrigger`
        SetRtrigger = 0x84,
        /// `CpuCmd_SetLtrigger`
        SetLtrigger = 0x85,
        /// `CpuCmd_PressAFor`
        PressAFor = 0x86,
        /// `CpuCmd_ReleaseAFor`
        ReleaseAFor = 0x87,
        /// `CpuCmd_PressBFor`
        PressBFor = 0x88,
        /// `CpuCmd_ReleaseBFor`
        ReleaseBFor = 0x89,
        /// `CpuCmd_PressXFor`
        PressXFor = 0x8A,
        /// `CpuCmd_ReleaseXFor`
        ReleaseXFor = 0x8B,
        /// `CpuCmd_PressYFor`
        PressYFor = 0x8C,
        /// `CpuCmd_ReleaseYFor`
        ReleaseYFor = 0x8D,
        /// `CpuCmd_WaitFor`
        WaitFor = 0x8E,
        /// `CpuCmd_LstickTowardDestination`
        LstickTowardDestination = 0x8F,
        /// `CpuCmd_LstickXTowardDestination`
        LstickXTowardDestination = 0x90,
        /// `CpuCmd_LstickXForward`
        LstickXForward = 0x91,
        /// `CpuCmd_WaitIfMotionId`
        WaitIfMotionId = 0x92,
        /// `CpuCmd_Unk0x93` — Set scenario ID?
        Unk0x93 = 0x93,
        /// `CpuCmd_LstickTowardFighter`
        LstickTowardFighter = 0x94,
        /// `CpuCmd_LstickXTowardFighter`
        LstickXTowardFighter = 0x95,
        /// `CpuCmd_OneArgEnd` — Previous commands take one argument
        OneArgEnd = 0xBF,
        /// `CpuCmd_LstickTowardDestinationClamped`
        LstickTowardDestinationClamped = 0xC0,
        /// `CpuCmd_LstickXTowardDestinationClamped`
        LstickXTowardDestinationClamped = 0xC1,
        /// `CpuCmd_LstickForwardClamped`
        LstickForwardClamped = 0xC2,
    }
}

impl CpuCmd {
    /// `CpuCmd_ZeroArgEnd` — alias of `CpuCmd_Done`; commands `<= ZERO_ARG_END` take zero arguments
    pub const ZERO_ARG_END: i32 = 0x7F;

    /// `CpuCmd_OneArgEnd` — same value as the `OneArgEnd` variant; commands in `(ZERO_ARG_END, ONE_ARG_END]` take one argument
    pub const ONE_ARG_END: i32 = 0xBF;

    /// `CpuCmd_Count` — one past the last command; the header asserts this fits in a `u8`
    pub const COUNT: i32 = 0xC3;
}

impl From<CpuCmd> for u8 {
    fn from(v: CpuCmd) -> u8 {
        // Every opcode is < COUNT <= 0xFF (STATIC_ASSERT in the header).
        v as i32 as u8
    }
}

impl TryFrom<u8> for CpuCmd {
    type Error = crate::InvalidDiscriminant;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        CpuCmd::try_from(i32::from(v))
    }
}
