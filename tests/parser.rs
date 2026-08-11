use datom::{Document, StructureHeader, StructureShape};

/// Source spans propagate through nested blocks: a deeply nested block's
/// span points at its exact region of the source by line and column.
#[test]
fn source_spans_propagate_through_nested_blocks() {
    let source = r#"(Record
  [Entry])"#;
    let document = Document::parse(source).expect("datom parses");
    let outer = document.root_object_at(0).expect("outer parenthesis");

    let outer_span = outer.source_span();
    assert_eq!(outer_span.start.line, 1);
    assert_eq!(outer_span.start.column, 1);
    assert_eq!(outer_span.start.byte_offset, 0);
    assert_eq!(outer_span.end.line, 2);
    assert_eq!(outer_span.end.column, 11);

    let inner = outer.root_object_at(1).expect("inner square-bracket");
    let inner_span = inner.source_span();
    assert_eq!(
        inner_span.start.line, 2,
        "nested block span tracks line through the newline",
    );
    assert_eq!(
        inner_span.start.column, 3,
        "nested block span tracks column at the opening delimiter",
    );
    assert_eq!(inner.reemit(document.source()), "[Entry]");
}

/// Dot-application is right-associative: `A.B.C` is `App(A, App(B, C))`.
#[test]
fn dot_application_is_right_associative() {
    let document = Document::parse("A.B.C").expect("datom parses");
    let root = document.root_object_at(0).expect("root application");

    assert!(root.is_application());
    let (head, payload) = root.as_application().expect("root is application");
    assert_eq!(head.demote_to_string(), Some("A"));
    assert!(payload.is_application());
    let (inner_head, inner_payload) = payload.as_application().expect("payload is application");
    assert_eq!(inner_head.demote_to_string(), Some("B"));
    assert_eq!(inner_payload.demote_to_string(), Some("C"));
}

/// Comments starting with `;;` are skipped during parsing.
#[test]
fn comments_are_skipped() {
    let source = r#";; this is a comment
alpha
;; another comment
beta"#;
    let document = Document::parse(source).expect("datom parses");
    assert_eq!(document.holds_root_objects(), 2);
    assert_eq!(
        document
            .root_object_at(0)
            .expect("first")
            .demote_to_string(),
        Some("alpha")
    );
    assert_eq!(
        document
            .root_object_at(1)
            .expect("second")
            .demote_to_string(),
        Some("beta")
    );
}

/// Structure header captures the first two levels of delimiter shape.
#[test]
fn structure_header_captures_first_two_levels() {
    let source = r#"(Input ((Record Entry) Drop))
(Output (Accepted))"#;
    let document = Document::parse(source).expect("datom parses");
    let header = document.structure_header();
    let observed: Vec<(StructureShape, u8)> = header
        .slots()
        .iter()
        .map(|slot| (slot.shape(), slot.child_count()))
        .collect();

    assert_eq!(
        observed,
        vec![
            (StructureShape::Document, 2),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Atom, 0),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Atom, 0),
            (StructureShape::Parenthesis, 1),
        ],
    );

    assert_eq!(
        StructureHeader::from_packed_word(header.packed_word()),
        header,
        "the header is a stable 64-bit structural triage word",
    );
}
