//! Canonical legacy-string projection.

/// Provisional placement note: `CanonicalText` is a data-bearing helper for
/// the still-open canonicality-tier placement fork.  It currently keeps the
/// bare-versus-curly decision beside Datom's text context rather than claiming
/// that the eventual shared Protos mechanism owns it.
pub(crate) struct CanonicalText<'text> {
    text: &'text str,
}

impl<'text> CanonicalText<'text> {
    pub(crate) fn new(text: &'text str) -> Self {
        Self { text }
    }

    pub(crate) fn is_bare(&self) -> bool {
        !self.text.is_empty()
            && self.text.chars().all(|character| {
                !character.is_whitespace()
                    && !matches!(
                        character,
                        '[' | ']'
                            | '{'
                            | '}'
                            | '('
                            | ')'
                            | '“'
                            | '”'
                            | '|'
                            | ';'
                            | '<'
                            | '>'
                            | '.'
                            | '\\'
                    )
            })
    }

    pub(crate) fn push_legacy_escaped(&self, output: &mut String) {
        output.push('“');
        for character in self.text.chars() {
            match character {
                '\\' => output.push_str("\\\\"),
                '“' => output.push_str("\\“"),
                '”' => output.push_str("\\”"),
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                _ => output.push(character),
            }
        }
        output.push('”');
    }
}
