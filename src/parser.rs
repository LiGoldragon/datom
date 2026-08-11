use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePosition {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    source: String,
    root_objects: Vec<Block>,
}

impl Document {
    pub fn parse(source: impl Into<String>) -> Result<Self, DatomError> {
        Self::parse_with_mode(source, ParseMode::Default)
    }

    /// Parse with an explicitly selected grammar extension. The default grammar
    /// keeps `(| … |)` as pipe text; structural-pipe consumers must opt in.
    pub fn parse_with_mode(source: impl Into<String>, mode: ParseMode) -> Result<Self, DatomError> {
        let source = source.into();
        let mut parser = Parser::new(&source, mode);
        let root_objects = parser.parse_document()?;
        Ok(Self {
            source,
            root_objects,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn root_objects(&self) -> &[Block] {
        &self.root_objects
    }

    pub fn holds_root_objects(&self) -> usize {
        self.root_objects.len()
    }

    pub fn root_object_at(&self, index: usize) -> Option<&Block> {
        self.root_objects.get(index)
    }

    pub fn structure_header(&self) -> StructureHeader {
        let mut builder = StructureHeaderBuilder::new();
        builder.push_document(self);
        builder.finish()
    }
}

/// Optional grammar extensions for consumers that carry a legacy structural
/// shape as data. This never changes [`Document::parse`]'s default grammar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParseMode {
    #[default]
    Default,
    StructuralPipe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    Delimited {
        delimiter: Delimiter,
        span: SourceSpan,
        root_objects: Vec<Block>,
    },
    /// A dot-application: a head object bound to the payload that immediately
    /// follows a glued period, as one node. `Head.Payload` (bare-atom payload),
    /// `Head.( … )` / `Head.[ … ]` / `Head.{ … }` (delimited payload), and
    /// `Head.( | … | )` (multiline-string payload) all parse to this. The dot is
    /// right-associative, so `A.B.C` is `App(A, App(B, C))`: the head is always
    /// the leftmost single segment and the payload is the remainder, itself an
    /// application when the remainder is dotted. The head is a single primary
    /// object — an atom or a delimited block — and the payload is one object.
    /// This is the raw-layer, expectation-agnostic binding: the period is a
    /// structural operator here, not atom text.
    Application {
        span: SourceSpan,
        head: Box<Block>,
        payload: Box<Block>,
    },
    PipeText(PipeText),
    Atom(Atom),
}

impl Block {
    pub fn source_span(&self) -> SourceSpan {
        match self {
            Self::Delimited { span, .. } => *span,
            Self::Application { span, .. } => *span,
            Self::PipeText(pipe_text) => pipe_text.span,
            Self::Atom(atom) => atom.span,
        }
    }

    pub fn reemit<'source>(&self, source: &'source str) -> &'source str {
        let span = self.source_span();
        &source[span.start.byte_offset..span.end.byte_offset]
    }

