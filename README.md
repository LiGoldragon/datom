# datom

Datom is serialization and deserialization only. It does not generate Rust;
that belongs to Ethos.

Datom reads expected Rust types directly and writes the same positional walk in
reverse. Text contains no field names or self-describing tags.

## Ruled surface

- Bare symbols, integers, dotted floats, `True`, `False`, `None`, and
  single-payload variants such as `Some.value`.
- Curly legacy strings: `“text”`. They nest, use ruled backslash escapes, and
  strip common indentation from multiline source text.
- Vectors: `[value value]`; brace records: `Type.{field-position …}`; maps:
  `Map.[key.value key.value]`.
- Dots are glued and right-associative. `;;` comments run to the line end.

Parentheses are reserved for the future Meaning shape. The codec recognizes
`(` and rejects it with a shape-not-yet-ruled error; it does not model Meaning
or its annotation vocabulary. Pipe text, parenthesis strings, angles, and
Datom generics are not supported syntax.
