# Diagnostics

Diagnostics identify the compiler stage and, when available, a one-based line and character column.

The CLI renders the source line and a caret for the primary location. Semantic analysis reports multiple independent name and type errors in a single pass. Lexing and parsing still stop at their first error because parser recovery is not implemented yet.

Columns currently count Unicode scalar values as processed by Rust's `char` iterator, not UTF-8 bytes or terminal display cells. Tabs and wide Unicode glyphs may therefore not visually align perfectly in the terminal.
