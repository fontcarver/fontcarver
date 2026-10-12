use std::collections::BTreeSet;

use read_fonts::TableProvider;

use super::{
    error,
    lexer::{finite_number, whitespace, Lexer, Token},
    sfnt,
};
use crate::Error;

pub(super) fn read(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.len() > super::MAX_SOURCE_BYTES || !bytes.starts_with(b"%!") {
        return Err(error(
            "expected a Type 42 PostScript program within the size limit",
        ));
    }
    let mut lexer = Lexer::new(bytes);
    let mut fields = BTreeSet::new();
    let mut embedded = None;
    let mut charstrings = Vec::new();
    let mut dictionary_depth = 0u8;
    while let Some(token) = lexer.next()? {
        match token {
            Token::Name(key) => {
                // Some downloaders name the font in the final definefont call.
                if lexer.peek()? == Some(Token::Word("exch")) {
                    lexer.next()?;
                    lexer.require(Token::Word("definefont"))?;
                    lexer.require(Token::Word("pop"))?;
                    continue;
                }
                if !fields.insert(key) {
                    return Err(error(format!("duplicate /{key} declaration")));
                }
                match key {
                    "FontType" => {
                        if lexer.integer()? != 42 {
                            return Err(error("FontType must be 42"));
                        }
                    }
                    "PaintType" | "WMode" => {
                        if lexer.integer()? != 0 {
                            return Err(error(
                                "stroked or vertical Type 42 wrappers are unsupported",
                            ));
                        }
                    }
                    "FontMatrix" => {
                        lexer.require(Token::Mark(b'['))?;
                        for value in [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
                            if lexer.number()? != value {
                                return Err(error("only identity Type 42 FontMatrix is supported"));
                            }
                        }
                        lexer.require(Token::Mark(b']'))?;
                    }
                    "FontBBox" => {
                        lexer.require(Token::Mark(b'['))?;
                        for _ in 0..4 {
                            lexer.number()?;
                        }
                        lexer.require(Token::Mark(b']'))?;
                    }
                    "sfnts" => {
                        embedded = Some(read_sfnts(&mut lexer)?);
                    }
                    "CharStrings" => {
                        charstrings = read_charstrings(&mut lexer)?;
                    }
                    "Encoding" => {
                        read_encoding(&mut lexer)?;
                    }
                    "FontInfo" => {
                        read_info(&mut lexer)?;
                    }
                    "Metrics" | "Metrics2" | "CDevProc" | "BuildGlyph" | "BuildChar"
                    | "GlyphDirectory" | "CIDFontType" | "CIDMap" | "MetricsCount" => {
                        return Err(error(format!(
                            "/{key} overrides or CID fonts are unsupported"
                        )));
                    }
                    _ => literal(&mut lexer, 0)?,
                }
                lexer.definition_end()?;
            }
            Token::Word("begin") => {
                dictionary_depth = dictionary_depth
                    .checked_add(1)
                    .filter(|n| *n <= 32)
                    .ok_or_else(|| error("dictionary nesting limit exceeded"))?;
            }
            Token::Word("end") => {
                dictionary_depth = dictionary_depth
                    .checked_sub(1)
                    .ok_or_else(|| error("unbalanced dictionary end"))?;
            }
            Token::Word(
                "dict" | "dup" | "readonly" | "currentdict" | "definefont" | "pop" | "exch"
                | "FontName",
            ) => {}
            Token::Word(word) if finite_number(word).is_ok() => {}
            _ => {
                return Err(error(
                    "unsupported executable PostScript or declaration syntax",
                ))
            }
        }
    }
    if dictionary_depth != 0 {
        return Err(error("unclosed font dictionary"));
    }
    for key in [
        "FontType",
        "FontMatrix",
        "FontBBox",
        "Encoding",
        "CharStrings",
        "sfnts",
    ] {
        if !fields.contains(key) {
            return Err(error(format!("missing /{key} declaration")));
        }
    }
    let data = embedded.ok_or_else(|| error("missing sfnts array"))?;
    let face = sfnt::validate(&data)?;
    let glyph_count = u32::from(face.maxp().map_err(|e| error(e.to_string()))?.num_glyphs());
    if charstrings.iter().any(|gid| *gid >= glyph_count) {
        return Err(error("CharStrings glyph index is out of range"));
    }
    Ok(data)
}

