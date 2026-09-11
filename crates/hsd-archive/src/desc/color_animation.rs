//! Color-overlay commands used by grMaterial / lb_80014258.
use super::{DescError, Result};
fn unsupported(offset: u32, reason: &'static str) -> DescError {
    DescError::InvalidColorAnimation { offset, reason }
}
use crate::Archive;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorCommand {
    Set([u8; 4]),
    Blend { color: [u8; 4], frames: u32 },
    Wait(u32),
    Disable,
    Complete,
}
/// Read a finite overlay program. Unsupported control/light commands are errors.
pub fn read(archive: &Archive, mut offset: u32) -> Result<Vec<ColorCommand>> {
    let reader = archive.reader();
    let mut commands = Vec::new();
    for _ in 0..1024 {
        let word = reader.u32(offset)?;
        offset = crate::add_offset(offset, 4)?;
        let command = match word >> 26 {
            10 => ColorCommand::Complete,
            11 => ColorCommand::Wait(word & 0x03ff_ffff),
            12 | 20 => ColorCommand::Disable,
            18 => {
                let color = reader.array(offset)?;
                offset = crate::add_offset(offset, 4)?;
                ColorCommand::Set(color)
            }
            19 => {
                let color = reader.array(offset)?;
                offset = crate::add_offset(offset, 4)?;
                ColorCommand::Blend {
                    color,
                    frames: word & 0x03ff_ffff,
                }
            }
            _ => return Err(unsupported(offset - 4, "color-overlay command")),
        };
        commands.push(command);
        if command == ColorCommand::Complete {
            return Ok(commands);
        }
    }
    Err(unsupported(offset, "unterminated color-overlay program"))
}
