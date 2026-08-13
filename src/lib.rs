//! Datom is pure positional typed data on the Protos structural substrate.
//!
//! Its types supply context; Protos owns lexical blocks, structural shapes,
//! and the one scoped walk lifecycle. Datom does not model Meaning.

mod datom;

pub use datom::{
    DatomEvidence, DatomFault, DatomProblem, Entry, EvidenceObserving, EvidencedRealizing,
    EvidencedTextualizing, Group, InterimNote, InterimNoteText, Projected, ProjectionViewing,
    RealizationViewing, Realized, Report, ReportText, TagList, Text,
};
