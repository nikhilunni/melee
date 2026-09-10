//! Additional PlCo fields read by input buffers and Wait predicates.
use crate::desc::{
    common::{read_common_data, IdleInputAttributes},
    FighterDescError,
};
use hsd_archive::{Archive, Reader};

#[derive(Debug, Clone)]
pub struct InputCommonData {
    pub thresholds: IdleInputAttributes,
    /// PlCo +20 (x20_radians), +98, +AC, +B0.
    pub tilt_angle: f32,
    /// PlCo +DC/+E0: aerial direction and C-stick edge thresholds.
    pub aerial_horizontal_threshold: f32,
    pub aerial_vertical_threshold: f32,
    /// PlCo +E4/+E8: L-cancel input age window and lag divisor.
    pub l_cancel_window: i32,
    pub l_cancel_divisor: f32,
    pub side_tilt_threshold: f32,
    pub up_tilt_threshold: f32,
    pub down_tilt_threshold: f32,
    /// PlCo +CC/+D0 and +D4/+D8; the two windows are floats in C.
    pub up_smash_threshold: f32,
    pub up_smash_window: f32,
    pub down_smash_threshold: f32,
    pub down_smash_window: f32,
    /// PlCo +218/+21C, special direction thresholds.
    pub special_side_threshold: f32,
    pub special_vertical_threshold: f32,
    /// PlCo +220: facing reversal threshold for side specials.
    pub special_reverse_threshold: f32,
    /// PlCo +224: age of the last horizontal smash for aerial neutral-B reversal.
    pub neutral_reverse_window: i32,
    /// PlCo +2A0 (powershield_input_window).
    pub powershield_window: i32,
    /// PlCo +314/+318, spot-dodge stick threshold and window.
    pub escape_threshold: f32,
    pub escape_window: i32,
    /// PlCo +7B8/+7BC/+7C0, ftCommon_8008031C joystick statistics.
    pub activity_stick_threshold: f32,
    pub activity_trigger_threshold: f32,
    pub activity_window: f32,
}
impl InputCommonData {
    pub fn read(archive: &Archive) -> Result<Self, FighterDescError> {
        let thresholds = read_common_data(archive)?.input;
        // read_common_data already validates the public and required root link.
        let root = archive
            .public("ftLoadCommonData")
            .expect("validated public");
        let offset = archive.link(root)?.expect("validated common data link");
        let r = Reader::new(archive.reader().slice(offset, 0x7C4)?);
        Ok(Self {
            thresholds,
            tilt_angle: r.f32(0x20)?,
            aerial_horizontal_threshold: r.f32(0xDC)?,
            aerial_vertical_threshold: r.f32(0xE0)?,
            l_cancel_window: r.s32(0xE4)?,
            l_cancel_divisor: r.f32(0xE8)?,
            side_tilt_threshold: r.f32(0x98)?,
            up_tilt_threshold: r.f32(0xAC)?,
            down_tilt_threshold: r.f32(0xB0)?,
            up_smash_threshold: r.f32(0xCC)?,
            up_smash_window: r.f32(0xD0)?,
            down_smash_threshold: r.f32(0xD4)?,
            down_smash_window: r.f32(0xD8)?,
            special_side_threshold: r.f32(0x218)?,
            special_vertical_threshold: r.f32(0x21C)?,
            special_reverse_threshold: r.f32(0x220)?,
            neutral_reverse_window: r.s32(0x224)?,
            powershield_window: r.s32(0x2A0)?,
            escape_threshold: r.f32(0x314)?,
            escape_window: r.s32(0x318)?,
            activity_stick_threshold: r.f32(0x7B8)?,
            activity_trigger_threshold: r.f32(0x7BC)?,
            activity_window: r.f32(0x7C0)?,
        })
    }
}
