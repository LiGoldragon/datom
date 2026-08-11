use std::collections::BTreeMap;

use datom::{DatomEncode, DatomSource};

#[test]
fn round_trip_string() {
    let original = "hello world".to_owned();
    let encoded = original.to_datom();
    assert_eq!(encoded, "(hello world)");
    let decoded = DatomSource::new(&encoded)
        .parse::<String>()
        .expect("string decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_bare_string() {
    let original = "symbol".to_owned();
    let encoded = original.to_datom();
    assert_eq!(encoded, "symbol");
    let decoded = DatomSource::new(&encoded)
        .parse::<String>()
        .expect("bare string decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_integer_u64() {
    let original = 42_u64;
    let encoded = original.to_datom();
    assert_eq!(encoded, "42");
    let decoded = DatomSource::new(&encoded)
        .parse::<u64>()
        .expect("u64 decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_integer_negative() {
    let original = -7_i64;
    let encoded = original.to_datom();
    assert_eq!(encoded, "-7");
    let decoded = DatomSource::new(&encoded)
        .parse::<i64>()
        .expect("i64 decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_bool_true() {
    let original = true;
    let encoded = original.to_datom();
    assert_eq!(encoded, "True");
    let decoded = DatomSource::new(&encoded)
        .parse::<bool>()
        .expect("bool decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_bool_false() {
    let original = false;
    let encoded = original.to_datom();
    assert_eq!(encoded, "False");
    let decoded = DatomSource::new(&encoded)
        .parse::<bool>()
        .expect("bool decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_vec_of_strings() {
    let original = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    let encoded = original.to_datom();
    assert_eq!(encoded, "[a b c]");
    let decoded = DatomSource::new(&encoded)
        .parse::<Vec<String>>()
        .expect("vec of strings decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_nested_vec() {
    let original = vec![vec!["a".to_owned()], vec!["b".to_owned()]];
    let encoded = original.to_datom();
    assert_eq!(encoded, "[[a] [b]]");
    let decoded = DatomSource::new(&encoded)
        .parse::<Vec<Vec<String>>>()
        .expect("nested vec decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_option_some() {
    let original = Some(42_u64);
    let encoded = original.to_datom();
    assert_eq!(encoded, "Some.42");
    let decoded = DatomSource::new(&encoded)
        .parse::<Option<u64>>()
        .expect("option some decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_option_none() {
    let original: Option<u64> = None;
    let encoded = original.to_datom();
    assert_eq!(encoded, "None");
    let decoded = DatomSource::new(&encoded)
        .parse::<Option<u64>>()
        .expect("option none decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_btreemap() {
    let mut original = BTreeMap::new();
    original.insert("alpha".to_owned(), 1_u64);
    original.insert("beta".to_owned(), 2_u64);
    let encoded = original.to_datom();
    assert_eq!(encoded, "Map.(alpha.1 beta.2)");
    let decoded = DatomSource::new(&encoded)
        .parse::<BTreeMap<String, u64>>()
        .expect("btreemap decodes");
    assert_eq!(decoded, original);
}

#[test]
fn round_trip_float() {
    let original = -122.3_f64;
    let encoded = original.to_datom();
    assert_eq!(encoded, "-122.3");
    let decoded = DatomSource::new(&encoded)
        .parse::<f64>()
        .expect("float decodes");
    assert_eq!(decoded, original);
}
