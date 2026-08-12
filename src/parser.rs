//! The one Datom lexical parser.
//!
//! It never builds a document tree.  Contexts ask this parser only for the
//! next structural evidence or for text owned by their active context.

use std::fmt;

use crate::expectation::{ProtosShape, ShapeProbe};

/// A ruled Datom read or write refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatomError {
    EndOfInput {
        expected: &'static str,
    },
    UnexpectedShape {
        expected: &'static str,
        found: ProtosShape,
    },
    UnexpectedClosingDelimiter(char),
    ShapeNotYetRuled {
        shape: ProtosShape,
    },
    PipeTextIsNotDatom,
    AnglesAreNotDatom,
    InvalidBareSymbol {
        text: String,
    },
    InvalidScalar {
        expected: &'static str,
        text: String,
    },
    InvalidEscape {
        escaped: char,
    },
    Unterminated {
        shape: ProtosShape,
    },
    ExpectedHead {
        expected: &'static str,
        found: Option<String>,
    },
    TrailingInput {
        text: String,
    },
    NonFiniteFloat,
}

impl fmt::Display for DatomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndOfInput { expected } => {
                write!(formatter, "expected {expected}, found end of input")
            }
            Self::UnexpectedShape { expected, found } => {
                write!(
                    formatter,
                    "expected {expected}, found {}",
                    found.description()
                )
            }
            Self::UnexpectedClosingDelimiter(delimiter) => {
                write!(formatter, "unexpected closing delimiter {delimiter}")
            }
            Self::ShapeNotYetRuled { shape } => write!(
                formatter,
                "{} is reserved for Meaning, whose shape is not yet ruled",
                shape.description()
            ),
            Self::PipeTextIsNotDatom => write!(formatter, "pipe text is not Datom"),
            Self::AnglesAreNotDatom => write!(formatter, "angle syntax and generics are not Datom"),
            Self::InvalidBareSymbol { text } => {
                write!(formatter, "{text:?} is not a bare Datom symbol")
            }
            Self::InvalidScalar { expected, text } => {
                write!(formatter, "{text:?} is not a {expected}")
            }
            Self::InvalidEscape { escaped } => {
                write!(formatter, "\\{escaped} is not a ruled string escape")
            }
            Self::Unterminated { shape } => {
                write!(formatter, "unterminated {}", shape.description())
            }
            Self::ExpectedHead { expected, found } => match found {
                Some(found) => write!(
                    formatter,
                    "expected dotted head {expected:?}, found {found:?}"
                ),
                None => write!(formatter, "expected dotted head {expected:?}"),
            },
            Self::TrailingInput { text } => {
                write!(formatter, "trailing Datom input begins with {text:?}")
            }
            Self::NonFiniteFloat => write!(formatter, "Datom does not serialize non-finite floats"),
        }
    }
}

impl std::error::Error for DatomError {}

/// The parser's cursor over one Datom source.  It is crate-private so no other
/// module can invent a competing text parser.
pub(crate) struct Parser<'source> {
    source: &'source str,
    index: usize,
}

impl<'source> Parser<'source> {
    pub(crate) fn new(source: &'source str) -> Self {
        Self { source, index: 0 }
    }

