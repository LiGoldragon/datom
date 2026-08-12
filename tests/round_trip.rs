use std::collections::BTreeMap;
use std::fmt::Debug;

use datom::{
    ContextName, DatomDecode, DatomEncode, DatomError, DatomRecord, DatomSource, DatomText,
    DecodeWalk, EncodeWalk, ProtosShape, ShapeDefined, ShapeProbe, WalkEvent,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Text(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextSelection {
    Legacy,
}

impl ShapeDefined for Text {
    type Selection = TextSelection;

    fn shapes() -> &'static [ProtosShape] {
        &[ProtosShape::CurlyQuoteDelimited]
    }

    fn select(met: ShapeProbe<'_>) -> Result<Self::Selection, DatomError> {
        match met.shape() {
            ProtosShape::CurlyQuoteDelimited => Ok(TextSelection::Legacy),
            found => Err(DatomError::UnexpectedShape {
                expected: "a legacy Text shape",
                found,
            }),
        }
    }
}

impl DatomDecode for Text {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        match Self::select(walk.probe()?)? {
            TextSelection::Legacy => walk.value::<String>().map(Self),
        }
    }
}

impl DatomEncode for Text {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.legacy_string(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TagList(Vec<Text>);

impl DatomDecode for TagList {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        walk.named_vector("Tags", |walk| {
            let mut tags = Vec::new();
            while !walk.at_terminator(ProtosShape::SquareBracketDelimited) {
                tags.push(walk.value::<Text>()?);
            }
            Ok(Self(tags))
        })
    }
}

impl DatomEncode for TagList {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.named_vector("Tags", |walk| {
            for tag in &self.0 {
                walk.value(tag)?;
            }
            Ok(())
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Group {
    title: Text,
    children: Vec<Entry>,
    annotations: BTreeMap<String, Text>,
}

impl DatomRecord for Group {
    const NAME: &'static str = "Group";

    fn read_fields(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        Ok(Self {
            title: walk.value::<Text>()?,
            children: walk.value::<Vec<Entry>>()?,
            annotations: walk.value::<BTreeMap<String, Text>>()?,
        })
    }

    fn write_fields(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.value(&self.title)?;
        walk.value(&self.children)?;
        walk.value(&self.annotations)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Entry {
    Note(Text),
    Group(Group),
    Tags(TagList),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntrySelection {
    Note,
    Group,
    Tags,
}

impl ShapeDefined for Entry {
    type Selection = EntrySelection;

    fn shapes() -> &'static [ProtosShape] {
        &[
            ProtosShape::CurlyQuoteDelimited,
            ProtosShape::DotBraced,
            ProtosShape::DotBracketed,
        ]
    }

    fn select(met: ShapeProbe<'_>) -> Result<Self::Selection, DatomError> {
        match (met.shape(), met.head()) {
            (ProtosShape::CurlyQuoteDelimited, _) => Ok(EntrySelection::Note),
            (ProtosShape::DotBraced, Some("Group")) => Ok(EntrySelection::Group),
            (ProtosShape::DotBracketed, Some("Tags")) => Ok(EntrySelection::Tags),
            (found, _) => Err(DatomError::UnexpectedShape {
                expected: "a ruled Entry shape",
                found,
            }),
        }
    }
}

impl DatomDecode for Entry {
    fn decode(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        match Self::select(walk.probe()?)? {
            EntrySelection::Note => walk.value::<Text>().map(Self::Note),
            EntrySelection::Group => walk.value::<Group>().map(Self::Group),
            EntrySelection::Tags => walk.value::<TagList>().map(Self::Tags),
        }
    }
}

impl DatomEncode for Entry {
    fn encode(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        match self {
            Self::Note(text) => text.encode(walk),
            Self::Group(group) => group.encode(walk),
            Self::Tags(tags) => tags.encode(walk),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Report {
    heading: Text,
    groups: BTreeMap<String, Vec<Entry>>,
    latest: Option<Text>,
}

impl DatomRecord for Report {
    const NAME: &'static str = "Report";

    fn read_fields(walk: &mut DecodeWalk<'_>) -> Result<Self, DatomError> {
        Ok(Self {
            heading: walk.value::<Text>()?,
            groups: walk.value::<BTreeMap<String, Vec<Entry>>>()?,
            latest: walk.value::<Option<Text>>()?,
        })
    }

    fn write_fields(&self, walk: &mut EncodeWalk) -> Result<(), DatomError> {
        walk.value(&self.heading)?;
        walk.value(&self.groups)?;
        walk.value(&self.latest)
    }
}

fn fixture() -> Report {
    let deep_group = Entry::Group(Group {
        title: Text("Deep } ] “ quote".to_owned()),
        children: vec![Entry::Note(Text("tail".to_owned()))],
        annotations: BTreeMap::from([(
            "remark".to_owned(),
            Text("child sees } ] only as text".to_owned()),
        )]),
    });
    let group = Entry::Group(Group {
        title: Text("Ops".to_owned()),
        children: vec![Entry::Note(Text("sub note".to_owned())), deep_group],
        annotations: BTreeMap::from([("kind".to_owned(), Text("core".to_owned()))]),
    });
    Report {
        heading: Text("Q3".to_owned()),
        groups: BTreeMap::from([(
            "north".to_owned(),
            vec![
                Entry::Note(Text("quick note".to_owned())),
                group,
                Entry::Tags(TagList(vec![
                    Text("alpha".to_owned()),
                    Text("beta".to_owned()),
                ])),
            ],
        )]),
        latest: Some(Text("inside } ] “ current context".to_owned())),
    }
}

fn witness<T>(fixture: T)
where
    T: DatomDecode + DatomEncode + Clone + Debug + PartialEq,
{
    let encoded = DatomText::encode(&fixture).expect("fixture writes");
    let decoded = DatomSource::new(encoded.as_str())
        .decode::<T>()
        .expect("fixture reads");
    assert_eq!(
        decoded, fixture,
        "read(write(value)) preserves the typed value"
    );
    assert_eq!(
        DatomText::encode(&decoded)
            .expect("decoded fixture rewrites")
            .as_str(),
        encoded.as_str(),
        "a fixture has a canonical textual round-trip witness"
    );
}

fn has_resume(events: &[WalkEvent], context: ContextName, position: usize) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            WalkEvent::Resume {
                context: observed,
                position: observed_position,
            } if *observed == context && *observed_position == position
        )
    })
}

#[test]
fn ruled_primitives_collections_variants_and_dotted_floats_have_witnesses() {
    witness("bare".to_owned());
    witness("spaces and } ] “ are legacy text".to_owned());
    witness(42_i64);
    witness(-7_i32);
    witness(true);
    witness(false);
    witness(-1.25_f64);
    witness(vec!["alpha".to_owned(), "beta gamma".to_owned()]);
    witness(Some(Some("value".to_owned())));
    witness::<Option<String>>(None);
    witness(BTreeMap::from([
        ("one".to_owned(), 1.5_f64),
        ("minus".to_owned(), -2.25_f64),
    ]));

    assert_eq!(
        DatomText::encode(&Some(Some("value".to_owned())))
            .unwrap()
            .as_str(),
        "Some.Some.value"
    );
    assert_eq!(DatomText::encode(&1.0_f64).unwrap().as_str(), "1.0");
}

#[test]
fn legacy_strings_nest_escape_and_strip_common_indentation() {
    let source = "“\n    first\n      second \\“quoted\\”\n    } ]\n”";
    let decoded = DatomSource::new(source).decode::<String>().unwrap();
    assert_eq!(decoded, "first\n  second “quoted”\n} ]");
    witness(decoded);

    let nested = DatomSource::new("“outer “inner” done”")
        .decode::<String>()
        .unwrap();
    assert_eq!(nested, "outer “inner” done");
    witness(nested);
}

#[test]
fn comments_are_trivia_only_outside_the_current_child_context() {
    let values = DatomSource::new("[alpha ;; skip this line\n beta]")
        .decode::<Vec<String>>()
        .unwrap();
    assert_eq!(values, vec!["alpha", "beta"]);
    witness(values);
}

#[test]
fn deep_shape_defined_fixture_round_trips_and_witnesses_resumption() {
    let report = fixture();
    witness(report.clone());

    let text = DatomText::encode(&report).unwrap();
    assert!(text.as_str().contains("\\“"));
    assert!(text.as_str().contains("Map.[north.["));
    assert!(text.as_str().contains("Group.{"));
    assert!(text.as_str().contains("Tags.["));

    let (decoded, events) = DatomSource::new(text.as_str())
        .decode_with_trace::<Report>()
        .unwrap();
    assert_eq!(decoded, report);

    for (context, position) in [
        (ContextName::Record("Report"), 1),
        (ContextName::Map, 1),
        (ContextName::Vector, 1),
        (ContextName::Record("Group"), 1),
        (ContextName::Vector, 1),
    ] {
        assert!(
            has_resume(&events, context, position),
            "the driver resumes {context:?} at its following position"
        );
    }
}
