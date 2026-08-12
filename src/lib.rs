//! Ruled Datom serialization and deserialization.
//!
//! Datom is positional typed data.  This crate has no Rust-generation role;
//! Rust generation belongs to Ethos.  Parenthesized Meaning is recognized as a
//! reserved shape and rejected until its vocabulary is ruled.

mod codec;
mod expectation;
mod parser;
mod pretty;

pub use codec::{
    ContextName, DatomDecode, DatomEncode, DatomRecord, DatomSource, DatomText, DecodeWalk,
    EncodeWalk, WalkEvent,
};
pub use expectation::{ProtosShape, ShapeDefined, ShapeProbe};
pub use parser::DatomError;
