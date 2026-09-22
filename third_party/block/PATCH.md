# Local patch

This directory contains `block` 0.1.6 from
<https://github.com/SSheldon/rust-block>, licensed under MIT.

The source changes make the opaque Objective-C `Class` type inhabited and
spell out the `C` ABI that older Rust editions selected by default. Rust
reports an extern static whose type is an empty enum as future incompatible,
and plans to reject it. The runtime symbol is used only by address, so a
private byte preserves the pointer representation without reading the symbol's
storage. Explicit `C` annotations preserve the crate's existing ABI while
avoiding deprecation warnings when this path dependency is compiled locally.

Remove this patch when GPUI Kit's resolved dependency graph no longer includes
`block` 0.1, or when `block` publishes the equivalent fix.
