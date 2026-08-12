//! Direct typed Datom contexts and their shared structural walk drivers.

use std::collections::BTreeMap;

use crate::DatomError;
use crate::expectation::{ProtosShape, ShapeProbe};
use crate::parser::Parser;
use crate::pretty::CanonicalText;

/// Provisional naming note: `DatomDecode` and `DatomEncode` deliberately keep
/// the read/write-pair fork open.  The alternative is one inseparable
/// transcoding trait.  This pair is only the smallest useful shape for this
/// ruled implementation and must not settle that fork.
pub trait DatomDecode: Sized {
    /// Read this type's own context from the active structural walk.
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError>;
}

/// See [`DatomDecode`]'s provisional-pair note.
pub trait DatomEncode {
    /// Project this type's own context into the active structural walk.
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError>;
}

/// Provisional shared-framework hook for brace records.
///
/// The generated-per-schema versus shared-framework codec fork is unresolved.
/// `DatomRecord` is deliberately a small shared-framework adapter, not a
/// decision that generated contexts will not replace it.
pub trait DatomRecord: Sized {
    /// The ruled bare record head.
    const NAME: &'static str;

    /// Read only this record's positional interior, after `NAME.{` has opened.
    fn read_fields(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError>;

    /// Write only this record's positional interior, after `NAME.{` has opened.
    fn write_fields(&self, walk: &mut EncodeWalk) -> Result<(), DatomError>;
}

/// Provisional name for a context tracked by the one walk driver.  The names
/// `ContextName`, `WalkFrame`, `WalkEvent`, `DecodeWalk`, and `EncodeWalk` are
/// all implementation spellings under the open main-type naming fork.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextName {
    Document,
    Value(&'static str),
    Record(&'static str),
    Vector,
    Map,
    DottedKey,
    LegacyString,
    Variant(&'static str),
}

/// A witness emitted by the one shared walk driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WalkEvent {
    Enter {
        context: ContextName,
        position: usize,
    },
    Exit {
        context: ContextName,
        position: usize,
    },
    Resume {
        context: ContextName,
        position: usize,
    },
}

/// A direct source-bound typed decoder.
///
/// Provisional name: `DatomSource` is only this implementation's source
/// carrier while the shared-framework versus generated-codec fork stays open.
pub struct DatomSource<'source> {
    text: &'source str,
}

impl<'source> DatomSource<'source> {
    /// Bind a Datom source without parsing it into a generic document.
    pub fn new(text: &'source str) -> Self {
        Self { text }
    }

    /// Decode one expected type directly from the source.
    pub fn decode<T: DatomDecode>(&self) -> Result<T, DatomError> {
        self.decode_with_trace().map(|(value, _)| value)
    }

    /// Decode one expected type and retain the driver's context-stack witness.
    pub fn decode_with_trace<T: DatomDecode>(&self) -> Result<(T, Vec<WalkEvent>), DatomError> {
        let mut walk = DecodeWalk::new(self.text);
        let value = walk.value::<T>()?;
        walk.finish()?;
        Ok((value, walk.into_events()))
    }
}

/// Canonical Datom text projected from one expected Rust value.
///
/// Provisional name: `DatomText` is the matching output carrier under the
/// same unresolved codec-framework fork as [`DatomSource`].
pub struct DatomText {
    text: String,
}

impl DatomText {
    /// Encode one expected Rust value without constructing a document tree.
    pub fn encode<T: DatomEncode>(value: &T) -> Result<Self, DatomError> {
        Self::encode_with_trace(value).map(|(text, _)| text)
    }

    /// Encode one expected Rust value and retain the shared walk witness.
    pub fn encode_with_trace<T: DatomEncode>(
        value: &T,
    ) -> Result<(Self, Vec<WalkEvent>), DatomError> {
        let mut walk = EncodeWalk::new();
        walk.value(value)?;
        let text = walk.finish()?;
        Ok((Self { text }, walk.into_events()))
    }

    /// The canonical Datom source.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Take ownership of the canonical Datom source.
    pub fn into_string(self) -> String {
        self.text
    }
}

/// The reading half of the one shared context-stack driver.
pub struct DecodeWalk<'source> {
    parser: Parser<'source>,
    structural: StructuralWalk,
}

