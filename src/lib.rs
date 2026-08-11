//! Structural Datom reader and codec.
//!
//! This crate recognizes Datom delimiters, spans, atoms, and block structure.
//! Its codec owns Datom value shapes for Rust values. Serialization and
//! deserialization only — no Rust code generation (that belongs to Ethos).

mod codec;
mod expectation;
mod parser;
mod pretty;

pub use codec::{
    DatomBlock, DatomBody, DatomBodyDecode, DatomBodyEncode, DatomBodyEncoding, DatomCollection,
    DatomDecode, DatomDecodeError, DatomDocumentBody, DatomDocumentDecode, DatomDocumentEncode,
    DatomDocumentEncoding, DatomEncode, DatomSource, DatomString,
};
pub use expectation::{DottedEntry, DottedExpectation};
pub use parser::{
    Atom, AtomClassification, Block, DatomError, Delimiter, Document, ParseMode, PipeText,
    SourcePosition, SourceSpan, StructureHeader, StructureShape, StructureSlot,
};
pub use pretty::{DatomOutputForm, PrettyLayout};
