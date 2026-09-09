//! Byte reads in `hsd_8039930C` (particle.c, retail 0x8039930C).
use crate::Error;
use hsd_archive::Reader;

pub(crate) struct Cursor<'a> {
    pub bytes: &'a [u8],
    pub pc: u16,
}
impl Cursor<'_> {
    pub fn byte(&mut self) -> Result<u8, Error> {
        let value = *self
            .bytes
            .get(self.pc as usize)
            .ok_or(Error::TruncatedProgram { pc: self.pc })?;
        self.pc = self.pc.wrapping_add(1);
        Ok(value)
    }
    pub fn short(&mut self) -> Result<u16, Error> {
        Ok((u16::from(self.byte()?) << 8) | u16::from(self.byte()?))
    }
    pub fn float(&mut self) -> Result<f32, Error> {
        let value = Reader::new(self.bytes)
            .f32(u32::from(self.pc))
            .map_err(|_| Error::TruncatedProgram { pc: self.pc })?;
        self.pc = self.pc.wrapping_add(4);
        Ok(value)
    }
    pub fn timer(&mut self) -> Result<u16, Error> {
        let first = self.byte()?;
        Ok(if first & 0x80 != 0 {
            (u16::from(first & 0x7f) << 8) | u16::from(self.byte()?)
        } else {
            u16::from(first)
        })
    }
}
