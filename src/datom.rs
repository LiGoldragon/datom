use std::collections::BTreeMap;

use protos::{
    Block, BlockScanning, CursorObserving, Head, Headed, Realize, RealizeDriving, RealizeScope,
    RealizeScoping, RealizeWalk, Shape, ShapeDefined, SourceText, StringCarrier, StringCarrying,
    Textualize, TextualizeDriving, TextualizeScope, TextualizeScoping, TextualizeWalk, WalkFault,
    WalkObservation, WalkObserving,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatomFault {
    pub problem: DatomProblem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatomProblem {
    Shape,
    Head,
    Position,
    ExtraPosition,
    MissingPosition,
    AmbiguousMapPair,
    Protos(WalkFault),
}

impl From<WalkFault> for DatomFault {
    fn from(problem: WalkFault) -> Self {
        Self {
            problem: DatomProblem::Protos(problem),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagList(pub Vec<Text>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Entry {
    Note(Text),
    Group(Group),
    Tags(TagList),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    pub title: Text,
    pub children: Vec<Entry>,
    pub annotations: BTreeMap<String, Text>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub heading: Text,
    pub groups: BTreeMap<String, Vec<Entry>>,
    pub latest: Option<Text>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterimNote {
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportText {
    pub source: SourceText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterimNoteText {
    pub source: SourceText,
}

/// Actual, read-only Protos transition evidence paired with a typed operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatomEvidence {
    observation: WalkObservation,
    cursor: usize,
}

/// A typed value returned together with the driver that actually produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Realized<T> {
    value: T,
    evidence: DatomEvidence,
}

/// Typed canonical text returned together with the driver that emitted it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projected<T> {
    text: T,
    evidence: DatomEvidence,
}

/// Read-only access to a typed operation's actual Protos driver evidence.
pub trait EvidenceObserving {
    fn observation(&self) -> &WalkObservation;
    fn cursor(&self) -> usize;
}

/// Read-only access to a realized typed value and its evidence.
pub trait RealizationViewing<T> {
    fn value(&self) -> &T;
    fn evidence(&self) -> &DatomEvidence;
}

/// Read-only access to canonical typed text and its evidence.
pub trait ProjectionViewing<T> {
    fn text(&self) -> &T;
    fn evidence(&self) -> &DatomEvidence;
}

/// Typed realization which returns the same Protos driver's transition facts.
pub trait EvidencedRealizing {
    type Value;

    fn realize_evidenced(&self) -> Result<Realized<Self::Value>, DatomFault>;
}

/// Typed textualization which returns the same Protos driver's transition facts.
pub trait EvidencedTextualizing {
    type Text;

    fn textualize_evidenced(&self) -> Result<Projected<Self::Text>, DatomFault>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OptionalText {
    value: Option<Text>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GroupMapEntry {
    key: String,
    value: Vec<Entry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextMapEntry {
    key: String,
    value: Text,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntryVector {
    value: Vec<Entry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GroupMap {
    value: BTreeMap<String, Vec<Entry>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextMap {
    value: BTreeMap<String, Text>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextSelection {
    Plain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntrySelection {
    Note,
    BareNote,
    Group,
    Tags,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupSelection {
    Payload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TagListSelection {
    Payload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MapSelection {
    Entries,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OptionalSelection {
    Bare,
    Some,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordPosition {
    ordinal: usize,
}

trait PositionAdvancing {
    fn next(&mut self) -> usize;
}

impl EvidenceObserving for DatomEvidence {
    fn observation(&self) -> &WalkObservation {
        &self.observation
    }

    fn cursor(&self) -> usize {
        self.cursor
    }
}

impl<T> RealizationViewing<T> for Realized<T> {
    fn value(&self) -> &T {
        &self.value
    }

    fn evidence(&self) -> &DatomEvidence {
        &self.evidence
    }
}

impl<T> ProjectionViewing<T> for Projected<T> {
    fn text(&self) -> &T {
        &self.text
    }

    fn evidence(&self) -> &DatomEvidence {
        &self.evidence
    }
}

impl PositionAdvancing for RecordPosition {
    fn next(&mut self) -> usize {
        let ordinal = self.ordinal;
        self.ordinal += 1;
        ordinal
    }
}

trait DatomRealizing: Sized {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault>;
}

trait DatomTextualizing {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault>;
}

trait TagPayloading: Sized {
    fn realize_tag_payload(scope: &mut RealizeScope<'_>) -> Result<Self, DatomFault>;
    fn textualize_tag_payload(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault>;
}

trait GroupPayloading: Sized {
    fn realize_group_payload(scope: &mut RealizeScope<'_>) -> Result<Self, DatomFault>;
    fn textualize_group_payload(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault>;
}

trait CarrierRealizing {
    fn text(&self) -> String;
}

// Headed payloads are never normalized globally. These are the only
// context-owned carrier/body admissions: Entry::Note validates Note,
// Entry::Group validates Group then selects GroupPayloading, Entry::Tags
// validates Tags then selects TagPayloading, OptionalText validates Some,
// TextMapEntry validates its key, GroupMapEntry validates its key, and record
// variants validate their own heads before entering positional bodies.

impl CarrierRealizing for StringCarrier {
    fn text(&self) -> String {
        match self {
            Self::Parenthesized(body) => {
                let mut text = String::new();
                let mut escaped = false;
                for character in body.chars() {
                    if escaped {
                        match character {
                            '\\' | '(' | ')' => text.push(character),
                            other => {
                                text.push('\\');
                                text.push(other);
                            }
                        }
                        escaped = false;
                    } else if character == '\\' {
                        escaped = true;
                    } else {
                        text.push(character);
                    }
                }
                if escaped {
                    text.push('\\');
                }
                text
            }
            Self::Bare(_) | Self::CurlyQuoted(_) => self.textual_body().to_owned(),
        }
    }
}

trait ParenthesisProjecting {
    fn parenthesized_body(&self) -> String;
}

impl ParenthesisProjecting for Text {
    fn parenthesized_body(&self) -> String {
        let characters: Vec<char> = self.0.chars().collect();
        let mut opens = Vec::new();
        let mut escaped = Vec::new();
        for (index, character) in characters.iter().enumerate() {
            match character {
                '(' => opens.push(index),
                ')' if opens.is_empty() => escaped.push(index),
                ')' => {
                    opens.pop();
                }
                _ => {}
            }
        }
        escaped.extend(opens);
        let mut projected = String::new();
        for (index, character) in characters.into_iter().enumerate() {
            if character == '\\' {
                projected.push('\\');
            }
            if escaped.contains(&index) {
                projected.push('\\');
            }
            projected.push(character);
        }
        projected
    }
}

trait BareProjecting {
    fn fits_bare(&self) -> bool;
}

impl BareProjecting for Text {
    fn fits_bare(&self) -> bool {
        let candidate = SourceText(self.0.clone());
        matches!(
            candidate.blocks(),
            Ok(blocks)
                if blocks.len() == 1
                    && blocks[0].shape == Shape::Bare
                    && blocks[0].body.0 == self.0
        )
    }
}

trait HeadReading {
    fn text(&self) -> String;
}

impl HeadReading for Head {
    fn text(&self) -> String {
        self.0.clone()
    }
}

trait PairDividing {
    fn divide(&self) -> Result<(String, Text), DatomFault>;
}

trait MapKeyChecking {
    fn group_key(&self) -> Result<(), DatomFault>;
    fn text_key(&self, value: &Text) -> Result<(), DatomFault>;
}

impl MapKeyChecking for String {
    fn group_key(&self) -> Result<(), DatomFault> {
        if self.is_empty() || self.contains('.') {
            return Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            });
        }
        let candidate = SourceText(format!("{}.[ ]", self));
        match candidate.blocks() {
            Ok(blocks)
                if blocks.len() == 1
                    && blocks[0].shape == Shape::DottedSquareBracketed
                    && blocks[0].head() == Some(&Head(self.clone())) =>
            {
                Ok(())
            }
            _ => Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            }),
        }
    }

    fn text_key(&self, value: &Text) -> Result<(), DatomFault> {
        if self.is_empty() || self.contains('.') {
            return Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            });
        }
        let candidate = if value.fits_bare() {
            SourceText(format!("{}.{}", self, value.0))
        } else {
            SourceText(format!("{}.({})", self, value.parenthesized_body()))
        };
        match candidate.blocks() {
            Ok(blocks)
                if blocks.len() == 1
                    && ((blocks[0].shape == Shape::Bare
                        && blocks[0].divide().is_ok_and(|pair| pair.0 == *self))
                        || (blocks[0].shape == Shape::DottedParenthesized
                            && blocks[0].head() == Some(&Head(self.clone())))) =>
            {
                Ok(())
            }
            _ => Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            }),
        }
    }
}

impl PairDividing for Block {
    fn divide(&self) -> Result<(String, Text), DatomFault> {
        if self.shape != Shape::Bare {
            return Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            });
        }
        let Some((key, value)) = self.body.0.split_once('.') else {
            return Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            });
        };
        if key.is_empty() || value.is_empty() || key.contains('.') {
            return Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            });
        }
        Ok((key.to_owned(), Text(value.to_owned())))
    }
}

impl ShapeDefined for Text {
    type Selection = TextSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::Bare, Shape::Parenthesized, Shape::CurlyQuoted]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        // Plain String for now: both carriers select it. Meaning belongs later
        // in structuredStringType.md under bead primary-xqb.8.5.
        (head.is_none() && Self::shapes().contains(&shape)).then_some(TextSelection::Plain)
    }
}

impl ShapeDefined for Entry {
    type Selection = EntrySelection;

    fn shapes() -> &'static [Shape] {
        &[
            Shape::Bare,
            Shape::DottedParenthesized,
            Shape::DottedCurlyQuoted,
            Shape::DottedBraced,
            Shape::DottedSquareBracketed,
        ]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        match (shape, head) {
            (Shape::Bare, None) => Some(EntrySelection::BareNote),
            (Shape::DottedParenthesized | Shape::DottedCurlyQuoted, Some(value))
                if value == &Head("Note".into()) =>
            {
                Some(EntrySelection::Note)
            }
            (Shape::DottedBraced, Some(value)) if value == &Head("Group".into()) => {
                Some(EntrySelection::Group)
            }
            (Shape::DottedSquareBracketed, Some(value)) if value == &Head("Tags".into()) => {
                Some(EntrySelection::Tags)
            }
            _ => None,
        }
    }
}

impl ShapeDefined for Group {
    type Selection = GroupSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::Braced]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::Braced && head.is_none()).then_some(GroupSelection::Payload)
    }
}

impl ShapeDefined for TagList {
    type Selection = TagListSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::SquareBracketed]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::SquareBracketed && head.is_none()).then_some(TagListSelection::Payload)
    }
}

