# alef-backend-magnus

Ruby (Magnus) backend for alef.

This crate generates Rust source code that uses the [Magnus](https://github.com/matsadler/magnus) framework to expose a Rust library as a native Ruby extension (C extension gem). It produces Ruby class definitions with constructors, instance methods, and static methods, using `magnus::function!` and `magnus::method!` macros. Enum types are mapped to Ruby constants, and types that conflict with Magnus internals (such as `Error`) are automatically renamed.

The backend supports multiple DTO generation styles: `struct`, `dry-struct`, and `data`. It also generates `.rbs` type signature files for Ruby's static type checking ecosystem (Steep, Sorbet).

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