    pub(crate) fn probe(&mut self) -> Result<ShapeProbe<'source>, DatomError> {
        self.skip_trivia();
        let first = self.current().ok_or(DatomError::EndOfInput {
            expected: "a Datom value",
        })?;
        match first {
            '“' => Ok(ShapeProbe::new(ProtosShape::CurlyQuoteDelimited, None)),
            '(' if self.rest().starts_with("(|") => Err(DatomError::PipeTextIsNotDatom),
            '(' => Ok(ShapeProbe::new(ProtosShape::ParenthesisDelimited, None)),
            '[' => Ok(ShapeProbe::new(ProtosShape::SquareBracketDelimited, None)),
            '{' => Ok(ShapeProbe::new(ProtosShape::BraceDelimited, None)),
            '|' => Err(DatomError::PipeTextIsNotDatom),
            '<' | '>' => Err(DatomError::AnglesAreNotDatom),
            ']' | '}' | '”' | ')' => Err(DatomError::UnexpectedClosingDelimiter(first)),
            _ => self.probe_symbol(),
        }
    }

    pub(crate) fn take_bare(&mut self, dot_terminates: bool) -> Result<&'source str, DatomError> {
        self.skip_trivia();
        let start = self.index;
        while let Some(character) = self.current() {
            if character == '<' || character == '>' {
                return Err(DatomError::AnglesAreNotDatom);
            }
            if (dot_terminates && character == '.') || self.is_bare_boundary(character) {
                break;
            }
            self.advance(character);
        }
        if self.index == start {
            return Err(DatomError::InvalidBareSymbol {
                text: self.rest().chars().take(1).collect(),
            });
        }
        Ok(&self.source[start..self.index])
    }

    pub(crate) fn take_legacy_string(&mut self) -> Result<String, DatomError> {
        self.skip_trivia();
        if self.current() != Some('“') {
            return Err(self.expected_shape("a legacy string", ProtosShape::CurlyQuoteDelimited));
        }
        self.advance('“');
        let mut depth = 1usize;
        let mut text = String::new();
        while let Some(character) = self.current() {
            match character {
                '\\' => {
                    self.advance(character);
                    let escaped = self.current().ok_or(DatomError::Unterminated {
                        shape: ProtosShape::CurlyQuoteDelimited,
                    })?;
                    self.advance(escaped);
                    match escaped {
                        '\\' | '“' | '”' => text.push(escaped),
                        'n' => text.push('\n'),
                        'r' => text.push('\r'),
                        't' => text.push('\t'),
                        _ => return Err(DatomError::InvalidEscape { escaped }),
                    }
                }
                '“' => {
                    self.advance(character);
                    depth += 1;
                    text.push(character);
                }
                '”' => {
                    if depth == 1 {
                        return Ok(self.strip_common_indentation(&text));
                    }
                    self.advance(character);
                    depth -= 1;
                    text.push(character);
                }
                _ => {
                    self.advance(character);
                    text.push(character);
                }
            }
        }
        Err(DatomError::Unterminated {
            shape: ProtosShape::CurlyQuoteDelimited,
        })
    }

    pub(crate) fn consume_named_shape(
        &mut self,
        expected_head: &'static str,
        expected_shape: ProtosShape,
    ) -> Result<(), DatomError> {
        let probe = self.probe()?;
        if matches!(
            probe.shape(),
            ProtosShape::ParenthesisDelimited | ProtosShape::DotParenthesized
        ) {
            return Err(DatomError::ShapeNotYetRuled {
                shape: ProtosShape::ParenthesisDelimited,
            });
        }
        if probe.shape() != expected_shape {
            return Err(DatomError::UnexpectedShape {
                expected: expected_shape.description(),
                found: probe.shape(),
            });
        }
        if probe.head() != Some(expected_head) {
            return Err(DatomError::ExpectedHead {
                expected: expected_head,
                found: probe.head().map(str::to_owned),
            });
        }
        self.index += expected_head.len();
        self.consume_dot()?;
        match expected_shape {
            ProtosShape::DotBraced => self.consume_literal('{'),
            ProtosShape::DotBracketed => self.consume_literal('['),
            ProtosShape::DotParenthesized => self.consume_literal('('),
            ProtosShape::DotApplied => Ok(()),
            _ => Err(DatomError::UnexpectedShape {
                expected: "a dotted application",
                found: expected_shape,
            }),
        }
    }

    pub(crate) fn consume_head_dot(
        &mut self,
        expected_head: &'static str,
    ) -> Result<(), DatomError> {
        let probe = self.probe()?;
        if matches!(
            probe.shape(),
            ProtosShape::ParenthesisDelimited | ProtosShape::DotParenthesized
        ) {
            return Err(DatomError::ShapeNotYetRuled {
                shape: ProtosShape::ParenthesisDelimited,
            });
        }
        if !probe.shape().is_dotted() {
            return Err(DatomError::UnexpectedShape {
                expected: "a dotted single-payload variant",
                found: probe.shape(),
            });
        }
        if probe.head() != Some(expected_head) {
            return Err(DatomError::ExpectedHead {
                expected: expected_head,
                found: probe.head().map(str::to_owned),
            });
        }
        self.index += expected_head.len();
        self.consume_dot()?;
        Ok(())
    }

    pub(crate) fn consume_terminator(&mut self, shape: ProtosShape) -> Result<(), DatomError> {
        match shape {
            ProtosShape::CurlyQuoteDelimited => self.consume_literal('”'),
            ProtosShape::SquareBracketDelimited => self.consume_literal(']'),
            ProtosShape::BraceDelimited => self.consume_literal('}'),
            ProtosShape::DotApplied => self.consume_dot(),
            ProtosShape::ParenthesisDelimited => self.consume_literal(')'),
            _ => Err(DatomError::UnexpectedShape {
                expected: "a context terminator",
                found: shape,
            }),
        }
    }

    pub(crate) fn consume_opener(&mut self, shape: ProtosShape) -> Result<(), DatomError> {
        self.skip_trivia();
        if self.current() == Some('(') {
            return Err(DatomError::ShapeNotYetRuled {
                shape: ProtosShape::ParenthesisDelimited,
            });
        }
        match shape {
            ProtosShape::CurlyQuoteDelimited => self.consume_literal('“'),
            ProtosShape::SquareBracketDelimited => self.consume_literal('['),
            ProtosShape::BraceDelimited => self.consume_literal('{'),
            ProtosShape::ParenthesisDelimited => self.consume_literal('('),
            _ => Err(DatomError::UnexpectedShape {
                expected: "a context opener",
                found: shape,
            }),
        }
    }

    pub(crate) fn at_terminator(&mut self, shape: ProtosShape) -> bool {
        self.skip_trivia();
        matches!(
            (shape, self.current()),
            (ProtosShape::SquareBracketDelimited, Some(']'))
                | (ProtosShape::BraceDelimited, Some('}'))
                | (ProtosShape::CurlyQuoteDelimited, Some('”'))
                | (ProtosShape::ParenthesisDelimited, Some(')'))
                | (ProtosShape::DotApplied, Some('.'))
        )
    }

    pub(crate) fn finish(&mut self) -> Result<(), DatomError> {
        self.skip_trivia();
        if self.index == self.source.len() {
            Ok(())
        } else {
            Err(DatomError::TrailingInput {
                text: self.rest().chars().take(16).collect(),
            })
        }
    }

    fn probe_symbol(&self) -> Result<ShapeProbe<'source>, DatomError> {
        let start = self.index;
        let end = self.symbol_end()?;
        if end == start {
            return Err(DatomError::InvalidBareSymbol {
                text: self.rest().chars().take(1).collect(),
            });
        }
        let text = &self.source[start..end];
        match text.find('.') {
            Some(offset) => {
                let head = &text[..offset];
                let after_dot = start + offset + 1;
                let shape = match self.source[after_dot..].chars().next() {
                    Some('{') => ProtosShape::DotBraced,
                    Some('[') => ProtosShape::DotBracketed,
                    Some('(') => ProtosShape::DotParenthesized,
                    _ => ProtosShape::DotApplied,
                };
                Ok(ShapeProbe::new(shape, Some(head)))
            }
            None => Ok(ShapeProbe::new(ProtosShape::BareSymbol, Some(text))),
        }
    }

    fn expected_shape(&mut self, expected: &'static str, fallback: ProtosShape) -> DatomError {
        match self.probe() {
            Ok(probe) => DatomError::UnexpectedShape {
                expected,
                found: probe.shape(),
            },
            Err(_) => DatomError::UnexpectedShape {
                expected,
                found: fallback,
            },
        }
    }

    fn consume_literal(&mut self, expected: char) -> Result<(), DatomError> {
        self.skip_trivia();
        match self.current() {
            Some(found) if found == expected => {
                self.advance(found);
                Ok(())
            }
            Some(found) if found == '<' || found == '>' => Err(DatomError::AnglesAreNotDatom),
            Some('|') => Err(DatomError::PipeTextIsNotDatom),
            Some(found) => Err(DatomError::UnexpectedClosingDelimiter(found)),
            None => Err(DatomError::EndOfInput {
                expected: "a context terminator",
            }),
        }
    }

    fn consume_dot(&mut self) -> Result<(), DatomError> {
        self.skip_trivia();
        match self.current() {
            Some('.') => {
                self.advance('.');
                Ok(())
            }
            Some(found) => Err(DatomError::UnexpectedClosingDelimiter(found)),
            None => Err(DatomError::EndOfInput {
                expected: "a glued dot",
            }),
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while let Some(character) = self.current() {
                if !character.is_whitespace() {
                    break;
                }
                self.advance(character);
            }
            if self.rest().starts_with(";;") {
                while let Some(character) = self.current() {
                    self.advance(character);
                    if character == '\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn symbol_end(&self) -> Result<usize, DatomError> {
        let mut end = self.index;
        for (offset, character) in self.rest().char_indices() {
            if character == '<' || character == '>' {
                return Err(DatomError::AnglesAreNotDatom);
            }
            if self.is_bare_boundary(character) {
                break;
            }
            end = self.index + offset + character.len_utf8();
        }
        Ok(end)
    }

    fn strip_common_indentation(&self, raw: &str) -> String {
        let raw = raw
            .strip_prefix("\r\n")
            .or_else(|| raw.strip_prefix('\n'))
            .unwrap_or(raw);
        let mut lines: Vec<&str> = raw.lines().collect();
        while lines.first().is_some_and(|line| line.trim().is_empty()) {
            lines.remove(0);
        }
        while lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.pop();
        }
        let common = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.as_bytes()
                    .iter()
                    .take_while(|byte| **byte == b' ' || **byte == b'\t')
                    .count()
            })
            .min()
            .unwrap_or(0);
        lines
            .into_iter()
            .map(|line| {
                if line.trim().is_empty() {
                    ""
                } else {
                    &line[common..]
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn is_bare_boundary(&self, character: char) -> bool {
        character.is_whitespace()
            || matches!(
                character,
                '[' | ']' | '{' | '}' | '(' | ')' | '“' | '”' | '|' | ';'
            )
    }

    fn current(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn rest(&self) -> &'source str {
        &self.source[self.index..]
    }

    fn advance(&mut self, character: char) {
        self.index += character.len_utf8();
    }
}