impl<'source> DecodeWalk<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            parser: Parser::new(source),
            structural: StructuralWalk::new(),
        }
    }

    /// Read one value and then resume the parent at its following position.
    pub fn value<T: DatomDecode>(&mut self) -> Result<T, DatomError> {
        let value = T::decode(self)?;
        self.structural.resume_current();
        Ok(value)
    }

    /// Enter a shape-selected type's context without advancing the enclosing
    /// positional frame.  The enclosing [`Self::value`] owns that one resume.
    pub fn selected<T: DatomDecode>(&mut self) -> Result<T, DatomError> {
        T::decode(self)
    }

    /// Inspect only the shape currently meaningful to this context.
    pub fn probe(&mut self) -> Result<ShapeProbe<'source>, DatomError> {
        self.parser.probe()
    }

    /// Read a context-local bare symbol.  A dot terminates only when the
    /// current scalar context itself has inherited the ruled map-key end.
    pub fn bare(&mut self) -> Result<&'source str, DatomError> {
        let dot_terminates = self
            .structural
            .current_terminator()
            .is_some_and(|terminator| terminator == ProtosShape::DotApplied);
        if !dot_terminates
            && matches!(
                self.parser.probe()?.shape(),
                ProtosShape::ParenthesisDelimited | ProtosShape::DotParenthesized
            )
        {
            return Err(DatomError::ShapeNotYetRuled {
                shape: ProtosShape::ParenthesisDelimited,
            });
        }
        self.parser.take_bare(dot_terminates)
    }

    /// Enter a scalar type context with no structural terminator.
    pub fn scalar<T>(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        let terminator = (self.structural.current_context() == Some(ContextName::DottedKey))
            .then_some(ProtosShape::DotApplied);
        self.with_context(ContextName::Value(name), terminator, interior)
    }

    /// Enter the curly legacy-string context.  Its only terminator is `”`;
    /// braces, brackets, and curly openers are text or nested text within it.
    pub fn legacy_string(&mut self) -> Result<String, DatomError> {
        self.with_context(
            ContextName::LegacyString,
            Some(ProtosShape::CurlyQuoteDelimited),
            |walk| walk.parser.take_legacy_string(),
        )
    }

    /// Enter one brace record context.
    pub fn record<T>(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(
            ContextName::Record(name),
            Some(ProtosShape::BraceDelimited),
            |walk| {
                walk.parser
                    .consume_named_shape(name, ProtosShape::DotBraced)?;
                interior(walk)
            },
        )
    }

    /// Enter one square-bracket vector context.
    pub fn vector<T>(
        &mut self,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(
            ContextName::Vector,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.parser
                    .consume_opener(ProtosShape::SquareBracketDelimited)?;
                interior(walk)
            },
        )
    }

    /// Enter a shape-headed vector such as `Tags.[…]`.  This is a local type
    /// context, not a second parser or a generic document-node interpretation.
    pub fn named_vector<T>(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(
            ContextName::Vector,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.parser
                    .consume_named_shape(name, ProtosShape::DotBracketed)?;
                interior(walk)
            },
        )
    }

    /// Enter the ruled `Map.[…]` context.
    pub fn map<T>(
        &mut self,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(
            ContextName::Map,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.parser
                    .consume_named_shape("Map", ProtosShape::DotBracketed)?;
                interior(walk)
            },
        )
    }

    /// Read the key half of one ruled `key.value` map entry.  The shared driver
    /// gives the child scalar context its dot terminator, then pops the key
    /// frame before the value reads.
    pub fn dotted_key<T>(
        &mut self,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(ContextName::DottedKey, None, interior)
    }

    /// Enter one single-payload variant after its glued head and dot.
    pub fn variant<T>(
        &mut self,
        head: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.with_context(ContextName::Variant(head), None, |walk| {
            walk.parser.consume_head_dot(head)?;
            interior(walk)
        })
    }

    /// Whether the active context is immediately followed by its terminator.
    pub fn at_terminator(&mut self, terminator: ProtosShape) -> bool {
        self.parser.at_terminator(terminator)
    }

    fn with_context<T>(
        &mut self,
        context: ContextName,
        terminator: Option<ProtosShape>,
        interior: impl FnOnce(&mut Self) -> Result<T, DatomError>,
    ) -> Result<T, DatomError> {
        self.structural.enter(context, terminator);
        let result = interior(self).and_then(|value| {
            if let Some(terminator) = terminator {
                self.parser.consume_terminator(terminator)?;
            }
            Ok(value)
        });
        self.structural.exit();
        result
    }

    fn finish(&mut self) -> Result<(), DatomError> {
        self.parser.finish()?;
        self.structural.finish()?;
        Ok(())
    }

    fn into_events(self) -> Vec<WalkEvent> {
        self.structural.into_events()
    }
}

