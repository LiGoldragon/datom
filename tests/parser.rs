use std::collections::BTreeMap;

use datom::{DatomError, DatomSource, ProtosShape};

fn assert_meaning_is_reserved(source: &str) {
    assert_eq!(
        DatomSource::new(source).decode::<String>(),
        Err(DatomError::ShapeNotYetRuled {
            shape: ProtosShape::ParenthesisDelimited,
        }),
        "{source:?} must be recognized only as unruled Meaning"
    );
}

#[test]
fn all_unruled_meaning_content_is_rejected_before_it_can_be_parsed() {
    assert_meaning_is_reserved("()");
    assert_meaning_is_reserved("(plain words)");
    assert_meaning_is_reserved("(Emphasis.{nested [content]})");
    assert_eq!(
        DatomSource::new("(vector-shaped content)").decode::<Vec<String>>(),
        Err(DatomError::ShapeNotYetRuled {
            shape: ProtosShape::ParenthesisDelimited,
        })
    );
    assert_eq!(
        DatomSource::new("Map.(key.value)").decode::<BTreeMap<String, String>>(),
        Err(DatomError::ShapeNotYetRuled {
            shape: ProtosShape::ParenthesisDelimited,
        })
    );
}

#[test]
fn pipe_text_parenthesis_strings_and_angles_are_not_compatibility_syntax() {
    assert_eq!(
        DatomSource::new("|old pipe text|").decode::<String>(),
        Err(DatomError::PipeTextIsNotDatom)
    );
    assert_eq!(
        DatomSource::new("(|old pipe text|)").decode::<String>(),
        Err(DatomError::PipeTextIsNotDatom)
    );
    assert_meaning_is_reserved("(old parenthesis string)");
    assert_eq!(
        DatomSource::new("<String>").decode::<String>(),
        Err(DatomError::AnglesAreNotDatom)
    );
    assert_eq!(
        DatomSource::new("Value.<String>").decode::<String>(),
        Err(DatomError::AnglesAreNotDatom)
    );
}
