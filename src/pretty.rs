//! Deterministic readability projection for Datom documents.
//!
//! The canonical Datom encoders emit a document on a single line. That form is
//! byte-stable and quotation-safe, but a large document is hard for a person to
//! read. [`PrettyLayout`] is an additive projection that reflows the same
//! document across indented lines without changing the canonical encoding.
//!
//! The projection is a pure reformatting. Every leaf — an atom or a pipe-text
//! block — re-emits the exact source bytes it was parsed from, and the layout
//! only adds whitespace between structural delimiters. Datom treats whitespace
//! as an object separator that is skipped adjacent to delimiters, so pretty
//! output always re-parses to the same block tree as its source. The layout
//! therefore never touches atom content, string escaping, or delimiter shape;
//! it decides only where line breaks and indentation belong.

use crate::{Block, DatomError, Delimiter, Document};

/// How a component CLI writes a canonical Datom reply to its output.
///
/// Every Datom-printing CLI shares one print shape: it encodes a typed reply to
/// a canonical single line and writes it. `DatomOutputForm` is the one place that
/// decides whether that line is written as-is or reflowed for reading, so each
/// print site stays a single call and the pretty decision is not re-derived at
/// every site. The default `Canonical` form is byte-identical to the bare
/// encoder output every consumer and golden depends on; `Pretty`, selected when
/// the caller passed `--pretty`, reflows the same document through
/// [`PrettyLayout`] and re-parses to the identical document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatomOutputForm {
    Canonical,
    Pretty,
}

impl DatomOutputForm {
    /// The output form a CLI should use given whether `--pretty` was requested.
    pub fn from_pretty_requested(pretty_requested: bool) -> Self {
        if pretty_requested {
            Self::Pretty
        } else {
            Self::Canonical
        }
    }

    /// Project an already-encoded canonical Datom line into the chosen form.
    ///
    /// A canonical encoder line always re-parses, so the pretty projection
    /// cannot fail in practice; if a caller ever passes text that does not
    /// parse, the canonical text is returned unchanged rather than lost.
    pub fn render(&self, canonical: &str) -> String {
        match self {
            Self::Canonical => canonical.to_owned(),
            Self::Pretty => PrettyLayout::standard()
                .render_datom(canonical)
                .unwrap_or_else(|_| canonical.to_owned()),
        }
    }
}

/// A deterministic readability layout for Datom documents.
///
/// The layout carries the two policy values that drive every break decision:
/// the target `line_width` a single-line form must fit within, and the
/// `indent_unit` number of spaces added per delimiter depth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrettyLayout {
    line_width: usize,
    indent_unit: usize,
}

impl Default for PrettyLayout {
    fn default() -> Self {
        Self::standard()
    }
}

impl PrettyLayout {
    /// The standard layout: an eighty-column target with two-space indentation.
    pub const fn standard() -> Self {
        Self {
            line_width: 80,
            indent_unit: 2,
        }
    }

    /// A layout with an explicit line-width target and indentation step.
    pub const fn new(line_width: usize, indent_unit: usize) -> Self {
        Self {
            line_width,
            indent_unit,
        }
    }

    /// Parse `source` as a Datom document and render its readable projection.
    ///
    /// This is the entry point for a CLI that holds an already-encoded Datom
    /// string and wants the same document reflowed for reading.
    pub fn render_datom(&self, source: &str) -> Result<String, DatomError> {
        let document = Document::parse(source)?;
        Ok(self.render_document(&document))
    }

    /// Render the readable projection of an already-parsed document.
    pub fn render_document(&self, document: &Document) -> String {
        let source = document.source();
        let mut rendered = String::new();
        for (index, root_object) in document.root_objects().iter().enumerate() {
            if index > 0 {
                rendered.push('\n');
            }
            self.render_block(root_object, source, 0, &mut rendered);
        }
        rendered
    }

