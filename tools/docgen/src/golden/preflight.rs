// SPDX-License-Identifier: AGPL-3.0-only
//! Allocation-free schema-aware inventory before any JSON value is retained.

use super::{Cause, Error, Limits, ensure, invalid};

/// Rational storage inventory, in decoded UTF-8 bytes, including repeated evidence.
#[derive(Debug, Default)]
pub(super) struct Inventory {
    /// Largest rational spelling.
    pub(super) largest_literal: usize,
    /// Sum of rational spelling lengths.
    pub(super) literal_bytes: usize,
}

/// Only schema positions that can contain bounded mathematical data.
#[derive(Clone, Copy)]
enum Role {
    /// Dataset envelope.
    Root,
    /// Root entry array.
    Entries,
    /// One operation row.
    Row,
    /// Original point or box.
    Input,
    /// Expression enclosure judgment and its exact endpoint spellings.
    Judgment,
    /// Published endpoint intervals.
    Golden,
    /// Endpoint evidence array.
    Evidence,
    /// One endpoint's input association.
    Endpoint,
    /// Endpoint route interval array.
    Routes,
    /// One route interval record.
    Route,
    /// One rational spelling.
    Number,
    /// A pair of rational spellings.
    Numbers,
    /// Nonmathematical metadata or a malformed schema position.
    Other,
}

impl Role {
    /// Object member association; unknown members remain syntax checked.
    const fn member(self, key: &[u8]) -> Self {
        match (self, key) {
            (Self::Root, b"entries") => Self::Entries,
            (Self::Row, b"input") => Self::Input,
            (Self::Row, b"golden") => Self::Golden,
            (Self::Row, b"evidence") => Self::Evidence,
            (Self::Row, b"judgment") => Self::Judgment,
            (Self::Row, b"exponent") | (Self::Input, b"point") | (Self::Endpoint, b"input") => {
                Self::Number
            }
            (Self::Input, b"box")
            | (Self::Golden, b"point" | b"lower_endpoint" | b"upper_endpoint")
            | (Self::Route | Self::Judgment, b"interval") => Self::Numbers,
            (Self::Endpoint, b"routes") => Self::Routes,
            _ => Self::Other,
        }
    }

    /// Array element association.
    const fn element(self) -> Self {
        match self {
            Self::Entries => Self::Row,
            Self::Evidence => Self::Endpoint,
            Self::Routes => Self::Route,
            Self::Numbers => Self::Number,
            _ => Self::Other,
        }
    }
}

/// Cursor borrows the already byte-bounded buffer; no string or container allocation.
struct Scan<'a> {
    /// Borrowed bounded UTF-8 document.
    bytes: &'a [u8],
    /// Next byte to inspect.
    cursor: usize,
    /// Caller-declared ceilings.
    limits: Limits,
    /// Rational byte counts collected without retaining strings.
    inventory: Inventory,
}

/// Check bounded structure before entering serde's allocating tree decoder.
pub(super) fn check(bytes: &[u8], limits: Limits) -> Result<Inventory, Error> {
    super::check_depth(bytes, limits.depth)?;
    std::str::from_utf8(bytes).map_err(|_| invalid(Cause::Json, "$"))?;
    let mut scan = Scan {
        bytes,
        cursor: 0,
        limits,
        inventory: Inventory::default(),
    };
    scan.value(Role::Root)?;
    scan.space();
    ensure(scan.cursor == bytes.len(), Cause::Json, "$")?;
    Ok(scan.inventory)
}