impl ShapeDefined for EntryVector {
    type Selection = ();

    fn shapes() -> &'static [Shape] {
        &[Shape::SquareBracketed]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::SquareBracketed && head.is_none()).then_some(())
    }
}

impl ShapeDefined for GroupMap {
    type Selection = MapSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::DottedSquareBracketed]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::DottedSquareBracketed && head == Some(&Head("Map".into())))
            .then_some(MapSelection::Entries)
    }
}

impl ShapeDefined for TextMap {
    type Selection = MapSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::DottedSquareBracketed]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::DottedSquareBracketed && head == Some(&Head("Map".into())))
            .then_some(MapSelection::Entries)
    }
}

impl ShapeDefined for Report {
    type Selection = ();

    fn shapes() -> &'static [Shape] {
        &[Shape::DottedBraced]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::DottedBraced && head == Some(&Head("Report".into()))).then_some(())
    }
}

impl ShapeDefined for InterimNote {
    type Selection = ();

    fn shapes() -> &'static [Shape] {
        &[Shape::DottedBraced]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        (shape == Shape::DottedBraced && head == Some(&Head("InterimNote".into()))).then_some(())
    }
}

impl ShapeDefined for OptionalText {
    type Selection = OptionalSelection;

    fn shapes() -> &'static [Shape] {
        &[
            Shape::Bare,
            Shape::DottedParenthesized,
            Shape::DottedCurlyQuoted,
        ]
    }

    fn select(shape: Shape, head: Option<&Head>) -> Option<Self::Selection> {
        match (shape, head) {
            (Shape::Bare, None) => Some(OptionalSelection::Bare),
            (Shape::DottedParenthesized | Shape::DottedCurlyQuoted, Some(value))
                if value == &Head("Some".into()) =>
            {
                Some(OptionalSelection::Some)
            }
            _ => None,
        }
    }
}

