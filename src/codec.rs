use std::collections::BTreeMap;
use std::fmt;

use crate::{Block, Delimiter, Document, expectation::DottedExpectation, parser::AtomCharacter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatomDecodeError {
    Parse(String),
    ExpectedSingleRoot {
        found: usize,
    },
    ExpectedDelimited {
        type_name: &'static str,
        delimiter: &'static str,
    },
    ExpectedRootCount {
        type_name: &'static str,
        expected: usize,
        found: usize,
    },
    ExpectedAtom {
        type_name: &'static str,
    },
    UnknownVariant {
        enum_name: &'static str,
        variant: String,
    },
    NonCanonicalStringDelimiter {
        value: String,
        canonical: String,
    },
    InvalidInteger {
        value: String,
    },
    InvalidValue {
        type_name: &'static str,
        value: String,
        reason: String,
    },
    ExpectedDottedEntry {
        expectation: &'static str,
    },
    DottedEntryCaseMismatch {
        expectation: &'static str,
        prefix: String,
    },
    DottedEntryMissingValue {
        expectation: &'static str,
    },
}

impl fmt::Display for DatomDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "{error}"),
            Self::ExpectedSingleRoot { found } => {
                write!(
                    formatter,
                    "expected exactly one Datom root object, found {found}"
                )
            }
            Self::ExpectedDelimited {
                type_name,
                delimiter,
            } => {
                write!(formatter, "expected {type_name} to be a {delimiter} block")
            }
            Self::ExpectedRootCount {
                type_name,
                expected,
                found,
            } => {
                write!(
                    formatter,
                    "expected {type_name} to hold {expected} root objects, found {found}"
                )
            }
            Self::ExpectedAtom { type_name } => write!(formatter, "expected {type_name} atom"),
            Self::UnknownVariant { enum_name, variant } => {
                write!(formatter, "unknown {enum_name} variant {variant}")
            }
            Self::NonCanonicalStringDelimiter { value, canonical } => write!(
                formatter,
                "non-canonical string delimiter for {value:?}: use {canonical}"
            ),
            Self::InvalidInteger { value } => write!(formatter, "invalid integer {value}"),
            Self::InvalidValue {
                type_name,
                value,
                reason,
            } => write!(formatter, "invalid {type_name} {value:?}: {reason}"),
            Self::ExpectedDottedEntry { expectation } => write!(
                formatter,
                "expected a {expectation} entry written as key.value"
            ),
            Self::DottedEntryCaseMismatch {
                expectation,
                prefix,
            } => write!(
                formatter,
                "{expectation} head {prefix:?} has the wrong case for this position"
            ),
            Self::DottedEntryMissingValue { expectation } => write!(
                formatter,
                "{expectation} entry ends at the period with no following value"
            ),
        }
    }
}

impl std::error::Error for DatomDecodeError {}

impl From<crate::DatomError> for DatomDecodeError {
    fn from(error: crate::DatomError) -> Self {
        Self::Parse(error.to_string())
    }
}

pub trait DatomDecode: Sized {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError>;
}

pub trait DatomEncode {
    fn to_datom(&self) -> String;
}

pub trait DatomBodyDecode: Sized {
    fn from_datom_body(body: &DatomBody<'_>) -> Result<Self, DatomDecodeError>;
}

pub trait DatomBodyEncode {
    fn to_datom_body(&self) -> DatomBodyEncoding;
}

pub trait DatomDocumentDecode: Sized {
    fn from_datom_document_body(body: &DatomDocumentBody<'_>) -> Result<Self, DatomDecodeError>;
}

pub trait DatomDocumentEncode {
    fn to_datom_document_body(&self) -> DatomDocumentEncoding;
}

pub struct DatomSource<'source> {
    source: &'source str,
}

impl<'source> DatomSource<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    pub fn parse_root(&self) -> Result<Block, DatomDecodeError> {
        let document = Document::parse(self.source)?;
        if document.holds_root_objects() != 1 {
            return Err(DatomDecodeError::ExpectedSingleRoot {
                found: document.holds_root_objects(),
            });
        }
        Ok(document
            .root_object_at(0)
            .expect("root count checked")
            .clone())
    }

    pub fn parse<Value>(&self) -> Result<Value, DatomDecodeError>
    where
        Value: DatomDecode,
    {
        let root = self.parse_root()?;
        Value::from_datom_block(&root)
    }

    pub fn parse_document_body<Value>(&self) -> Result<Value, DatomDecodeError>
    where
        Value: DatomDocumentDecode,
    {
        let document = Document::parse(self.source)?;
        let body = DatomDocumentBody::new(&document);
        Value::from_datom_document_body(&body)
    }

    pub fn parse_body<Value>(&self) -> Result<Value, DatomDecodeError>
    where
        Value: DatomBodyDecode,
    {
        let document = Document::parse(self.source)?;
        let body = DatomBody::from_document(&document);
        Value::from_datom_body(&body)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DatomBody<'body> {
    root_objects: &'body [Block],
}

