# alef-core

Core types, config schema, and backend trait for the alef polyglot binding generator.

alef-core defines the foundational types that all other alef crates depend on. It provides the intermediate representation (IR) used to describe a Rust crate's public API surface, including `ApiSurface`, `TypeDef`, `FunctionDef`, `EnumDef`, `ErrorDef`, `FieldDef`, `MethodDef`, and `TypeRef`. The IR is serializable via serde and serves as the exchange format between the extraction and code generation stages.

The crate also defines the `Backend` trait that all language backends implement, the `AlefConfig` schema for `alef.toml` configuration files, the `Language` enum covering all supported target languages, and the `GeneratedFile` / `Capabilities` types used to describe backend output. Configuration covers per-language options, output paths, scaffold metadata, adapter patterns, exclusion rules, and feature propagation.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