impl DatomRealizing for Text {
    fn realize_block(_: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        if Self::select(block.shape, block.head()).is_none() {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        }
        let Some(carrier) = &block.string_carrier else {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        };
        Ok(Self(carrier.text()))
    }
}

impl DatomTextualizing for Text {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        if self.fits_bare() {
            scope.textualize_block(Shape::Bare, None, |body| {
                body.emit_scalar(&self.0);
                Ok(())
            })
        } else {
            scope.textualize_block(Shape::Parenthesized, None, |body| {
                body.emit_scalar(&self.parenthesized_body());
                Ok(())
            })
        }
    }
}

impl DatomRealizing for EntryVector {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        if Self::select(block.shape, block.head()).is_none() {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        }
        Ok(Self {
            value: scope
                .realize_body(&mut |child_scope, child| Entry::realize_block(child_scope, child))?,
        })
    }
}

impl DatomTextualizing for EntryVector {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        scope.textualize_block(Shape::SquareBracketed, None, |body| {
            for entry in &self.value {
                entry.textualize_in(body)?;
            }
            Ok(())
        })
    }
}

impl TagPayloading for TagList {
    fn realize_tag_payload(scope: &mut RealizeScope<'_>) -> Result<Self, DatomFault> {
        Ok(Self(scope.realize_body(&mut |child_scope, child| {
            Text::realize_block(child_scope, child)
        })?))
    }