impl<'body> DatomBody<'body> {
    pub fn new(root_objects: &'body [Block]) -> Self {
        Self { root_objects }
    }

    pub fn from_document(document: &'body Document) -> Self {
        Self::new(document.root_objects())
    }

    pub fn from_delimited(
        block: &'body Block,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<Self, DatomDecodeError> {
        let root_objects =
            block
                .as_delimited(delimiter)
                .ok_or(DatomDecodeError::ExpectedDelimited {
                    type_name,
                    delimiter: delimiter.description(),
                })?;
        Ok(Self::new(root_objects))
    }

    pub fn root_objects(&self) -> &'body [Block] {
        self.root_objects
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'body [Block], DatomDecodeError> {
        let found = self.root_objects().len();
        if found != expected {
            return Err(DatomDecodeError::ExpectedRootCount {
                type_name,
                expected,
                found,
            });
        }
        Ok(self.root_objects())
    }
}

pub struct DatomDocumentBody<'document> {
    body: DatomBody<'document>,
}

impl<'document> DatomDocumentBody<'document> {
    pub fn new(document: &'document Document) -> Self {
        Self {
            body: DatomBody::from_document(document),
        }
    }

    pub fn from_body(body: DatomBody<'document>) -> Self {
        Self { body }
    }

    pub fn as_body(&self) -> &DatomBody<'document> {
        &self.body
    }

    pub fn root_objects(&self) -> &'document [Block] {
        self.body.root_objects()
    }

    pub fn expect_fields(
        &self,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'document [Block], DatomDecodeError> {
        self.body.expect_fields(type_name, expected)
    }
}

pub struct DatomBodyEncoding {
    fields: Vec<String>,
}

impl DatomBodyEncoding {
    pub fn new(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    pub fn to_datom(&self) -> String {
        self.fields.join("\n")
    }

    pub fn to_delimited_datom(&self, delimiter: Delimiter) -> String {
        delimiter.wrap(self.fields.iter().cloned())
    }
}

pub type DatomDocumentEncoding = DatomBodyEncoding;

pub struct DatomBlock<'block> {
    block: &'block Block,
}

