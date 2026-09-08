//! Minimal UBJSON (Draft 12) reader, just enough for the `.slp` container.
//!
//! A replay is a UBJSON object with two keys: `raw` (an optimised `uint8`
//! array holding the event byte stream) and `metadata` (an ordinary UBJSON
//! object). See SPEC.md, "UBJSON" and "The `metadata` Element".

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

/// A decoded UBJSON value. Integers are widened to `i64`; `uint8` arrays in
/// optimised form are kept as [`Ubj::Bytes`].
#[derive(Debug, Clone, PartialEq)]
pub enum Ubj {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// `H` high-precision number, kept as its decimal text.
    HighPrecision(String),
    Str(String),
    Array(Vec<Ubj>),
    Bytes(Vec<u8>),
    Object(BTreeMap<String, Ubj>),
}

impl Ubj {
    pub fn get(&self, key: &str) -> Option<&Ubj> {
        match self {
            Ubj::Object(m) => m.get(key),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Ubj::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Ubj::Str(s) => Some(s),
            _ => None,
        }
    }
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.pos.min(self.buf.len())..]
    }

    pub fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    pub fn u8(&mut self) -> Result<u8> {
        let b = *self
            .buf
            .get(self.pos)
            .context("ubjson: unexpected end of data")?;
        self.pos += 1;
        Ok(b)
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).context("ubjson: length overflow")?;
        if end > self.buf.len() {
            bail!(
                "ubjson: wanted {n} bytes at {}, only {} left",
                self.pos,
                self.buf.len() - self.pos
            );
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub fn expect(&mut self, marker: u8) -> Result<()> {
        let b = self.u8()?;
        if b != marker {
            bail!(
                "ubjson: expected marker {:?} at {}, found {:?}",
                marker as char,
                self.pos - 1,
                b as char
            );
        }
        Ok(())
    }

    /// Read an integer value whose type marker is `marker`.
    fn int_with_marker(&mut self, marker: u8) -> Result<i64> {
        Ok(match marker {
            b'i' => self.u8()? as i8 as i64,
            b'U' => self.u8()? as i64,
            b'I' => i16::from_be_bytes(self.take(2)?.try_into().unwrap()) as i64,
            b'l' => i32::from_be_bytes(self.take(4)?.try_into().unwrap()) as i64,
            b'L' => i64::from_be_bytes(self.take(8)?.try_into().unwrap()),
            m => bail!(
                "ubjson: expected integer marker, found {:?} at {}",
                m as char,
                self.pos - 1
            ),
        })
    }

    /// Read a length: an integer with its own type marker, non-negative.
    pub fn length(&mut self) -> Result<usize> {
        let m = self.u8()?;
        let n = self.int_with_marker(m)?;
        usize::try_from(n).context("ubjson: negative length")
    }

    /// Read a string body (length + UTF-8 bytes), no leading `S` marker.
    pub fn string_body(&mut self) -> Result<String> {
        let n = self.length()?;
        let bytes = self.take(n)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    /// Read a value whose type marker has already been consumed.
    fn value_with_marker(&mut self, marker: u8) -> Result<Ubj> {
        Ok(match marker {
            b'Z' => Ubj::Null,
            b'T' => Ubj::Bool(true),
            b'F' => Ubj::Bool(false),
            b'i' | b'U' | b'I' | b'l' | b'L' => Ubj::Int(self.int_with_marker(marker)?),
            b'd' => Ubj::Float(f32::from_be_bytes(self.take(4)?.try_into().unwrap()) as f64),
            b'D' => Ubj::Float(f64::from_be_bytes(self.take(8)?.try_into().unwrap())),
            b'H' => Ubj::HighPrecision(self.string_body()?),
            b'C' => Ubj::Str((self.u8()? as char).to_string()),
            b'S' => Ubj::Str(self.string_body()?),
            b'[' => self.array()?,
            b'{' => self.object()?,
            m => bail!(
                "ubjson: unknown marker {:?} (0x{m:02X}) at {}",
                m as char,
                self.pos - 1
            ),
        })
    }

    /// Read a full value, marker included. Skips `N` no-ops.
    pub fn value(&mut self) -> Result<Ubj> {
        loop {
            let m = self.u8()?;
            if m != b'N' {
                return self.value_with_marker(m);
            }
        }
    }

    /// Optional `$type` and `#count` container header. Marker `[`/`{` already consumed.
    fn container_header(&mut self) -> Result<(Option<u8>, Option<usize>)> {
        let mut ty = None;
        let mut count = None;
        if self.peek() == Some(b'$') {
            self.pos += 1;
            ty = Some(self.u8()?);
            // Per spec, a type marker must be followed by a count.
            self.expect(b'#')?;
            count = Some(self.length()?);
        } else if self.peek() == Some(b'#') {
            self.pos += 1;
            count = Some(self.length()?);
        }
        Ok((ty, count))
    }

    fn array(&mut self) -> Result<Ubj> {
        let (ty, count) = self.container_header()?;
        match (ty, count) {
            (Some(b'U'), Some(n)) => Ok(Ubj::Bytes(self.take(n)?.to_vec())),
            (Some(t), Some(n)) => {
                let mut v = Vec::with_capacity(n.min(1 << 16));
                for _ in 0..n {
                    v.push(match t {
                        // Typed containers of these carry no payload bytes.
                        b'Z' => Ubj::Null,
                        b'T' => Ubj::Bool(true),
                        b'F' => Ubj::Bool(false),
                        _ => self.value_with_marker(t)?,
                    });
                }
                Ok(Ubj::Array(v))
            }
            (None, Some(n)) => {
                let mut v = Vec::with_capacity(n.min(1 << 16));
                for _ in 0..n {
                    v.push(self.value()?);
                }
                Ok(Ubj::Array(v))
            }
            (None, None) => {
                let mut v = Vec::new();
                loop {
                    match self.peek() {
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(Ubj::Array(v));
                        }
                        Some(_) => v.push(self.value()?),
                        None => bail!("ubjson: unterminated array"),
                    }
                }
            }
            (Some(_), None) => unreachable!("type without count is rejected in container_header"),
        }
    }

    fn object(&mut self) -> Result<Ubj> {
        let (ty, count) = self.container_header()?;
        let mut m = BTreeMap::new();
        let read_val = |r: &mut Self| -> Result<Ubj> {
            match ty {
                Some(b'Z') => Ok(Ubj::Null),
                Some(b'T') => Ok(Ubj::Bool(true)),
                Some(b'F') => Ok(Ubj::Bool(false)),
                Some(t) => r.value_with_marker(t),
                None => r.value(),
            }
        };
        match count {
            Some(n) => {
                for _ in 0..n {
                    let k = self.string_body()?;
                    let v = read_val(self)?;
                    m.insert(k, v);
                }
            }
            None => loop {
                match self.peek() {
                    Some(b'}') => {
                        self.pos += 1;
                        break;
                    }
                    Some(b'N') => self.pos += 1,
                    Some(_) => {
                        let k = self.string_body()?;
                        let v = read_val(self)?;
                        m.insert(k, v);
                    }
                    None => bail!("ubjson: unterminated object"),
                }
            },
        }
        Ok(Ubj::Object(m))
    }
}