    fn textualize_tag_payload(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        for text in &self.0 {
            text.textualize_in(scope)?;
        }
        Ok(())
    }
}

impl DatomRealizing for GroupMapEntry {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        if block.shape != Shape::DottedSquareBracketed {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        }
        let Some(head) = block.head() else {
            return Err(DatomFault {
                problem: DatomProblem::Head,
            });
        };
        let key = head.text();
        key.group_key()?;
        let value = scope
            .realize_body(&mut |child_scope, child| Entry::realize_block(child_scope, child))?;
        Ok(Self { key, value })
    }
}

impl DatomTextualizing for GroupMapEntry {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        self.key.group_key()?;
        let head = Head(self.key.clone());
        scope.textualize_block(Shape::DottedSquareBracketed, Some(&head), |body| {
            for entry in &self.value {
                entry.textualize_in(body)?;
            }
            Ok(())
        })
    }
}

impl DatomRealizing for GroupMap {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        let Some(MapSelection::Entries) = Self::select(block.shape, block.head()) else {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        };
        let entries = scope.realize_body(&mut |child_scope, child| {
            GroupMapEntry::realize_block(child_scope, child)
        })?;
        let mut map = BTreeMap::new();
        for entry in entries {
            if map.insert(entry.key, entry.value).is_some() {
                return Err(DatomFault {
                    problem: DatomProblem::Position,
                });
            }
        }
        Ok(Self { value: map })
    }
}

impl DatomTextualizing for GroupMap {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        for key in self.value.keys() {
            key.group_key()?;
        }
        let map = Head("Map".into());
        scope.textualize_block(Shape::DottedSquareBracketed, Some(&map), |body| {
            for (key, value) in &self.value {
                GroupMapEntry {
                    key: key.clone(),
                    value: value.clone(),
                }
                .textualize_in(body)?;
            }
            Ok(())
        })
    }
}

impl DatomRealizing for TextMapEntry {
    fn realize_block(_scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        match block.shape {
            Shape::Bare => {
                let (key, value) = block.divide()?;
                key.text_key(&value)?;
                Ok(Self { key, value })
            }
            Shape::DottedParenthesized | Shape::DottedCurlyQuoted => {
                let Some(head) = block.head() else {
                    return Err(DatomFault {
                        problem: DatomProblem::Head,
                    });
                };
                let key = head.text();
                let Some(carrier) = &block.string_carrier else {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                };
                let value = Text(carrier.text());
                key.text_key(&value)?;
                Ok(Self { key, value })
            }
            _ => Err(DatomFault {
                problem: DatomProblem::AmbiguousMapPair,
            }),
        }
    }
}