impl Scan<'_> {
    /// Skip only JSON whitespace.
    fn space(&mut self) {
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.cursor += 1;
        }
    }

    /// Require one structural byte.
    fn take(&mut self, byte: u8) -> Result<(), Error> {
        self.space();
        ensure(self.bytes.get(self.cursor) == Some(&byte), Cause::Json, "$")?;
        self.cursor += 1;
        Ok(())
    }

    /// Read a value without retaining it; serde later validates primitive spelling.
    fn value(&mut self, role: Role) -> Result<(), Error> {
        self.space();
        match self.bytes.get(self.cursor) {
            Some(b'{') => self.object(role),
            Some(b'[') => self.array(role),
            Some(b'"') => self.string(matches!(role, Role::Number)).map(|_| ()),
            Some(_) => {
                let start = self.cursor;
                while self.bytes.get(self.cursor).is_some_and(|b| {
                    !matches!(b, b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t')
                }) {
                    self.cursor += 1;
                }
                ensure(self.cursor > start, Cause::Json, "$")
            }
            None => Err(invalid(Cause::Json, "$")),
        }
    }

    /// Refuse an excess entry before scanning or retaining that entry.
    fn array(&mut self, role: Role) -> Result<(), Error> {
        self.take(b'[')?;
        self.space();
        if self.bytes.get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(());
        }
        let mut count = 0;
        loop {
            if matches!(role, Role::Entries) {
                ensure(count < self.limits.entries, Cause::Entries, "$.entries")?;
            }
            self.value(role.element())?;
            count += 1;
            self.space();
            if self.bytes.get(self.cursor) == Some(&b']') {
                self.cursor += 1;
                return Ok(());
            }
            self.take(b',')?;
        }
    }

    /// A fixed stack key buffer recognizes the closed schema, including escaped keys.
    fn object(&mut self, role: Role) -> Result<(), Error> {
        self.take(b'{')?;
        self.space();
        if self.bytes.get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(());
        }
        loop {
            let (key, length) = self.string(false)?;
            self.take(b':')?;
            let child = if length <= key.len() {
                role.member(&key[..length])
            } else {
                Role::Other
            };
            self.value(child)?;
            self.space();
            if self.bytes.get(self.cursor) == Some(&b'}') {
                self.cursor += 1;
                return Ok(());
            }
            self.take(b',')?;
        }
    }

    /// Count decoded bytes before allocation, including JSON escapes and surrogate pairs.
    fn string(&mut self, rational: bool) -> Result<([u8; 32], usize), Error> {
        self.take(b'"')?;
        let mut key = [0; 32];
        let mut length = 0;
        loop {
            let byte = *self
                .bytes
                .get(self.cursor)
                .ok_or_else(|| invalid(Cause::Json, "$"))?;
            self.cursor += 1;
            if byte == b'"' {
                break;
            }
            ensure(byte >= 32, Cause::Json, "$")?;
            let (ascii, size) = if byte == b'\\' {
                self.escape()?
            } else {
                (byte, 1)
            };
            if length < key.len() {
                key[length] = ascii;
            }
            length += size;
            if rational {
                ensure(
                    length <= self.limits.literal_bytes,
                    Cause::Literal,
                    "rational spelling",
                )?;
            }
        }
        if rational {
            self.inventory.largest_literal = self.inventory.largest_literal.max(length);
            self.inventory.literal_bytes += length;
        }
        Ok((key, length))
    }

    /// Decode escape length; non-ASCII keys cannot equal a schema key.
    fn escape(&mut self) -> Result<(u8, usize), Error> {
        let byte = *self
            .bytes
            .get(self.cursor)
            .ok_or_else(|| invalid(Cause::Json, "$"))?;
        self.cursor += 1;
        match byte {
            b'"' | b'\\' | b'/' => Ok((byte, 1)),
            b'b' | b'f' | b'n' | b'r' | b't' => Ok((0, 1)),
            b'u' => {
                let first = self.hex()?;
                let code = if (0xd800..=0xdbff).contains(&first) {
                    // No whitespace is admitted within a surrogate pair.
                    ensure(
                        self.bytes.get(self.cursor..self.cursor + 2) == Some(b"\\u"),
                        Cause::Json,
                        "$",
                    )?;
                    self.cursor += 2;
                    let second = self.hex()?;
                    ensure((0xdc00..=0xdfff).contains(&second), Cause::Json, "$")?;
                    0x10000 + (first - 0xd800) * 1024 + second - 0xdc00
                } else {
                    first
                };
                let character = char::from_u32(code).ok_or_else(|| invalid(Cause::Json, "$"))?;
                Ok((u8::try_from(code).unwrap_or(255), character.len_utf8()))
            }
            _ => Err(invalid(Cause::Json, "$")),
        }
    }

    /// Exactly four hexadecimal digits, without allocating a temporary string.
    fn hex(&mut self) -> Result<u32, Error> {
        let mut value = 0;
        for _ in 0..4 {
            let byte = *self
                .bytes
                .get(self.cursor)
                .ok_or_else(|| invalid(Cause::Json, "$"))?;
            self.cursor += 1;
            let digit = char::from(byte)
                .to_digit(16)
                .ok_or_else(|| invalid(Cause::Json, "$"))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }
}
