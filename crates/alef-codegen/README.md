# alef-codegen

Shared codegen utilities for the alef polyglot binding generator.

alef-codegen provides the common code generation infrastructure that language backends build upon. It includes the `TypeMapper` trait for translating IR types to language-specific type strings, struct/enum/function/method generators, From/Into conversion generators for binding-to-core and core-to-binding directions, and a Rust file builder for assembling generated source files. The crate also provides naming convention utilities (snake_case, camelCase, PascalCase) via the `heck` crate and template rendering via `minijinja`.

Backends use these shared generators to produce consistent binding code without duplicating the logic for iterating over API surface definitions, generating match arms for enums, or building conversion functions.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
