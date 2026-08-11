# datom

Datom text serialization and deserialization.

No Rust code generation lives here — that belongs to Ethos.

## Syntax overview

Datom is a data notation: strictly typed, super dense, no field names. Values
are positional and the expected type is known at every position.

- **Atoms** — bare tokens: `symbol`, `42`, `-7`, `True`, `False`
- **Delimited blocks** — parentheses `( ... )`, square brackets `[ ... ]`,
  braces `{ ... }`
- **Dot-application** — right-associative structural binding: `Head.Payload`,
  `Map.(alpha.1 beta.2)`, `Some.42`
- **Pipe text** — literal strings: `(|text with (parens) and spaces|)`
- **Comments** — `;;` to end of line

Strings use the least delimiter that carries their content faithfully: bare
atoms for simple tokens, parentheses for space-separated words, pipe text for
everything else.

## Scope

This crate is the serialization and deserialization codec only. It owns Datom
value shapes for Rust values (strings, integers, floats, booleans, vectors,
maps, options). No schema semantics live in this crate.