    pub fn is_parenthesis(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::Parenthesis,
                ..
            }
        )
    }

    pub fn is_square_bracket(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::SquareBracket,
                ..
            }
        )
    }

    pub fn is_brace(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::Brace,
                ..
            }
        )
    }

    pub fn is_pipe_parenthesis(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::PipeParenthesis,
                ..
            }
        )
    }

    pub fn is_pipe_brace(&self) -> bool {
        matches!(
            self,
            Self::Delimited {
                delimiter: Delimiter::PipeBrace,
                ..
            }
        )
    }

    pub fn is_pipe_text(&self) -> bool {
        matches!(self, Self::PipeText(_))
    }

    pub fn is_atom(&self) -> bool {
        matches!(self, Self::Atom(_))
    }

    pub fn is_application(&self) -> bool {
        matches!(self, Self::Application { .. })
    }

    pub fn is_delimited_with(&self, delimiter: Delimiter) -> bool {
        matches!(self, Self::Delimited { delimiter: found, .. } if *found == delimiter)
    }

    /// The head and payload of a dot-application, or `None` for any other
    /// block. The head is the object left of the period (itself an application
    /// for a dotted chain); the payload is the single primary object right of
    /// the period.
    pub fn as_application(&self) -> Option<(&Block, &Block)> {
        match self {
            Self::Application { head, payload, .. } => Some((head, payload)),
            Self::Delimited { .. } | Self::PipeText(_) | Self::Atom(_) => None,
        }
    }

    /// The head object of a dot-application — the object left of the period.
    pub fn application_head(&self) -> Option<&Block> {
        self.as_application().map(|(head, _)| head)
    }

    /// The payload object of a dot-application — the object right of the period.
    pub fn application_payload(&self) -> Option<&Block> {
        self.as_application().map(|(_, payload)| payload)
    }

    pub fn as_delimited(&self, delimiter: Delimiter) -> Option<&[Block]> {
        match self {
            Self::Delimited {
                delimiter: found,
                root_objects,
                ..
            } if *found == delimiter => Some(root_objects),
            Self::Delimited { .. }
            | Self::Application { .. }
            | Self::PipeText(_)
            | Self::Atom(_) => None,
        }
    }

    pub fn holds_root_objects(&self) -> usize {
        match self {
            Self::Delimited { root_objects, .. } => root_objects.len(),
            Self::Application { .. } | Self::PipeText(_) | Self::Atom(_) => 0,
        }
    }

    pub fn holds_single_root_object(&self) -> bool {
        self.holds_root_objects() == 1
    }

    pub fn holds_two_root_objects(&self) -> bool {
        self.holds_root_objects() == 2
    }

    pub fn root_object_at(&self, index: usize) -> Option<&Block> {
        match self {
            Self::Delimited { root_objects, .. } => root_objects.get(index),
            Self::Application { .. } | Self::PipeText(_) | Self::Atom(_) => None,
        }
    }

    pub fn root_objects(&self) -> &[Block] {
        match self {
            Self::Delimited { root_objects, .. } => root_objects,
            Self::Application { .. } | Self::PipeText(_) | Self::Atom(_) => &[],
        }
    }

    pub fn atom(&self) -> Option<&Atom> {
        match self {
            Self::Atom(atom) => Some(atom),
            Self::Delimited { .. } | Self::Application { .. } | Self::PipeText(_) => None,
        }
    }

    pub fn qualifies_as_symbol(&self) -> bool {
        self.atom().is_some_and(Atom::qualifies_as_symbol)
    }

    pub fn qualifies_as_pascal_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_pascal_case_symbol)
    }

    pub fn qualifies_as_camel_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_camel_case_symbol)
    }

    pub fn qualifies_as_kebab_case_symbol(&self) -> bool {
        self.atom()
            .is_some_and(Atom::qualifies_as_kebab_case_symbol)
    }

    pub fn demote_to_string(&self) -> Option<&str> {
        match self {
            Self::Atom(atom) => Some(atom.text.as_str()),
            Self::PipeText(pipe_text) => Some(pipe_text.text.as_str()),
            Self::Delimited { .. } | Self::Application { .. } => None,
        }
    }

    /// The flat dotted text of an atom or a dotted chain of atoms, joining the
    /// segments with periods: `Atom("42")` -> `"42"`, `App(rustfmt, skip)` ->
    /// `"rustfmt.skip"`, `App(App(-122, 3), ...)` and so on. `None` when any
    /// segment is a delimited or pipe-text block, since those carry no flat
    /// text form. This is how an expected-type reader recovers a literal that
    /// the structural period split apart — a dotted Rust path, a float whose
    /// fractional period is a structural dot, or a dotted map value.
    pub fn dotted_text(&self) -> Option<String> {
        match self {
            Self::Atom(atom) => Some(atom.text.clone()),
            Self::Application { head, payload, .. } => {
                let head = head.dotted_text()?;
                let payload = payload.dotted_text()?;
                Some(format!("{head}.{payload}"))
            }
            Self::Delimited { .. } | Self::PipeText(_) => None,
        }
    }

    pub fn structure_shape(&self) -> StructureShape {
        match self {
            Self::Delimited {
                delimiter: Delimiter::Parenthesis,
                ..
            } => StructureShape::Parenthesis,
            Self::Delimited {
                delimiter: Delimiter::SquareBracket,
                ..
            } => StructureShape::SquareBracket,
            Self::Delimited {
                delimiter: Delimiter::Brace,
                ..
            } => StructureShape::Brace,
            Self::Delimited {
                delimiter: Delimiter::PipeParenthesis,
                ..
            } => StructureShape::PipeParenthesis,
            Self::Delimited {
                delimiter: Delimiter::PipeBrace,
                ..
            } => StructureShape::PipeBrace,
            Self::Application { .. } => StructureShape::Application,
            Self::PipeText(_) => StructureShape::PipeText,
            Self::Atom(_) => StructureShape::Atom,
        }
    }

    pub fn structure_slot(&self) -> StructureSlot {
        StructureSlot::new(self.structure_shape(), self.holds_root_objects())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delimiter {
    Parenthesis,
    SquareBracket,
    Brace,
    PipeParenthesis,
    PipeBrace,
}

impl Delimiter {
    pub fn opening_text(self) -> &'static str {
        match self {
            Self::Parenthesis => "(",
            Self::SquareBracket => "[",
            Self::Brace => "{",
            Self::PipeParenthesis => "(|",
            Self::PipeBrace => "{|",
        }
    }

    pub fn closing_text(self) -> &'static str {
        match self {
            Self::Parenthesis => ")",
            Self::SquareBracket => "]",
            Self::Brace => "}",
            Self::PipeParenthesis => "|)",
            Self::PipeBrace => "|}",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::PipeParenthesis => "pipe parenthesis",
            Self::PipeBrace => "pipe brace",
        }
    }

    pub fn wrap(self, children: impl IntoIterator<Item = String>) -> String {
        let children = children.into_iter().collect::<Vec<_>>();
        format!(
            "{}{}{}",
            self.opening_text(),
            children.join(" "),
            self.closing_text()
        )
    }

    fn closing(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::SquareBracket => ']',
            Self::Brace => '}',
            Self::PipeParenthesis => ')',
            Self::PipeBrace => '}',
        }
    }

    fn from_opening(character: char) -> Option<Self> {
        match character {
            '(' => Some(Self::Parenthesis),
            '[' => Some(Self::SquareBracket),
            '{' => Some(Self::Brace),
            _ => None,
        }
    }

    fn from_closing(character: char) -> Option<Self> {
        match character {
            ')' => Some(Self::Parenthesis),
            ']' => Some(Self::SquareBracket),
            '}' => Some(Self::Brace),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureHeader {
    slots: Vec<StructureSlot>,
}

impl StructureHeader {
    pub const MAXIMUM_SLOTS: usize = 8;

    pub fn slots(&self) -> &[StructureSlot] {
        &self.slots
    }

    pub fn packed_word(&self) -> u64 {
        let mut word = 0_u64;
        for (index, slot) in self.slots.iter().enumerate().take(Self::MAXIMUM_SLOTS) {
            word |= u64::from(slot.packed_byte()) << (index * 8);
        }
        word
    }

    pub fn from_packed_word(word: u64) -> Self {
        let mut slots = Vec::new();
        for index in 0..Self::MAXIMUM_SLOTS {
            let byte = ((word >> (index * 8)) & 0xff) as u8;
            if byte == 0 && index > 0 {
                break;
            }
            slots.push(StructureSlot::from_packed_byte(byte));
        }
        Self { slots }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructureSlot {
    shape: StructureShape,
    child_count: u8,
}

impl StructureSlot {
    pub fn new(shape: StructureShape, child_count: usize) -> Self {
        if child_count > Self::MAXIMUM_CHILD_COUNT {
            return Self::overflow();
        }
        Self {
            shape,
            child_count: child_count as u8,
        }
    }

    const MAXIMUM_CHILD_COUNT: usize = 15;

    pub fn overflow() -> Self {
        Self {
            shape: StructureShape::Unknown,
            child_count: Self::MAXIMUM_CHILD_COUNT as u8,
        }
    }

    pub fn shape(&self) -> StructureShape {
        self.shape
    }

    pub fn child_count(&self) -> u8 {
        self.child_count
    }

    pub fn packed_byte(&self) -> u8 {
        (self.shape.code() << 4) | self.child_count
    }

    pub fn from_packed_byte(byte: u8) -> Self {
        Self {
            shape: StructureShape::from_code(byte >> 4),
            child_count: byte & 0x0f,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructureShape {
    Document,
    Atom,
    Parenthesis,
    SquareBracket,
    Brace,
    PipeText,
    Application,
    PipeParenthesis,
    PipeBrace,
    Unknown,
}

impl StructureShape {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Atom => "atom",
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
            Self::PipeText => "pipe text",
            Self::Application => "application",
            Self::PipeParenthesis => "pipe parenthesis",
            Self::PipeBrace => "pipe brace",
            Self::Unknown => "unknown",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Document => 0,
            Self::Atom => 1,
            Self::Parenthesis => 2,
            Self::SquareBracket => 3,
            Self::Brace => 4,
            Self::PipeText => 5,
            Self::Application => 6,
            Self::PipeParenthesis => 7,
            Self::PipeBrace => 8,
            Self::Unknown => 15,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Document,
            1 => Self::Atom,
            2 => Self::Parenthesis,
            3 => Self::SquareBracket,
            4 => Self::Brace,
            5 => Self::PipeText,
            6 => Self::Application,
            7 => Self::PipeParenthesis,
            8 => Self::PipeBrace,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
struct StructureHeaderBuilder {
    slots: Vec<StructureSlot>,
}

impl StructureHeaderBuilder {
    fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn push_document(&mut self, document: &Document) {
        self.push_slot(StructureSlot::new(
            StructureShape::Document,
            document.holds_root_objects(),
        ));
        for root_object in document.root_objects() {
            self.push_block(root_object, 1);
        }
    }

    fn push_block(&mut self, block: &Block, depth: usize) {
        if depth > 2 {
            return;
        }
        if !self.push_slot(block.structure_slot()) {
            return;
        }
        if depth == 2 {
            return;
        }
        for child in block.root_objects() {
            self.push_block(child, depth + 1);
        }
    }

    fn push_slot(&mut self, slot: StructureSlot) -> bool {
        if self.slots.len() < StructureHeader::MAXIMUM_SLOTS {
            self.slots.push(slot);
            true
        } else {
            if let Some(last) = self.slots.last_mut() {
                *last = StructureSlot::overflow();
            }
            false
        }
    }

    fn finish(self) -> StructureHeader {
        StructureHeader { slots: self.slots }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeText {
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Atom {
    text: String,
    span: SourceSpan,
}

/// A lightweight classification compatibility surface for consumers that need
/// to decide whether an owned string can be emitted as a bare Datom atom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtomClassification {
    SymbolCandidate,
    Other,
}

impl AtomClassification {
    pub fn classify(text: &str) -> Self {
        if !text.is_empty()
            && text.chars().all(|character| {
                !character.is_whitespace()
                    && !matches!(character, '"' | '.' | '(' | ')' | '[' | ']' | '{' | '}')
            })
        {
            Self::SymbolCandidate
        } else {
            Self::Other
        }
    }
}

impl Atom {
    pub(crate) fn new(text: String, span: SourceSpan) -> Self {
        Self { text, span }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn source_span(&self) -> SourceSpan {
        self.span
    }

    /// Split `text` at its first period into a prefix slice and, when text
    /// follows the period, a remainder slice. `None` when `text` carries no
    /// period. This is the single source of the dotted split rule: both the
    /// atom-level [`split_at_first_dot`](Self::split_at_first_dot) and the
    /// string-level dotted reader consult it, so the block-level and
    /// string-level paths never duplicate the splitting logic. A period is
    /// ordinary text; only a consumer that expects a dotted prefix at this
    /// position calls this, so the split is expectation-driven rather than a
    /// content classification.
    pub(crate) fn split_text_at_first_dot(text: &str) -> Option<(&str, Option<&str>)> {
        let dot = text.find('.')?;
        let prefix = &text[..dot];
        let remainder = &text[dot + 1..];
        let remainder = if remainder.is_empty() {
            None
        } else {
            Some(remainder)
        };
        Some((prefix, remainder))
    }

    pub fn qualifies_as_symbol(&self) -> bool {
        !self.text.is_empty()
            && self
                .text
                .chars()
                .all(|character| AtomCharacter::new(character).is_symbol())
    }

    pub fn qualifies_as_pascal_case_symbol(&self) -> bool {
        self.qualifies_as_symbol()
            && self
                .text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_uppercase())
            && !self.text.contains('-')
    }

    pub fn qualifies_as_camel_case_symbol(&self) -> bool {
        self.qualifies_as_symbol()
            && self
                .text
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_lowercase())
            && !self.text.contains('-')
    }

    pub fn qualifies_as_kebab_case_symbol(&self) -> bool {
        self.qualifies_as_symbol() && self.text.contains('-')
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatomError {
    UnexpectedClose {
        found: char,
        position: SourcePosition,
    },
    UnclosedDelimiter {
        delimiter: Delimiter,
        position: SourcePosition,
    },
    UnclosedPipeText {
        position: SourcePosition,
    },
    /// A period appeared where an object was expected — the head of a
    /// dot-application is missing. A dot binds a head to a following payload,
    /// so it can never start an object.
    UnexpectedDot {
        position: SourcePosition,
    },
    /// A dot-application period is not immediately followed by a glued payload
    /// object: whitespace, a comment, a closing delimiter, or the end of input
    /// followed the period. The payload must be glued to the period with no
    /// intervening separator.
    DanglingApplication {
        position: SourcePosition,
    },
}

impl fmt::Display for DatomError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedClose { found, position } => write!(
                formatter,
                "unexpected closing delimiter `{found}` at {}:{}",
                position.line, position.column
            ),
            Self::UnclosedDelimiter {
                delimiter,
                position,
            } => write!(
                formatter,
                "unclosed `{}` delimiter opened at {}:{}",
                delimiter.opening_text(),
                position.line,
                position.column
            ),
            Self::UnclosedPipeText { position } => write!(
                formatter,
                "unclosed `(|` pipe text opened at {}:{}",
                position.line, position.column
            ),
            Self::UnexpectedDot { position } => write!(
                formatter,
                "unexpected `.` with no head object at {}:{}",
                position.line, position.column
            ),
            Self::DanglingApplication { position } => write!(
                formatter,
                "dot-application `.` at {}:{} is not followed by a glued payload object",
                position.line, position.column
            ),
        }
    }
}

impl std::error::Error for DatomError {}

struct Parser<'source> {
    source: &'source str,
    mode: ParseMode,
    cursor: Cursor,
}

impl<'source> Parser<'source> {
    fn new(source: &'source str, mode: ParseMode) -> Self {
        Self {
            source,
            mode,
            cursor: Cursor::default(),
        }
    }

    fn parse_document(&mut self) -> Result<Vec<Block>, DatomError> {
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Ok(root_objects);
            };
            if Delimiter::from_closing(character).is_some() {
                return Err(DatomError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    /// Parse one object: a primary object and, when a glued period follows, the
    /// dot-application that binds it to the remainder. Binding is
    /// right-associative: `A.B.C` is `App(A, App(B, C))`, so the head is always
    /// the leftmost single segment and the payload is everything after the
    /// first period. This is the reading every authored form wants —
    /// `Visibility.name.Type`, `key.Value`, `Public.Newtype.( … )` — where the
    /// head is a prefix and the payload is the rest. A period binds only when
    /// it is glued to both its head and its payload; a period followed by
    /// whitespace, a comment, a closing delimiter, or the end of input is a
    /// dangling application error.
    fn parse_object(&mut self) -> Result<Block, DatomError> {
        let head = self.parse_primary()?;
        if self.peek() != Some('.') {
            return Ok(head);
        }
        let dot = self.cursor.position();
        self.bump();
        if !self.at_primary_start() {
            return Err(DatomError::DanglingApplication { position: dot });
        }
        let payload = self.parse_object()?;
        let span = SourceSpan {
            start: head.source_span().start,
            end: payload.source_span().end,
        };
        Ok(Block::Application {
            span,
            head: Box::new(head),
            payload: Box::new(payload),
        })
    }

    /// Parse a single primary object: a delimited block, a pipe-text block, or
    /// a bare atom. A primary never consumes a trailing dot-application; that
    /// binding is [`parse_object`]'s job. A leading period has no head object
    /// and is rejected.
    fn parse_primary(&mut self) -> Result<Block, DatomError> {
        match self.peek() {
            Some('(') if self.peek_next() == Some('|') => match self.mode {
                ParseMode::Default => self.parse_pipe_text(')'),
                ParseMode::StructuralPipe => self.parse_pipe_delimited(Delimiter::PipeParenthesis),
            },
            Some('(') => self.parse_delimited(Delimiter::Parenthesis),
            // The bracket-pipe form is an unambiguous legacy spelling for raw
            // text. Retain it in both modes so structural `(| … |)` adoption
            // does not force raw schema fixtures to lose their text literals.
            Some('[') if self.peek_next() == Some('|') => self.parse_pipe_text(']'),
            Some('[') => self.parse_delimited(Delimiter::SquareBracket),
            Some('{')
                if self.peek_next() == Some('|') && self.mode == ParseMode::StructuralPipe =>
            {
                self.parse_pipe_delimited(Delimiter::PipeBrace)
            }
            Some('{') => self.parse_delimited(Delimiter::Brace),
            Some('.') => Err(DatomError::UnexpectedDot {
                position: self.cursor.position(),
            }),
            // A misplaced pipe-close (`|)`) at an object position would make
            // `parse_atom` return a zero-width atom without advancing — the
            // enclosing loop would then spin forever, growing the block vector
            // until it exhausts memory. Reject it so the parser always makes
            // progress on malformed input.
            Some('|') if self.at_pipe_delimiter_close() => Err(DatomError::UnexpectedClose {
                found: self.peek_next().unwrap_or('|'),
                position: self.cursor.position(),
            }),
            Some(_) => Ok(self.parse_atom()),
            None => Ok(self.parse_atom()),
        }
    }

    /// Whether the cursor is on a character that can begin a primary object —
    /// used to decide whether a dot-application period has a glued payload.
    fn at_primary_start(&self) -> bool {
        match self.peek() {
            None => false,
            Some('.') => false,
            Some(character) if character.is_whitespace() => false,
            Some(character) if Delimiter::from_closing(character).is_some() => false,
            Some(_) if self.at_comment_start() => false,
            Some(_) if self.at_pipe_delimiter_close() => false,
            Some(_) => true,
        }
    }

    fn parse_delimited(&mut self, delimiter: Delimiter) -> Result<Block, DatomError> {
        let start = self.cursor.position();
        self.bump();
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Err(DatomError::UnclosedDelimiter {
                    delimiter,
                    position: start,
                });
            };
            if character == delimiter.closing() {
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::Delimited {
                    delimiter,
                    span: SourceSpan { start, end },
                    root_objects,
                });
            }
            if Delimiter::from_closing(character).is_some() {
                return Err(DatomError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    fn parse_pipe_delimited(&mut self, delimiter: Delimiter) -> Result<Block, DatomError> {
        let start = self.cursor.position();
        self.bump();
        self.bump();
        let mut root_objects = Vec::new();
        loop {
            self.skip_spacing();
            let Some(character) = self.peek() else {
                return Err(DatomError::UnclosedDelimiter {
                    delimiter,
                    position: start,
                });
            };
            if character == '|' && self.peek_next() == Some(delimiter.closing()) {
                self.bump();
                self.bump();
                return Ok(Block::Delimited {
                    delimiter,
                    span: SourceSpan {
                        start,
                        end: self.cursor.position(),
                    },
                    root_objects,
                });
            }
            if Delimiter::from_closing(character).is_some() {
                return Err(DatomError::UnexpectedClose {
                    found: character,
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.parse_object()?);
        }
    }

    fn parse_pipe_text(&mut self, closing: char) -> Result<Block, DatomError> {
        let start = self.cursor.position();
        self.bump();
        self.bump();
        let mut text = String::new();
        while let Some(character) = self.peek() {
            if character == '\\' {
                self.bump();
                if let Some(escaped) = self.peek() {
                    text.push(escaped);
                    self.bump();
                } else {
                    text.push('\\');
                }
            } else if character == '|' && self.peek_next() == Some(closing) {
                self.bump();
                self.bump();
                let end = self.cursor.position();
                return Ok(Block::PipeText(PipeText {
                    text,
                    span: SourceSpan { start, end },
                }));
            } else {
                text.push(character);
                self.bump();
            }
        }
        Err(DatomError::UnclosedPipeText { position: start })
    }

    fn parse_atom(&mut self) -> Block {
        let start = self.cursor.position();
        while let Some(character) = self.peek() {
            if character.is_whitespace()
                || character == '.'
                || Delimiter::from_opening(character).is_some()
                || Delimiter::from_closing(character).is_some()
                || self.at_comment_start()
                || self.at_pipe_delimiter_close()
            {
                break;
            }
            self.bump();
        }
        let end = self.cursor.position();
        let text = self.source[start.byte_offset..end.byte_offset].to_owned();
        Block::Atom(Atom::new(text, SourceSpan { start, end }))
    }

    fn at_pipe_delimiter_close(&self) -> bool {
        self.peek() == Some('|') && matches!(self.peek_next(), Some(')') | Some('}'))
    }

    fn at_comment_start(&self) -> bool {
        self.peek() == Some(';') && self.peek_next() == Some(';')
    }

    fn skip_spacing(&mut self) {
        loop {
            match self.peek() {
                Some(character) if character.is_whitespace() => {
                    self.bump();
                }
                Some(';') if self.peek_next() == Some(';') => {
                    while let Some(character) = self.peek() {
                        self.bump();
                        if character == '\n' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor.byte_offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut characters = self.source[self.cursor.byte_offset..].chars();
        characters.next()?;
        characters.next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor.byte_offset += character.len_utf8();
        if character == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 1;
        } else {
            self.cursor.column += 1;
        }
        Some(character)
    }
}

#[derive(Clone, Debug)]
struct Cursor {
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }
}

impl Cursor {
    fn position(&self) -> SourcePosition {
        SourcePosition {
            byte_offset: self.byte_offset,
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomCharacter {
    character: char,
}

impl AtomCharacter {
    pub(crate) fn new(character: char) -> Self {
        Self { character }
    }

    fn is_symbol(&self) -> bool {
        self.is_bare_string()
    }

    pub(crate) fn is_bare_string(&self) -> bool {
        // The period is a structural dot-application operator in the raw
        // grammar, so it can never sit inside a bare atom. A string whose
        // content carries a period is therefore delimited, never bare.
        !self.character.is_whitespace()
            && !matches!(
                self.character,
                '"' | '.' | '(' | ')' | '[' | ']' | '{' | '}'
            )
    }
}