impl DatomTextualizing for TextMapEntry {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        self.key.text_key(&self.value)?;
        if self.value.fits_bare() {
            return scope.textualize_block(Shape::Bare, None, |body| {
                body.emit_scalar(&format!("{}.{}", self.key, self.value.0));
                Ok(())
            });
        }
        let head = Head(self.key.clone());
        scope.textualize_block(Shape::DottedParenthesized, Some(&head), |body| {
            body.emit_scalar(&self.value.parenthesized_body());
            Ok(())
        })
    }
}

impl DatomRealizing for TextMap {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        let Some(MapSelection::Entries) = Self::select(block.shape, block.head()) else {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        };
        let entries = scope.realize_body(&mut |child_scope, child| {
            TextMapEntry::realize_block(child_scope, child)
        })?;
        let mut map = BTreeMap::new();
        for entry in entries {
            if map.insert(entry.key, entry.value).is_some() {
                return Err(DatomFault {
                    problem: DatomProblem::Position,
                });
            }
        }
        Ok(Self { value: map })
    }
}

impl DatomTextualizing for TextMap {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        for (key, value) in &self.value {
            key.text_key(value)?;
        }
        let map = Head("Map".into());
        scope.textualize_block(Shape::DottedSquareBracketed, Some(&map), |body| {
            for (key, value) in &self.value {
                TextMapEntry {
                    key: key.clone(),
                    value: value.clone(),
                }
                .textualize_in(body)?;
            }
            Ok(())
        })
    }
}

impl GroupPayloading for Group {
    fn realize_group_payload(scope: &mut RealizeScope<'_>) -> Result<Self, DatomFault> {
        let mut position = RecordPosition { ordinal: 0 };
        let mut title = None;
        let mut children = None;
        let mut annotations = None;
        scope.realize_body(&mut |child_scope, child| {
            match position.next() {
                0 => title = Some(Text::realize_block(child_scope, child)?),
                1 => children = Some(EntryVector::realize_block(child_scope, child)?.value),
                2 => annotations = Some(TextMap::realize_block(child_scope, child)?.value),
                _ => {
                    return Err(DatomFault {
                        problem: DatomProblem::ExtraPosition,
                    });
                }
            }
            Ok(())
        })?;
        Ok(Self {
            title: title.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
            children: children.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
            annotations: annotations.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
        })
    }

    fn textualize_group_payload(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        self.title.textualize_in(scope)?;
        EntryVector {
            value: self.children.clone(),
        }
        .textualize_in(scope)?;
        TextMap {
            value: self.annotations.clone(),
        }
        .textualize_in(scope)
    }
}

impl DatomRealizing for Entry {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        match Self::select(block.shape, block.head()).ok_or(DatomFault {
            problem: DatomProblem::Shape,
        })? {
            EntrySelection::Note => {
                let Some(carrier) = &block.string_carrier else {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                };
                Ok(Self::Note(Text(carrier.text())))
            }
            EntrySelection::BareNote => {
                let (head, text) = block.divide()?;
                if head != "Note" {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                }
                Ok(Self::Note(text))
            }
            EntrySelection::Group => {
                let Some(GroupSelection::Payload) = Group::select(Shape::Braced, None) else {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                };
                Ok(Self::Group(Group::realize_group_payload(scope)?))
            }
            EntrySelection::Tags => {
                let Some(TagListSelection::Payload) = TagList::select(Shape::SquareBracketed, None)
                else {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                };
                Ok(Self::Tags(TagList::realize_tag_payload(scope)?))
            }
        }
    }
}

