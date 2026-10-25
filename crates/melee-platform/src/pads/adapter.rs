//! The Wii U GameCube adapter (WUP-028, and clones in Wii U mode): a HID
//! device the host opens itself, starts by writing 0x13, then reads
//! 37-byte reports carrying four ports (the layout Dolphin's GCAdapter.cpp
//! reads). The bytes are the controllers' own, so they map one to one.

/// USB ids of the adapter.
pub const VENDOR: u16 = 0x057e;
pub const PRODUCT: u16 = 0x0337;
/// The command that starts the adapter's reports.
pub const START_COMMAND: u8 = 0x13;
/// An input report: 0x21, then nine bytes per port.
pub const REPORT_LEN: usize = 37;
const REPORT_ID: u8 = 0x21;

/// One port's controller as the adapter reports it (raw wire bytes: sticks
/// centred near 128, not yet origin-adjusted).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdapterPad {
    /// Both button bytes: [`buttons`] bits.
    pub buttons: u16,
    pub stick: [u8; 2],
    pub cstick: [u8; 2],
    pub triggers: [u8; 2],
}

/// [`AdapterPad::buttons`] bits (first byte, then second byte).
pub mod buttons {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const X: u16 = 1 << 2;
    pub const Y: u16 = 1 << 3;
    pub const DPAD_LEFT: u16 = 1 << 4;
    pub const DPAD_RIGHT: u16 = 1 << 5;
    pub const DPAD_DOWN: u16 = 1 << 6;
    pub const DPAD_UP: u16 = 1 << 7;
    pub const START: u16 = 1 << 8;
    pub const Z: u16 = 1 << 9;
    pub const R: u16 = 1 << 10;
    pub const L: u16 = 1 << 11;
}

/// The four ports of one report; `None` for an empty port. A report that
/// is not an input report gives `None`.
pub fn parse_report(report: &[u8]) -> Option<[Option<AdapterPad>; 4]> {
    if report.len() < REPORT_LEN || report[0] != REPORT_ID {
        return None;
    }
    let mut ports = [None; 4];
    for (port, slot) in ports.iter_mut().enumerate() {
        let b = &report[1 + 9 * port..1 + 9 * (port + 1)];
        // High nibble of the status byte: 1 wired, 2 wireless, 0 empty.
        if b[0] >> 4 == 0 {
            continue;
        }
        *slot = Some(AdapterPad {
            buttons: u16::from(b[1]) | u16::from(b[2]) << 8,
            stick: [b[3], b[4]],
            cstick: [b[5], b[6]],
            triggers: [b[7], b[8]],
        });
    }
    Some(ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_names_each_ports_controller() {
        let mut report = [0u8; REPORT_LEN];
        report[0] = 0x21;
        // Port 1: wired, A and Start held, stick right, L half in.
        report[1..10].copy_from_slice(&[0x10, 0x01, 0x01, 200, 128, 128, 128, 90, 30]);
        // Port 3: a WaveBird with Z and d-pad up.
        report[19..28].copy_from_slice(&[0x20, 0x80, 0x02, 128, 127, 129, 128, 20, 20]);
        let ports = parse_report(&report).unwrap();
        assert_eq!(
            ports[0],
            Some(AdapterPad {
                buttons: buttons::A | buttons::START,
                stick: [200, 128],
                cstick: [128, 128],
                triggers: [90, 30],
            })
        );
        assert_eq!(ports[1], None);
        assert_eq!(ports[2].unwrap().buttons, buttons::DPAD_UP | buttons::Z);
        assert_eq!(ports[3], None);
        assert_eq!(parse_report(&report[..20]), None);
        report[0] = 0x11;
        assert_eq!(parse_report(&report), None);
    }
}
