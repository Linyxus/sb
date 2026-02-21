use anyhow::{bail, Result};

/// Zero-copy byte reader for TASTy binary data.
pub struct TastyReader<'a> {
    data: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> TastyReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            end: data.len(),
        }
    }

    pub fn sub_reader(&self, start: usize, end: usize) -> Self {
        Self {
            data: self.data,
            pos: start,
            end,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.end - self.pos
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    pub fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.end {
            bail!("unexpected end of data at offset {}", self.pos);
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.pos + n > self.end {
            bail!(
                "unexpected end of data: need {} bytes at offset {}",
                n,
                self.pos
            );
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Read a natural number in TASTy's base-128 big-endian encoding.
    /// Bytes 0x00-0x7F are continuation bytes (7 payload bits).
    /// Bytes 0x80-0xFF are terminal bytes (payload = byte & 0x7F).
    #[inline(always)]
    pub fn read_nat(&mut self) -> Result<u64> {
        let b = self.read_byte()?;
        // Fast path: single terminal byte (most common)
        if b & 0x80 != 0 {
            return Ok((b & 0x7F) as u64);
        }
        // Multi-byte
        let mut acc = b as u64;
        loop {
            let b = self.read_byte()?;
            if b & 0x80 != 0 {
                acc = (acc << 7) | (b & 0x7F) as u64;
                return Ok(acc);
            }
            acc = (acc << 7) | b as u64;
        }
    }

    /// Read a signed integer in TASTy's base-128 big-endian 2's complement encoding.
    /// Same byte format as Nat, but the accumulated value is sign-extended.
    pub fn read_int(&mut self) -> Result<i64> {
        let mut result: i64 = 0;
        let mut count: u32 = 0;
        loop {
            let b = self.read_byte()?;
            result = (result << 7) | (b & 0x7F) as i64;
            count += 1;
            if b & 0x80 != 0 {
                break;
            }
        }
        // Sign-extend from count*7 bits to 64 bits
        let bits = count * 7;
        if bits < 64 {
            let shift = 64 - bits;
            result = (result << shift) >> shift;
        }
        Ok(result)
    }

    pub fn read_utf8(&mut self, len: usize) -> Result<&'a str> {
        let bytes = self.read_bytes(len)?;
        std::str::from_utf8(bytes)
            .map_err(|e| anyhow::anyhow!("invalid UTF-8 in TASTy at offset {}: {}", self.pos, e))
    }

    /// Read uncompressed (full-length) 64-bit value.
    pub fn read_uncompressed_long(&mut self) -> Result<i64> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn end(&self) -> usize {
        self.end
    }
}