fn read_sfnts(lexer: &mut Lexer<'_>) -> Result<Vec<u8>, Error> {
    lexer.require(Token::Mark(b'['))?;
    let mut result = Vec::new();
    while lexer.peek()? != Some(Token::Mark(b']')) {
        let Some(Token::Hex(hex)) = lexer.next()? else {
            return Err(error("sfnts requires literal hexadecimal strings"));
        };
        let start = result.len();
        let mut high = None;
        for &byte in hex {
            if whitespace(byte) {
                continue;
            }
            let nibble = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(error("invalid sfnts hexadecimal digit")),
            };
            if let Some(first) = high.take() {
                if result.len() >= sfnt::MAX_SFNT_BYTES || result.len() - start >= 65_535 {
                    return Err(error("sfnts data or string size limit exceeded"));
                }
                result.push(first * 16 + nibble);
            } else {
                high = Some(nibble);
            }
        }
        if high.is_some() {
            return Err(error("sfnts hex string has an incomplete byte"));
        }
        if !(result.len() - start).is_multiple_of(2) {
            if result.last() != Some(&0) {
                return Err(error("odd sfnts string requires a zero padding byte"));
            }
            result.pop();
        }
        if result.len() == start {
            return Err(error("empty sfnts string"));
        }
    }
    lexer.next()?;
    if result.is_empty() {
        return Err(error("empty sfnts array"));
    }
    Ok(result)
}

fn read_charstrings(lexer: &mut Lexer<'_>) -> Result<Vec<u32>, Error> {
    let count = lexer.integer()?;
    if count > 65_535 {
        return Err(error("CharStrings dictionary size limit exceeded"));
    }
    lexer.require(Token::Word("dict"))?;
    if lexer.peek()? == Some(Token::Word("dup")) {
        lexer.next()?;
    }
    lexer.require(Token::Word("begin"))?;
    let mut names = BTreeSet::new();
    let mut gids = Vec::new();
    let mut notdef = false;
    while lexer.peek()? != Some(Token::Word("end")) {
        let Some(Token::Name(name)) = lexer.next()? else {
            return Err(error("expected CharStrings glyph name"));
        };
        let gid = lexer.integer()?;
        if !names.insert(name) || names.len() > 65_535 {
            return Err(error("duplicate or excessive CharStrings glyph names"));
        }
        if name == ".notdef" {
            if gid != 0 {
                return Err(error(".notdef must refer to glyph zero"));
            }
            notdef = true;
        }
        gids.push(gid);
        lexer.require(Token::Word("def"))?;
    }
    lexer.next()?;
    if !notdef {
        return Err(error("CharStrings must contain .notdef"));
    }
    Ok(gids)
}

fn read_encoding(lexer: &mut Lexer<'_>) -> Result<(), Error> {
    match lexer.next()? {
        Some(Token::Mark(b'[')) => {
            for _ in 0..256 {
                if !matches!(lexer.next()?, Some(Token::Name(_))) {
                    return Err(error("Encoding must contain 256 literal names"));
                }
            }
            lexer.require(Token::Mark(b']'))?;
        }
        Some(Token::Word("StandardEncoding" | "ISOLatin1Encoding")) => {}
        Some(Token::Word("256")) => {
            // Recognize this common declarative initializer without executing it.
            for expected in [
                Token::Word("array"),
                Token::Word("0"),
                Token::Word("1"),
                Token::Word("255"),
                Token::Mark(b'{'),
                Token::Word("1"),
                Token::Word("index"),
                Token::Word("exch"),
                Token::Name(".notdef"),
                Token::Word("put"),
                Token::Mark(b'}'),
                Token::Word("for"),
            ] {
                lexer.require(expected)?;
            }
            while lexer.peek()? == Some(Token::Word("dup")) {
                lexer.next()?;
                if lexer.integer()? > 255 {
                    return Err(error("Encoding index exceeds 255"));
                }
                if !matches!(lexer.next()?, Some(Token::Name(_))) {
                    return Err(error("Encoding entry requires a literal name"));
                }
                lexer.require(Token::Word("put"))?;
            }
        }
        _ => return Err(error("unsupported computed Encoding")),
    }
    Ok(())
}

fn read_info(lexer: &mut Lexer<'_>) -> Result<(), Error> {
    if lexer.integer()? > 256 {
        return Err(error("FontInfo dictionary size limit exceeded"));
    }
    lexer.require(Token::Word("dict"))?;
    if lexer.peek()? == Some(Token::Word("dup")) {
        lexer.next()?;
    }
    lexer.require(Token::Word("begin"))?;
    let mut count = 0;
    while lexer.peek()? != Some(Token::Word("end")) {
        if !matches!(lexer.next()?, Some(Token::Name(_))) {
            return Err(error("expected FontInfo key"));
        }
        count += 1;
        if count > 256 {
            return Err(error("FontInfo entry limit exceeded"));
        }
        literal(lexer, 0)?;
        lexer.definition_end()?;
    }
    lexer.next()?;
    Ok(())
}

fn literal(lexer: &mut Lexer<'_>, depth: usize) -> Result<(), Error> {
    if depth >= 32 {
        return Err(error("PostScript literal nesting limit exceeded"));
    }
    match lexer.next()? {
        Some(Token::Name(_) | Token::String | Token::Word("true" | "false" | "null")) => Ok(()),
        Some(Token::Word(word)) => finite_number(word).map(|_| ()),
        Some(Token::Mark(b'[')) => {
            while lexer.peek()? != Some(Token::Mark(b']')) {
                literal(lexer, depth + 1)?;
            }
            lexer.next()?;
            Ok(())
        }
        _ => Err(error("expected literal PostScript metadata value")),
    }
}
