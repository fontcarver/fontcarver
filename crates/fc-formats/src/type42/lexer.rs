use super::error;
use crate::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Token<'a> {
    Word(&'a str),
    Name(&'a str),
    String,
    Hex(&'a [u8]),
    Mark(u8),
}

pub(super) struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    count: usize,
    peeked: Option<Token<'a>>,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            count: 0,
            peeked: None,
        }
    }

    pub(super) fn peek(&mut self) -> Result<Option<Token<'a>>, Error> {
        if self.peeked.is_none() {
            self.peeked = self.scan()?;
        }
        Ok(self.peeked)
    }

    pub(super) fn next(&mut self) -> Result<Option<Token<'a>>, Error> {
        if self.peeked.is_some() {
            return Ok(self.peeked.take());
        }
        self.scan()
    }

    pub(super) fn require(&mut self, expected: Token<'_>) -> Result<(), Error> {
        if self.next()? == Some(expected) {
            Ok(())
        } else {
            Err(error(format!("expected {expected:?}")))
        }
    }

    pub(super) fn number(&mut self) -> Result<f64, Error> {
        match self.next()? {
            Some(Token::Word(word)) => finite_number(word),
            _ => Err(error("expected literal number")),
        }
    }

    pub(super) fn integer(&mut self) -> Result<u32, Error> {
        let number = self.number()?;
        if number.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&number) {
            Ok(number as u32)
        } else {
            Err(error("expected nonnegative integer"))
        }
    }

    pub(super) fn definition_end(&mut self) -> Result<(), Error> {
        if self.peek()? == Some(Token::Word("readonly")) {
            self.next()?;
        }
        self.require(Token::Word("def"))
    }

    fn scan(&mut self) -> Result<Option<Token<'a>>, Error> {
        while let Some(&byte) = self.bytes.get(self.pos) {
            if whitespace(byte) {
                self.pos += 1;
            } else if byte == b'%' {
                while self
                    .bytes
                    .get(self.pos)
                    .is_some_and(|b| !matches!(b, b'\r' | b'\n'))
                {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
        let Some(&byte) = self.bytes.get(self.pos) else {
            return Ok(None);
        };
        self.count += 1;
        if self.count > 2_000_000 {
            return Err(error("PostScript token limit exceeded"));
        }
        self.pos += 1;
        match byte {
            b'[' | b']' | b'{' | b'}' => Ok(Some(Token::Mark(byte))),
            b'(' => {
                self.string()?;
                Ok(Some(Token::String))
            }
            b'<' => {
                if self.bytes.get(self.pos) == Some(&b'<') {
                    return Err(error(
                        "computed or dictionary-literal syntax is unsupported",
                    ));
                }
                let start = self.pos;
                while self.bytes.get(self.pos).is_some_and(|b| *b != b'>') {
                    self.pos += 1;
                }
                let end = self.pos;
                if self.bytes.get(self.pos) != Some(&b'>') {
                    return Err(error("unterminated hex string"));
                }
                self.pos += 1;
                Ok(Some(Token::Hex(&self.bytes[start..end])))
            }
            b')' | b'>' => Err(error("unexpected PostScript delimiter")),
            _ => {
                let literal_name = byte == b'/';
                let start = if literal_name { self.pos } else { self.pos - 1 };
                while self.bytes.get(self.pos).is_some_and(|b| !delimiter(*b)) {
                    self.pos += 1;
                }
                let raw = &self.bytes[start..self.pos];
                if raw.is_empty() || raw.len() > 255 || !raw.is_ascii() {
                    return Err(error("invalid or oversized PostScript token"));
                }
                let word =
                    std::str::from_utf8(raw).map_err(|_| error("invalid PostScript token"))?;
                Ok(Some(if literal_name {
                    Token::Name(word)
                } else {
                    Token::Word(word)
                }))
            }
        }
    }

    fn string(&mut self) -> Result<(), Error> {
        let start = self.pos;
        let mut depth = 1;
        while let Some(&byte) = self.bytes.get(self.pos) {
            self.pos += 1;
            if self.pos - start > 65_535 {
                return Err(error("PostScript string limit exceeded"));
            }
            match byte {
                b'\\' => {
                    if self.bytes.get(self.pos).is_none() {
                        break;
                    }
                    self.pos += 1;
                }
                b'(' => {
                    depth += 1;
                    if depth > 32 {
                        return Err(error("PostScript string nesting limit exceeded"));
                    }
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        Err(error("unterminated PostScript string"))
    }
}

pub(super) fn finite_number(word: &str) -> Result<f64, Error> {
    word.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| error("expected finite literal number"))
}

pub(super) fn whitespace(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 12 | b'\r' | b' ')
}

fn delimiter(byte: u8) -> bool {
    whitespace(byte) || b"()<>[]{}/%".contains(&byte)
}

pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .bytes()
            .all(|b| (33..=126).contains(&b) && !delimiter(b))
}