    /// Render one block, assuming it begins at column `depth * indent_unit`.
    ///
    /// A block whose single-line form fits the remaining width is emitted
    /// inline. Otherwise a delimited block breaks: its opening delimiter, its
    /// children reflowed across indented lines, and its closing delimiter
    /// returned to the block's own indentation. A block with no children carries
    /// nothing to break and is always inline.
    fn render_block(&self, block: &Block, source: &str, depth: usize, rendered: &mut String) {
        let inline = block.render_inline(source);
        match block {
            Block::Delimited {
                delimiter,
                root_objects,
                ..
            } if !root_objects.is_empty() && !self.fits(&inline, depth) => {
                self.render_broken(*delimiter, root_objects, source, depth, rendered);
            }
            // A dot-application that overflows breaks inside its payload: the
            // head chain stays glued to the period, and the payload — normally
            // the delimited block that made the line long — reflows at this
            // same depth. `Public.Newtype.( ... )` keeps `Public.Newtype.(` on
            // one line and reflows the body.
            Block::Application { head, payload, .. } if !self.fits(&inline, depth) => {
                rendered.push_str(&head.render_inline(source));
                rendered.push('.');
                self.render_block(payload, source, depth, rendered);
            }
            Block::Delimited { .. }
            | Block::Application { .. }
            | Block::PipeText(_)
            | Block::Atom(_) => {
                rendered.push_str(&inline);
            }
        }
    }

    /// Render a delimited block across lines: opening delimiter, then the
    /// children reflowed one indent step deeper, then the closing delimiter back
    /// at the block's own indentation.
    fn render_broken(
        &self,
        delimiter: Delimiter,
        children: &[Block],
        source: &str,
        depth: usize,
        rendered: &mut String,
    ) {
        rendered.push_str(delimiter.opening_text());
        let mut previous: Option<&Block> = None;
        for child in children {
            let glued = previous.is_some_and(|previous| previous.immediately_precedes(child));
            if !glued {
                rendered.push('\n');
                self.push_indentation(depth + 1, rendered);
            }
            self.render_block(child, source, depth + 1, rendered);
            previous = Some(child);
        }
        rendered.push('\n');
        self.push_indentation(depth, rendered);
        rendered.push_str(delimiter.closing_text());
    }

    /// Whether `inline` fits the line width when placed at `depth` indentation.
    fn fits(&self, inline: &str, depth: usize) -> bool {
        depth * self.indent_unit + inline.chars().count() <= self.line_width
    }

    fn push_indentation(&self, depth: usize, rendered: &mut String) {
        for _ in 0..depth * self.indent_unit {
            rendered.push(' ');
        }
    }
}

impl Block {
    /// The canonical single-line form of this block's subtree.
    ///
    /// Leaves re-emit their exact source bytes, so atom content and pipe-text
    /// escaping are preserved verbatim; a delimited block wraps its children
    /// joined by single spaces. This is both the width measure the readability
    /// layout consults and the text it emits when a block fits on one line.
    pub fn render_inline(&self, source: &str) -> String {
        match self {
            Self::Delimited {
                delimiter,
                root_objects,
                ..
            } => {
                let mut rendered = String::from(delimiter.opening_text());
                let mut previous: Option<&Block> = None;
                for child in root_objects {
                    if previous.is_some_and(|previous| !previous.immediately_precedes(child)) {
                        rendered.push(' ');
                    }
                    rendered.push_str(&child.render_inline(source));
                    previous = Some(child);
                }
                rendered.push_str(delimiter.closing_text());
                rendered
            }
            Self::Application { head, payload, .. } => {
                format!(
                    "{}.{}",
                    head.render_inline(source),
                    payload.render_inline(source)
                )
            }
            Self::PipeText(_) | Self::Atom(_) => self.reemit(source).to_owned(),
        }
    }

    /// Whether this block ends exactly where `next` begins, with no source text
    /// between them. Source-adjacent siblings carry no separator in canonical
    /// Datom — a dotted-prefix head `key.` glued to its value block, or a
    /// capitalized application head `Head.` glued to its argument group — so the
    /// readability layout keeps them on one line without inventing a space.
    fn immediately_precedes(&self, next: &Block) -> bool {
        self.source_span().end.byte_offset == next.source_span().start.byte_offset
    }
}
