//! The fixed structural vocabulary used by the Datom walk.

use crate::DatomError;

/// Provisional naming note: `ProtosShape`, its flat variants, `ShapeProbe`,
/// and `ShapeDefined` are the currently smallest names for the open
/// Protos-shape naming fork.  The vocabulary itself is fixed; only these
/// spellings remain open.  The implementation bead records every name here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtosShape {
    /// A context-local unstructured symbol.  `BareSymbol` is provisional over
    /// the earlier `BareAtom` spelling.
    BareSymbol,
    CurlyQuoteDelimited,
    ParenthesisDelimited,
    SquareBracketDelimited,
    BraceDelimited,
    DotApplied,
    DotBraced,
    DotBracketed,
    DotParenthesized,
}

impl ProtosShape {
    /// Whether this shape begins with a glued dot after its head.
    pub(crate) fn is_dotted(self) -> bool {
        matches!(
            self,
            Self::DotApplied | Self::DotBraced | Self::DotBracketed | Self::DotParenthesized
        )
    }

    /// A stable diagnostic spelling for this fixed vocabulary.
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::BareSymbol => "bare symbol",
            Self::CurlyQuoteDelimited => "curly-quote-delimited text",
            Self::ParenthesisDelimited => "parenthesis-delimited Meaning",
            Self::SquareBracketDelimited => "square-bracket-delimited vector",
            Self::BraceDelimited => "brace-delimited record",
            Self::DotApplied => "dotted application",
            Self::DotBraced => "dotted brace application",
            Self::DotBracketed => "dotted bracket application",
            Self::DotParenthesized => "dotted parenthesis application",
        }
    }
}

/// The lexical evidence a context gives to a shape-discriminated type.
///
/// `ShapeProbe` is provisional naming under the same Protos-shape naming fork
/// as [`ProtosShape`].  It deliberately contains no value and performs no
/// parsing: discrimination receives only the met shape and optional head.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeProbe<'source> {
    shape: ProtosShape,
    head: Option<&'source str>,
}

impl<'source> ShapeProbe<'source> {
    pub(crate) fn new(shape: ProtosShape, head: Option<&'source str>) -> Self {
        Self { shape, head }
    }

    /// The fixed shape met at this position.
    pub fn shape(self) -> ProtosShape {
        self.shape
    }

    /// The optional dotted-application head, without its glued dot.
    pub fn head(self) -> Option<&'source str> {
        self.head
    }
}

/// Selects a variant from a met standard shape.
///
/// Provisional name: `ShapeDefined` is one candidate in the unresolved trait
/// naming fork.  This trait is intentionally discrimination-only: an impl may
/// select its `Selection`, but it must not read an interior.  The selected
/// type's [`crate::DatomDecode`] context owns that later read.
pub trait ShapeDefined: Sized {
    /// A type-local variant or selection token; it must not be a parsed value.
    type Selection;

    /// The standard shapes this type recognizes at this one position.
    fn shapes() -> &'static [ProtosShape];

    /// Select only the type-local variant announced by `met`.
    fn select(met: ShapeProbe<'_>) -> Result<Self::Selection, DatomError>;
}