/// The writing half of the one shared structural walk driver.
pub struct EncodeWalk {
    output: String,
    structural: StructuralWalk,
}

impl EncodeWalk {
    fn new() -> Self {
        Self {
            output: String::new(),
            structural: StructuralWalk::new(),
        }
    }

    /// Write one value and resume the parent at its following position.
    pub fn value<T: DatomEncode>(&mut self, value: &T) -> Result<(), DatomError> {
        self.separate_current();
        self.unseparated_value(value)
    }

    /// Project a shape-selected type's context without separating or advancing
    /// the enclosing positional frame.  The enclosing [`Self::value`] owns
    /// that one resume.
    pub fn selected<T: DatomEncode>(&mut self, value: &T) -> Result<(), DatomError> {
        value.encode(self)
    }

    /// Write unseparated source owned by the active type context.
    pub fn raw(&mut self, source: &str) {
        self.output.push_str(source);
    }

    /// Enter a scalar type context with no structural terminator.
    pub fn scalar(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(ContextName::Value(name), None, interior)
    }

    /// Enter and write one curly legacy-string context.
    pub fn legacy_string(&mut self, value: &str) -> Result<(), DatomError> {
        self.with_context(
            ContextName::LegacyString,
            Some(ProtosShape::CurlyQuoteDelimited),
            |walk| {
                CanonicalText::new(value).push_legacy_escaped(&mut walk.output);
                walk.output.pop();
                Ok(())
            },
        )
    }

    /// Enter and write one brace record context.
    pub fn record(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(
            ContextName::Record(name),
            Some(ProtosShape::BraceDelimited),
            |walk| {
                walk.raw(name);
                walk.raw(".{");
                interior(walk)
            },
        )
    }

    /// Enter and write one square-bracket vector context.
    pub fn vector(
        &mut self,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(
            ContextName::Vector,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.raw("[");
                interior(walk)
            },
        )
    }

    /// Enter and write one shape-headed vector such as `Tags.[…]`.
    pub fn named_vector(
        &mut self,
        name: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(
            ContextName::Vector,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.raw(name);
                walk.raw(".[");
                interior(walk)
            },
        )
    }

    /// Enter and write one ruled `Map.[…]` context.
    pub fn map(
        &mut self,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(
            ContextName::Map,
            Some(ProtosShape::SquareBracketDelimited),
            |walk| {
                walk.raw("Map.[");
                interior(walk)
            },
        )
    }

    /// Write one glued `key.value` entry in the current map context.
    pub fn map_entry<K: DatomEncode, V: DatomEncode>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), DatomError> {
        self.separate_current();
        self.structural.enter(ContextName::DottedKey, None);
        self.unseparated_value(key)?;
        self.raw(".");
        self.structural.exit();
        self.unseparated_value(value)
    }

    /// Enter and write one single-payload variant after its glued head and dot.
    pub fn variant(
        &mut self,
        head: &'static str,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.with_context(ContextName::Variant(head), None, |walk| {
            walk.raw(head);
            walk.raw(".");
            interior(walk)
        })
    }

    fn with_context(
        &mut self,
        context: ContextName,
        terminator: Option<ProtosShape>,
        interior: impl FnOnce(&mut Self) -> Result<(), DatomError>,
    ) -> Result<(), DatomError> {
        self.structural.enter(context, terminator);
        let result = interior(self);
        if result.is_ok() {
            if let Some(terminator) = terminator {
                self.write_terminator(terminator);
            }
        }
        self.structural.exit();
        result
    }

    fn separate_current(&mut self) {
        if self
            .structural
            .current_position()
            .is_some_and(|position| position > 0)
        {
            self.output.push(' ');
        }
    }

    fn write_terminator(&mut self, terminator: ProtosShape) {
        match terminator {
            ProtosShape::CurlyQuoteDelimited => self.output.push('”'),
            ProtosShape::SquareBracketDelimited => self.output.push(']'),
            ProtosShape::BraceDelimited => self.output.push('}'),
            ProtosShape::DotApplied => self.output.push('.'),
            ProtosShape::ParenthesisDelimited => self.output.push(')'),
            _ => {}
        }
    }

    fn finish(&mut self) -> Result<String, DatomError> {
        self.structural.finish()?;
        Ok(std::mem::take(&mut self.output))
    }

    fn unseparated_value<T: DatomEncode>(&mut self, value: &T) -> Result<(), DatomError> {
        self.selected(value)?;
        self.structural.resume_current();
        Ok(())
    }

    fn into_events(self) -> Vec<WalkEvent> {
        self.structural.into_events()
    }
}

