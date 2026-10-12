//! The Gecko List event (0x3D, Slippi 3.3.0+): the code table the game ran.
//!
//! The recorder copies the game's Gecko code table into the replay before
//! Game Start (slippi-ssbm-asm `Recording/SendGameInfo.asm`), split over
//! Message Splitter events (0x10). The table is Dolphin's codehandler
//! format: the magic `00D0C0DE 00D0C0DE`, then codes of one or more
//! eight-byte lines, then a terminator. So a replay names the codes its
//! game ran; nothing about them has to be read from its frames.
//!
//! Only the code types Slippi's lists use are decoded: a 32-bit write
//! (`04`), a 16-bit and an 8-bit fill (`02`, `00`), a string write (`06`)
//! and an inserted assembly block (`C2`).

/// The code table's first eight bytes.
const MAGIC: [u8; 8] = [0x00, 0xD0, 0xC0, 0xDE, 0x00, 0xD0, 0xC0, 0xDE];

/// One code of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code<'a> {
    /// `04`: the word stored at `address`.
    Write32 { address: u32, value: u32 },
    /// `00`, `02`, `06`: bytes or halfwords filled or copied at `address`;
    /// the body is the code's second word (`00`, `02`) or its string (`06`).
    Fill { address: u32, body: &'a [u8] },
    /// `C2`: instructions run in place of the one at `address`. The body
    /// ends with the handler's return slot (a zero word, after a `nop`
    /// when the code has an even count of instructions).
    Inject { address: u32, body: &'a [u8] },
}

impl Code<'_> {
    pub fn address(&self) -> u32 {
        match *self {
            Code::Write32 { address, .. }
            | Code::Fill { address, .. }
            | Code::Inject { address, .. } => address,
        }
    }
}

/// The codes of a table, in order. Stops at the terminator or at the
/// first line of a type Slippi's lists do not use.
pub fn codes(table: &[u8]) -> impl Iterator<Item = Code<'_>> {
    let mut rest = table.strip_prefix(&MAGIC).unwrap_or(table);
    std::iter::from_fn(move || {
        let (line, after) = rest.split_first_chunk::<8>()?;
        let first = u32::from_be_bytes([line[0], line[1], line[2], line[3]]);
        let second = u32::from_be_bytes([line[4], line[5], line[6], line[7]]);
        // The top seven bits are the type; bit 24 extends the address
        // (0x80000000 or 0x81000000 base).
        let address = 0x8000_0000 | (first & 0x01FF_FFFF);
        let (code, after) = match line[0] & 0xFE {
            0x04 => (
                Code::Write32 {
                    address,
                    value: second,
                },
                after,
            ),
            0x00 | 0x02 => (
                Code::Fill {
                    address,
                    body: &line[4..],
                },
                after,
            ),
            0x06 => {
                let padded = (second as usize).div_ceil(8) * 8;
                let body = after.get(..second as usize)?;
                (Code::Fill { address, body }, after.get(padded..)?)
            }
            0xC2 => {
                let length = (second as usize).checked_mul(8)?;
                let body = after.get(..length)?;
                (Code::Inject { address, body }, &after[length..])
            }
            _ => return None,
        };
        rest = after;
        Some(code)
    })
}

/// The word a `04` code stores at `address`, if the table has one.
pub fn write_at(table: &[u8], address: u32) -> Option<u32> {
    codes(table).find_map(|code| match code {
        Code::Write32 { address: at, value } if at == address => Some(value),
        _ => None,
    })
}

/// The body of the `C2` code at `address`, if the table has one.
pub fn injection_at(table: &[u8], address: u32) -> Option<&[u8]> {
    codes(table).find_map(|code| match code {
        Code::Inject { address: at, body } if at == address => Some(body),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(lines: &[[u32; 2]]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for [a, b] in lines {
            bytes.extend(a.to_be_bytes());
            bytes.extend(b.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn a_code_table_lists_its_writes_and_injections() {
        let bytes = table(&[
            [0x0421_AAE4, 0x6000_0000],
            [0xC21D_4578, 0x0000_0002],
            [0xFFE0_0890, 0x3D80_801D],
            [0x6000_0000, 0x0000_0000],
            [0x0245_C390, 0x0000_0125],
            [0x0600_3000, 0x0000_0009],
            [0x0102_0304, 0x0506_0708],
            [0x0900_0000, 0x0000_0000],
            [0x0403_0C7C, 0x3800_0064],
            [0xF000_0000, 0x0000_0000],
            // Past the terminator: never read.
            [0x0400_0000, 0x1234_5678],
        ]);
        let listed: Vec<Code> = codes(&bytes).collect();
        assert_eq!(listed.len(), 5);
        assert_eq!(
            listed[0],
            Code::Write32 {
                address: 0x8021_AAE4,
                value: 0x6000_0000
            }
        );
        assert_eq!(listed[1].address(), 0x801D_4578);
        assert!(matches!(listed[1], Code::Inject { body, .. } if body.len() == 16));
        assert!(
            matches!(listed[2], Code::Fill { address: 0x8045_C390, body } if body == [0, 0, 1, 0x25])
        );
        assert!(matches!(listed[3], Code::Fill { address: 0x8000_3000, body } if body.len() == 9));
        assert_eq!(write_at(&bytes, 0x8003_0C7C), Some(0x3800_0064));
        assert_eq!(write_at(&bytes, 0x8000_0000), None);
        assert_eq!(injection_at(&bytes, 0x801D_4578).map(<[u8]>::len), Some(16));
        assert_eq!(injection_at(&bytes, 0x8021_AAE4), None);
    }

    #[test]
    fn a_truncated_code_ends_the_list() {
        let mut bytes = table(&[[0x0421_AAE4, 0x6000_0000], [0xC21D_4578, 0x0000_0004]]);
        bytes.extend([0; 12]);
        assert_eq!(codes(&bytes).count(), 1);
    }
}