impl<'block> DatomBlock<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn expect_children(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
        expected: usize,
    ) -> Result<&'block [Block], DatomDecodeError> {
        self.expect_body(delimiter, type_name)?
            .expect_fields(type_name, expected)
    }

    pub fn expect_delimited(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<&'block [Block], DatomDecodeError> {
        Ok(self.expect_body(delimiter, type_name)?.root_objects())
    }

    pub fn expect_body(
        &self,
        delimiter: Delimiter,
        type_name: &'static str,
    ) -> Result<DatomBody<'block>, DatomDecodeError> {
        DatomBody::from_delimited(self.block, delimiter, type_name)
    }

    pub fn parse_string(&self) -> Result<String, DatomDecodeError> {
        // Pipe text carries a literal, but a pipe wrapper around content that a
        // simpler canonical form could hold is non-canonical and rejected.
        if self.block.is_pipe_text() {
            let text = self
                .block
                .demote_to_string()
                .expect("pipe text demotes to its literal");
            DatomString::new(text).reject_redundant_delimiter(StringForm::PipeText)?;
            return Ok(text.to_owned());
        }
        // A bare atom or a dotted chain of atoms is the string's flat text: an
        // expected `String` reclaims the text the structural period split into a
        // raw dot-application, exactly as the numeric readers reclaim a
        // fractional literal. The rejoin is case-blind — `file.txt`, `Foo.bar`,
        // and a multi-dot host such as `nix.prometheus.goldragon.criome` all
        // rejoin to their bare content regardless of atom case.
        if let Some(text) = self.block.dotted_text() {
            return Ok(text);
        }
        // A parenthesis holds space-joined children, each itself a string.
        if let Some(root_objects) = self.block.as_delimited(Delimiter::Parenthesis) {
            let text = root_objects
                .iter()
                .map(|block| DatomBlock::new(block).parse_string())
                .collect::<Result<Vec<_>, _>>()
                .map(|parts| parts.join(" "))?;
            DatomString::new(&text).reject_redundant_delimiter(StringForm::Parenthesis)?;
            return Ok(text);
        }
        Err(DatomDecodeError::ExpectedDelimited {
            type_name: "String",
            delimiter: "string atom, dotted application, or parenthesis",
        })
    }

    pub fn parse_integer(&self) -> Result<u64, DatomDecodeError> {
        let value = self.parse_numeric_text("Integer")?;
        value
            .parse::<u64>()
            .map_err(|_| DatomDecodeError::InvalidInteger { value })
    }

    pub fn parse_u16(&self) -> Result<u16, DatomDecodeError> {
        let value = self.parse_integer()?;
        u16::try_from(value).map_err(|_| DatomDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u8(&self) -> Result<u8, DatomDecodeError> {
        let value = self.parse_integer()?;
        u8::try_from(value).map_err(|_| DatomDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_u32(&self) -> Result<u32, DatomDecodeError> {
        let value = self.parse_integer()?;
        u32::try_from(value).map_err(|_| DatomDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_signed_integer(&self) -> Result<i64, DatomDecodeError> {
        let value = self.parse_numeric_text("SignedInteger")?;
        value
            .parse::<i64>()
            .map_err(|_| DatomDecodeError::InvalidInteger { value })
    }

    pub fn parse_i32(&self) -> Result<i32, DatomDecodeError> {
        let value = self.parse_signed_integer()?;
        i32::try_from(value).map_err(|_| DatomDecodeError::InvalidInteger {
            value: value.to_string(),
        })
    }

    pub fn parse_float(&self) -> Result<f64, DatomDecodeError> {
        let value = self.parse_numeric_text("Float")?;
        value
            .parse::<f64>()
            .map_err(|_| DatomDecodeError::InvalidValue {
                type_name: "Float",
                value,
                reason: "expected a finite or non-finite Rust f64 literal".to_owned(),
            })
    }

    /// The flat literal text of a numeric block. A number that carries a
    /// fractional period — `-122.3` — is a dot-application at the raw layer, so
    /// its literal is reconstructed from the dotted segments rather than read
    /// as a single atom; a period-free integer is a bare atom whose text is the
    /// same reconstruction of one segment.
    fn parse_numeric_text(&self, type_name: &'static str) -> Result<String, DatomDecodeError> {
        self.block
            .dotted_text()
            .ok_or(DatomDecodeError::ExpectedAtom { type_name })
    }

    pub fn parse_boolean(&self) -> Result<bool, DatomDecodeError> {
        let value = self
            .block
            .demote_to_string()
            .ok_or(DatomDecodeError::ExpectedAtom {
                type_name: "Boolean",
            })?;
        match value {
            "True" => Ok(true),
            "False" => Ok(false),
            other => Err(DatomDecodeError::UnknownVariant {
                enum_name: "Boolean",
                variant: other.to_owned(),
            }),
        }
    }
}

/// The one canonical Datom surface form a string's content takes. The forms are
/// mutually exclusive by construction — [`DatomString::canonical_form`] picks the
/// least-delimited form that carries the content faithfully — so both the
/// encoder and the redundant-delimiter check read a single classification rather
/// than scattering per-character conditionals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringForm {
    BareDotted,
    Parenthesis,
    PipeText,
}

pub struct DatomString<'value> {
    value: &'value str,
}

impl<'value> DatomString<'value> {
    pub fn new(value: &'value str) -> Self {
        Self { value }
    }

    pub fn format(&self) -> String {
        match self.canonical_form() {
            StringForm::BareDotted => self.value.to_owned(),
            StringForm::Parenthesis => format!("({})", self.value),
            StringForm::PipeText => format!("(|{}|)", self.escape_pipe_text()),
        }
    }

    /// The single canonical Datom form for this string's content. The three forms
    /// are ordered by how little delimiter they spend, and the first one that can
    /// carry the content faithfully wins, so each form narrows to its honest role:
    ///
    /// - [`StringForm::BareDotted`] — content that is a period-joined chain of
    ///   bare atoms, so the raw parser rebuilds a dot-application whose flat
    ///   dotted text is exactly the content: `schema`, `file.txt`,
    ///   `nix.prometheus.goldragon.criome`. A period is a structural operator at
    ///   the raw layer, but an expected `String` reclaims the split text, so a
    ///   period-bearing string no longer needs an escape.
    /// - [`StringForm::Parenthesis`] — content that is single-space-separated
    ///   words, each itself bare-dotted, so the space-joined `( … )` form
    ///   rebuilds it: `alpha beta`, `version 1.2`.
    /// - [`StringForm::PipeText`] — everything else: delimiter glyphs, newlines
    ///   and indentation, comment markers, pipe-close markers, irregular
    ///   whitespace, or the empty string. Only the literal-preserving
    ///   `( | … | )` form carries these, with close markers escaped.
    fn canonical_form(&self) -> StringForm {
        if self.qualifies_as_bare_dotted_string() {
            StringForm::BareDotted
        } else if self.qualifies_as_parenthesized_string() {
            StringForm::Parenthesis
        } else {
            StringForm::PipeText
        }
    }

    /// Whether the content is a non-empty period-joined chain of bare atoms. Each
    /// period-separated segment must be a non-empty run of bare-atom characters,
    /// so the raw parser rebuilds a dot-application (or a lone atom) whose flat
    /// dotted text equals the content. A comment marker breaks atom scanning, so
    /// content carrying `;;` is never bare.
    fn qualifies_as_bare_dotted_string(&self) -> bool {
        if self.value.is_empty() || self.value.contains(";;") {
            return false;
        }
        self.value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| AtomCharacter::new(character).is_bare_string())
        })
    }

    /// Whether the content is single-space-separated words that each qualify as
    /// bare-dotted. The space-joined `( … )` form skips whitespace adjacent to
    /// object boundaries, so only single ASCII spaces between non-empty words
    /// survive a round trip: a leading or trailing space, a doubled space, or any
    /// non-space whitespace forces the literal-preserving pipe form instead.
    fn qualifies_as_parenthesized_string(&self) -> bool {
        if !self.value.contains(' ') {
            return false;
        }
        if self
            .value
            .chars()
            .any(|character| character.is_whitespace() && character != ' ')
        {
            return false;
        }
        self.value
            .split(' ')
            .all(|word| DatomString::new(word).qualifies_as_bare_dotted_string())
    }

    fn reject_redundant_delimiter(&self, used: StringForm) -> Result<(), DatomDecodeError> {
        if self.canonical_form() != used {
            return Err(DatomDecodeError::NonCanonicalStringDelimiter {
                value: self.value.to_owned(),
                canonical: self.format(),
            });
        }
        Ok(())
    }

    fn escape_pipe_text(&self) -> String {
        let mut escaped = String::new();
        let mut characters = self.value.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\\' {
                escaped.push('\\');
                escaped.push('\\');
            } else if character == '|' && characters.peek() == Some(&')') {
                escaped.push('\\');
                escaped.push('|');
            } else {
                escaped.push(character);
            }
        }
        escaped
    }
}