/// The two top-level members of a `.slp` file.
pub struct SlpContainer<'a> {
    /// The event byte stream. If the file was never finalised (raw length
    /// written as 0) this is everything after the `raw` header and
    /// `metadata` is `None`.
    pub raw: &'a [u8],
    pub raw_len_was_zero: bool,
    pub metadata: Option<Ubj>,
}

/// Split a `.slp` file into its `raw` byte stream and `metadata` object.
///
/// Expected prefix (SPEC.md): `{ U 3 r a w [ $ U # l XXXX`.
pub fn split_container(buf: &[u8]) -> Result<SlpContainer<'_>> {
    let mut r = Reader::new(buf);
    r.expect(b'{')
        .context("not a .slp file: missing leading '{'")?;
    let mut raw: Option<(&[u8], bool)> = None;
    let mut metadata = None;
    loop {
        match r.peek() {
            None => break,
            Some(b'}') => break,
            Some(b'N') => r.pos += 1,
            Some(_) => {}
        }
        let key = r.string_body().context("reading top-level key")?;
        match key.as_str() {
            "raw" => {
                r.expect(b'[')?;
                r.expect(b'$')?;
                r.expect(b'U')?;
                r.expect(b'#')?;
                let n = r.length()?;
                if n == 0 {
                    // Unfinalised file: the stream runs to the end of the file
                    // and there is no reliable metadata.
                    raw = Some((r.remaining(), true));
                    break;
                }
                raw = Some((
                    r.take(n)
                        .context("raw array shorter than its declared length")?,
                    false,
                ));
            }
            "metadata" => metadata = Some(r.value().context("parsing metadata")?),
            _ => {
                // Unknown top-level member: parse and drop it.
                r.value()?;
            }
        }
    }
    let (raw, raw_len_was_zero) = raw.context("not a .slp file: no `raw` member")?;
    Ok(SlpContainer {
        raw,
        raw_len_was_zero,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_object() {
        // {"a": 1, "b": "hi", "c": [true, 2.5]}
        let mut b = vec![b'{'];
        b.extend(b"U\x01a"); // key "a"
        b.extend(b"U\x01"); // uint8 1
        b.extend(b"U\x01b");
        b.extend(b"SU\x02hi");
        b.extend(b"U\x01c");
        b.extend(b"[T");
        b.push(b'd');
        b.extend(2.5f32.to_be_bytes());
        b.extend(b"]}");
        let v = Reader::new(&b).value().unwrap();
        assert_eq!(v.get("a").and_then(Ubj::as_i64), Some(1));
        assert_eq!(v.get("b").and_then(Ubj::as_str), Some("hi"));
        assert_eq!(
            v.get("c"),
            Some(&Ubj::Array(vec![Ubj::Bool(true), Ubj::Float(2.5)]))
        );
    }

    #[test]
    fn splits_container_with_zero_raw_length() {
        let mut b = b"{U\x03raw[$U#l\x00\x00\x00\x00".to_vec();
        b.extend([0x35, 0x01]);
        let c = split_container(&b).unwrap();
        assert!(c.raw_len_was_zero);
        assert_eq!(c.raw, &[0x35, 0x01]);
        assert!(c.metadata.is_none());
    }
}
