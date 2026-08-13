# datom

Datom is pure positional typed data on the published Protos substrate. It
does not generate Rust; that belongs to Ethos. Protos owns lexical blocks,
shape vocabulary, string carriers, and the only structural walk. Datom adds
only context-sensitive typed realization and textual projection.

## Current typed surface

- `Report`, `Entry`, `Group`, `TagList`, `Text`, and `InterimNote` are the
  concrete positional models used by the registered design examples.
- `ReportText` and `InterimNoteText` are concrete typed textual carriers. They
  implement Protos `Realize`; their matching real types implement Protos
  `Textualize`.
- `EvidencedRealizing` and `EvidencedTextualizing` return those same typed
  values/texts paired with read-only transition evidence copied from the
  actual Protos driver; dialect code never owns or proxies `Walk`.
- Records read and write field positions only. Variants carry their Head;
  their payload is headless in its own context.
- Strings project bare exactly when the Protos scanner can carry the complete
  value as one bare block; otherwise they use parenthesis. Curly quotes are
  accepted as the legacy input carrier. Parenthesis projection preserves a
  trailing backslash, literal escaped parentheses, and balanced nested pairs.
  Meaning remains deferred to
  `structuredStringType.md` and bead `primary-xqb.8.5`.
- `Map.[north.[…]]` is a keyed vector entry with one structural Protos frame.
  Plain `Map.[kind.core]` is an unambiguous bare pair. Keys containing dots,
  and delimited keys followed by `.`, are deliberately unsupported pending a
  psyche ruling; they return a Datom fault rather than changing Protos.

Canonical text is a block projection, not preservation of original whitespace
or the legacy curly carrier spelling.