pub struct DatomCollection<'block> {
    block: &'block Block,
}

impl<'block> DatomCollection<'block> {
    pub fn new(block: &'block Block) -> Self {
        Self { block }
    }

    pub fn parse_vector<Element, Parse>(
        &self,
        parse: Parse,
    ) -> Result<Vec<Element>, DatomDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Element, DatomDecodeError>,
    {
        self.block
            .as_delimited(Delimiter::SquareBracket)
            .ok_or(DatomDecodeError::ExpectedDelimited {
                type_name: "Vec",
                delimiter: Delimiter::SquareBracket.description(),
            })?
            .iter()
            .map(parse)
            .collect()
    }

    pub fn parse_map<Key, Value, ParseKey, ParseValue>(
        &self,
        mut parse_key: ParseKey,
        mut parse_value: ParseValue,
    ) -> Result<BTreeMap<Key, Value>, DatomDecodeError>
    where
        Key: Ord,
        ParseKey: FnMut(&Block) -> Result<Key, DatomDecodeError>,
        ParseValue: FnMut(&Block) -> Result<Value, DatomDecodeError>,
    {
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(DatomDecodeError::ExpectedDelimited {
                    type_name: "Map",
                    delimiter: "Map.( ... ) application",
                })?;
        if head.demote_to_string() != Some("Map") {
            return Err(DatomDecodeError::UnknownVariant {
                enum_name: "Map",
                variant: head.dotted_text().unwrap_or_default(),
            });
        }
        let entries = payload.as_delimited(Delimiter::Parenthesis).ok_or(
            DatomDecodeError::ExpectedDelimited {
                type_name: "Map",
                delimiter: Delimiter::Parenthesis.description(),
            },
        )?;
        let mut map = BTreeMap::new();
        for entry_block in entries {
            let entry =
                DottedExpectation::Uncapitalized.read_entry(std::slice::from_ref(entry_block))?;
            let key = parse_key(entry.key())?;
            let value = parse_value(entry.value())?;
            map.insert(key, value);
        }
        Ok(map)
    }