/// Provisional shared driver name: `StructuralWalk` is the sole owner of
/// context entry, exit, parent positions, resumption, and trace events for
/// both directions.  It does not settle the shared-framework fork; it is the
/// smallest adapter proving one structural walk has two directions.
struct StructuralWalk {
    frames: Vec<WalkFrame>,
    events: Vec<WalkEvent>,
}

impl StructuralWalk {
    fn new() -> Self {
        let document = WalkFrame::new(ContextName::Document, None);
        Self {
            frames: vec![document],
            events: vec![WalkEvent::Enter {
                context: ContextName::Document,
                position: 0,
            }],
        }
    }

    fn enter(&mut self, context: ContextName, terminator: Option<ProtosShape>) {
        self.frames.push(WalkFrame::new(context, terminator));
        self.events.push(WalkEvent::Enter {
            context,
            position: 0,
        });
    }

    fn exit(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.events.push(WalkEvent::Exit {
                context: frame.context,
                position: frame.position,
            });
        }
    }

    fn resume_current(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.position += 1;
            self.events.push(WalkEvent::Resume {
                context: frame.context,
                position: frame.position,
            });
        }
    }

    fn current_context(&self) -> Option<ContextName> {
        self.frames.last().map(|frame| frame.context)
    }

    fn current_terminator(&self) -> Option<ProtosShape> {
        self.frames.last().and_then(|frame| frame.terminator)
    }

    fn current_position(&self) -> Option<usize> {
        self.frames.last().map(|frame| frame.position)
    }

    fn finish(&mut self) -> Result<(), DatomError> {
        if self.frames.len() != 1 {
            return Err(DatomError::TrailingInput {
                text: "unclosed structural context".to_owned(),
            });
        }
        self.exit();
        Ok(())
    }

    fn into_events(self) -> Vec<WalkEvent> {
        self.events
    }
}

/// Provisional internal stack frame; [`StructuralWalk`] owns its lifecycle.
struct WalkFrame {
    context: ContextName,
    position: usize,
    terminator: Option<ProtosShape>,
}

impl WalkFrame {
    fn new(context: ContextName, terminator: Option<ProtosShape>) -> Self {
        Self {
            context,
            position: 0,
            terminator,
        }
    }
}

impl<T: DatomRecord> DatomDecode for T {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.record(T::NAME, T::read_fields)
    }
}

impl<T: DatomRecord> DatomEncode for T {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.record(T::NAME, |walk| self.write_fields(walk))
    }
}

impl DatomDecode for String {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.scalar("String", |walk| match walk.probe()?.shape() {
            ProtosShape::CurlyQuoteDelimited => walk.legacy_string(),
            _ if walk
                .structural
                .current_terminator()
                .is_some_and(|terminator| terminator == ProtosShape::DotApplied) =>
            {
                Ok(walk.bare()?.to_owned())
            }
            ProtosShape::BareSymbol => Ok(walk.bare()?.to_owned()),
            ProtosShape::ParenthesisDelimited | ProtosShape::DotParenthesized => {
                Err(DatomError::ShapeNotYetRuled {
                    shape: ProtosShape::ParenthesisDelimited,
                })
            }
            found => Err(DatomError::UnexpectedShape {
                expected: "a Datom string",
                found,
            }),
        })
    }
}

impl DatomEncode for String {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.scalar("String", |walk| {
            let canonical = CanonicalText::new(self);
            if canonical.is_bare() {
                walk.raw(self);
                Ok(())
            } else {
                walk.legacy_string(self)
            }
        })
    }
}

