# alef-backend-ffi

C FFI backend for alef.

This crate generates a complete C-compatible FFI layer for a Rust library, producing `#[no_mangle] extern "C"` functions, opaque handle types, and all supporting infrastructure. Generated code includes allocate/free pairs for every type, field accessors, JSON serialization/deserialization functions, enum-to-integer conversions, a thread-local error reporting mechanism, a Tokio runtime for async operations, and a `build.rs` with `cbindgen.toml` for automatic C header generation.

The backend optionally generates a visitor callback system with a `#[repr(C)]` callback struct covering up to 42 trait methods, enabling host languages to implement Rust trait behavior via C function pointers. This FFI layer serves as the foundation for the Go, Java, and C# backends, which generate wrapper code that calls through these C bindings.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