impl DatomTextualizing for Entry {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        match self {
            Self::Note(text) => {
                if text.fits_bare() {
                    return scope.textualize_block(Shape::Bare, None, |body| {
                        body.emit_scalar(&format!("Note.{}", text.0));
                        Ok(())
                    });
                }
                let head = Head("Note".into());
                scope.textualize_block(Shape::DottedParenthesized, Some(&head), |body| {
                    body.emit_scalar(&text.parenthesized_body());
                    Ok(())
                })
            }
            Self::Group(group) => {
                let head = Head("Group".into());
                scope.textualize_block(Shape::DottedBraced, Some(&head), |body| {
                    let Some(GroupSelection::Payload) = Group::select(Shape::Braced, None) else {
                        return Err(DatomFault {
                            problem: DatomProblem::Shape,
                        });
                    };
                    group.textualize_group_payload(body)
                })
            }
            Self::Tags(tags) => {
                let head = Head("Tags".into());
                scope.textualize_block(Shape::DottedSquareBracketed, Some(&head), |body| {
                    let Some(TagListSelection::Payload) =
                        TagList::select(Shape::SquareBracketed, None)
                    else {
                        return Err(DatomFault {
                            problem: DatomProblem::Shape,
                        });
                    };
                    tags.textualize_tag_payload(body)
                })
            }
        }
    }
}

impl DatomRealizing for OptionalText {
    fn realize_block(_scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        match Self::select(block.shape, block.head()).ok_or(DatomFault {
            problem: DatomProblem::Shape,
        })? {
            OptionalSelection::Bare if block.body.0 == "None" => Ok(Self { value: None }),
            OptionalSelection::Bare => {
                let (head, value) = block.divide()?;
                if head != "Some" {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                }
                head.text_key(&value)?;
                Ok(Self { value: Some(value) })
            }
            OptionalSelection::Some => {
                let Some(carrier) = &block.string_carrier else {
                    return Err(DatomFault {
                        problem: DatomProblem::Shape,
                    });
                };
                Ok(Self {
                    value: Some(Text(carrier.text())),
                })
            }
        }
    }
}

impl DatomTextualizing for OptionalText {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        match &self.value {
            None => scope.textualize_block(Shape::Bare, None, |body| {
                body.emit_scalar("None");
                Ok(())
            }),
            Some(text) => {
                if text.fits_bare() {
                    "Some".to_owned().text_key(text)?;
                    return scope.textualize_block(Shape::Bare, None, |body| {
                        body.emit_scalar(&format!("Some.{}", text.0));
                        Ok(())
                    });
                }
                let head = Head("Some".into());
                scope.textualize_block(Shape::DottedParenthesized, Some(&head), |body| {
                    body.emit_scalar(&text.parenthesized_body());
                    Ok(())
                })
            }
        }
    }
}

impl DatomRealizing for Report {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        if Self::select(block.shape, block.head()).is_none() {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        }
        let mut position = RecordPosition { ordinal: 0 };
        let mut heading = None;
        let mut groups = None;
        let mut latest = None;
        scope.realize_body(&mut |child_scope, child| {
            match position.next() {
                0 => heading = Some(Text::realize_block(child_scope, child)?),
                1 => groups = Some(GroupMap::realize_block(child_scope, child)?.value),
                2 => latest = Some(OptionalText::realize_block(child_scope, child)?.value),
                _ => {
                    return Err(DatomFault {
                        problem: DatomProblem::ExtraPosition,
                    });
                }
            }
            Ok(())
        })?;
        Ok(Self {
            heading: heading.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
            groups: groups.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
            latest: latest.ok_or(DatomFault {
                problem: DatomProblem::MissingPosition,
            })?,
        })
    }
}

impl DatomTextualizing for Report {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        self.heading.textualize_in(scope)?;
        GroupMap {
            value: self.groups.clone(),
        }
        .textualize_in(scope)?;
        OptionalText {
            value: self.latest.clone(),
        }
        .textualize_in(scope)
    }
}

impl DatomRealizing for InterimNote {
    fn realize_block(scope: &mut RealizeScope<'_>, block: &Block) -> Result<Self, DatomFault> {
        if Self::select(block.shape, block.head()).is_none() {
            return Err(DatomFault {
                problem: DatomProblem::Shape,
            });
        }
        let mut position = RecordPosition { ordinal: 0 };
        let mut fields = Vec::new();
        scope.realize_body(&mut |child_scope, child| {
            if position.next() >= 4 {
                return Err(DatomFault {
                    problem: DatomProblem::ExtraPosition,
                });
            }
            fields.push(Text::realize_block(child_scope, child)?.0);
            Ok(())
        })?;
        if fields.len() != 4 {
            return Err(DatomFault {
                problem: DatomProblem::MissingPosition,
            });
        }
        Ok(Self {
            a: fields.remove(0),
            b: fields.remove(0),
            c: fields.remove(0),
            d: fields.remove(0),
        })
    }
}

