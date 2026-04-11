# alef-extract

Rust source extraction for alef.

alef-extract parses Rust source files using `syn` and extracts the public API surface into the alef IR (`ApiSurface`). Starting from root source files (typically `lib.rs`), it recursively resolves `mod` declarations to walk the full module tree. It handles `pub use` re-exports (including cross-crate workspace re-exports), `#[cfg]`-gated fields, doc comments, serde `rename_all` attributes, newtype resolution, `#[derive]` trait detection (Clone, Default, thiserror), and default value extraction for struct fields.

The extractor produces a complete `ApiSurface` containing struct definitions, enum definitions, error enums, free functions, and impl block methods -- everything downstream backends need to generate language bindings.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