impl DatomDecode for bool {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.scalar("bool", |walk| match walk.bare()? {
            "True" => Ok(true),
            "False" => Ok(false),
            text => Err(DatomError::InvalidScalar {
                expected: "boolean",
                text: text.to_owned(),
            }),
        })
    }
}

impl DatomEncode for bool {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.scalar("bool", |walk| {
            walk.raw(if *self { "True" } else { "False" });
            Ok(())
        })
    }
}

macro_rules! ruled_integers {
    ($($integer:ty),+ $(,)?) => {
        $(
            impl DatomDecode for $integer {
                fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
                    walk.scalar(stringify!($integer), |walk| {
                        let text = walk.bare()?;
                        text.parse::<$integer>().map_err(|_| DatomError::InvalidScalar {
                            expected: stringify!($integer),
                            text: text.to_owned(),
                        })
                    })
                }
            }

            impl DatomEncode for $integer {
                fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
                    walk.scalar(stringify!($integer), |walk| {
                        walk.raw(&self.to_string());
                        Ok(())
                    })
                }
            }
        )+
    };
}

// Provisional internal macro name; it is merely the repeated scalar context,
// not another codec abstraction or a decision in the framework fork.
ruled_integers!(u8, u16, u32, u64, i8, i16, i32, i64);

impl DatomDecode for f64 {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.scalar("f64", |walk| {
            let text = walk.bare()?;
            if !text.contains('.') {
                return Err(DatomError::InvalidScalar {
                    expected: "dotted float",
                    text: text.to_owned(),
                });
            }
            text.parse::<f64>().map_err(|_| DatomError::InvalidScalar {
                expected: "dotted float",
                text: text.to_owned(),
            })
        })
    }
}

impl DatomEncode for f64 {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        if !self.is_finite() {
            return Err(DatomError::NonFiniteFloat);
        }
        walk.scalar("f64", |walk| {
            let mut text = self.to_string();
            if !text.contains(['.', 'e', 'E']) {
                text.push_str(".0");
            }
            walk.raw(&text);
            Ok(())
        })
    }
}

impl<T: DatomDecode> DatomDecode for Vec<T> {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.vector(|walk| {
            let mut values = Vec::new();
            while !walk.at_terminator(ProtosShape::SquareBracketDelimited) {
                values.push(walk.value::<T>()?);
            }
            Ok(values)
        })
    }
}

impl<T: DatomEncode> DatomEncode for Vec<T> {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.vector(|walk| {
            for value in self {
                walk.value(value)?;
            }
            Ok(())
        })
    }
}

impl<K: DatomDecode + Ord, V: DatomDecode> DatomDecode for BTreeMap<K, V> {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.map(|walk| {
            let mut entries = BTreeMap::new();
            while !walk.at_terminator(ProtosShape::SquareBracketDelimited) {
                let key = walk.dotted_key(|walk| walk.value::<K>())?;
                let value = walk.value::<V>()?;
                entries.insert(key, value);
            }
            Ok(entries)
        })
    }
}

impl<K: DatomEncode + Ord, V: DatomEncode> DatomEncode for BTreeMap<K, V> {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.map(|walk| {
            for (key, value) in self {
                walk.map_entry(key, value)?;
            }
            Ok(())
        })
    }
}

impl<T: DatomDecode> DatomDecode for Option<T> {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.scalar("Option", |walk| {
            let probe = walk.probe()?;
            match (probe.shape(), probe.head()) {
                (ProtosShape::BareSymbol, Some("None")) => {
                    let _ = walk.bare()?;
                    Ok(None)
                }
                (shape, Some("Some")) if shape.is_dotted() => {
                    walk.variant("Some", |walk| walk.value::<T>().map(Some))
                }
                (ProtosShape::ParenthesisDelimited | ProtosShape::DotParenthesized, _) => {
                    Err(DatomError::ShapeNotYetRuled {
                        shape: ProtosShape::ParenthesisDelimited,
                    })
                }
                (found, _) => Err(DatomError::UnexpectedShape {
                    expected: "None or Some.value",
                    found,
                }),
            }
        })
    }
}

impl<T: DatomEncode> DatomEncode for Option<T> {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.scalar("Option", |walk| match self {
            None => {
                walk.raw("None");
                Ok(())
            }
            Some(value) => walk.variant("Some", |walk| walk.value(value)),
        })
    }
}