impl DatomTextualizing for InterimNote {
    fn textualize_in(&self, scope: &mut TextualizeScope<'_>) -> Result<(), DatomFault> {
        for value in [&self.a, &self.b, &self.c, &self.d] {
            Text(value.clone()).textualize_in(scope)?;
        }
        Ok(())
    }
}

impl EvidencedRealizing for ReportText {
    type Value = Report;

    fn realize_evidenced(&self) -> Result<Realized<Self::Value>, DatomFault> {
        let mut walk = RealizeWalk::default();
        let mut values = walk.realize_source(&self.source, |scope, block| {
            Report::realize_block(scope, block)
        })?;
        if values.len() != 1 {
            return Err(DatomFault {
                problem: DatomProblem::Position,
            });
        }
        Ok(Realized {
            value: values.remove(0),
            evidence: DatomEvidence {
                observation: walk.observation(),
                cursor: walk.cursor(),
            },
        })
    }
}

impl EvidencedRealizing for InterimNoteText {
    type Value = InterimNote;

    fn realize_evidenced(&self) -> Result<Realized<Self::Value>, DatomFault> {
        let mut walk = RealizeWalk::default();
        let mut values = walk.realize_source(&self.source, |scope, block| {
            InterimNote::realize_block(scope, block)
        })?;
        if values.len() != 1 {
            return Err(DatomFault {
                problem: DatomProblem::Position,
            });
        }
        Ok(Realized {
            value: values.remove(0),
            evidence: DatomEvidence {
                observation: walk.observation(),
                cursor: walk.cursor(),
            },
        })
    }
}

impl EvidencedTextualizing for Report {
    type Text = ReportText;

    fn textualize_evidenced(&self) -> Result<Projected<Self::Text>, DatomFault> {
        let mut walk = TextualizeWalk::default();
        let head = Head("Report".into());
        let result: Result<(), DatomFault> = walk.textualize_source(|scope| {
            scope.textualize_block(Shape::DottedBraced, Some(&head), |body| {
                self.textualize_in(body)
            })
        });
        result.map(|()| Projected {
            text: ReportText {
                source: walk.textual_source(),
            },
            evidence: DatomEvidence {
                observation: walk.observation(),
                cursor: walk.cursor(),
            },
        })
    }
}

impl EvidencedTextualizing for InterimNote {
    type Text = InterimNoteText;

    fn textualize_evidenced(&self) -> Result<Projected<Self::Text>, DatomFault> {
        let mut walk = TextualizeWalk::default();
        let head = Head("InterimNote".into());
        let result: Result<(), DatomFault> = walk.textualize_source(|scope| {
            scope.textualize_block(Shape::DottedBraced, Some(&head), |body| {
                self.textualize_in(body)
            })
        });
        result.map(|()| Projected {
            text: InterimNoteText {
                source: walk.textual_source(),
            },
            evidence: DatomEvidence {
                observation: walk.observation(),
                cursor: walk.cursor(),
            },
        })
    }
}

impl Realize for ReportText {
    type Real = Report;
    type Fault = DatomFault;

    fn realize(&self) -> Result<Self::Real, Self::Fault> {
        self.realize_evidenced().map(|realized| realized.value)
    }
}

impl Realize for InterimNoteText {
    type Real = InterimNote;
    type Fault = DatomFault;

    fn realize(&self) -> Result<Self::Real, Self::Fault> {
        self.realize_evidenced().map(|realized| realized.value)
    }
}

impl Textualize for Report {
    type Textual = Result<ReportText, DatomFault>;

    fn textualize(&self) -> Self::Textual {
        self.textualize_evidenced().map(|projected| projected.text)
    }
}

impl Textualize for InterimNote {
    type Textual = Result<InterimNoteText, DatomFault>;

    fn textualize(&self) -> Self::Textual {
        self.textualize_evidenced().map(|projected| projected.text)
    }
}