    pub fn parse_option<Inner, Parse>(
        &self,
        mut parse: Parse,
    ) -> Result<Option<Inner>, DatomDecodeError>
    where
        Parse: FnMut(&Block) -> Result<Inner, DatomDecodeError>,
    {
        if self.block.demote_to_string() == Some("None") {
            return Ok(None);
        }
        let (head, payload) =
            self.block
                .as_application()
                .ok_or(DatomDecodeError::ExpectedDelimited {
                    type_name: "Option",
                    delimiter: "None atom or Some.payload application",
                })?;
        let tag = head
            .demote_to_string()
            .ok_or(DatomDecodeError::ExpectedAtom {
                type_name: "Option tag",
            })?;
        if tag != "Some" {
            return Err(DatomDecodeError::UnknownVariant {
                enum_name: "Option",
                variant: tag.to_owned(),
            });
        }
        Ok(Some(parse(payload)?))
    }
}

impl DatomDecode for String {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_string()
    }
}

impl DatomEncode for String {
    fn to_datom(&self) -> String {
        DatomString::new(self).format()
    }
}

impl DatomDecode for u64 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_integer()
    }
}

impl DatomEncode for u64 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for u8 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_u8()
    }
}

impl DatomEncode for u8 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for u16 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_u16()
    }
}

impl DatomEncode for u16 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for u32 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_u32()
    }
}

impl DatomEncode for u32 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for i32 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_i32()
    }
}

impl DatomEncode for i32 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for i64 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_signed_integer()
    }
}

impl DatomEncode for i64 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for f64 {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_float()
    }
}

impl DatomEncode for f64 {
    fn to_datom(&self) -> String {
        self.to_string()
    }
}

impl DatomDecode for bool {
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomBlock::new(block).parse_boolean()
    }
}

impl DatomEncode for bool {
    fn to_datom(&self) -> String {
        if *self {
            "True".to_owned()
        } else {
            "False".to_owned()
        }
    }
}

impl<Element> DatomDecode for Vec<Element>
where
    Element: DatomDecode,
{
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomCollection::new(block).parse_vector(Element::from_datom_block)
    }
}

impl<Element> DatomEncode for Vec<Element>
where
    Element: DatomEncode,
{
    fn to_datom(&self) -> String {
        Delimiter::SquareBracket.wrap(self.iter().map(Element::to_datom))
    }
}

impl<Element> DatomBodyDecode for Vec<Element>
where
    Element: DatomDecode,
{
    fn from_datom_body(body: &DatomBody<'_>) -> Result<Self, DatomDecodeError> {
        body.root_objects()
            .iter()
            .map(Element::from_datom_block)
            .collect()
    }
}

impl<Element> DatomBodyEncode for Vec<Element>
where
    Element: DatomEncode,
{
    fn to_datom_body(&self) -> DatomBodyEncoding {
        DatomBodyEncoding::new(self.iter().map(Element::to_datom).collect())
    }
}

impl<Key, Value> DatomDecode for BTreeMap<Key, Value>
where
    Key: DatomDecode + Ord,
    Value: DatomDecode,
{
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomCollection::new(block).parse_map(Key::from_datom_block, Value::from_datom_block)
    }
}

impl<Key, Value> DatomEncode for BTreeMap<Key, Value>
where
    Key: DatomEncode,
    Value: DatomEncode,
{
    fn to_datom(&self) -> String {
        let mut entries: Vec<String> = Vec::new();
        for (key, value) in self {
            entries.push(format!("{}.{}", Key::to_datom(key), Value::to_datom(value)));
        }
        format!("Map.{}", Delimiter::Parenthesis.wrap(entries))
    }
}

impl<Inner> DatomDecode for Option<Inner>
where
    Inner: DatomDecode,
{
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        DatomCollection::new(block).parse_option(Inner::from_datom_block)
    }
}

impl<Inner> DatomEncode for Option<Inner>
where
    Inner: DatomEncode,
{
    fn to_datom(&self) -> String {
        match self {
            Some(inner) => format!("Some.{}", Inner::to_datom(inner)),
            None => "None".to_owned(),
        }
    }
}

impl<Inner> DatomDecode for Box<Inner>
where
    Inner: DatomDecode,
{
    fn from_datom_block(block: &Block) -> Result<Self, DatomDecodeError> {
        Inner::from_datom_block(block).map(Box::new)
    }
}

impl<Inner> DatomEncode for Box<Inner>
where
    Inner: DatomEncode,
{
    fn to_datom(&self) -> String {
        Inner::to_datom(self)
    }
}
